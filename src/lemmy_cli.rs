use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use lemmy_client::{
    lemmy_api_common::{
        lemmy_db_schema::{ListingType, SortType},
        lemmy_db_views::structs::PaginationCursor,
        person::GetPersonDetails,
        post::GetPosts,
        site::Search,
    },
    ClientOptions, LemmyClient,
};

use crate::{CommonCli, LinkSource};

pub struct LemmyLinkSource {
    page_cursor: Option<PaginationCursor>,
    page_index: Option<i64>,
    args: Args,
}

impl LemmyLinkSource {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            args: Args::parse(),
            page_cursor: None,
            page_index: None,
        }
    }
}

#[derive(Debug, clap::Parser)]
#[clap(about, author, version)]
pub struct Args {
    #[clap(flatten)]
    pub common_cli: CommonCli,

    /// Server to connect to (just the domain, no protocol)
    #[clap()]
    pub server: String,

    /// Whether to use HTTPS or not
    #[clap(short, long, default_value = "true")]
    pub secure: bool,

    /// Resource to consume
    #[clap(default_value = "")]
    pub resource: String,

    /// Criterium to sort by
    pub sort_by: Option<SortBy>,

    /// Whether to get only local content or also content from federate sites
    pub listing_from: Option<ListingFrom>,
}

#[derive(Debug, clap::ValueEnum, Clone, Copy, strum::AsRefStr)]
pub enum SortBy {
    Active,
    Hot,
    New,
    Old,
    TopDay,
    TopWeek,
    TopMonth,
    TopYear,
    TopAll,
    MostComments,
    NewComments,
    TopHour,
    TopSixHour,
    TopTwelveHour,
    TopThreeMonths,
    TopSixMonths,
    TopNineMonths,
    Controversial,
    Scaled,
}

#[derive(Debug, clap::ValueEnum, Clone, Copy, strum::AsRefStr)]
pub enum ListingFrom {
    All,
    Local,
    // Subscribed,
    // ModeratorView,
}

#[async_trait]
impl LinkSource for LemmyLinkSource {
    async fn request(&mut self) -> Result<Vec<String>> {
        let client = LemmyClient::new(ClientOptions {
            domain: self.args.server.clone(),
            secure: self.args.secure,
        });

        let limit = self.args.common_cli.buffer_increase as i64;
        let sort_by = self
            .args
            .sort_by
            .map(|s| SortType::try_from(s.as_ref()))
            .and_then(Result::ok);
        let listing_from = self
            .args
            .listing_from
            .map(|s| ListingType::try_from(s.as_ref()))
            .and_then(Result::ok);

        let posts = match &self.args.resource.get(0..2) {
            Some("c/") | Some("r/") | None => {
                let mut get_post_options = GetPosts {
                    limit: Some(limit),
                    page_cursor: self.page_cursor.clone(),
                    sort: sort_by,
                    ..Default::default()
                };

                if self.args.resource.len() > 2 {
                    get_post_options.community_name = Some(self.args.resource[2..].to_string());
                };
                let post_list = client.list_posts(get_post_options).await.unwrap();
                self.page_cursor = post_list.next_page;
                post_list.posts
            }
            Some("u/") => {
                let get_person_options = GetPersonDetails {
                    limit: Some(limit),
                    page: self.page_index,
                    ..Default::default()
                };

                let person_post_list = client.get_person(get_person_options).await.unwrap();
                *self.page_index.get_or_insert(0) += 1;
                person_post_list.posts
            }
            _ => {
                let search_options = Search {
                    limit: Some(limit),
                    q: self.args.resource.clone(),
                    page: self.page_index,
                    sort: sort_by,
                    listing_type: listing_from,
                    ..Default::default()
                };
                let search_results = client.search(search_options).await.unwrap();
                *self.page_index.get_or_insert(0) += 1;
                search_results.posts
            }
        };

        // let after = result.data.after.clone();
        let url_vec = posts
            .iter()
            .filter_map(|c| c.post.url.to_owned())
            .map(|db_url| db_url.to_string())
            .collect();
        Ok(url_vec)
    }

    fn get_common_cli(&self) -> CommonCli {
        self.args.common_cli.clone()
    }
}
