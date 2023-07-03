use crate::{
    AlternativeTitles, Service, ServiceEpisodeDetails, ServiceEpisodeUser, ServiceTitle,
    ServiceType,
};
use async_trait::async_trait;
use std::{fs, path::PathBuf};

pub struct Local {}

#[async_trait]
impl Service for Local {
    async fn new(cache_dir: PathBuf) -> Result<Self, String> {
        let tokens_path = cache_dir.join("tokens");
        if !tokens_path.exists() {
            fs::write(tokens_path, String::new()).map_err(|err| err.to_string())?;
        }
        Ok(Self {})
    }
    async fn login(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn auth(&mut self) {}
    async fn init_show(&mut self, _id: usize) -> Result<(), String> {
        Ok(())
    }
    async fn search_title(&mut self, _potential_title: &str) -> Result<Vec<ServiceTitle>, String> {
        Ok(Vec::new())
    }
    async fn get_title(&mut self, _id: usize) -> Result<String, String> {
        Ok(String::new())
    }
    async fn get_alternative_titles(
        &mut self,
        _id: usize,
    ) -> Result<Option<AlternativeTitles>, String> {
        Ok(None)
    }
    async fn get_episode_count(&mut self, _id: usize) -> Result<Option<usize>, String> {
        Ok(None)
    }
    async fn get_user_entry_details(
        &mut self,
        _id: usize,
    ) -> Result<Option<ServiceEpisodeUser>, String> {
        Ok(None)
    }
    async fn get_episodes(
        &mut self,
        _id: usize,
        _precise_score: bool,
    ) -> Result<Vec<ServiceEpisodeDetails>, String> {
        Ok(Vec::new())
    }
    async fn set_progress(&mut self, _id: usize, progress: usize) -> Result<usize, String> {
        Ok(progress)
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

        let result = local_service.get_episodes(222, false).await;
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
