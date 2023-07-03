use std::path::PathBuf;
use async_trait::async_trait;
use time::OffsetDateTime;
use lib_mal::prelude::fields::AnimeFields;
use lib_mal::prelude::options::{Status, StatusUpdate};
use lib_mal::prelude::ListStatus;
use lib_mal::{ClientBuilder, MALClientTrait};
use crate::{ServiceTitle, Service, ServiceType, ServiceEpisodeUser, EpisodeStatus, ServiceEpisodeDetails, AlternativeTitles};

pub struct MAL<T> {
    client: T,
    challenge: String,
    state: String,
    url: Option<String>,
}

#[async_trait]
impl<T: MALClientTrait + Send + Sync> Service for MAL<T> {
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
            .map_err(|err| err.to_string())?;
        self.url = None;
        Ok(())
    }
    async fn auth(&mut self) {
        self.url = if self.client.need_auth() {
            let url;
            (url, self.challenge, self.state) = self.client.get_auth_parts();
            Some(url)
        } else {
            None
        }
    }
    async fn init_show(&mut self, id: usize) -> Result<(), String> {
        if self.get_user_entry_details(id).await?.is_none() {
            // add to plan to watch
            let mut update = StatusUpdate::new();
            update.status(Status::PlanToWatch);
            self.client
                .update_user_anime_status(id, update)
                .await
                .map_err(|err| format!("Update user's list: {err}"))?;
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
            .map_err(|err| format!("MAL search result: {err}"))?
            .data
            .iter()
            .map(|entry| ServiceTitle {
                service_id: entry.node.id,
                title: entry.node.title.to_string(),
            })
            .collect())
    }
    async fn get_title(&mut self, id: usize) -> Result<String, String> {
        Ok(self
            .client
            .get_anime_details(id, AnimeFields::Title)
            .await
            .map_err(|err| format!("Anime title: {err}"))?
            .show
            .title)
    }
    async fn get_alternative_titles(&mut self, id: usize) -> Result<Option<AlternativeTitles>, String> {
        Ok(self
            .client
            .get_anime_details(id, AnimeFields::AlternativeTitles)
            .await
            .map_err(|err| format!("Alternative anime titles: {err}"))?
            .alternative_titles
            .map(|titles| {
                AlternativeTitles {
                    synonyms: titles.synonyms,
                    languages: titles.languages,
                }
            }))
    }
    async fn get_episode_count(&mut self, id: usize) -> Result<Option<usize>, String> {
        Ok(self.client
            .get_anime_details(id, AnimeFields::NumEpisodes)
            .await
            .map_err(|err| format!("Anime episode count: {err}"))?
            .num_episodes)
    }
    async fn get_user_entry_details(&mut self, id: usize) -> Result<Option<ServiceEpisodeUser>, String> {
        Ok(self.client
            .get_anime_details(id, AnimeFields::MyListStatus)
            .await
            .map_err(|err| format!("Anime details: {err}"))?
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
    async fn get_episodes(&mut self, id: usize, precise_score: bool) -> Result<Vec<ServiceEpisodeDetails>, String> {
        self.client
            .get_anime_episodes(id, precise_score)
            .await
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
                        score: episode.score,
                        filler: episode.filler,
                        recap: episode.recap,
                    }
                })
                .collect::<Vec<_>>()
            })
            .map_err(|err| format!("Get episodes: {err}"))
    }
    async fn set_progress(&mut self, id: usize, progress: usize) -> Result<usize, String> {
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
            .map_err(|err| format!("Anime details: {err}"))?
            .num_episodes
            .map_or(usize::MAX, |count| {
                if count == 0 {
                    usize::MAX
                } else {
                    count
                }
            });
        let actual_progress = updated_status.num_episodes_watched.unwrap_or(progress);
        if actual_progress >= episode_count {
            let mut update = StatusUpdate::new();
            update.status(Status::Completed);
            if updated_status.finish_date.is_none() {
                update.finish_date(&format!("{local_date}"));
            }
            self.update_status(id, update).await?;
            // ask user for a score?
        }
        Ok(actual_progress)
    }
    fn get_service_type(&self) -> ServiceType {
        ServiceType::MAL
    }
    fn is_logged_in(&self) -> bool {
        !self.client.need_auth()
    }
    fn get_url(&self) -> Option<String> {
        self.url.clone()
    }
}

impl<T: MALClientTrait + Send + Sync> MAL<T> {
    async fn update_status(&mut self, id: usize, update: StatusUpdate) -> Result<ListStatus, String> {
        self.client
            .update_user_anime_status(id, update)
            .await
            .map_err(|err| format!("Update user's list: {err}"))
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

#[cfg(test)]
mod tests {
    use lib_mal::MockMALClient;
    use reqwest::Client;

    use super::*;

    #[tokio::test]
    async fn test_init_show() {
        let mut client = create_logged_in_client().await;
        let p2w = client.init_show(30230).await;
        let not_on_list = client.init_show(21).await;
        assert!(p2w.is_ok());
        assert!(not_on_list.is_ok());
    }

    #[tokio::test]
    async fn test_search() {
        let mut client = create_logged_in_client().await;
        let search_result = client.search_title("doesn't matter").await;
        assert!(search_result.is_ok());

        let search_vec = search_result.unwrap();
        assert_eq!(search_vec.len(), 3);

        let last_opt = search_vec.get(2);
        assert!(last_opt.is_some());

        let last = last_opt.unwrap();
        assert_eq!(last.service_id, 459);
    }

    #[tokio::test]
    async fn test_get_title() {
        let mut client = create_logged_in_client().await;
        let title_result = client.get_title(21).await;
        assert!(title_result.is_ok());
        let title = title_result.unwrap();
        assert_eq!(title, "One Piece");
    }

    #[tokio::test]
    async fn test_get_episode_count() {
        let mut client = create_logged_in_client().await;
        let count_result = client.get_episode_count(21).await;
        assert!(count_result.is_ok());
        let count = count_result.unwrap();
        assert_eq!(count, Some(0));

        let count_result = client.get_episode_count(30230).await;
        assert!(count_result.is_ok());
        let count = count_result.unwrap();
        assert_eq!(count, Some(51));
    }

    #[tokio::test]
    async fn test_get_user_entry_details() {
        // not on user's list
        let mut client = create_logged_in_client().await;
        let details_result = client.get_user_entry_details(21).await;
        assert!(details_result.is_ok());
        let details = details_result.unwrap();
        assert!(details.is_none());

        // plan to watch
        let details_result = client.get_user_entry_details(30230).await;
        assert!(details_result.is_ok());
        let details = details_result.unwrap();
        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(details.status, Some(EpisodeStatus::PlanToWatch));
        assert_eq!(details.score, Some(0));
        assert_eq!(details.rewatch_count, Some(0));
        assert_eq!(details.is_rewatching, Some(false));
        assert_eq!(details.updated_at, Some(String::from("2017-11-11T19:51:22+00:00")));
    }

    #[tokio::test]
    async fn test_get_anime_episodes() {
        let mut client = create_logged_in_client().await;
        let episodes_result = client.get_episodes(21, false).await;
        assert!(episodes_result.is_ok());
        let episodes = episodes_result.unwrap();
        // mocked get_anime_episodes returns an empty vector
        assert!(episodes.is_empty());
    }

    #[tokio::test]
    async fn test_get_url_not_logged_in() {
        let mut client = generate_test_client();
        client.auth().await;
        let url = client.get_url().unwrap();
        assert!(url.contains("https://example.com/&client_id=client_secret"));
    }

    #[tokio::test]
    async fn test_get_url_logged_in() {
        let mut client = create_logged_in_client().await;
        client.auth().await;
        let url = client.get_url();
        assert!(url.is_none());
    }

    async fn create_logged_in_client() -> MAL::<MockMALClient> {
        let mut client = generate_test_client();
        _=client.login().await;
        client
    }

    fn generate_test_client() -> MAL::<MockMALClient> {
        MAL::<MockMALClient> {
            client: MockMALClient::new(
                String::from("client_secret"), 
                PathBuf::new(), 
                String::new(), 
                Client::new(), 
                false, 
                true
            ),
            challenge: String::new(),
            state: String::new(),
            url: Some(String::new()),
        }
    }
}