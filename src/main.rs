#![forbid(unsafe_code)]

use std::process::{Command, Stdio};

use anyhow::Result;
use clap::Parser;
use mpvipc::*;

use roux::util::{FeedOption, TimePeriod};

#[derive(Debug, clap::Parser)]
#[clap(about, author, version)]
struct Args {
    /// Type of feed to browse
    #[clap(arg_enum)]
    feed_type: FeedType,

    /// Subreddit to consume
    #[clap()]
    resource: String,

    /// Criterium to sort by
    ///
    /// Ignore if consuming a user feed
    #[clap(arg_enum, default_value = "hot")]
    sort_by: SortBy,

    /// When looking at "top", what time period to consider
    #[clap(arg_enum)]
    period: Option<Period>,

    /// Fetch new submissions when playlist contains less than this
    #[clap(short, long, default_value = "20")]
    min_buffer_size: usize,

    /// Number of elements to fetch
    #[clap(short = 'i', long, default_value = "20")]
    buffer_increase: usize,

    /// User agent to pass to mpv
    #[clap(long)]
    user_agent: Option<String>,
}

#[derive(Debug, clap::ArgEnum, Clone, Copy)]
pub enum FeedType {
    Sub,
    User,
}

#[derive(Debug, clap::ArgEnum, Clone, Copy)]
pub enum SortBy {
    Hot,
    Top,
    Latest,
    Rising,
}

#[derive(Debug, clap::ArgEnum, Clone, Copy)]
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

impl Args {
    async fn request(
        &self,
        limit: u32,
        after: Option<String>,
    ) -> Result<(Vec<String>, Option<String>)> {
        let mut options = FeedOption::new().limit(limit);
        options.after = after;

        options.period = self.period.map(|p| p.into());

        let options = Some(options);
        let result = match (self.feed_type, self.sort_by) {
            (FeedType::Sub, SortBy::Hot) => {
                roux::subreddit::Subreddit::new(&self.resource)
                    .hot(limit, options)
                    .await
            }

            (FeedType::Sub, SortBy::Latest) => {
                roux::subreddit::Subreddit::new(&self.resource)
                    .latest(limit, options)
                    .await
            }

            (FeedType::Sub, SortBy::Rising) => {
                roux::subreddit::Subreddit::new(&self.resource)
                    .rising(limit, options)
                    .await
            }
            (FeedType::Sub, SortBy::Top) => {
                roux::subreddit::Subreddit::new(&self.resource)
                    .top(limit, options)
                    .await
            }
            (FeedType::User, _) => {
                roux::user::User::new(&self.resource)
                    .submitted(options)
                    .await
            }
        }?;

        let after = result.data.after.clone();
        let url_vec = result
            .data
            .children
            .iter()
            .filter_map(|c| c.data.url.as_ref())
            .cloned()
            .collect();
        Ok((url_vec, after))
    }
}

fn hash<T: std::hash::Hash>(hashable: T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hashable.hash(&mut hasher);
    std::hash::Hasher::finish(&hasher)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let ipc_socket = "/tmp/mpvsocket";

    // prepare mpv
    let _mpv_process = Command::new("mpv")
        .arg("--idle=yes")
        .arg("--force-window")
        .arg(
            args.user_agent
                .as_ref()
                .map(|ua| format!("--user-agent={ua}"))
                .unwrap_or(String::new()),
        )
        .arg(format!("--input-ipc-server={ipc_socket}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("failed to execute child");

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let mut mpv = Mpv::connect("/tmp/mpvsocket")?;

    let mut after = None;
    let (new, new_after) = args
        .request(args.min_buffer_size as u32, after.clone())
        .await?;
    after = new_after;

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
                let (new, new_after) = args
                    .request(args.buffer_increase as u32, after.clone())
                    .await?;
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
        }
    }
}
