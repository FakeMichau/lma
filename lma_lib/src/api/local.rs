use std::path::PathBuf;
use async_trait::async_trait;
use crate::{ServiceTitle, Service, ServiceType, ServiceEpisodeUser, ServiceEpisodeDetails, AlternativeTitles};

pub struct Local {
}

#[async_trait]
impl Service for Local {
    async fn new(_cache_dir: PathBuf) -> Result<Self, String>  {
        Ok(Self {})
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
    async fn get_alternative_titles(&mut self, _id: u32) -> Result<Option<AlternativeTitles>, String> {
        Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_local_service() {
        let result = Local::new(PathBuf::new()).await;
        assert!(result.is_ok());

        let mut local_service = Local {};
        
        let result = local_service.login().await;
        assert!(result.is_ok());
        
        let result = local_service.init_show(123).await;
        assert!(result.is_ok());
        
        let result = local_service.search_title("some_title").await;
        assert_eq!(result, Ok(Vec::new()));
        
        let result = local_service.get_title(456).await;
        assert_eq!(result, Ok(String::new()));

        let result = local_service.get_alternative_titles(727).await;
        assert_eq!(result, Ok(None));
        
        let result = local_service.get_episode_count(789).await;
        assert_eq!(result, Ok(None));
        
        let result = local_service.get_user_entry_details(111).await;
        assert_eq!(result, Ok(None));
        
        let result = local_service.get_episodes(222).await;
        assert_eq!(result, Ok(Vec::new()));
        
        let result = local_service.set_progress(333, 50).await;
        assert!(result.is_ok());
        
        let service_type = local_service.get_service_type();
        assert_eq!(service_type, ServiceType::Local);
        
        let is_logged_in = local_service.is_logged_in();
        assert!(is_logged_in);
        
        let url = local_service.get_url();
        assert_eq!(url, Some("Using local service stub".to_owned()));
    }
}
