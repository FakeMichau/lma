use std::{path::PathBuf, fs};
use directories::ProjectDirs;
use lma::ServiceType;
use serde::{Serialize, Deserialize};
use ratatui::style::Color as TermColor;

pub struct Config {
    service: ServiceType,
    data_dir: PathBuf,
    colors: TermColors,
}

#[derive(Serialize, Deserialize, Clone)]
struct ConfigFile {
    service: Option<ServiceType>,
    data_dir: Option<PathBuf>,
    colors: Option<Colors>,
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

pub struct TermColors {
    pub text: TermColor,
    pub text_watched: TermColor,
    pub text_deleted: TermColor,
    pub highlight: TermColor,
    pub highlight_dark: TermColor,
}

impl From<Color> for TermColor {
    fn from(val: Color) -> Self {
        Self::Rgb(val.r, val.g, val.b)
    }
}

impl Config {
    pub fn build(config_dir: &PathBuf, data_dir: &PathBuf) -> Result<Self, String> {
        fs::create_dir_all(config_dir).map_err(|err| format!("Can't create config directory: {err}"))?;
        fs::create_dir_all(data_dir).map_err(|err| format!("Can't create data directory: {err}"))?;
        let config_file = config_dir.join("Settings.toml");

        let default_service = if cfg!(debug_assertions) { ServiceType::Local } else { ServiceType::MAL };
        let default_colors = Colors::default();
        let default_config = ConfigFile {
            data_dir: Some(data_dir.clone()),
            colors: Some(default_colors.clone()),
            service: Some(default_service.clone()),
        };

        let config = if config_file.exists() {
            let data = fs::read_to_string(config_file).map_err(|err| format!("Config can't be read: {err}"))?;
            toml::from_str(&data)
                .map_err(|err| format!("Can't parse the config: {}", err.message().to_owned()))?
        } else {
            let default_config_str = toml::to_string(&default_config).map_err(|err| format!("Can't serialized the config: {err}"))?;
            fs::write(config_file, default_config_str).map_err(|err| format!("Can't save default config: {err}"))?;
            default_config.clone()
        };

        let service = config.service.unwrap_or(default_service);
        let data_dir = config.data_dir.unwrap_or_else(|| default_config.data_dir.unwrap());
        let colors = config.colors.unwrap_or_else(|| default_colors.clone());
        let term_colors = TermColors {
            text: colors.text.unwrap_or_else(|| default_colors.text.unwrap()).into(),
            text_watched: colors
                .text_watched
                .unwrap_or_else(|| default_colors.text_watched.unwrap())
                .into(),
            text_deleted: colors
                .text_deleted
                .unwrap_or_else(|| default_colors.text_deleted.unwrap())
                .into(),
            highlight: colors
                .highlight
                .unwrap_or_else(|| default_colors.highlight.unwrap())
                .into(),
            highlight_dark: colors
                .highlight_dark
                .unwrap_or_else(|| default_colors.highlight_dark.unwrap())
                .into(),
        };

        Ok(Self {
            data_dir,
            colors: term_colors,
            service,
        })
    }

    pub fn default() -> Result<Self, String> {
        let project_dirs = 
            ProjectDirs::from("", "FakeMichau", "lma").unwrap_or_else(|| ProjectDirs::from_path(PathBuf::new()).unwrap());
        
        return if cfg!(debug_assertions) {
            Self::build(
                &PathBuf::default(),
                &PathBuf::default(),
            )
        } else {
            Self::build(
                &project_dirs.config_dir().to_path_buf(),
                &project_dirs.data_dir().to_path_buf(),
            )
        };
    }

    pub const fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub const fn colors(&self) -> &TermColors {
        &self.colors
    }

    pub const fn service(&self) -> &ServiceType {
        &self.service
    }
}