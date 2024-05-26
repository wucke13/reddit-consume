#![forbid(unsafe_code)]

use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use anyhow::Result;
use async_trait::async_trait;
use mpvipc::*;

use tokio::time::sleep;

pub mod lemmy_cli;
pub mod reddit_cli;

#[async_trait]
pub(crate) trait LinkSource {
    async fn request(&mut self) -> Result<Vec<String>>;
    fn get_common_cli(&self) -> CommonCli;
}

#[derive(Clone, Debug, clap::Parser)]
pub struct CommonCli {
    /// Fetch new submissions when playlist contains less than this
    #[clap(short, long, default_value = "20")]
    pub min_buffer_size: usize,

    /// Number of elements to fetch
    #[clap(short = 'i', long, default_value = "20")]
    pub buffer_increase: usize,

    /// User agent to pass to mpv
    #[clap(long)]
    pub user_agent: Option<String>,
}

fn hash<T: std::hash::Hash>(hashable: T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hashable.hash(&mut hasher);
    std::hash::Hasher::finish(&hasher)
}

#[tokio::main]
async fn main() -> Result<()> {
    let arg0 = std::env::args().next().expect("args[0] not set");

    let mut link_source: Box<dyn LinkSource> = if arg0.ends_with("lemmy-consume") {
        Box::new(lemmy_cli::LemmyLinkSource::new())
    } else if arg0.ends_with("reddit-consume") {
        Box::new(reddit_cli::RedditLinkSource::new())
    } else {
        panic!("i have identity crisis")
    };

    let common_cli = link_source.get_common_cli().clone();

    let ipc_socket = "/tmp/mpvsocket";

    let new = link_source.request().await?;

    // prepare mpv
    let mut mpv_process = Command::new("mpv")
        .args([
            "null://",
            "--idle=yes",
            "--quiet",
            "--force-window",
            &format!("--input-ipc-server={ipc_socket}"),
            &common_cli
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
        for line in mpv_reader.lines().map_while(Result::ok) {
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
            if p_len - p_pos <= common_cli.min_buffer_size {
                match link_source.request().await {
                    Ok(new) => {
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
