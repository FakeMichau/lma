pub mod api;
pub use api::*;
use rusqlite::{params, Connection, Result};
use std::ffi::OsStr;
use std::fs;
use std::collections::HashMap;

pub struct AnimeList {
    db_connection: Connection,
    service: MAL,
}

impl AnimeList {
    pub fn get_list(&self) -> Result<Vec<Show>> {
        let mut stmt = self.db_connection.prepare("
            SELECT Shows.id, Shows.title, Shows.sync_service_id, Shows.episode_count, Shows.progress,
            COALESCE(Episodes.episode_number, -1) AS episode_number, COALESCE(Episodes.path, '') AS path
            FROM Shows
            LEFT JOIN Episodes ON Shows.id = Episodes.show_id;
        ")?;
        let mut shows: HashMap<i64, Show> = HashMap::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let show_id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let sync_service_id: i64 = row.get(2)?;
            let episode_count: i64 = row.get(3)?;
            let progress: i64 = row.get(4)?;
            let episode_number: i64 = row.get(5)?;
            let path: String = row.get(6)?;

            // I'm using a hashmap just for this step, find a way to avoid it?
            let show = shows.entry(show_id).or_insert_with(|| Show {
                id: show_id,
                title,
                progress,
                episode_count,
                episodes: Vec::new(),
                sync_service_id,
            });
            if episode_number != -1 {
                show.episodes.push(Episode {
                    number: episode_number,
                    path,
                });
            }
        }
        let mut shows: Vec<Show> = shows.into_iter().map(|(_, show)| show).collect();
        shows.sort_by_key(|show| show.id);
        shows.iter_mut().for_each(|show| {
            show.episodes.sort_by_key(|episode| episode.number);
        });
        // shows
        //     .iter_mut()
        //     .for_each(|show| {
        //         show.title = self.list_titles(&show.title).first().unwrap_or(&show.title).clone()
        //     });
        Ok(shows)
    }
    pub fn add_show(
        &self,
        title: &str,
        sync_service_id: i64,
        episode_count: i64,
        progress: i64,
    ) -> Result<(), String> {
        self.db_connection.execute(
            "REPLACE INTO Shows (title, sync_service_id, episode_count, progress) VALUES (?1, ?2, ?3, ?4)", 
            params![
                title,
                sync_service_id,
                episode_count,
                progress,
            ]
        ).map(|_| ()).map_err(|e| e.to_string())
    }
    pub fn add_episode(&self, show_id: i64, episode_number: i64, path: &str) -> Result<(), String> {
        self.db_connection
            .execute(
                "REPLACE INTO Episodes (show_id, episode_number, path) VALUES (?1, ?2, ?3)",
                params![show_id, episode_number, path,],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn get_files(&self, path: &str) -> Result<Vec<String>, std::io::Error> {
        let read_dir = fs::read_dir(path)?;
        let mut files = read_dir
            .into_iter()
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .filter(|r| r.is_file())
            .map(|dir| {
                let filename = dir.file_stem().unwrap_or_default();
                AnimeList::extract_title(filename)
                    .unwrap_or(filename.to_string_lossy().to_string())
                    .trim()
                    .to_string()
            })
            .collect::<Vec<_>>();
        files.sort();
        Ok(files)
    }

    pub fn guess_shows_title(&self, path: &str) -> Result<String, std::io::Error> {
        Ok(AnimeList::remove_after_last_dash(
            self.get_files(path)?.first().unwrap_or(&"".to_string()),
        ))
    }

    fn extract_title(input: &OsStr) -> Option<String> {
        let input_str = input.to_string_lossy();
        let start_index = input_str.find(']').map(|i| i + 2);
        let end_index = input_str
            .rfind(" [")
            .or_else(|| input_str.rfind(']'))
            .unwrap_or(input_str.len());

        if let (Some(start), end) = (start_index, end_index) {
            Some(input_str[start..end].to_string())
        } else {
            None
        }
    }

    fn remove_after_last_dash(input: &str) -> String {
        if let Some(index) = input.rfind('-') {
            let trimmed = &input[..index].trim();
            return trimmed.to_string();
        }
        input.to_string()
    }

    pub fn list_titles(&self, potential_title: &str) -> Vec<String> {
        self.service.get_title_list(potential_title)
    }

    #[allow(dead_code)]
    fn remove_entry() {}
    #[allow(dead_code)]
    fn refresh_filesystem() {}
}

pub struct Show {
    pub id: i64,
    pub title: String,
    pub sync_service_id: i64,
    pub episode_count: i64,
    pub episodes: Vec<Episode>,
    pub progress: i64,
}

pub struct Episode {
    pub number: i64,
    pub path: String,
}

pub fn create(service: MAL) -> AnimeList {
    let path = "./database.db3";
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
        CREATE TABLE Shows (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT, sync_service_id INTEGER UNIQUE, episode_count INTEGER, progress INTEGER);
        CREATE TABLE Episodes (show_id INTEGER, episode_number INTEGER, path TEXT, PRIMARY KEY (show_id, episode_number), FOREIGN KEY (show_id) REFERENCES Shows(id));
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
