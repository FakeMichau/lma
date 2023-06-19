mod mal;
use std::path::PathBuf;

pub use mal::*;
use async_trait::async_trait;

pub struct ServiceTitle {
    pub service_id: u32,
    pub title: String,
}

#[async_trait]
pub trait Service {
    async fn new(cache_dir: PathBuf) -> Self;
    async fn login(&mut self);
    async fn auth(&mut self);
    async fn init_show(&mut self, id: u32);
    async fn search_title(&mut self, potential_title: &str) -> Vec<ServiceTitle>;
    async fn get_title(&mut self, id: u32) -> String;
    async fn get_episode_count(&mut self, id: u32) -> Option<u32>;
    async fn set_progress(&mut self, id: u32, progress: u32);
    fn get_url(&self) -> &Option<String>;
    fn is_logged_in(&self) -> bool;
}
