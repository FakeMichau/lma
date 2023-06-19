use std::path::PathBuf;
use async_trait::async_trait;
use time::OffsetDateTime;

use lib_mal::{
    prelude::{
        fields::AnimeFields,
        options::{Status, StatusUpdate},
        ListStatus, EpisodeNode,
    },
    ClientBuilder, MALClient, MALError,
};

use crate::{ServiceTitle, Service};

pub struct MAL {
    client: MALClient,
    challenge: String,
    state: String,
    url: Option<String>,
}

#[async_trait]
impl Service for MAL {
    async fn new(cache_dir: PathBuf) -> Self {
        let token = "8f7bd7e31dcf4f931949fc0b418c76d8".to_string();
        let client = ClientBuilder::new()
            .secret(token)
            .caching(true)
            .cache_dir(Some(cache_dir))
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
    async fn login(&mut self) {
        let redirect_uri = "localhost:2525";
        self.client
            .auth(&redirect_uri, &self.challenge, &self.state)
            .await
            .expect("Unable to log in");
        self.client.need_auth = false; // should be in the library
        self.url = None;
    }
    async fn auth(&mut self) {
        self.url = if self.client.need_auth {
            let url;
            (url, self.challenge, self.state) = self.client.get_auth_parts();
            Some(url)
        } else {
            None
        }
    }
    async fn init_show(&mut self, id: u32) {
        match self.get_user_entry_details(id).await {
            Some(_existing_show) => { /* leave as is */ }
            None => {
                // add to plan to watch
                let mut update = StatusUpdate::new();
                update.status(Status::PlanToWatch);
                self.client
                    .update_user_anime_status(id, update)
                    .await
                    .expect("Update user's list"); // likely will fail
            }
        }
    }
    async fn search_title(&mut self, potential_title: &str) -> Vec<ServiceTitle> {
        // what does it do when it returns 0 results?
        // pad titles below 3 characters to avoid errors
        let mut padded_title = String::from(potential_title);
        while padded_title.len() < 3 {
            padded_title += r"\&nbsp";
        }
        self.client
            .get_anime_list(&padded_title, 20)
            .await
            .expect("MAL search result")
            .data
            .iter()
            .map(|entry| ServiceTitle {
                service_id: entry.node.id,
                title: entry.node.title.to_string(),
            })
            .collect()
    }
    async fn get_title(&mut self, id: u32) -> String {
        self.client
            .get_anime_details(id, AnimeFields::Title)
            .await
            .expect("Anime title") // likely will fail
            .show
            .title
    }
    async fn get_episode_count(&mut self, id: u32) -> Option<u32> {
        self.client
            .get_anime_details(id, AnimeFields::NumEpisodes)
            .await
            .expect("Anime episode count") // likely will fail
            .num_episodes
    }
    async fn set_progress(&mut self, id: u32, progress: u32) {
        let mut update = StatusUpdate::new();
        update.num_watched_episodes(progress);
        if progress == 0 {
            update.status(Status::PlanToWatch);
            update.start_date("");
        } else {
            update.status(Status::Watching);
        }
        let updated_status = self.update_status(id, update).await;

        let local_date = OffsetDateTime::now_utc().date();
        if let None = updated_status.start_date {
            if progress == 1 {
                let mut update = StatusUpdate::new();
                update.start_date(&format!("{}", local_date));
                self.update_status(id, update).await;
            }
        }
        let episode_count = self.client
            .get_anime_details(id, AnimeFields::NumEpisodes)
            .await
            .expect("Anime details") // likely will fail
            .num_episodes
            .map(|count| {
                if count == 0 {
                    u32::MAX
                } else {
                    count
                }
            })
            .unwrap_or(u32::MAX);
        if updated_status.num_episodes_watched.unwrap_or_default() >= episode_count {
            let mut update = StatusUpdate::new();
            update.status(Status::Completed);
            if let None = updated_status.finish_date {
                update.finish_date(&format!("{}", local_date));
            }
            self.update_status(id, update).await;
            // ask user for a score?
        }
    }
    fn is_logged_in(&self) -> bool {
        !self.client.need_auth
    }
    fn get_url(&self) -> Option<String> {
        self.url.clone()
    }

    // TEMP
    // remove lib_mal dep
    async fn get_episodes(&mut self, id: u32) -> Result<Vec<EpisodeNode>, MALError> {
        self.client
            .get_anime_episodes(id)
            .await
            .map(|episodes| {
                episodes.data
            })
    }

    // remove lib_mal dep
    async fn get_user_entry_details(&mut self, id: u32) -> Option<ListStatus> {
        self.client
            .get_anime_details(id, AnimeFields::MyListStatus)
            .await
            .expect("Anime details") // likely will fail
            .my_list_status
    }
}

impl MAL {
    // remove lib_mal dep
    async fn update_status(&mut self, id: u32, update: StatusUpdate) -> ListStatus {
        self.client
            .update_user_anime_status(id, update)
            .await
            .expect("Update user's list") // likely will fail
    }
}
