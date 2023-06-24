pub mod mal;
pub mod local;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum ServiceType {
    MAL,
    Local
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ServiceTitle {
    pub service_id: u32,
    pub title: String,
}

#[derive(PartialEq, Eq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
pub enum EpisodeStatus {
    None,
    Watching,
    Completed,
    OnHold,
    Dropped,
    PlanToWatch,
}

#[derive(PartialEq, Eq, Debug)]
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
    async fn new(cache_dir: PathBuf) -> Result<Self, String> where Self: Sized;
    async fn login(&mut self) -> Result<(), String>;
    async fn auth(&mut self);
    async fn init_show(&mut self, id: u32) -> Result<(), String>;
    async fn search_title(&mut self, potential_title: &str) -> Result<Vec<ServiceTitle>, String>;
    async fn get_title(&mut self, id: u32) -> Result<String, String>;
    async fn get_episodes(&mut self, id: u32) -> Result<Vec<ServiceEpisodeDetails>, String>;
    async fn get_episode_count(&mut self, id: u32) -> Result<Option<u32>, String>;
    async fn get_user_entry_details(&mut self, id: u32) -> Result<Option<ServiceEpisodeUser>, String>;
    async fn set_progress(&mut self, id: u32, progress: u32) -> Result<(), String>;
    fn get_service_type(&self) -> ServiceType;
    fn get_url(&self) -> Option<String>;
    fn is_logged_in(&self) -> bool;
}
