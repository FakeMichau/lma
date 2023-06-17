use std::{path::PathBuf, fs};
use directories::ProjectDirs;
use serde::{Serialize, Deserialize};
use ratatui::style::Color as TermColor;

pub(crate) struct Config {
    data_dir: PathBuf,
    colors: TermColors
}

#[derive(Serialize, Deserialize, Clone)]
struct ConfigFile {
    data_dir: Option<PathBuf>,
    colors: Option<Colors>
}

#[derive(Serialize, Deserialize, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Serialize, Deserialize, Clone)]
struct Colors {
    text: Color,
    text_watched: Color,
    text_deleted: Color,
    highlight: Color,
    highlight_dark: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            text: Color { r:220, g:220, b: 220 },
            text_watched: Color { r:70, g:70, b: 70 },
            text_deleted: Color { r:200, g:0, b: 0 },
            highlight: Color { r:91, g:174, b: 36 },
            highlight_dark: Color { r:25, g:65, b: 10 },
        }
    }
}

pub(crate) struct TermColors {
    pub(crate) text: TermColor,
    pub(crate) text_watched: TermColor,
    pub(crate) text_deleted: TermColor,
    pub(crate) highlight: TermColor,
    pub(crate) highlight_dark: TermColor,
}

impl Config {
    pub(crate) fn new(config_dir: PathBuf, data_dir: PathBuf) -> Self {
        fs::create_dir_all(&config_dir).expect("Config dir creation");
        fs::create_dir_all(&data_dir).expect("Data dir creation");
        let config_file = config_dir.join("Settings.toml");

        let default_config = ConfigFile {
            data_dir: Some(data_dir),
            colors: Some(Colors::default())
        };

        let config = if config_file.exists() {
            let data = fs::read_to_string(config_file).expect("Config can't be read");
            toml::from_str(&data).expect("Can't parse the config")
        } else {
            let default_config_str = toml::to_string(&default_config).expect("Config serialized");
            fs::write(config_file, default_config_str).expect("Default config creation");
            default_config.clone()
        };

        let colors = config.colors.unwrap_or(default_config.colors.unwrap());
        let mapped_colors = TermColors {
            text: TermColor::Rgb(colors.text.r, colors.text.g, colors.text.b),
            text_watched: TermColor::Rgb(colors.text_watched.r, colors.text_watched.g, colors.text_watched.b),
            text_deleted: TermColor::Rgb(colors.text_deleted.r, colors.text_deleted.g, colors.text_deleted.b),
            highlight: TermColor::Rgb(colors.highlight.r, colors.highlight.g, colors.highlight.b),
            highlight_dark: TermColor::Rgb(colors.highlight_dark.r, colors.highlight_dark.g, colors.highlight_dark.b),
        };

        Config {
            data_dir: config.data_dir.unwrap_or(default_config.data_dir.unwrap()),
            colors: mapped_colors
        }
    }

    pub(crate) fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub(crate) fn colors(&self) -> &TermColors {
        &self.colors
    }
}

impl Default for Config {
    fn default() -> Self {
        let project_dirs = 
            ProjectDirs::from("", "FakeMichau", "lma").expect("Default project dirs");
        
        return if cfg!(debug_assertions) {
            Config::new(
                PathBuf::default(),
                PathBuf::default()
            )
        } else {
            Config::new(
                project_dirs.config_dir().to_path_buf(),
                project_dirs.data_dir().to_path_buf()
            )
        };
    }
}