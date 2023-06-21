use std::path::PathBuf;
use async_trait::async_trait;
use time::OffsetDateTime;
use lib_mal::prelude::fields::AnimeFields;
use lib_mal::prelude::options::{Status, StatusUpdate};
use lib_mal::prelude::ListStatus;
use lib_mal::{ClientBuilder, MALClient};
use crate::{ServiceTitle, Service, ServiceType, ServiceEpisodeUser, EpisodeStatus, ServiceEpisodeDetails};

pub struct MAL {
    client: MALClient,
    challenge: String,
    state: String,
    url: Option<String>,
}

#[async_trait]
impl Service for MAL {
    async fn new(cache_dir: PathBuf) -> Result<Self, String> {
        let token = "8f7bd7e31dcf4f931949fc0b418c76d8".to_string();
        let client = ClientBuilder::new()
            .secret(token)
            .caching(true)
            .cache_dir(Some(cache_dir))
            .build_with_refresh()
            .await
            .map_err(|e| e.to_string())?;

        Ok(Self {
            client,
            challenge: String::new(),
            state: String::new(),
            url: Some(String::new()),
        })
    }
    async fn login(&mut self) -> Result<(), String> {
        let redirect_uri = "localhost:2525";
        self.client
            .auth(redirect_uri, &self.challenge, &self.state)
            .await
            .expect("Unable to log in");
        self.client.need_auth = false; // should be in the library
        self.url = None;
        Ok(())
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
    async fn init_show(&mut self, id: u32) -> Result<(), String> {
        if self.get_user_entry_details(id).await?.is_none() {
            // add to plan to watch
            let mut update = StatusUpdate::new();
            update.status(Status::PlanToWatch);
            self.client
                .update_user_anime_status(id, update)
                .await
                .expect("Update user's list"); // likely will fail
        }
        Ok(())
    }
    async fn search_title(&mut self, potential_title: &str) -> Result<Vec<ServiceTitle>, String> {
        // what does it do when it returns 0 results?
        // pad titles below 3 characters to avoid errors
        let mut padded_title = String::from(potential_title);
        while padded_title.len() < 3 {
            padded_title += r"\&nbsp";
        }
        Ok(self
            .client
            .get_anime_list(&padded_title, 20)
            .await
            .expect("MAL search result")
            .data
            .iter()
            .map(|entry| ServiceTitle {
                service_id: entry.node.id,
                title: entry.node.title.to_string(),
            })
            .collect())
    }
    async fn get_title(&mut self, id: u32) -> Result<String, String> {
        Ok(self
            .client
            .get_anime_details(id, AnimeFields::Title)
            .await
            .expect("Anime title") // likely will fail
            .show
            .title)
    }
    async fn get_episode_count(&mut self, id: u32) -> Result<Option<u32>, String> {
        Ok(self.client
            .get_anime_details(id, AnimeFields::NumEpisodes)
            .await
            .expect("Anime episode count") // likely will fail
            .num_episodes)
    }
    async fn get_user_entry_details(&mut self, id: u32) -> Result<Option<ServiceEpisodeUser>, String> {
        Ok(self.client
            .get_anime_details(id, AnimeFields::MyListStatus)
            .await
            .expect("Anime details") // likely will fail
            .my_list_status
            .map(|episode_status| {
                ServiceEpisodeUser {
                    status: to_episode_status(episode_status.status),
                    progress: episode_status.num_episodes_watched,
                    score: episode_status.score,
                    is_rewatching: episode_status.is_rewatching,
                    rewatch_count: episode_status.num_times_rewatched,
                    updated_at: episode_status.updated_at,
                    start_date: episode_status.start_date,
                    finish_date: episode_status.finish_date,
                    comments: episode_status.comments,
                }
            }))
    }
    async fn get_episodes(&mut self, id: u32) -> Result<Vec<ServiceEpisodeDetails>, String> {
        Ok(self.client
            .get_anime_episodes(id)
            .await // todo: extract mal error
            .map(|episodes| episodes.data)
            .map(|vec| {
                vec.into_iter()
                .map(|episode| {
                    ServiceEpisodeDetails {
                        number: episode.mal_id,
                        title: episode.title,
                        title_japanese: episode.title_japanese,
                        title_romanji: episode.title_romanji,
                        duration: episode.duration,
                        aired: episode.aired,
                        filler: episode.filler,
                        recap: episode.recap,
                    }
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default())
    }
    async fn set_progress(&mut self, id: u32, progress: u32) -> Result<(), String> {
        let mut update = StatusUpdate::new();
        update.num_watched_episodes(progress);
        if progress == 0 {
            update.status(Status::PlanToWatch);
            update.start_date("");
        } else {
            update.status(Status::Watching);
        }
        let updated_status = self.update_status(id, update).await?;

        let local_date = OffsetDateTime::now_utc().date();
        if updated_status.start_date.is_none() && progress == 1 {
            let mut update = StatusUpdate::new();
            update.start_date(&format!("{local_date}"));
            self.update_status(id, update).await?;
        }
        let episode_count = self.client
            .get_anime_details(id, AnimeFields::NumEpisodes)
            .await
            .expect("Anime details") // likely will fail
            .num_episodes
            .map_or(u32::MAX, |count| {
                if count == 0 {
                    u32::MAX
                } else {
                    count
                }
            });
        if updated_status.num_episodes_watched.unwrap_or_default() >= episode_count {
            let mut update = StatusUpdate::new();
            update.status(Status::Completed);
            if updated_status.finish_date.is_none() {
                update.finish_date(&format!("{local_date}"));
            }
            self.update_status(id, update).await?;
            // ask user for a score?
        }
        Ok(())
    }
    fn get_service_type(&self) -> ServiceType {
        ServiceType::MAL
    }
    fn is_logged_in(&self) -> bool {
        !self.client.need_auth
    }
    fn get_url(&self) -> Option<String> {
        self.url.clone()
    }
}

impl MAL {
    async fn update_status(&mut self, id: u32, update: StatusUpdate) -> Result<ListStatus, String> {
        Ok(self.client
            .update_user_anime_status(id, update)
            .await
            .expect("Update user's list")) // likely will fail
    }
}

fn to_episode_status(status: Option<String>) -> Option<EpisodeStatus> {
    status.map(|status_str| {
        match status_str.as_str() {
            "watching" => EpisodeStatus::Watching,
            "completed" => EpisodeStatus::Completed,
            "on_hold" => EpisodeStatus::OnHold,
            "dropped" => EpisodeStatus::Dropped,
            "plan_to_watch" => EpisodeStatus::PlanToWatch,
            _ => EpisodeStatus::None,
        }
    })
}
