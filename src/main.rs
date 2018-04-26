extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate regex;
extern crate rayon;

use clap::{Arg, App};
use rayon::prelude::*;
use regex::{RegexSetBuilder};
use serde_json::{Value};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::fs::File;
use std::io::{BufReader, BufRead, Result,Write};

static HACKER_NEWS_URL: &'static str = "https://hacker-news.firebaseio.com/v0/topstories.json";

fn get_keyword_file(file_arg: Option<&str>) -> Result<File> {
    match file_arg {
        Some(file) => {
            println!("Choosing file from config option...", );

            File::open(file)
        },
        None => {
            println!("Using default file", );
            let mut default_keyword_location = std::env::home_dir().expect("Unable to open home dir path");
            default_keyword_location.push(".interestingHacker");

            let default_file = if default_keyword_location.exists() {
                File::open(default_keyword_location)
            } else {
                let mut new_file = File::create(".interestingHacker")?;
                new_file.write_all(b"*").expect("Unable to write to new file");
                Ok(new_file)
            };

           default_file
        }
    }
}

fn main() {
    let matches = App::new("Interesting Hacker")
                          .version("1.0")
                          .author("John Lewis <lewisjnerd@gmail.com>")
                          .about("Interesting Hacker fetches the top 200 HackerNews stories and filters them buy your keywords in ~/.interestingHacker")
                          .arg(Arg::with_name("config")
                               .short("c")
                               .long("config")
                               .value_name("FILE")
                               .help("Sets a custom keyword file")
                               .takes_value(true))
                          .get_matches();

    // Attempt to get user defined keyword file or use the default.
    // Just unwrap because ATM I don't care about panics
    let keyword_file = get_keyword_file(matches.value_of("config")).unwrap();

    // Open the File and read all the words out of it and build a RegexSet
    let buffered = BufReader::new(keyword_file);
    let mut keywords = Vec::new();
    for line in buffered.lines() {
        keywords.push(line.unwrap());
    }

    let set = RegexSetBuilder::new(&keywords)
        .case_insensitive(true)
        .build()
        .unwrap();

    // spawn thread to hand fetching stories and send matched stories acros the channel back to the main thread.
    // Move receiver into thread scope.
    let (sender, receiver): (Sender<Vec<Value>>, Receiver<Vec<Value>>) = mpsc::channel();
    thread::spawn(move || {
        let client = reqwest::Client::new();
        let raw_text = client.get(HACKER_NEWS_URL).send().expect("Unable to get topstories");

        let json: Value = serde_json::from_reader(raw_text).expect("unable to parse topstories");

        let story_ids = json.as_array().unwrap();
    
        let top_stories = story_ids.par_iter()
            // We really only care about the top 300 stories
            .take(300)
            .map(|item| {
                let story_url= format!("https://hacker-news.firebaseio.com/v0/item/{}.json", &item);
                client.get(&story_url).send().unwrap()
            })
            .map(|json|{
                serde_json::from_reader(json).unwrap()
            })
            .filter(|story: &Value|{
                set.is_match(story["title"].as_str().unwrap_or(""))
            })
            .collect::<Vec<Value>>();

            // send stories back to main thread.
            sender.send(top_stories).unwrap();
    });
    
    println!("Fetching Stories....");
    // Wait for channel to receive filtered stories
    let top_stories = receiver.recv().unwrap();

    // Print out your filtered stories.
    for story in top_stories { 
        println!("story title: {}, comments: {}", story["title"], story["kids"].as_array().unwrap_or(&Vec::new()).len());
        println!("\tStory URL: {}", story["url"]);
        println!("----------------------------------", )
    }
}
