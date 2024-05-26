use async_trait::async_trait;

use anyhow::{Context, Result};
use clap::Parser;
use roux::util::{FeedOption, TimePeriod};

use crate::{CommonCli, LinkSource};

pub struct RedditLinkSource {
    page: Option<String>,
    args: Args,
}

impl RedditLinkSource {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            page: None,
            args: Args::parse(),
        }
    }
}

#[derive(Debug, clap::Parser)]
#[clap(about, author, version)]
pub struct Args {
    #[clap(flatten)]
    common: CommonCli,

    /// Resource to consume
    #[clap()]
    pub resource: String,

    /// Criterium to sort by
    ///
    /// Ignore if consuming a user feed
    #[clap(default_value = "hot")]
    pub sort_by: SortBy,

    /// When looking at "top", what time period to consider
    pub period: Option<Period>,

    /// Include NSFW content
    #[clap(short, long)]
    pub nsfw: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum FeedType {
    Sub,
    User,
}

#[derive(Debug, clap::ValueEnum, Clone, Copy)]
pub enum SortBy {
    Hot,
    Top,
    Latest,
    Rising,
}

#[derive(Debug, clap::ValueEnum, Clone, Copy)]
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

#[async_trait]
impl LinkSource for RedditLinkSource {
    async fn request(&mut self) -> Result<Vec<String>> {
        let limit = self.args.common.buffer_increase as u32;
        let mut options = FeedOption::new().limit(limit);
        options.after = self.page.clone();

        options.period = self.args.period.map(|p| p.into());
        let feed_type = if self.args.resource.starts_with("u/") {
            FeedType::User
        } else if self.args.resource.starts_with("r/") {
            FeedType::Sub
        } else {
            println!("did not find that, here are some search results");
            let mut options = FeedOption::new();
            options.limit = Some(limit);

            let suggestions = roux::Subreddits::search(
                &format!(
                    "{}&include_over_18={}",
                    self.args.resource,
                    if self.args.nsfw { "on" } else { "off" }
                ),
                Some(limit),
                Some(options),
            )
            .await
            .context("unable to fetch suggestions")?;
            for s in suggestions.data.children.into_iter().map(|x| x.data) {
                println!(
                    "{} - {}",
                    s.display_name_prefixed.unwrap(),
                    s.title.unwrap()
                );
            }

            std::process::exit(0);
        };

        let options = Some(options);
        let resource = &self.args.resource.clone().split_off(2);
        let result = match (feed_type, self.args.sort_by) {
            (FeedType::Sub, SortBy::Hot) => {
                roux::subreddit::Subreddit::new(resource)
                    .hot(limit, options)
                    .await
            }

            (FeedType::Sub, SortBy::Latest) => {
                roux::subreddit::Subreddit::new(resource)
                    .latest(limit, options)
                    .await
            }

            (FeedType::Sub, SortBy::Rising) => {
                roux::subreddit::Subreddit::new(resource)
                    .rising(limit, options)
                    .await
            }
            (FeedType::Sub, SortBy::Top) => {
                roux::subreddit::Subreddit::new(resource)
                    .top(limit, options)
                    .await
            }
            (FeedType::User, _) => roux::user::User::new(resource).submitted(options).await,
        }
        .context("unable to create stream")?;

        self.page = result.data.after.clone();
        let url_vec = result
            .data
            .children
            .iter()
            .filter_map(|c| c.data.url.as_ref())
            .cloned()
            .collect();
        Ok(url_vec)
    }

    fn get_common_cli(&self) -> CommonCli {
        self.args.common.clone()
    }
}
