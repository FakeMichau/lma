mod api;
pub use api::{local::Local, mal::MAL};
pub use api::{
    AlternativeTitles, EpisodeStatus, Service, ServiceEpisodeDetails, ServiceEpisodeUser,
    ServiceTitle, ServiceType,
};
pub use lib_mal::*;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

pub struct AnimeList<T: Service> {
    db_connection: Connection,
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

impl<T: Service> AnimeList<T> {
    pub fn get_list(&self) -> Result<Vec<Show>, rusqlite::Error> {
        let mut stmt = self.db_connection.prepare(
            "
            SELECT Shows.id, Shows.title, Shows.sync_service_id, Shows.progress,
            COALESCE(Episodes.episode_number, 0) AS episode_number, 
                Episodes.path, 
                Episodes.title, 
                Episodes.extra_info,
                Episodes.score
            FROM Shows
            LEFT JOIN Episodes ON Shows.id = Episodes.show_id;
        ",
        )?;
        let mut shows: Vec<Show> = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let show_id: usize = row.get(0)?;
            let title: String = row.get(1)?;
            let service_id: usize = row.get(2)?;
            let progress: usize = row.get(3)?;
            let episode_number: usize = row.get(4)?;
            let path: String = row.get(5).unwrap_or_default();
            let episode_title: String = row.get(6).unwrap_or_default();
            let extra_info: usize = row.get(7).unwrap_or_default();
            let episode_score: f32 = row.get(8).unwrap_or_default();
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
    pub fn add_show(
        &self,
        title: &str,
        sync_service_id: usize,
        progress: usize,
    ) -> Result<usize, String> {
        let mut stmt = self
            .db_connection
            .prepare(
                "INSERT INTO Shows (title, sync_service_id, progress) 
            VALUES (?1, ?2, ?3)
            RETURNING *",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query(params![title, sync_service_id, progress])
            .map_err(|e| e.to_string())?;

        let row = rows
            .next()
            .map_err(|e| e.to_string())?
            .ok_or("Can't get row")?;
        let local_id: usize = row.get(0).map_err(|e| e.to_string())?;
        Ok(local_id)
    }

    pub fn get_local_show_id(&self, title: &str) -> Result<usize, String> {
        let mut stmt = self
            .db_connection
            .prepare(
                "SELECT id FROM Shows 
            WHERE title=?1",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query(params![title]).map_err(|e| e.to_string())?;

        let row = rows
            .next()
            .map_err(|e| e.to_string())?
            .ok_or("Can't get row")?;
        let local_id: usize = row.get(0).map_err(|e| e.to_string())?;
        Ok(local_id)
    }

    pub fn set_progress(&self, id: usize, progress: usize) -> Result<(), String> {
        self.db_connection
            .execute(
                "UPDATE Shows
            SET progress = ?2
            WHERE id = ?1",
                params![id, progress,],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_episode(
        &self,
        show_id: usize,
        episode_number: usize,
        path: &str,
        title: &str,
        extra_info: usize,
        score: f32,
    ) -> Result<(), String> {
        self.db_connection
            .execute(
                "REPLACE INTO Episodes (show_id, episode_number, path, title, extra_info, score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![show_id, episode_number, path, title, extra_info, score],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_progress(&mut self, rt: &Runtime) -> Result<(), String> {
        if !self.service.is_logged_in() {
            return Err(String::from("Can't progress, user not logged in"));
        }
        for show in self.get_list().map_err(|e| e.to_string())? {
            let service_id = show.service_id;
            let user_entry_details =
                rt.block_on(self.service.get_user_entry_details(service_id))?;
            let user_service_progress_current = user_entry_details
                .map(|details| details.progress)
                .unwrap_or_default()
                .unwrap_or_default();

            let local_progress_current = show.progress;
            match user_service_progress_current.cmp(&local_progress_current) {
                Ordering::Greater => self
                    .set_progress(show.local_id, user_service_progress_current)
                    .map_err(|e| format!("Can't set progress: {e}")),
                Ordering::Less => {
                    let actual_progress = rt
                        .block_on(
                            self.service
                                .set_progress(service_id, local_progress_current),
                        )
                        .unwrap_or(local_progress_current);
                    // in case of going beyond number of episodes
                    if actual_progress < local_progress_current {
                        self.set_progress(show.local_id, actual_progress)
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

    pub fn remove_entry(&self, show_id: usize) -> Result<(), String> {
        self.db_connection
            .execute("DELETE FROM Episodes WHERE show_id = ?1", params![show_id])
            .map_err(|e| e.to_string())?;
        self.db_connection
            .execute("DELETE FROM Shows WHERE id = ?1", params![show_id])
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

pub fn create<T: Service>(
    service: T,
    data_path: &Path,
    title_sort: &TitleSort,
) -> Result<AnimeList<T>, String> {
    let path = data_path.join("database.db3");
    let db_connection =
        Connection::open(path).map_err(|err| format!("Can't create db connection {err}"))?;
    db_connection
        .is_readonly(rusqlite::DatabaseName::Main)
        .map_err(|err| format!("Can't check if db is read only {err}"))?;
    match db_connection.execute_batch(
        "
        CREATE TABLE Shows (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT UNIQUE, sync_service_id INTEGER UNIQUE, progress INTEGER);
        CREATE TABLE Episodes (show_id INTEGER, episode_number INTEGER, path TEXT, title TEXT, extra_info INTEGER, score REAL, PRIMARY KEY (show_id, episode_number), FOREIGN KEY (show_id) REFERENCES Shows(id));
        "
    ) {
        Ok(_) => {
            #[cfg(debug_assertions)]
            dbg!("Tables created");
        },
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
        db_connection,
        service,
        title_sort: title_sort.clone(),
    })
}
