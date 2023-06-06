use std::path::PathBuf;

use lib_mal::{ClientBuilder, MALClient};

use crate::ServiceTitle;

pub struct MAL {
    client: MALClient,
    challenge: String,
    state: String,
    url: Option<String>,
}

impl MAL {
    pub async fn new() -> Self {
        let token = "8f7bd7e31dcf4f931949fc0b418c76d8".to_string();
        let client = ClientBuilder::new()
            .secret(token)
            .caching(true)
            .cache_dir(Some(PathBuf::new()))
            .build_with_refresh()
            .await
            .unwrap();

        Self {
            client,
            challenge: String::new(),
            state: String::new(),
            url: Some(String::new()),
        }
    }

    pub async fn auth(&mut self) {
        self.url = if self.client.need_auth {
            let url;
            (url, self.challenge, self.state) = self.client.get_auth_parts();
            Some(url)
        } else {
            None
        }
    }

    pub fn get_url(&self) -> &Option<String> {
        &self.url
    }

    pub async fn login(&mut self) {
        let redirect_uri = "localhost:2525";
        self.client
            .auth(&redirect_uri, &self.challenge, &self.state)
            .await
            .expect("Unable to log in");
        self.client.need_auth = false; // should be in the library
        self.url = None;
    }

    pub async fn test(&self) {
        let anime = self.client.get_anime_details(80, None).await.unwrap();
        println!(
            "{}: started airing on {}, ended on {}, ranked #{}",
            anime.show.title,
            anime.start_date.unwrap(),
            anime.end_date.unwrap(),
            anime.rank.unwrap()
        );
    }

    pub async fn search_title(&mut self, potential_title: &str) -> Vec<ServiceTitle> {
        // what does it do when it returns 0 results?
        self
            .client
            .get_anime_list(potential_title, 20)
            .await
            .expect("MAL search result")
            .data
            .iter()
            .map(|entry| {
                ServiceTitle{ id: entry.node.id, title: entry.node.title.to_string() }
            })
            .collect()
    }
}
