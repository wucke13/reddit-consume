#[macro_use]
extern crate clap;
extern crate reqwest;
extern crate serde_json;

use clap::{App, Arg};

mod reddit;
use reddit::{RedditRequest, SortBy, Timeslot};

fn main() {
    let matches = App::new("My Super Program")
        .version("1.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Does awesome things")
        .arg(
            Arg::with_name("subreddit")
                .value_name("SUBREDDIT")
                .help("Sets which subreddit to consume")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("sort-by")
                .short("s")
                .long("sort-by")
                .help("Sort by either hot, top, new, controversial or rising")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("timeslot")
                .short("t")
                .long("timeslot")
                .help("show posts from either the last hour, day, week, month, year or all")
                .takes_value(true),
        )
        .get_matches();

    let subreddit = matches.value_of("subreddit").unwrap().to_string();
    let sort_by = match matches.value_of("sort-by").unwrap_or("hot") {
        "hot" => SortBy::Hot,
        "top" => SortBy::Top(match matches.value_of("timeslot").unwrap_or("day") {
            "hour" => Timeslot::Hour,
            "day" => Timeslot::Day,
            "week" => Timeslot::Week,
            "month" => Timeslot::Month,
            "year" => Timeslot::Year,
            "all" => Timeslot::All,
            _ => panic!("unable to parse timeslot"),
        }),
        "new" => SortBy::New,
        "controversial" => {
            SortBy::Controversial(match matches.value_of("timeslot").unwrap_or("day") {
                "hour" => Timeslot::Hour,
                "day" => Timeslot::Day,
                "week" => Timeslot::Week,
                "month" => Timeslot::Month,
                "year" => Timeslot::Year,
                "all" => Timeslot::All,
                _ => panic!("unable to parse timeslot"),
            })
        }
        "rising" => SortBy::Rising,
        _ => panic!("unable to parse sort-by"),
    };

    let rr = RedditRequest {
        subreddit: subreddit,
        sort_by: sort_by,
        after: None,
    };

    rr.play();
}
