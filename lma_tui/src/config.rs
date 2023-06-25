use std::{path::PathBuf, fs};
use directories::ProjectDirs;
use serde::{Serialize, Deserialize};
use ratatui::style::Color as TermColor;
use lma_lib::{ServiceType, TitleSort};

pub struct Config {
    service: ServiceType,
    data_dir: PathBuf,
    colors: TermColors,
    title_sort: TitleSort,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct ConfigFile {
    service: Option<ServiceType>,
    data_dir: Option<PathBuf>,
    colors: Option<Colors>,
    title_sort: Option<TitleSort>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
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
            text: Some(Color {
                r: 220,
                g: 220,
                b: 220,
            }),
            text_watched: Some(Color {
                r: 70,
                g: 70,
                b: 70,
            }),
            text_deleted: Some(Color { r: 200, g: 0, b: 0 }),
            highlight: Some(Color {
                r: 91,
                g: 174,
                b: 36,
            }),
            highlight_dark: Some(Color {
                r: 25,
                g: 65,
                b: 10,
            }),
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
        create_dirs(config_dir, data_dir)?;
        let config_file = config_dir.join("Settings.toml");

        let default_service = ServiceType::MAL;
        let default_title_sort = TitleSort::LocalIdAsc;
        let default_colors = Colors::default();
        let default_config = ConfigFile {
            data_dir: Some(data_dir.clone()),
            colors: Some(default_colors.clone()),
            service: Some(default_service.clone()),
            title_sort: Some(default_title_sort.clone()),
        };

        let config = if config_file.exists() {
            let data = fs::read_to_string(config_file)
                .map_err(|err| format!("Config can't be read: {err}"))?;
            parse_config_file(&data)?
        } else {
            create_store_default_config(&default_config, config_file)?
        };

        let service = config.service.unwrap_or(default_service);
        let title_sort = config.title_sort.unwrap_or(default_title_sort);
        let data_dir = config
            .data_dir
            .unwrap_or_else(|| default_config.data_dir.expect("Hardcoded value"));
        let colors = config.colors.unwrap_or_else(|| default_colors.clone());
        let term_colors = TermColors {
            text: colors
                .text
                .unwrap_or_else(|| default_colors.text.expect("Hardcoded value"))
                .into(),
            text_watched: colors
                .text_watched
                .unwrap_or_else(|| default_colors.text_watched.expect("Hardcoded value"))
                .into(),
            text_deleted: colors
                .text_deleted
                .unwrap_or_else(|| default_colors.text_deleted.expect("Hardcoded value"))
                .into(),
            highlight: colors
                .highlight
                .unwrap_or_else(|| default_colors.highlight.expect("Hardcoded value"))
                .into(),
            highlight_dark: colors
                .highlight_dark
                .unwrap_or_else(|| default_colors.highlight_dark.expect("Hardcoded value"))
                .into(),
        };

        Ok(Self {
            data_dir,
            colors: term_colors,
            service,
            title_sort,
        })
    }

    pub fn default() -> Result<Self, String> {
        let project_dirs = ProjectDirs::from("", "FakeMichau", "lma").unwrap_or_else(|| {
            ProjectDirs::from_path(PathBuf::new()).expect("Backup default path")
        });

        return if cfg!(debug_assertions) {
            Self::build(&PathBuf::default(), &PathBuf::default())
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

    pub const fn title_sort(&self) -> &TitleSort {
        &self.title_sort
    }
}

fn create_store_default_config(
    default_config: &ConfigFile,
    config_file: PathBuf,
) -> Result<ConfigFile, String> {
    let default_config_str = toml::to_string(default_config)
        .map_err(|err| format!("Can't serialized the config: {err}"))?;
    fs::write(config_file, default_config_str)
        .map_err(|err| format!("Can't save default config: {err}"))?;
    Ok(default_config.clone())
}

fn parse_config_file(data: &str) -> Result<ConfigFile, String> {
    toml::from_str(data)
        .map_err(|err| format!("Can't parse the config: {}", err.message().to_owned()))
}

fn create_dirs(config_dir: &PathBuf, data_dir: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Can't create config directory: {err}"))?;
    fs::create_dir_all(data_dir).map_err(|err| format!("Can't create data directory: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_creation() {
        let default_config = Config::default().is_ok();
        assert!(default_config);
    }

    #[test]
    fn config_build() {
        let config = Config::build(&PathBuf::from("."), &PathBuf::from(".")).is_ok();
        assert!(config);
    }

    #[test]
    fn passing_parse_config() {
        let config_string = "
            service = \"MAL\"
            data_dir = \"\"
            title_sort = \"LocalIdAsc\"
            [colors.text]
            r = 220
            g = 220
            b = 220
            [colors.text_watched]
            r = 70
            g = 70
            b = 70
            [colors.text_deleted]
            r = 200
            g = 0
            b = 0
            [colors.highlight]
            r = 91
            g = 174
            b = 36
            [colors.highlight_dark]
            r = 25
            g = 65
            b = 10
        ";
        let parsed_config_file = parse_config_file(config_string).expect("Parsed config");
        let expected_config_file = ConfigFile {
            service: Some(ServiceType::MAL),
            data_dir: Some(PathBuf::new()),
            colors: Some(Colors {
                text: Some(Color { r:220, g:220, b: 220 }),
                text_watched: Some(Color { r:70, g:70, b: 70 }),
                text_deleted: Some(Color { r:200, g:0, b: 0 }),
                highlight: Some(Color { r:91, g:174, b: 36 }),
                highlight_dark: Some(Color { r:25, g:65, b: 10 }),
            }),
            title_sort: Some(TitleSort::LocalIdAsc),
        };
        assert_eq!(parsed_config_file, expected_config_file);
    }

    #[test]
    fn failing_parse_config_1() {
        let config_string = "
            service = \"trolololo\"
            data_dir = \"\"
            [colors.text]
            r = 220
            g = 220
            b = 220
            [colors.text_watched]
            r = 70
            g = 70
            b = 70
            [colors.text_deleted]
            r = 200
            g = 0
            b = 0
            [colors.highlight]
            r = 91
            g = 174
            b = 36
            [colors.highlight_dark]
            r = 25
            g = 65
            b = 10
        ";
        let parsed_config = parse_config_file(config_string).unwrap_err();
        let correct_error = parsed_config
            .contains("Can't parse the config: unknown variant `trolololo`");
        assert!(correct_error, "Tests wrong service name");
    }

    #[test]
    fn failing_parse_config_2() {
        let config_string = "
            service = \"MAL\"
            data_dir = \"\"
            [colors.text]
            r = 220
            g = 220
            b = 220
            [colors.text_watched]
            r = 70
            g = 70
            [colors.text_deleted]
            r = 200
            g = 0
            b = 0
            [colors.highlight]
            r = 91
            g = 174
            b = 36
            [colors.highlight_dark]
            r = 25
            g = 65
            b = 10
        ";
        let parsed_config = parse_config_file(config_string).unwrap_err();
        let correct_error = parsed_config.contains("Can't parse the config: missing field `b`");
        assert!(correct_error, "Tests missing color channel");
    }

    #[test]
    fn color_conversion() {
        let color = Color {
            r: 69,
            g: 69,
            b: 69,
        };
        let converted_color: TermColor = color.into();
        let expected_termcolor = TermColor::Rgb(69, 69, 69);
        assert_eq!(converted_color, expected_termcolor);
    }
}
