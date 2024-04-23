#![forbid(unsafe_code)]

use clap::Parser;

use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use anyhow::Result;
use mpvipc::*;

use tokio::time::sleep;

pub mod reddit_cli;

pub(crate) trait LinkSource {
    async fn request(
        &self,
        limit: u32,
        after: Option<String>,
    ) -> Result<(Vec<String>, Option<String>)>;
}

fn hash<T: std::hash::Hash>(hashable: T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hashable.hash(&mut hasher);
    std::hash::Hasher::finish(&hasher)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = reddit_cli::Args::parse();

    let ipc_socket = "/tmp/mpvsocket";

    let mut after = None;
    let (new, new_after) = args
        .request(args.min_buffer_size as u32, after.clone())
        .await?;
    after = new_after;

    // prepare mpv
    let mut mpv_process = Command::new("mpv")
        .args([
            "null://",
            "--idle=yes",
            "--quiet",
            "--force-window",
            &format!("--input-ipc-server={ipc_socket}"),
            &args
                .user_agent
                .as_ref()
                .map(|ua| format!("--user-agent={ua}"))
                .unwrap_or_default(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute child");
    let stdout = mpv_process
        .stdout
        .take()
        .expect("unable to take mpv stdout");

    // spawn a thread for organized printing
    std::thread::spawn(move || {
        let mpv_reader = BufReader::new(stdout);
        for line in mpv_reader.lines().filter_map(Result::ok) {
            println!("mpv {line}");
        }
    });

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let mut mpv = Mpv::connect("/tmp/mpvsocket")?;

    let mut dedup_list = std::collections::HashSet::new();
    for url in new {
        if !dedup_list.insert(hash(&url)) {
            continue;
        }
        println!("adding {url}");
        mpv.playlist_add(
            &url,
            PlaylistAddTypeOptions::File,
            PlaylistAddOptions::Append,
        )?;
    }

    mpv.playlist_play_id(0)?;
    loop {
        if let Event::EndFile = mpv.event_listen()? {
            let p_len: usize = mpv.get_property("playlist-count")?;
            let p_pos: usize = mpv.get_property("playlist-pos")?;
            if p_len - p_pos <= args.min_buffer_size {
                println!("after = {after:?}");
                match args
                    .request(args.buffer_increase as u32, after.clone())
                    .await
                {
                    Ok((new, new_after)) => {
                        after = new_after;

                        for url in new {
                            if !dedup_list.insert(hash(&url)) {
                                continue;
                            }
                            println!("adding {url}");
                            mpv.playlist_add(
                                &url,
                                PlaylistAddTypeOptions::File,
                                PlaylistAddOptions::Append,
                            )?;
                        }
                    }
                    Err(e) => {
                        println!("an error occured: {e}, retrying in 5 seconds");
                        sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}
