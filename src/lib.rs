pub mod api;
pub use api::*;
use rusqlite::{params, Connection, Result};
use tokio::runtime::Runtime;
use std::ffi::OsStr;
use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AnimeList<T: Service> {
    db_connection: Connection,
    pub service: T,
}

impl<T: Service> AnimeList<T> {
    pub fn get_list(&self) -> Result<Vec<Show>> {
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
        let mut shows: Vec<Show> = shows.into_iter().map(|(_, show)| show).collect();
        shows.sort_by_key(|show| show.local_id);
        shows.iter_mut().for_each(|show| {
            show.episodes.sort_by_key(|episode| episode.number);
        });
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

        let local_id: i64 = rows.next().map_err(|e| e.to_string())?.unwrap().get(0).unwrap();
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

        let local_id: i64 = rows.next().unwrap().unwrap().get(0).unwrap();
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

    pub fn update_progress(&mut self, rt: &Runtime) {
        if !self.service.is_logged_in() {
            return
        }
        self.get_list()
            .expect("List from the local database")
            .into_iter()
            .for_each(|show| {
                let user_entry_details = rt.block_on(self
                    .service
                    .get_user_entry_details(show.service_id.try_into().unwrap())
                );
                let user_service_progress_current = user_entry_details
                    .map(|details| details.num_episodes_watched)
                    .unwrap_or_default()
                    .unwrap_or_default();

                let local_progress_current = show.progress as u32;
                // progress different between local and service
                if user_service_progress_current > local_progress_current {
                    self.set_progress(show.local_id, user_service_progress_current as i64)
                        .expect("Set local progress");
                } else if user_service_progress_current < local_progress_current {
                    rt.block_on(
                        self.service.set_progress(show.service_id as u32, local_progress_current)
                    );
                }
            })
    }

    pub fn get_video_file_paths(path: &str) -> Result<Vec<PathBuf>, std::io::Error> {
        let read_dir = fs::read_dir(path)?;
        let mut files = read_dir
            .into_iter()
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .filter(|r| {
                r.is_file() && 
                ["webm", "mkv", "vob", "ogg", "gif", "avi", "mov", "wmv", "mp4", "m4v", "3gp"]
                    .into_iter()
                    .any(|ext| {
                        r.extension()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .contains(ext)
                    })
            })
            .map(|path| path)
            .collect::<Vec<_>>();
        files.sort();
        Ok(files)
    }

    pub fn guess_shows_title(&self, path: &str) -> Result<String, std::io::Error> {
        Ok(AnimeList::<T>::remove_after_last_dash(
            &AnimeList::<T>::get_video_file_paths(&path)?
                .iter()
                .map(|dir| {
                    let filename = dir.file_stem().unwrap_or_default();
                    AnimeList::<T>::cleanup_title(filename)
                })
                .next()
                .unwrap_or("".to_string()),
        ))
    }

    pub fn count_video_files(&self, path: &str) -> Result<usize, std::io::Error> {
        Ok(AnimeList::<T>::get_video_file_paths(path)?.len())
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
            return trimmed.to_string();
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

pub struct Show {
    pub local_id: i64,
    pub title: String,
    pub service_id: i64,
    pub episodes: Vec<Episode>,
    pub progress: i64,
}

pub struct Episode {
    pub title: String,
    pub number: i64,
    pub path: PathBuf,
    pub file_deleted: bool,
    pub recap: bool,
    pub filler: bool,
}

pub fn create<T: Service>(service: T, data_path: &PathBuf) -> AnimeList<T> {
    let path = data_path.join("database.db3");
    let db_connection = match Connection::open(path) {
        Ok(conn) => conn,
        Err(why) => panic!("Cry - {}", why),
    };
    if db_connection
        .is_readonly(rusqlite::DatabaseName::Main)
        .expect("shouldn't realistically return an error")
    {
        panic!("Database is read-only");
    }
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
                    eprintln!("Table creation failed: {}", why.to_string());
                }
            }
    };
    AnimeList {
        db_connection,
        service,
    }
}
