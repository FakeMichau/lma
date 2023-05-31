use std::collections::HashMap;
use rusqlite::{Connection, Result, params};

pub struct AnimeList {
    db_connection: Connection
}

impl AnimeList {
    pub fn get_list(&self) -> Result<Vec<(i64, Show)>> {
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

            let show = shows.entry(show_id).or_insert_with(|| Show {
                title,
                progress,
                episode_count,
                episodes: Vec::new(),
                sync_service_id,
            });
            if episode_number != -1 {
                show.episodes.push((episode_number, path));
            }
        }
        let mut shows: Vec<(i64, Show)> = shows.into_iter().collect();
        shows.sort_by_key(|(k, _)| *k);
        shows.iter_mut().for_each(|(_, show)| {
            show.episodes.sort_by_key(|(k, _)| *k);
        });
        Ok(shows)
    }
    pub fn add_show(&self, title: &str, sync_service_id: i64, episode_count: i64, progress: i64) -> Result<(), String> {
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
        self.db_connection.execute(
            "REPLACE INTO Episodes (show_id, episode_number, path) VALUES (?1, ?2, ?3)", 
            params![
                show_id,
                episode_number,
                path,
            ]
        ).map(|_| ()).map_err(|e| e.to_string())
    }
    fn remove_entry() {
        
    }
    fn refresh_filesystem() {
        
    }
}

pub struct Show {
    pub title: String,
    pub sync_service_id: i64,
    pub episode_count: i64,
    pub episodes: Vec<(i64, String)>,
    pub progress: i64,
}

pub fn create() -> AnimeList {
    let path = "./database.db3";
    let db_connection = match Connection::open(path) {
        Ok(conn) => conn,
        Err(why) => panic!("Cry - {}", why),
    };
    if db_connection.is_readonly(rusqlite::DatabaseName::Main).expect("shouldn't realistically return an error") {
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
    AnimeList { db_connection }
}
