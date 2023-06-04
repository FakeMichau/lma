use super::Details;
use serde::{Serialize, Deserialize};
use std::time::{self, Duration, Instant};
use std::thread;

pub struct MAL {
    token: String,
    requests: Vec<Request>,
}

struct Request {
    timestamp: Instant,
}

impl MAL {
    pub fn new(token: &str) -> Self {
        Self { 
            token: "".to_owned(),
            requests: Vec::new(),
        }
    }
    pub fn set_token(&mut self, token: &str) {
        self.token = token.to_owned();
    }
    fn time_to_next_request(&self) -> Duration {
        const PER_SEC: usize = 3;
        const PER_MINUTE: usize = 60;
        let now = Instant::now();
        let in_the_last_second = self
            .requests
            .iter()
            .filter(|request| {
               request.timestamp.duration_since(now).as_millis() < 1000
            })
            .count();
        let oldest_violation_second = if in_the_last_second >= PER_SEC {
            Some(self.requests.get(self.requests.len() - PER_SEC + 1).expect("Limit shouldn't be set at 1").timestamp)
        } else {
            None
        };

        let in_the_last_minute = self
            .requests
            .iter()
            .filter(|timestamp| {
                timestamp.timestamp.duration_since(now).as_secs() < 60
            })
            .count();
        let oldest_violation_minute = if in_the_last_minute >= PER_MINUTE {
            Some(self.requests.get(self.requests.len() - PER_MINUTE + 1).expect("Limit shouldn't be set at 1").timestamp)
        } else {
            None
        };

        if let Some(timestamp) = oldest_violation_second {
            timestamp.duration_since(now)
        } else if let Some(timestamp) = oldest_violation_minute {
            timestamp.duration_since(now)
        } else {
            time::Duration::from_millis(0)
        }
    }
    fn wait_for_next_request(&mut self) {
        thread::sleep(self.time_to_next_request());
        self.requests.push(Request{ timestamp: Instant::now()})
    }
}

impl Details for MAL {
    fn get_title_list(&mut self, potential_title: &str) -> Vec<String> {
        self.wait_for_next_request();
        let mal_entires = search_anime(potential_title);
        let res = mal_entires
            .iter()
            .map(|entry| {
                entry.title.clone().unwrap_or_else(|| {
                    entry.title_japanese.clone().unwrap_or_else(|| {
                        entry.title_english.clone().unwrap_or_default()
                    })
                })
            })
            .collect();
        res
    }
}

// slow down the requests
fn search_anime(name: &str) -> Vec<MALEntry> {
    let url = format!("https://api.jikan.moe/v4/anime?q={}&limit=10", name);
    let api_result = reqwest::blocking::get(url);
    match api_result {
        Ok(api_response) => {
            match api_response.status() {
                reqwest::StatusCode::OK => {
                    api_response.json::<List>().expect("Issue parsing json").data
                },
                _ => {
                    print!("Non-OK response from the server: {:?}", api_response);
                    Vec::new()
                },
            }
        },
        Err(why) => {
            print!("Error getting a response: {}", why);
            Vec::new()
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct List {
    data: Vec<MALEntry>
}

#[derive(Serialize, Deserialize, Debug)]
struct MALEntry {
    #[serde(rename = "mal_id")]
    id: u32,
    title: Option<String>,
    title_english: Option<String>,
    title_japanese: Option<String>,
    episodes: Option<u64>,
}