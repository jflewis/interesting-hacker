extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate regex;

use serde_json::{Value};
use regex::{RegexSetBuilder};

static HACKER_NEWS_URL: &'static str = "https://hacker-news.firebaseio.com/v0/topstories.json";

static KEYWORDS: [&'static str; 7] = ["Node", "Rust", "Web", "blockchain", "linux", "pizza", "microsoft"];

fn main() {
    let set = RegexSetBuilder::new(&KEYWORDS)
        .case_insensitive(true)
        .build()
        .unwrap();
    let client = reqwest::Client::new();

    let raw_text = client.get(HACKER_NEWS_URL).send().expect("Unable to get topstories");

    let json: Value = serde_json::from_reader(raw_text).expect("unable to parse topstories");

    let thing = json.as_array().unwrap();
    
    let top_stories = thing.into_iter().
        take(100).
        map(|item| {
            let story_url = format!("https://hacker-news.firebaseio.com/v0/item/{}.json", &item);
            serde_json::from_reader(client.get(&story_url).send().unwrap()).unwrap()
        }).
        filter(|story: &Value|{
            set.is_match(story["title"].as_str().unwrap_or(""))
        })
        .collect::<Vec<Value>>();

    for story in top_stories { 
        println!("story title: {}, comments: {}", story["title"], story["kids"].as_array().unwrap_or(&Vec::new()).len());
        println!("\tStory URL: {}", story["url"]);
        println!("----------------------------------", )
    }
}
