use std::path::PathBuf;
use async_trait::async_trait;

use lib_mal::{
    prelude::{EpisodeNode, ListStatus},
    MALError,
};

use crate::{ServiceTitle, Service};

pub struct Local {
}

#[async_trait]
impl Service for Local {
    async fn new(_cache_dir: PathBuf) -> Self {
        Self {}
    }
    async fn login(&mut self) {
    }
    async fn auth(&mut self) {
    }
    async fn init_show(&mut self, _id: u32) {
    }
    async fn search_title(&mut self, _potential_title: &str) -> Vec<ServiceTitle> {
        Vec::new()
    }
    async fn get_title(&mut self, _id: u32) -> String {
        String::new()
    }
    async fn get_episode_count(&mut self, _id: u32) -> Option<u32> {
        None
    }
    async fn set_progress(&mut self, _id: u32, _progress: u32) {
    }
    fn is_logged_in(&self) -> bool {
        true
    }
    fn get_url(&self) -> Option<String> {
        Some("Using local service stub".to_owned())
    }

    // TEMP
    // remove lib_mal dep
    async fn get_episodes(&mut self, _id: u32) -> Result<Vec<EpisodeNode>, MALError> {
        Err(MALError::new(
            "Using local service stub",
            "Using local service stub",
            "Using local service stub".to_string()
        ))
    }

    // remove lib_mal dep
    async fn get_user_entry_details(&mut self, _id: u32) -> Option<ListStatus> {
        None
    }
}
