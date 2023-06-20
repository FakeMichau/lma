pub mod mal;
pub mod local;
use std::path::PathBuf;
use async_trait::async_trait;

#[derive(PartialEq)]
pub enum ServiceType {
    MAL,
    Local
}

#[derive(Clone)]
pub struct ServiceTitle {
    pub service_id: u32,
    pub title: String,
}

pub struct ServiceEpisodeUser {
    pub status: Option<EpisodeStatus>,
    pub progress: Option<u32>,
    pub score: Option<u8>,
    pub is_rewatching: Option<bool>,
    pub rewatch_count: Option<u32>,
    pub updated_at: Option<String>,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub comments: Option<String>,
}

pub enum EpisodeStatus {
    None,
    Watching,
    Completed,
    OnHold,
    Dropped,
    PlanToWatch,
}

pub struct ServiceEpisodeDetails {
    pub number: Option<u32>,
    pub title: Option<String>,
    pub title_japanese: Option<String>,
    pub title_romanji: Option<String>,
    pub duration: Option<u32>,
    pub aired: Option<String>,
    pub filler: Option<bool>,
    pub recap: Option<bool>,
}

#[async_trait]
pub trait Service {
    async fn new(cache_dir: PathBuf) -> Self;
    async fn login(&mut self);
    async fn auth(&mut self);
    async fn init_show(&mut self, id: u32);
    async fn search_title(&mut self, potential_title: &str) -> Vec<ServiceTitle>;
    async fn get_title(&mut self, id: u32) -> String;
    async fn get_episodes(&mut self, id: u32) -> Vec<ServiceEpisodeDetails>;
    async fn get_episode_count(&mut self, id: u32) -> Option<u32>;
    async fn get_user_entry_details(&mut self, id: u32) -> Option<ServiceEpisodeUser>;
    async fn set_progress(&mut self, id: u32, progress: u32);
    fn get_service_type(&self) -> ServiceType;
    fn get_url(&self) -> Option<String>;
    fn is_logged_in(&self) -> bool;
}
