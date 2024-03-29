use crate::ui::main_menu::HeaderType;
use crossterm::event::KeyCode;
use directories::ProjectDirs;
use lma_lib::{ServiceType, TitleSort};
use ratatui::style::Color as TermColor;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[allow(clippy::struct_excessive_bools)]
#[allow(clippy::struct_field_names)]
pub struct Config {
    service: ServiceType,
    pub config_file_path: PathBuf,
    pub data_dir: PathBuf,
    pub colors: TermColors,
    pub title_sort: TitleSort,
    pub key_binds: KeyBinds,
    pub headers: Headers,
    pub path_instead_of_title: bool,
    pub update_progress_on_start: bool,
    pub relative_episode_score: bool,
    pub precise_score: bool,
    pub autofill_title: bool,
    pub english_show_titles: bool,
}

pub struct Headers {
    pub shows: Vec<HeaderType>,
    pub episodes: Vec<HeaderType>,
}

impl TryFrom<HeadersFile> for Headers {
    type Error = String;

    fn try_from(value: HeadersFile) -> Result<Self, String> {
        let get_header_vec = |str: &str| -> Result<Vec<HeaderType>, String> {
            let mut header_vec: Vec<HeaderType> = Vec::new();
            for header in str.split(',') {
                let header: Result<_, String> = match header.trim() {
                    "title" => Ok(if header_vec.contains(&HeaderType::Title) {
                        None
                    } else {
                        Some(HeaderType::title())
                    }),
                    "number" => Ok(Some(HeaderType::number())),
                    "extra" => Ok(Some(HeaderType::extra())),
                    "score" => Ok(Some(HeaderType::score())),
                    other => Err(format!("Trying to parse non-existent header: {other}"))?,
                };
                let valid_header = header?;
                if let Some(header) = valid_header {
                    header_vec.push(header);
                }
            }
            Ok(header_vec)
        };

        Ok(Self {
            shows: get_header_vec(&value.shows)?,
            episodes: get_header_vec(&value.episodes)?,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct HeadersFile {
    shows: String,
    episodes: String,
}

impl Default for HeadersFile {
    fn default() -> Self {
        Self {
            shows: String::from("title"),
            episodes: String::from("number, title, score, extra"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct ConfigFile {
    service: Option<ServiceType>,
    data_dir: Option<PathBuf>,
    colors: Option<Colors>,
    title_sort: Option<TitleSort>,
    key_binds: Option<KeyBinds>,
    headers: Option<HeadersFile>,
    path_instead_of_title: Option<bool>,
    update_progress_on_start: Option<bool>,
    relative_episode_score: Option<bool>,
    precise_score: Option<bool>,
    autofill_title: Option<bool>,
    english_show_titles: Option<bool>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            data_dir: Some(PathBuf::new()),
            colors: Some(Colors::default()),
            service: Some(ServiceType::MAL),
            title_sort: Some(TitleSort::LocalIdAsc),
            key_binds: Some(KeyBinds::default()),
            headers: Some(HeadersFile::default()),
            precise_score: Some(true),
            english_show_titles: Some(false),
            autofill_title: Some(true),
            path_instead_of_title: Some(false),
            update_progress_on_start: Some(false),
            relative_episode_score: Some(false),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct KeyBinds {
    pub move_up: KeyCode,
    pub move_down: KeyCode,
    pub backwards: KeyCode,
    pub forwards: KeyCode,
    pub confirmation: KeyCode,
    pub close: KeyCode,
    pub delete: KeyCode,
    pub quit: KeyCode,
    pub enter_inputting: KeyCode,
    pub new_show: KeyCode,
    pub new_episode: KeyCode,
    pub progress_inc: KeyCode,
    pub progress_dec: KeyCode,
    pub login: KeyCode,
}

impl Default for KeyBinds {
    fn default() -> Self {
        Self {
            move_up: KeyCode::Up,
            move_down: KeyCode::Down,
            backwards: KeyCode::Left,
            forwards: KeyCode::Right,
            confirmation: KeyCode::Enter,
            close: KeyCode::Esc,
            delete: KeyCode::Delete,
            quit: KeyCode::Char('q'),
            enter_inputting: KeyCode::Char('e'),
            new_show: KeyCode::Char('n'),
            new_episode: KeyCode::Char('e'),
            progress_inc: KeyCode::Char('.'),
            progress_dec: KeyCode::Char(','),
            login: KeyCode::Char('l'),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Color {
    hex: String,
}

impl Color {
    fn new(hex: &str) -> Self {
        Self {
            hex: String::from(hex),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            hex: "#000000".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Colors {
    text: Option<Color>,
    text_deleted: Option<Color>,
    highlight: Option<Color>,
    highlight_dark: Option<Color>,
    secondary: Option<Color>,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            text: Some(Color::new("#DCDCDC")),
            text_deleted: Some(Color::new("#C1292E")),
            highlight: Some(Color::new("#F2D202")),
            highlight_dark: Some(Color::new("#161925")),
            secondary: Some(Color::new("#F45D01")),
        }
    }
}

#[derive(Clone)]
pub struct TermColors {
    pub text: TermColor,
    pub text_deleted: TermColor,
    pub highlight: TermColor,
    pub highlight_dark: TermColor,
    pub secondary: TermColor,
}

impl TryFrom<Color> for TermColor {
    type Error = String;

    fn try_from(color: Color) -> Result<Self, String> {
        let hex = color.hex.trim_start_matches('#');
        let chars: Vec<_> = hex.chars().collect();
        let colors = match hex.len() {
            3 => split_hex(&chars, 1)?,
            6 => split_hex(&chars, 2)?,
            _ => return Err(format!("Wrong hex value: {}", color.hex)),
        };
        Ok(Self::Rgb(colors[0], colors[1], colors[2]))
    }
}

fn split_hex(chars: &[char], size: usize) -> Result<Vec<u8>, String> {
    chars
        .chunks(size)
        .map(|chunk| {
            let color = chunk.iter().collect::<String>();
            let hex_color = if size == 1 {
                format!("{color}{color}")
            } else {
                color
            };
            u8::from_str_radix(&hex_color, 16)
        })
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|err| format!("Couldn't split hex: {err}"))
}

impl TryFrom<Colors> for TermColors {
    type Error = String;

    fn try_from(colors: Colors) -> Result<Self, String> {
        let unwrap_and_try_into = |color: Option<Color>| -> Result<TermColor, String> {
            color.ok_or("No value")?.try_into()
        };

        Ok(Self {
            text: unwrap_and_try_into(colors.text)?,
            text_deleted: unwrap_and_try_into(colors.text_deleted)?,
            highlight: unwrap_and_try_into(colors.highlight)?,
            highlight_dark: unwrap_and_try_into(colors.highlight_dark)?,
            secondary: unwrap_and_try_into(colors.secondary)?,
        })
    }
}

impl Config {
    pub fn build(config_dir: &Path, data_dir: &Path) -> Result<Self, String> {
        create_dirs(config_dir, data_dir)?;
        let config_file_path = config_dir.join("Settings.toml");

        let default_config = ConfigFile {
            data_dir: Some(data_dir.to_path_buf()),
            ..Default::default()
        };

        parse_config(config_file_path, default_config)
    }

    pub fn create_personalized(&self, service: ServiceType) -> Result<(), String> {
        let default_config = ConfigFile {
            data_dir: Some(self.data_dir.clone()),
            service: Some(service),
            ..Default::default()
        };
        let default_config_str = toml::to_string(&default_config)
            .map_err(|err| format!("Can't serialized the config: {err}"))?;
        fs::write(&self.config_file_path, default_config_str)
            .map_err(|err| format!("Can't save default config: {err}"))
    }

    pub fn default() -> Result<Self, String> {
        let project_dirs = ProjectDirs::from("", "FakeMichau", "lma").unwrap_or_else(|| {
            ProjectDirs::from_path(PathBuf::new()).expect("Backup default path")
        });

        return if cfg!(feature = "portable") {
            match env::current_exe() {
                Ok(exe_path) => match exe_path.parent() {
                    Some(path) => Self::build(path, path),
                    None => Err(String::from("Can't get path to the current directory"))?,
                },
                Err(e) => Err(format!("Can't write to the current directory: {e}"))?,
            }
        } else if cfg!(debug_assertions) {
            Self::build(&PathBuf::default(), &PathBuf::default())
        } else {
            Self::build(project_dirs.config_dir(), project_dirs.data_dir())
        };
    }

    pub const fn service(&self) -> &ServiceType {
        &self.service
    }
}

fn parse_config(config_file_path: PathBuf, default_config: ConfigFile) -> Result<Config, String> {
    let config_file = if config_file_path.exists() {
        let data = fs::read_to_string(&config_file_path)
            .map_err(|err| format!("Config can't be read: {err}"))?;
        parse_config_file(&data)?
    } else {
        default_config.clone()
    };

    macro_rules! get_setting_or_default {
        ($s:ident) => {
            config_file
                .$s
                .or(default_config.$s)
                .expect("Default config has values")
        };
    }

    Ok(Config {
        config_file_path,
        colors: get_setting_or_default!(colors).try_into()?,
        data_dir: get_setting_or_default!(data_dir),
        service: get_setting_or_default!(service),
        title_sort: get_setting_or_default!(title_sort),
        key_binds: get_setting_or_default!(key_binds),
        headers: get_setting_or_default!(headers).try_into()?,
        precise_score: get_setting_or_default!(precise_score),
        autofill_title: get_setting_or_default!(autofill_title),
        english_show_titles: get_setting_or_default!(english_show_titles),
        path_instead_of_title: get_setting_or_default!(path_instead_of_title),
        update_progress_on_start: get_setting_or_default!(update_progress_on_start),
        relative_episode_score: get_setting_or_default!(relative_episode_score),
    })
}

fn parse_config_file(data: &str) -> Result<ConfigFile, String> {
    toml::from_str(data)
        .map_err(|err| format!("Can't parse the config: {}", err.message().to_owned()))
}

fn create_dirs(config_dir: &Path, data_dir: &Path) -> Result<(), String> {
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
            path_instead_of_title = false
            update_progress_on_start = true
            relative_episode_score = true
            autofill_title = true
            english_show_titles = true
            precise_score = true
            [headers]
            shows = \"title\"
            episodes = \"title\"
            [colors.text]
            hex = \"#DCDCDC\"
            [colors.text_deleted]
            hex = \"#C80000\"
            [colors.highlight]
            hex = \"#5BAE24\"
            [colors.highlight_dark]
            hex = \"#19410A\"
            [colors.secondary]
            hex = \"#3C87CD\"
            [key_binds]
            move_up = \"Up\"
            move_down = \"Down\"
            backwards = \"Left\"
            forwards = \"Right\"
            confirmation = \"Enter\"
            delete = \"Delete\"
            close = \"Esc\"
            [key_binds.quit]
            Char = \"q\"
            [key_binds.enter_inputting]
            Char = \"e\"
            [key_binds.new_show]
            Char = \"n\"
            [key_binds.new_episode]
            Char = \"e\"
            [key_binds.progress_inc]
            Char = \".\"
            [key_binds.progress_dec]
            Char = \",\"
            [key_binds.login]
            Char = \"l\"
        ";
        let parsed_config_file = parse_config_file(config_string).expect("Parsed config");
        let expected_config_file = ConfigFile {
            service: Some(ServiceType::MAL),
            data_dir: Some(PathBuf::new()),
            colors: Some(Colors {
                text: Some(Color {
                    hex: String::from("#DCDCDC"),
                }),
                text_deleted: Some(Color {
                    hex: String::from("#C80000"),
                }),
                highlight: Some(Color {
                    hex: String::from("#5BAE24"),
                }),
                highlight_dark: Some(Color {
                    hex: String::from("#19410A"),
                }),
                secondary: Some(Color {
                    hex: String::from("#3C87CD"),
                }),
            }),
            title_sort: Some(TitleSort::LocalIdAsc),
            key_binds: Some(KeyBinds {
                move_up: KeyCode::Up,
                move_down: KeyCode::Down,
                backwards: KeyCode::Left,
                forwards: KeyCode::Right,
                confirmation: KeyCode::Enter,
                close: KeyCode::Esc,
                delete: KeyCode::Delete,
                quit: KeyCode::Char('q'),
                enter_inputting: KeyCode::Char('e'),
                new_show: KeyCode::Char('n'),
                new_episode: KeyCode::Char('e'),
                progress_inc: KeyCode::Char('.'),
                progress_dec: KeyCode::Char(','),
                login: KeyCode::Char('l'),
            }),
            path_instead_of_title: Some(false),
            update_progress_on_start: Some(true),
            relative_episode_score: Some(true),
            english_show_titles: Some(true),
            autofill_title: Some(true),
            precise_score: Some(true),
            headers: Some(HeadersFile {
                shows: String::from("title"),
                episodes: String::from("title"),
            }),
        };
        assert_eq!(parsed_config_file, expected_config_file);
    }

    #[test]
    fn failing_parse_config_1() {
        let config_string = "
            service = \"trolololo\"
            data_dir = \"\"
            [colors.text]
            hex = \"#464646\"
            [colors.text_deleted]
            hex = \"#C80000\"
            [colors.highlight]
            hex = \"#5BAE24\"
            [colors.highlight_dark]
            hex = \"#19410A\"
        ";
        let parsed_config = parse_config_file(config_string).unwrap_err();
        let correct_error =
            parsed_config.contains("Can't parse the config: unknown variant `trolololo`");
        assert!(correct_error, "Tests wrong service name");
    }

    #[test]
    fn failing_parse_config_2() {
        let config_string = "
            service = \"MAL\"
            data_dir = \"\"
            [colors.text]
            he = \"#464646\"
            [colors.text_deleted]
            hex = \"#C80000\"
            [colors.highlight]
            hex = \"#5BAE24\"
            [colors.highlight_dark]
            hex = \"#19410A\"
        ";
        let parsed_config = parse_config_file(config_string).unwrap_err();
        let correct_error = parsed_config.contains("Can't parse the config: missing field `hex`");
        assert!(correct_error, "Tests missing color channel");
    }

    #[test]
    fn color_conversion() {
        let color = Color {
            hex: String::from("#454545"),
        };
        let converted_color: TermColor = color.try_into().expect("Hardcoded color");
        let expected_termcolor = TermColor::Rgb(69, 69, 69);
        assert_eq!(converted_color, expected_termcolor);
    }

    #[test]
    fn failing_color_conversion() {
        let color = Color {
            hex: String::from("#45454"),
        };
        let converted_color: Result<TermColor, _> = color.try_into();
        let correct_error = converted_color.unwrap_err().contains("Wrong hex value");
        assert!(correct_error, "Tests incorrect hex value");
    }

    #[test]
    fn try_from_valid_color() {
        let color = Color {
            hex: "#FF0000".to_string(),
        };
        let term_color: Result<TermColor, String> = color.try_into();
        assert!(term_color.is_ok());
        assert_eq!(term_color.unwrap(), TermColor::Rgb(255, 0, 0));
    }

    #[test]
    fn try_from_color_no_hash() {
        let color = Color {
            hex: "420072".to_string(),
        };
        let term_color: Result<TermColor, String> = color.try_into();
        assert!(term_color.is_ok());
        assert_eq!(term_color.unwrap(), TermColor::Rgb(66, 0, 114));
    }

    #[test]
    fn try_from_invalid_hex() {
        let color = Color {
            hex: "#FF00".to_string(),
        };
        let term_color: Result<TermColor, String> = color.try_into();
        assert!(term_color.is_err());
        assert_eq!(term_color.unwrap_err(), "Wrong hex value: #FF00");
    }

    #[test]
    fn split_hex_three_chars() {
        let chars: Vec<char> = vec!['F', 'F', '0'];
        let colors = split_hex(&chars, 1);
        assert!(colors.is_ok());
        assert_eq!(colors.unwrap(), vec![255, 255, 0]);
    }

    #[test]
    fn split_hex_six_chars() {
        let chars: Vec<char> = vec!['F', 'F', '0', '0', '0', '0'];
        let colors = split_hex(&chars, 2);
        assert!(colors.is_ok());
        assert_eq!(colors.unwrap(), vec![255, 0, 0]);
    }

    #[test]
    fn split_wrong_hex() {
        let chars: Vec<char> = vec!['R', 'F', '0', '0', '0', '0'];
        let colors = split_hex(&chars, 2);
        assert!(colors.is_err());
        assert!(colors.unwrap_err().contains("Couldn't split hex"));
    }
}
