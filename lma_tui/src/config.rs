use crossterm::event::KeyCode;
use directories::ProjectDirs;
use lma_lib::{ServiceType, TitleSort};
use ratatui::style::Color as TermColor;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
pub struct Config {
    config_file_path: PathBuf,
    service: ServiceType,
    data_dir: PathBuf,
    colors: TermColors,
    title_sort: TitleSort,
    key_binds: KeyBinds,
    path_instead_of_title: bool,
    autofill_title: bool,
    english_show_titles: bool,
    update_progress_on_start: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct ConfigFile {
    service: Option<ServiceType>,
    data_dir: Option<PathBuf>,
    colors: Option<Colors>,
    title_sort: Option<TitleSort>,
    key_binds: Option<KeyBinds>,
    path_instead_of_title: Option<bool>,
    autofill_title: Option<bool>,
    english_show_titles: Option<bool>,
    update_progress_on_start: Option<bool>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            data_dir: Some(PathBuf::new()),
            colors: Some(Colors::default()),
            service: Some(ServiceType::MAL),
            title_sort: Some(TitleSort::LocalIdAsc),
            key_binds: Some(KeyBinds::default()),
            path_instead_of_title: Some(false),
            autofill_title: Some(true),
            english_show_titles: Some(false),
            update_progress_on_start: Some(false),
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
    text_watched: Option<Color>,
    text_deleted: Option<Color>,
    highlight: Option<Color>,
    highlight_dark: Option<Color>,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            text: Some(Color::new("#DCDCDC")),
            text_watched: Some(Color::new("#464646")),
            text_deleted: Some(Color::new("#C1292E")),
            highlight: Some(Color::new("#F2D202")),
            highlight_dark: Some(Color::new("#161925")),
        }
    }
}

#[derive(Clone)]
pub struct TermColors {
    pub text: TermColor,
    pub text_watched: TermColor,
    pub text_deleted: TermColor,
    pub highlight: TermColor,
    pub highlight_dark: TermColor,
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
    chars.chunks(size)
        .map(|chunk| {
            let color = chunk.iter().collect::<String>();
            let hex_color = if size == 1 { format!("{color}{color}") } else { color };
            u8::from_str_radix(&hex_color, 16)
        })
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|err| format!("Couldn't split hex: {err}"))
}

impl TermColors {
    fn from_colors(
        default_colors: Option<Colors>,
        config_file_colors: Option<Colors>,
    ) -> Result<Self, String> {
        let default_colors = default_colors.expect("Default config has values");
        let colors = config_file_colors.unwrap_or_else(|| default_colors.clone());
        macro_rules! get_color_or_default {
            ($s:ident) => {
                colors
                    .$s
                    .or(default_colors.$s)
                    .expect("Default config has values")
                    .try_into()?
            };
        }
        Ok(Self {
            text: get_color_or_default!(text),
            text_watched: get_color_or_default!(text_watched),
            text_deleted: get_color_or_default!(text_deleted),
            highlight: get_color_or_default!(highlight),
            highlight_dark: get_color_or_default!(highlight_dark),
        })
    }
}

impl Config {
    pub fn build(config_dir: &PathBuf, data_dir: &PathBuf) -> Result<Self, String> {
        create_dirs(config_dir, data_dir)?;
        let config_file_path = config_dir.join("Settings.toml");

        let default_config = ConfigFile {
            data_dir: Some(data_dir.clone()),
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

    pub const fn key_binds(&self) -> &KeyBinds {
        &self.key_binds
    }

    pub const fn path_instead_of_title(&self) -> bool {
        self.path_instead_of_title
    }

    pub const fn autofill_title(&self) -> bool {
        self.autofill_title
    }

    pub const fn english_show_titles(&self) -> bool {
        self.english_show_titles
    }

    pub const fn update_progress_on_start(&self) -> bool {
        self.update_progress_on_start
    }

    pub const fn config_file_path(&self) -> &PathBuf {
        &self.config_file_path
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
        colors: TermColors::from_colors(default_config.colors, config_file.colors)?,
        data_dir: get_setting_or_default!(data_dir),
        service: get_setting_or_default!(service),
        title_sort: get_setting_or_default!(title_sort),
        key_binds: get_setting_or_default!(key_binds),
        path_instead_of_title: get_setting_or_default!(path_instead_of_title),
        autofill_title: get_setting_or_default!(autofill_title),
        english_show_titles: get_setting_or_default!(english_show_titles),
        update_progress_on_start: get_setting_or_default!(update_progress_on_start),
    })
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
            path_instead_of_title = false
            autofill_title = true
            english_show_titles = true
            update_progress_on_start = true
            [colors.text]
            hex = \"#DCDCDC\"
            [colors.text_watched]
            hex = \"#464646\"
            [colors.text_deleted]
            hex = \"#C80000\"
            [colors.highlight]
            hex = \"#5BAE24\"
            [colors.highlight_dark]
            hex = \"#19410A\"
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
                text_watched: Some(Color {
                    hex: String::from("#464646"),
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
            autofill_title: Some(true),
            english_show_titles: Some(true),
            update_progress_on_start: Some(true),
        };
        assert_eq!(parsed_config_file, expected_config_file);
    }

    #[test]
    fn failing_parse_config_1() {
        let config_string = "
            service = \"trolololo\"
            data_dir = \"\"
            [colors.text]
            hex = \"#DCDCDC\"
            [colors.text_watched]
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
            he = \"#DCDCDC\"
            [colors.text_watched]
            hex = \"#464646\"
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
        let color = Color { hex: "#FF0000".to_string() };
        let term_color: Result<TermColor, String> = color.try_into();
        assert!(term_color.is_ok());
        assert_eq!(term_color.unwrap(), TermColor::Rgb(255, 0, 0));
    }

    #[test]
    fn try_from_color_no_hash() {
        let color = Color { hex: "420072".to_string() };
        let term_color: Result<TermColor, String> = color.try_into();
        assert!(term_color.is_ok());
        assert_eq!(term_color.unwrap(), TermColor::Rgb(66, 0, 114));
    }

    #[test]
    fn try_from_invalid_hex() {
        let color = Color { hex: "#FF00".to_string() };
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
