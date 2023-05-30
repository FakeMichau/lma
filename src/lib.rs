use std::collections::HashMap;
use rusqlite::{Connection, Result};

pub struct AnimeList {
    db_connection: Connection
}

impl AnimeList {
    pub fn get_list(&self) -> Result<HashMap<i64, Show>> {
        let mut stmt = self.db_connection.prepare("
            SELECT Shows.id, Shows.progress, Shows.episode_count, Shows.sync_service_id,
            Episodes.episode_number, Episodes.path
            FROM Shows
            JOIN Episodes ON Shows.id = Episodes.show_id;
        ")?;
        let mut shows: HashMap<i64, Show> = HashMap::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let show_id: i64 = row.get(0)?;
            let progress: i64 = row.get(1)?;
            let episode_count: i64 = row.get(2)?;
            let sync_service_id: i64 = row.get(3)?;
            let episode_number: i64 = row.get(4)?;
            let path: String = row.get(5)?;

            let show = shows.entry(show_id).or_insert_with(|| Show {
                progress,
                episode_count,
                episodes: HashMap::new(),
                sync_service_id,
            });
            show.episodes.insert(episode_number, path);
        }
        Ok(shows)
    }
    fn add_entry() {

    }
    fn remove_entry() {
        
    }
    fn refresh_filesystem() {
        
    }
}

#[derive(Debug)]
pub struct Show {
    progress: i64,
    episode_count: i64,
    episodes: HashMap<i64, String>,
    sync_service_id: i64
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
    match db_connection.execute(
        "
        CREATE TABLE Shows (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            progress INTEGER,
            sync_service_id INTEGER,
            episode_count INTEGER
        );
        ", ()
    ) {
            Ok(_) => println!("Table Shows created"),
            Err(why) => {
                if why.to_string().contains("already exists") {
                    println!("Table creation failed: table Shows already exists");
                } else {
                    eprintln!("Table creation failed: {}", why.to_string());
                }
            }
    };    
    match db_connection.execute(
        "
        CREATE TABLE Episodes (
            show_id INTEGER,
            episode_number INTEGER,
            path TEXT,
            PRIMARY KEY (show_id, episode_number),
            FOREIGN KEY (show_id) REFERENCES Shows(id)
        );
        ", ()
    ) {
            Ok(_) => println!("Table Episodes created"),
            Err(why) => {
                if why.to_string().contains("already exists") {
                    println!("Table creation failed: table Episodes already exists");
                } else {
                    eprintln!("Table creation failed: {}", why.to_string());
                }
            }
    };
    AnimeList { db_connection }
}
