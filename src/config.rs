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
    text: Option<Color>,
    text_watched: Option<Color>,
    text_deleted: Option<Color>,
    highlight: Option<Color>,
    highlight_dark: Option<Color>,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            text: Some(Color { r:220, g:220, b: 220 }),
            text_watched: Some(Color { r:70, g:70, b: 70 }),
            text_deleted: Some(Color { r:200, g:0, b: 0 }),
            highlight: Some(Color { r:91, g:174, b: 36 }),
            highlight_dark: Some(Color { r:25, g:65, b: 10 }),
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

impl Into<TermColor> for Color {
    fn into(self) -> TermColor {
        TermColor::Rgb(self.r, self.g, self.b)
    }
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
            toml::from_str(&data).map_err(|err| err.message().to_owned()).expect("Can't parse the config")
        } else {
            let default_config_str = toml::to_string(&default_config).expect("Config serialized");
            fs::write(config_file, default_config_str).expect("Default config creation");
            default_config.clone()
        };

        let data_dir = config.data_dir.unwrap_or(default_config.data_dir.unwrap());
        let default_colors = Colors::default();
        let colors = config.colors.unwrap_or(default_colors.clone());
        let term_colors = TermColors {
            text: colors.text.unwrap_or(default_colors.text.unwrap()).into(),
            text_watched: colors.text_watched.unwrap_or(default_colors.text_watched.unwrap()).into(),
            text_deleted: colors.text_deleted.unwrap_or(default_colors.text_deleted.unwrap()).into(),
            highlight: colors.highlight.unwrap_or(default_colors.highlight.unwrap()).into(),
            highlight_dark: colors.highlight_dark.unwrap_or(default_colors.highlight_dark.unwrap()).into(),
        };

        Config {
            data_dir,
            colors: term_colors
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