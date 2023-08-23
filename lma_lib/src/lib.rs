mod api;
pub use api::{local::Local, mal::MAL};
pub use api::{
    AlternativeTitles, EpisodeStatus, Service, ServiceEpisodeDetails, ServiceEpisodeUser,
    ServiceTitle, ServiceType,
};
pub use lib_mal::*;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Sqlite, SqlitePool};
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct AnimeList<T: Service + Send + Sync> {
    db_connection: sqlx::Pool<Sqlite>,
    pub service: T,
    pub title_sort: TitleSort,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum TitleSort {
    LocalIdAsc,
    LocalIdDesc,
    TitleAsc,
    TitleDesc,
    ServiceIdAsc,
    ServiceIdDesc,
}

impl<T: Service + Send + Sync> AnimeList<T> {
    pub async fn get_list(&self) -> Result<Vec<Show>, String> {
        let rows = sqlx::query!(
            "
            SELECT Shows.id, Shows.title, Shows.sync_service_id, Shows.progress,
            COALESCE(Episodes.episode_number, 0) AS episode_number, 
                Episodes.path, 
                Episodes.title as episode_title, 
                Episodes.extra_info,
                Episodes.score as episode_score
            FROM Shows
            LEFT JOIN Episodes ON Shows.id = Episodes.show_id;
        ",
        )
        .fetch_all(&self.db_connection)
        .await
        .map_err(|e| e.to_string())?;

        let mut shows: Vec<Show> = Vec::new();

        for row in rows {
            let show_id: usize = usize::try_from(row.id).map_err(|e| e.to_string())?;
            let title: String = row.title.ok_or("Can't get title")?;
            let service_id: usize =
                usize::try_from(row.sync_service_id.ok_or("Can't get service_id")?)
                    .map_err(|e| e.to_string())?;
            let progress: usize = usize::try_from(row.progress.ok_or("Can't get progress")?)
                .map_err(|e| e.to_string())?;
            let episode_number: usize =
                usize::try_from(row.episode_number).map_err(|e| e.to_string())?;
            let path: String = row.path.unwrap_or_default();
            let episode_title: String = row.episode_title.unwrap_or_default();
            let extra_info: usize =
                usize::try_from(row.extra_info.unwrap_or_default()).map_err(|e| e.to_string())?;
            #[allow(clippy::cast_possible_truncation)]
            let episode_score: f32 = row.episode_score.unwrap_or_default() as f32;
            let recap = extra_info & (1 << 0) != 0;
            let filler = extra_info & (1 << 1) != 0;

            let show_with_no_episodes = Show {
                local_id: show_id,
                title,
                progress,
                episodes: Vec::new(),
                service_id,
            };
            if episode_number == 0 || !shows.iter().any(|s| s.local_id == show_id) {
                shows.push(show_with_no_episodes);
            }
            if episode_number != 0 {
                if let Some(show) = shows.iter_mut().find(|s| s.local_id == show_id) {
                    show.episodes.push(Episode {
                        number: episode_number,
                        path: PathBuf::from(&path),
                        title: episode_title,
                        file_deleted: !PathBuf::from(path).exists(),
                        score: episode_score,
                        recap,
                        filler,
                    });
                }
            }
        }
        shows.sort_by(|show1, show2| match self.title_sort {
            TitleSort::LocalIdAsc => show1.local_id.cmp(&show2.local_id),
            TitleSort::LocalIdDesc => show2.local_id.cmp(&show1.local_id),
            TitleSort::TitleAsc => show1.title.cmp(&show2.title),
            TitleSort::TitleDesc => show2.title.cmp(&show1.title),
            TitleSort::ServiceIdAsc => show1.service_id.cmp(&show2.service_id),
            TitleSort::ServiceIdDesc => show2.service_id.cmp(&show1.service_id),
        });
        for show in &mut shows {
            show.episodes.sort_by_key(|episode| episode.number);
        }
        Ok(shows)
    }

    /// Returns local id of the added show
    pub async fn add_show(
        &self,
        title: &str,
        sync_service_id: usize,
        progress: usize,
    ) -> Result<usize, String> {
        let sync_service_id = u32::try_from(sync_service_id).map_err(|e| e.to_string())?;
        let progress = u32::try_from(progress).map_err(|e| e.to_string())?;
        let rows = sqlx::query!(
            "INSERT INTO Shows (title, sync_service_id, progress) 
            VALUES (?1, ?2, ?3)
            RETURNING *",
            title,
            sync_service_id,
            progress
        )
        .fetch_all(&self.db_connection)
        .await
        .map_err(|e| e.to_string())?;

        let row = rows.first().ok_or("Can't get row")?;
        let local_id: usize = usize::try_from(row.id).map_err(|e| e.to_string())?;
        Ok(local_id)
    }

    pub async fn get_local_show_id(&self, title: &str) -> Result<usize, String> {
        let rows = sqlx::query!(
            "SELECT id FROM Shows 
            WHERE title=?1",
            title
        )
        .fetch_all(&self.db_connection)
        .await
        .map_err(|e| e.to_string())?;

        let row = rows.first().ok_or("Can't get row")?;
        let local_id = usize::try_from(row.id.ok_or("Can't get id from show name")?)
            .map_err(|e| e.to_string())?;
        Ok(local_id)
    }

    pub async fn set_progress(&self, id: usize, progress: usize) -> Result<(), String> {
        let id = u32::try_from(id).map_err(|e| e.to_string())?;
        let progress = u32::try_from(progress).map_err(|e| e.to_string())?;
        sqlx::query!(
            "UPDATE Shows
            SET progress = ?2
            WHERE id = ?1",
            id,
            progress
        )
        .execute(&self.db_connection)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn add_episode(
        &self,
        show_id: usize,
        episode_number: usize,
        path: &str,
        title: &str,
        extra_info: usize,
        score: f32,
    ) -> Result<(), String> {
        let show_id = u32::try_from(show_id).map_err(|e| e.to_string())?;
        let episode_number = u32::try_from(episode_number).map_err(|e| e.to_string())?;
        let extra_info = u32::try_from(extra_info).map_err(|e| e.to_string())?;
        sqlx::query!(
            "REPLACE INTO Episodes (show_id, episode_number, path, title, extra_info, score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            show_id, episode_number, path, title, extra_info, score
        )
        .execute(&self.db_connection)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn update_progress(&mut self) -> Result<(), String> {
        if !self.service.is_logged_in() {
            return Err(String::from("Can't progress, user not logged in"));
        }
        for show in self.get_list().await? {
            let service_id = show.service_id;
            let user_entry_details = self.service.get_user_entry_details(service_id).await?;
            let user_service_progress_current = user_entry_details
                .map(|details| details.progress)
                .unwrap_or_default()
                .unwrap_or_default();

            let local_progress_current = show.progress;
            match user_service_progress_current.cmp(&local_progress_current) {
                Ordering::Greater => self
                    .set_progress(show.local_id, user_service_progress_current)
                    .await
                    .map_err(|e| format!("Can't set progress: {e}")),
                Ordering::Less => {
                    let actual_progress = self
                        .service
                        .set_progress(service_id, local_progress_current)
                        .await
                        .unwrap_or(local_progress_current);
                    // in case of going beyond number of episodes
                    if actual_progress < local_progress_current {
                        self.set_progress(show.local_id, actual_progress)
                            .await
                            .map_err(|e| format!("Can't set progress: {e}"))?;
                    }
                    Ok(())
                }
                Ordering::Equal => Ok(()),
            }?;
        }
        Ok(())
    }

    pub fn get_video_file_paths(path: &PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
        if is_video_file(path) {
            return Ok(vec![path.clone()]);
        }
        let mut files = Vec::new();
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            if is_video_file(&path) {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }

    pub fn guess_shows_title(&self, path: &PathBuf) -> Result<String, String> {
        let mut fname = String::new();
        let guessed_title = Self::get_video_file_paths(path)
            .map_err(|err| err.to_string())?
            .iter()
            .map(|dir| {
                let filename = dir.file_stem().unwrap_or_default();
                fname = filename.to_string_lossy().to_string();
                Self::cleanup_title(filename)
            })
            .next()
            .map_or(String::new(), |str| Self::remove_after_last_dash(&str));

        Ok(if guessed_title.is_empty() {
            fname
        } else {
            guessed_title
        })
    }

    pub fn count_video_files(&self, path: &PathBuf) -> Result<usize, std::io::Error> {
        Ok(Self::get_video_file_paths(path)?.len())
    }

    fn cleanup_title(input: &OsStr) -> String {
        let input = input.to_string_lossy();
        let mut result = String::new();
        let mut depth_square = 0;
        let mut depth_paren = 0;

        for ch in input.chars() {
            match ch {
                '[' => depth_square += 1,
                ']' => depth_square -= 1,
                '(' => depth_paren += 1,
                ')' => depth_paren -= 1,
                _ if depth_square == 0 && depth_paren == 0 => result.push(ch),
                _ => (),
            }
        }

        result.trim().to_string()
    }

    fn remove_after_last_dash(input: &str) -> String {
        if let Some(index) = input.rfind('-') {
            let trimmed = &input[..index].trim();
            return (*trimmed).to_string();
        }
        input.to_string()
    }

    pub async fn remove_entry(&self, show_id: usize) -> Result<(), String> {
        let show_id = u32::try_from(show_id).map_err(|e| e.to_string())?;
        sqlx::query!("DELETE FROM Episodes WHERE show_id = ?1", show_id)
            .execute(&self.db_connection)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query!("DELETE FROM Shows WHERE id = ?1", show_id)
            .execute(&self.db_connection)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn is_video_file(r: &Path) -> bool {
    r.is_file()
        && [
            "webm", "mkv", "vob", "ogg", "gif", "avi", "mov", "wmv", "mp4", "m4v", "3gp",
        ]
        .into_iter()
        .any(|ext| {
            r.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .contains(ext)
        })
}

#[derive(Clone)]
pub struct Show {
    pub local_id: usize,
    pub title: String,
    pub service_id: usize,
    pub episodes: Vec<Episode>,
    pub progress: usize,
}

#[derive(Default, Clone)]
pub struct Episode {
    pub title: String,
    pub number: usize,
    pub path: PathBuf,
    pub file_deleted: bool,
    pub score: f32,
    pub recap: bool,
    pub filler: bool,
}

pub async fn create<T: Service + Send + Sync>(
    service: T,
    data_path: &Path,
    title_sort: &TitleSort,
) -> Result<AnimeList<T>, String> {
    let path = data_path.join("database.db3");
    let url = format!("sqlite:{}", path.to_string_lossy());
    let options = SqliteConnectOptions::from_str(&url)
        .map_err(|err| format!("Can't create db options {err}"))?
        .create_if_missing(true);
    let db_pool = SqlitePool::connect_with(options)
        .await
        .map_err(|err| format!("Can't create db connection {err}"))?;
    let result = sqlx::query!("
        CREATE TABLE IF NOT EXISTS Shows (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT UNIQUE, sync_service_id INTEGER UNIQUE, progress INTEGER);
        CREATE TABLE IF NOT EXISTS Episodes (show_id INTEGER, episode_number INTEGER, path TEXT, title TEXT, extra_info INTEGER, score REAL, PRIMARY KEY (show_id, episode_number), FOREIGN KEY (show_id) REFERENCES Shows(id))
    ")
    .execute(&db_pool)
    .await;

    match result {
        Ok(_) => {
            #[cfg(debug_assertions)]
            dbg!("Tables created");
        }
        Err(why) => {
            if why.to_string().contains("already exists") {
                #[cfg(debug_assertions)]
                dbg!("Table creation failed: tables already exist");
            } else {
                Err::<AnimeList<T>, String>(format!("Table creation failed: {why}"))?;
            }
        }
    };
    Ok(AnimeList {
        db_connection: db_pool,
        service,
        title_sort: title_sort.clone(),
    })
}
