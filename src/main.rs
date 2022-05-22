#![forbid(unsafe_code)]

use std::process::{Command, Stdio};

use clap::Parser;
use mpvipc::*;
use roux::util::{FeedOption, TimePeriod};

//use reddit::{RedditRequest, SortBy, Timeslot};

#[derive(Debug, clap::Parser)]
#[clap(about, author, version)]
struct Opts {
    #[clap()]
    subreddit: String,

    #[clap(subcommand)]
    sort_by: SortBy,
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

impl Into<TimePeriod> for Period {
    fn into(self) -> TimePeriod {
        match self {
            Period::Hour => TimePeriod::Now,
            Period::Day => TimePeriod::Today,
            Period::Week => TimePeriod::ThisWeek,
            Period::Month => TimePeriod::ThisMonth,
            Period::Year => TimePeriod::ThisYear,
            Period::All => TimePeriod::AllTime,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // io::Result<()> {
    let opts = Opts::parse();

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

    let sub = roux::subreddit::Subreddit::new(&opts.subreddit);
    let limit = 50;
    let options = match opts.sort_by {
        SortBy::Top(p) => Some(FeedOption::new().period(p.into())),
        _ => None,
    };

    let submissions = match opts.sort_by {
        SortBy::Hot => sub.hot(limit, options).await,
        SortBy::Top(_) => sub.top(limit, options).await,
        SortBy::New => sub.latest(limit, options).await,
        SortBy::Rising => sub.rising(limit, options).await,
    }?;

    //
    std::thread::sleep(std::time::Duration::from_secs(1));
    let mpv = Mpv::connect("/tmp/mpvsocket")?;

    for url in submissions
        .data
        .children
        .iter()
        .map(|c| c.data.url.as_ref())
        .filter_map(std::convert::identity)
    {
        println!("adding {url}");
        mpv.playlist_add(
            &url,
            PlaylistAddTypeOptions::File,
            PlaylistAddOptions::Append,
        )?;
    }

    mpv.playlist_play_id(0)?;

    Ok(())
}
