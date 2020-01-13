extern crate clap;
extern crate reqwest;
extern crate serde_json;

use clap::{App, Arg};

mod reddit;
use reddit::{RedditRequest, SortBy, Timeslot};

fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("resource")
                .value_name("RESOURCE")
                .help("Sets which reddit reosurce to consume. For example r/gif")
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

    let resource = matches.value_of("resource").unwrap().to_string();
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

    let mut rr: RedditRequest = reddit::reddit_request(&resource, sort_by);

    println!("{}", rr.get_url());

    rr.play();
}
