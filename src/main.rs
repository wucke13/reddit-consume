extern crate reqwest;
extern crate serde_json;

use std::process::Command;
use std::collections::HashMap;

use serde_json::{Deserializer ,Result, Value,};

pub enum SortBy{
    hot,
    top,
    new,
    controversial,
    rising,
}

fn reddit_url_builder (subreddit: &str, sort: Option<&str>, timeslot: Option<&str>)-> String{
    let mut result = format!("https://reddit.com/r/{}", subreddit);
    if sort.is_some() {
        result.push_str(&format!("/{}", sort.unwrap()));
    }
    result.push_str(".json");

    if timeslot.is_some() {
        result.push_str(&format!("?t={}", timeslot.unwrap()));
    }
    result
}

fn main() {
    let subreddit = "gif";
    let sort = "top";
    let timeslot = "all";
    let url = reddit_url_builder(subreddit,Some(sort),Some(timeslot));

    //let sort = [ "hot", "top", "new", "controversial", "rising" ];
    // if top, choose between t one of (hour, day, week, month, year, all)

    loop{
        let response = reqwest::get(&url).expect("unable to perform HTTP request")
            .text().expect("unable to retrieve body from HTTP response");
        let json : Value = serde_json::from_str(&response).expect("unable to parse JSON in HTTP response body");

        let posts : Vec<Value> = json["data"]["children"].as_array().unwrap().to_vec();
        let urls = posts.into_iter().map(|post| post["data"]["url"].as_str().unwrap());



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


        loop{}
    }
}
