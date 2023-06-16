use std::{path::PathBuf, fs};
use directories::ProjectDirs;

pub(crate) struct Config {
    config_dir: PathBuf,
    data_dir: PathBuf,
}

impl Config {
    pub(crate) fn new(config_dir: PathBuf, data_dir: PathBuf) -> Self {
        Config {
            config_dir,
            data_dir,
        }
    }

    pub(crate) fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

impl Default for Config {
    fn default() -> Self {
        let project_dirs = 
            ProjectDirs::from("", "FakeMichau", "lma").expect("Default project dirs");
        fs::create_dir_all(project_dirs.config_dir()).expect("Default config dir creation");
        fs::create_dir_all(project_dirs.data_dir()).expect("Default data dir creation");
        
        Self { 
            config_dir: {
                if cfg!(debug_assertions) {
                    PathBuf::default()
                } else {
                    project_dirs.config_dir().to_path_buf()
                }
            },
            data_dir: {
                if cfg!(debug_assertions) {
                    PathBuf::default()
                } else {
                    project_dirs.data_dir().to_path_buf()
                }
            },
        }
    }
}