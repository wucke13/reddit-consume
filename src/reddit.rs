use std::fmt::{self, Display};
use std::process::Command;

use ureq::SerdeValue;

#[derive(Debug)]
pub enum SortBy {
    Hot,
    Top(Timeslot),
    New,
    Controversial(Timeslot),
    Rising,
}

#[derive(Debug)]
pub enum Timeslot {
    Hour,
    Day,
    Week,
    Month,
    Year,
    All,
}

impl Display for SortBy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:?}", self);
        let until = s.find('(').unwrap_or(s.len());
        write!(f, "{}", &s.to_lowercase()[..until])
    }
}

impl Display for Timeslot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:?}", self);
        write!(f, "{}", &s.to_lowercase())
    }
}

pub struct RedditRequest {
    pub resource: String,
    pub sort_by: SortBy,
    pub after: Option<String>,
    posts: Vec<SerdeValue>,
}

impl RedditRequest {
    pub fn new(resource: &str, sort_by: SortBy) -> Self {
        Self {
            resource: resource.to_string(),
            sort_by: sort_by,
            after: None,
            posts: Vec::new(),
        }
    }

    pub fn get_url(&self) -> String {
        let mut result = format!(
            "https://reddit.com/{}/{}.json?limit=100",
            self.resource, self.sort_by
        );

        let timeslot = match &self.sort_by {
            SortBy::Top(timeslot) => Some(timeslot),
            SortBy::Controversial(timeslot) => Some(timeslot),
            _ => None,
        };

        if timeslot.is_some() {
            result.push_str(&format!("&t={}", timeslot.unwrap()));
        }

        result
    }

    pub fn fetch_posts(&mut self) {
        let response = ureq::get(&self.get_url()).call();
        let json = response.into_json().unwrap();

        self.posts = json["data"]["children"].as_array().unwrap().to_vec();
    }

    pub fn get_post_urls(&self) -> Option<Vec<String>> {
        let urls = self
            .posts
            .clone()
            .into_iter()
            .map(|post| format!("{}", post["data"]["url"].as_str().unwrap()));
        Some(urls.collect())
    }

    pub fn page_forward(&mut self) {
        self.after = Some(self.posts.last().unwrap()["data"]["name"].to_string());
    }

    pub fn play(&mut self) {
        self.fetch_posts();
        let urls = self.get_post_urls().unwrap();
        Command::new("mpv")
            .arg("--loop-file")
            .args(urls)
            .output()
            .expect("failed to run mpv");

        // json["data"]["children"].array.map!(j => j["data"]["url"].str).array;
        // after = json["data"]["children"].array.back["data"]["name"].str;

        // auto mpvArgs = ["mpv"];
        // if (loop)
        // {
        //     mpvArgs ~= "--loop-file";
        // }
        // execute(mpvArgs ~ urls);
    }
}
