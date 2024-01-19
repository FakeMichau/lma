pub mod local;
pub mod mal;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum ServiceType {
    MAL,
    Local,
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

pub trait Service: Send + Sync {
    fn new(cache_dir: PathBuf) -> impl std::future::Future<Output = Result<Self, String>> + Send
    where
        Self: Sized;
    fn login(&mut self) -> impl std::future::Future<Output = Result<(), String>> + Send;
    fn auth(&mut self) -> impl std::future::Future<Output = ()> + Send;
    fn init_show(&mut self, id: usize) -> impl std::future::Future<Output = Result<(), String>> + Send;
    fn search_title(&mut self, potential_title: &str) -> impl std::future::Future<Output = Result<Vec<ServiceTitle>, String>> + Send;
    fn get_title(&mut self, id: usize) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn get_alternative_titles(
        &mut self,
        id: usize,
    ) -> impl std::future::Future<Output = Result<Option<AlternativeTitles>, String>> + Send;
    fn get_episodes(
        &mut self,
        id: usize,
        precise_score: bool,
    ) -> impl std::future::Future<Output = Result<Vec<ServiceEpisodeDetails>, String>> + Send;
    fn get_episode_count(&mut self, id: usize) -> impl std::future::Future<Output = Result<Option<usize>, String>> + Send;
    fn get_user_entry_details(
        &mut self,
        id: usize,
    ) -> impl std::future::Future<Output = Result<Option<ServiceEpisodeUser>, String>> + Send;
    /// Returns actual progress set on the service
    fn set_progress(&mut self, id: usize, progress: usize) -> impl std::future::Future<Output = Result<usize, String>> + Send;
    fn get_service_type(&self) -> ServiceType;
    fn get_url(&self) -> Option<String>;
    fn is_logged_in(&self) -> bool;
}
