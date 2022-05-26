#![forbid(unsafe_code)]

use std::process::{Command, Stdio};

use clap::Parser;
use mpvipc::*;

use roux::util::{FeedOption, TimePeriod};

//use reddit::{RedditRequest, SortBy, Timeslot};

#[derive(Debug, clap::Parser)]
#[clap(about, author, version)]
struct Args {
    #[clap()]
    subreddit: String,

    #[clap(subcommand)]
    sort_by: SortBy,

    /// Minimum number of submissions in the playlist before fetching new elements
    #[clap(short, long, default_value = "20")]
    min_buffer_size: usize,

    /// Number of elements to fetch when buffer-size is reached
    #[clap(short = 'i', long, default_value = "20")]
    buffer_increase: usize,
}

#[derive(Debug, clap::Subcommand, Clone, Copy)]
pub enum SortBy {
    Hot,
    #[clap(subcommand)]
    Top(Period),
    New,
    Rising,
}

#[derive(Debug, clap::Subcommand, Clone, Copy)]
pub enum Period {
    Hour,
    Day,
    Week,
    Month,
    Year,
    All,
}

impl From<Period> for TimePeriod {
    fn from(p: Period) -> TimePeriod {
        match p {
            Period::Hour => TimePeriod::Now,
            Period::Day => TimePeriod::Today,
            Period::Week => TimePeriod::ThisWeek,
            Period::Month => TimePeriod::ThisMonth,
            Period::Year => TimePeriod::ThisYear,
            Period::All => TimePeriod::AllTime,
        }
    }
}

async fn request(
    subreddit: &str,
    sort_by: SortBy,
    limit: u32,
    after: Option<String>,
) -> anyhow::Result<(Vec<String>, Option<String>)> {
    let sub = roux::subreddit::Subreddit::new(subreddit);

    let mut options = FeedOption::new().limit(limit);
    options.after = after;

    if let SortBy::Top(p) = sort_by {
        options.period = Some(p.into());
    }

    let options = Some(options);
    let result = match sort_by {
        SortBy::Hot => sub.hot(limit, options).await,
        SortBy::Top(_) => sub.top(limit, options).await,
        SortBy::New => sub.latest(limit, options).await,
        SortBy::Rising => sub.rising(limit, options).await,
    }?;
    let after = result.data.after.clone();

    Ok((
        result
            .data
            .children
            .iter()
            .filter_map(|c| c.data.url.as_ref())
            .cloned()
            .collect(),
        after,
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Args::parse();

    let ipc_socket = "/tmp/mpvsocket";

    // prepare mpv
    let _mpv_process = Command::new("mpv")
        .arg("--idle=yes")
        .arg("--force-window")
        .arg(format!("--input-ipc-server={ipc_socket}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("failed to execute child");

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let mut mpv = Mpv::connect("/tmp/mpvsocket").unwrap();

    let mut after = None;
    let (new, new_after) = request(
        &opts.subreddit,
        opts.sort_by,
        opts.min_buffer_size as u32,
        after.clone(),
    )
    .await?;
    after = new_after;

    for url in new {
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
            if p_len - p_pos <= opts.min_buffer_size {
                println!("after = {after:?}");
                let (new, new_after) = request(
                    &opts.subreddit,
                    opts.sort_by,
                    opts.buffer_increase as u32,
                    after.clone(),
                )
                .await?;
                after = new_after;

                for url in new {
                    println!("adding {url}");
                    mpv.playlist_add(
                        &url,
                        PlaylistAddTypeOptions::File,
                        PlaylistAddOptions::Append,
                    )?;
                }
            }
        }
    }
}
