mod api;
pub use api::{local::Local, mal::MAL};
pub use api::{ServiceTitle, Service, ServiceType, ServiceEpisodeUser, EpisodeStatus, ServiceEpisodeDetails, AlternativeTitles};
pub use lib_mal::*;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::cmp::Ordering;
use rusqlite::{params, Connection, Result};
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
        let mut stmt = self.db_connection.prepare("
            SELECT Shows.id, Shows.title, Shows.sync_service_id, Shows.progress,
            COALESCE(Episodes.episode_number, -1) AS episode_number, COALESCE(Episodes.path, '') AS path, COALESCE(Episodes.title, '') AS episode_title, COALESCE(Episodes.extra_info, 0) AS extra_info
            FROM Shows
            LEFT JOIN Episodes ON Shows.id = Episodes.show_id;
        ")?;
        let mut shows: HashMap<i64, Show> = HashMap::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let show_id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let sync_service_id: i64 = row.get(2)?;
            let progress: i64 = row.get(3)?;
            let episode_number: i64 = row.get(4)?;
            let path: String = row.get(5)?;
            let episode_title: String = row.get(6)?;
            let extra_info: i64 = row.get(7)?;
            let recap = extra_info & (1 << 0) != 0;
            let filler = extra_info & (1 << 1) != 0;

            // I'm using a hashmap just for this step, find a way to avoid it?
            let show = shows.entry(show_id).or_insert_with(|| Show {
                local_id: show_id,
                title,
                progress,
                episodes: Vec::new(),
                service_id: sync_service_id,
            });
            if episode_number != -1 {
                show.episodes.push(Episode {
                    number: episode_number,
                    path: PathBuf::from(&path),
                    title: episode_title,
                    file_deleted: !PathBuf::from(path).exists(),
                    recap,
                    filler,
                });
            }
        }
        let mut shows: Vec<Show> = shows.into_values().collect();
        shows.sort_by(|show1, show2| {
            match self.title_sort {
                TitleSort::LocalIdAsc => show1.local_id.cmp(&show2.local_id),
                TitleSort::LocalIdDesc => show2.local_id.cmp(&show1.local_id),
                TitleSort::TitleAsc => show1.title.cmp(&show2.title),
                TitleSort::TitleDesc => show2.title.cmp(&show1.title),
                TitleSort::ServiceIdAsc => show1.service_id.cmp(&show2.service_id),
                TitleSort::ServiceIdDesc => show2.service_id.cmp(&show1.service_id),
            }
        });
        for show in &mut shows {
            show.episodes.sort_by_key(|episode| episode.number);
        }
        Ok(shows)
    }

    /// Returns local id of the added show
    pub fn add_show(&self, title: &str, sync_service_id: i64, progress: i64) -> Result<i64, String> {
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

        let row = rows.next().map_err(|e| e.to_string())?.ok_or("Can't get row")?;
        let local_id: i64 = row.get(0).map_err(|e| e.to_string())?;
        Ok(local_id)
    }

    pub fn get_local_show_id(&self, title: &str) -> Result<i64, String> {
        let mut stmt = self
            .db_connection
            .prepare(
                "SELECT id FROM Shows 
            WHERE title=?1",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query(params![title])
            .map_err(|e| e.to_string())?;

        let row = rows.next().map_err(|e| e.to_string())?.ok_or("Can't get row")?;
        let local_id: i64 = row.get(0).map_err(|e| e.to_string())?;
        Ok(local_id)
    }

    pub fn set_progress(&self, id: i64, progress: i64) -> Result<(), String> {
        self.db_connection.execute(
            "UPDATE Shows
            SET progress = ?2
            WHERE id = ?1", 
            params![
                id,
                progress,
            ]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_episode(&self, show_id: i64, episode_number: i64, path: &str, title: &str, extra_info: i64) -> Result<(), String> {
        self.db_connection
            .execute(
                "REPLACE INTO Episodes (show_id, episode_number, path, title, extra_info) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![show_id, episode_number, path, title, extra_info],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn add_episode_service_id(&self, sync_service_id: i64, episode_number: i64, path: &str, title: &str) -> Result<(), String> {
        self.db_connection
            .execute(
                "INSERT INTO Episodes (show_id, episode_number, path, title)
                SELECT id, ?2, ?3, ?4
                FROM Shows
                WHERE sync_service_id = ?1;
                ",
                params![sync_service_id, episode_number, path, title],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn update_progress(&mut self, rt: &Runtime) -> Result<(), String> {
        if !self.service.is_logged_in() {
            return Err(String::from("Can't progress, user not logged in"));
        }
        for show in self.get_list().map_err(|e| e.to_string())? {
            let service_id = u32::try_from(show.service_id)
                .map_err(|e: std::num::TryFromIntError| e.to_string())?;
            let user_entry_details =
                rt.block_on(self.service.get_user_entry_details(service_id))?;
            let user_service_progress_current = user_entry_details
                .map(|details| details.progress)
                .unwrap_or_default()
                .unwrap_or_default();

            let local_progress_current = u32::try_from(show.progress)
                .map_err(|e: std::num::TryFromIntError| e.to_string())?;
            match user_service_progress_current.cmp(&local_progress_current) {
                Ordering::Greater => {
                    self.set_progress(show.local_id, i64::from(user_service_progress_current))
                        .map_err(|e| format!("Can't set progress: {e}"))?;
                    Ok(())
                }
                Ordering::Less => rt.block_on(
                    self.service
                        .set_progress(service_id, local_progress_current),
                ),
                Ordering::Equal => Ok(()),
            }?;
        }
        Ok(())
    }

    pub fn get_video_file_paths(path: &PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
        if is_video_file(path) {
            return Ok(vec![path.clone()])
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

    pub fn guess_shows_title(&self, path: &PathBuf) -> Result<String, std::io::Error> {
        Ok(Self::remove_after_last_dash(
            &Self::get_video_file_paths(path)?
                .iter()
                .map(|dir| {
                    let filename = dir.file_stem().unwrap_or_default();
                    Self::cleanup_title(filename)
                })
                .next()
                .unwrap_or_default(),
        ))
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

    pub fn remove_entry(&self, show_id: i64) -> Result<(), String> {
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
    r.is_file() &&
    ["webm", "mkv", "vob", "ogg", "gif", "avi", "mov", "wmv", "mp4", "m4v", "3gp"]
        .into_iter()
        .any(|ext| {
            r.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .contains(ext)
        })
}

pub struct Show {
    pub local_id: i64,
    pub title: String,
    pub service_id: i64,
    pub episodes: Vec<Episode>,
    pub progress: i64,
}

#[derive(Default)]
pub struct Episode {
    pub title: String,
    pub number: i64,
    pub path: PathBuf,
    pub file_deleted: bool,
    pub recap: bool,
    pub filler: bool,
}

pub fn create<T: Service>(service: T, data_path: &Path, title_sort: &TitleSort) -> Result<AnimeList<T>, String> {
    let path = data_path.join("database.db3");
    let db_connection = Connection::open(path)
        .map_err(|err| format!("Can't create db connection {err}"))?;
    db_connection.is_readonly(rusqlite::DatabaseName::Main)
        .map_err(|err| format!("Can't check if db is read only {err}"))?;
    match db_connection.execute_batch(
        "
        CREATE TABLE Shows (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT UNIQUE, sync_service_id INTEGER UNIQUE, progress INTEGER);
        CREATE TABLE Episodes (show_id INTEGER, episode_number INTEGER, path TEXT, title TEXT, extra_info INTEGER, PRIMARY KEY (show_id, episode_number), FOREIGN KEY (show_id) REFERENCES Shows(id));
        "
    ) {
        Ok(_) => println!("Tables created"),
        Err(why) => {
            if why.to_string().contains("already exists") {
                println!("Table creation failed: tables already exist");
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
