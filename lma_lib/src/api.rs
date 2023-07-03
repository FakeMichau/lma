pub mod mal;
pub mod local;
use std::{path::PathBuf, collections::HashMap};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum ServiceType {
    MAL,
    Local
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ServiceTitle {
    pub service_id: usize,
    pub title: String,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ServiceEpisodeUser {
    pub status: Option<EpisodeStatus>,
    pub progress: Option<usize>,
    pub score: Option<u8>,
    pub is_rewatching: Option<bool>,
    pub rewatch_count: Option<usize>,
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

#[derive(PartialEq, Debug)]
pub struct ServiceEpisodeDetails {
    pub number: Option<usize>,
    pub title: Option<String>,
    pub title_japanese: Option<String>,
    pub title_romanji: Option<String>,
    pub duration: Option<usize>,
    pub aired: Option<String>,
    pub score: Option<f32>,
    pub filler: Option<bool>,
    pub recap: Option<bool>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct AlternativeTitles {
    pub synonyms: Vec<String>,
    pub languages: HashMap<String, String>,
}

#[async_trait]
pub trait Service {
    async fn new(cache_dir: PathBuf) -> Result<Self, String> where Self: Sized;
    async fn login(&mut self) -> Result<(), String>;
    async fn auth(&mut self);
    async fn init_show(&mut self, id: usize) -> Result<(), String>;
    async fn search_title(&mut self, potential_title: &str) -> Result<Vec<ServiceTitle>, String>;
    async fn get_title(&mut self, id: usize) -> Result<String, String>;
    async fn get_alternative_titles(&mut self, id: usize) -> Result<Option<AlternativeTitles>, String>;
    async fn get_episodes(&mut self, id: usize, precise_score: bool) -> Result<Vec<ServiceEpisodeDetails>, String>;
    async fn get_episode_count(&mut self, id: usize) -> Result<Option<usize>, String>;
    async fn get_user_entry_details(&mut self, id: usize) -> Result<Option<ServiceEpisodeUser>, String>;
    /// Returns actual progress set on the service
    async fn set_progress(&mut self, id: usize, progress: usize) -> Result<usize, String>;
    fn get_service_type(&self) -> ServiceType;
    fn get_url(&self) -> Option<String>;
    fn is_logged_in(&self) -> bool;
}
