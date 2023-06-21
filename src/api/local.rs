use std::path::PathBuf;
use async_trait::async_trait;
use crate::{ServiceTitle, Service, ServiceType, ServiceEpisodeUser, ServiceEpisodeDetails};

pub struct Local {
}

#[async_trait]
impl Service for Local {
    async fn new(_cache_dir: PathBuf) -> Self {
        Self {}
    }
    async fn login(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn auth(&mut self) {
    }
    async fn init_show(&mut self, _id: u32) -> Result<(), String> {
        Ok(())
    }
    async fn search_title(&mut self, _potential_title: &str) -> Result<Vec<ServiceTitle>, String> {
        Ok(Vec::new())
    }
    async fn get_title(&mut self, _id: u32) -> Result<String, String> {
        Ok(String::new())
    }
    async fn get_episode_count(&mut self, _id: u32) -> Result<Option<u32>, String> {
        Ok(None)
    }
    async fn get_user_entry_details(&mut self, _id: u32) -> Result<Option<ServiceEpisodeUser>, String> {
        Ok(None)
    }
    async fn get_episodes(&mut self, _id: u32) -> Result<Vec<ServiceEpisodeDetails>, String> {
        Ok(Vec::new())
    }
    async fn set_progress(&mut self, _id: u32, _progress: u32) -> Result<(), String> {
        Ok(())
    }
    fn get_service_type(&self) -> ServiceType {
        ServiceType::Local
    }
    fn is_logged_in(&self) -> bool {
        true
    }
    fn get_url(&self) -> Option<String> {
        Some("Using local service stub".to_owned())
    }
}
