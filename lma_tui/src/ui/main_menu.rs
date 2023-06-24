use std::path::PathBuf;
use std::process::{Command, Stdio};
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, text::{Span, Line}};
use tokio::runtime::Runtime;
use lma_lib::{AnimeList, Show, Episode, Service};
use crate::app::App;
use super::{SelectionDirection, FocusedWindow, popup::insert_show::InsertState};

pub struct StatefulList {
    shows_state: ListState,
    episodes_state: ListState,
    selecting_episode: bool,
    selected_local_id: i64,
    list_cache: Vec<Show>,
}

impl StatefulList {
    pub fn new<T: Service>(shows: &AnimeList<T>) -> Result<Self, String> {
        let list_cache = shows.get_list().map_err(|e| e.to_string())?;
        Ok(Self {
            shows_state: ListState::default(),
            selecting_episode: false,
            episodes_state: ListState::default(),
            selected_local_id: 0,
            list_cache,
        })
    }

    pub fn delete<T: Service>(&mut self, shows: &AnimeList<T>) -> Result<(), String> {
        if self.selecting_episode {
            // todo: delete just an episode
        } else {
            shows.remove_entry(self.selected_local_id)?;
            self.update_cache(shows)?;
            self.update_selected_id(self.shows_state.selected().unwrap_or_default());
        }
        Ok(())
    }

    pub fn move_selection<T: Service>(&mut self, direction: &SelectionDirection, shows: &AnimeList<T>) -> Result<(), String> {
        if self.selecting_episode {
            self.move_episode_selection(direction);
        } else {
            self.update_cache(shows)?;
            self.move_show_selection(direction);
        }
        Ok(())
    }

    fn move_episode_selection(&mut self, direction: &SelectionDirection) {
        let Some(selected_show) = self.selected_show() else {
            return
        };
        let episodes_len = selected_show.episodes.len();
        let i = Self::select_element(
            episodes_len,
            self.episodes_state.selected(),
            direction,
        );
        self.episodes_state.select(Some(i));
    }

    fn move_show_selection(&mut self, direction: &SelectionDirection) {
        let i = Self::select_element(self.list_cache.len(), self.shows_state.selected(), direction);
        self.shows_state.select(Some(i));
        self.update_selected_id(i);
    }

    fn update_selected_id(&mut self, index: usize) {
        self.selected_local_id = self.list_cache.get(index).map_or(0, |show| show.local_id);
    }

    pub fn selected_show(&self) -> Option<&Show> {
        self.shows_state
            .selected()
            .and_then(|index| self.list_cache.get(index))
    }

    pub fn move_progress<T: Service>(&mut self, direction: &SelectionDirection, shows: &mut AnimeList<T>, rt: &Runtime) -> Result<(), String> {
        let Some(selected_show) = self.selected_show() else {
            return Ok(())
        };
        let offset = match direction {
            SelectionDirection::Next => 1,
            SelectionDirection::Previous => -1,
        };
        let progress = selected_show.progress + offset;
        shows.set_progress(selected_show.local_id, progress)
            .expect("Set local progress");
        rt.block_on(
        shows.service.set_progress(
            u32::try_from(selected_show.service_id).map_err(|e| e.to_string())?, 
            u32::try_from(progress).map_err(|e| e.to_string())?
        ))?;
        self.update_cache(shows)?;
        Ok(())
    }

    pub fn select(&mut self) {
        if self.selecting_episode {
            // navigating inside the episodes tab
            let selected_episode = self
                .episodes_state
                .selected()
                .unwrap_or_default();

            let path = self
                .list_cache
                .iter()
                .find(|show| show.local_id == self.selected_local_id)
                .as_ref()
                .map_or_else(PathBuf::new, |show| {
                    show.episodes
                        .get(selected_episode)
                        .map(|episode| episode.path.clone())
                        .unwrap_or_default()
                });
            
            if path.exists() && cfg!(target_os = "linux") {
                _ = Command::new("xdg-open")
                    .arg(path)
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .spawn();
            }
        } else if let Some(selected_id) = self.shows_state.selected() {
            if let Some(show) = self.list_cache.get(selected_id) {
                if !show.episodes.is_empty() {
                    let index = show
                        .episodes
                        .iter()
                        .position(|episode| episode.number == show.progress)
                        .map_or(0, |pos| (pos + 1) % show.episodes.len());
        
                    self.episodes_state.select(Some(index));
                    self.selecting_episode = true;
                }
            }
        }
    }
    pub fn unselect(&mut self) {
        self.episodes_state.select(None);
        self.selecting_episode = false;
    }

    const fn select_element(
        list_length: usize,
        selected_element: Option<usize>,
        direction: &SelectionDirection,
    ) -> usize {
        if list_length == 0 {
            return 0;
        }
        match selected_element {
            Some(i) => match direction {
                SelectionDirection::Next => (i + 1) % list_length,
                SelectionDirection::Previous => (list_length + i - 1) % list_length,
            },
            None => 0,
        }
    }

    pub fn update_cache<T: Service>(&mut self, shows: &AnimeList<T>) -> Result<(), String> {
        self.list_cache = shows.get_list().map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn build<B: Backend, T: Service>(frame: &mut Frame<'_, B>, app: &mut App<T>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Max(1)].as_ref())
        .split(frame.size());

    // Split the bigger chunk into halves
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    let items: Vec<_> = app
        .list_state
        .list_cache
        .iter()
        .map(|show| {
            ListItem::new(show.title.clone())
                .style(Style::default().fg(app.config.colors().text))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .bg(app.config.colors().highlight)
                .add_modifier(Modifier::BOLD),
        );

    // Iterate through all elements in the `items` app
    let episodes: Vec<ListItem> = app
        .list_state
        .list_cache
        .iter()
        .filter(|show| show.local_id == app.list_state.selected_local_id)
        .flat_map(|show| {
            let mut temp: Vec<ListItem> = Vec::new();
            for episode in &show.episodes {
                let mut style = Style::default();
                if episode.number <= show.progress { 
                    style = style.fg(app.config.colors().text_watched);
                }
                if episode.file_deleted {
                    style = style.fg(app.config.colors().text_deleted);
                }
                // maybe make a config for that in the future
                let episode_display_name = if episode.title.is_empty() {
                    episode.path.file_name().unwrap_or_default().to_string_lossy().into()
                } else {
                    episode.title.clone()
                };
                let mut new_episode = ListItem::new(format!("{} {}", episode.number, episode_display_name)).style(style);
                append_extra_info(
                    &mut new_episode,
                    main_chunks[1].width,
                    episode,
                    episode_display_name,
                    style,
                );
                temp.push(new_episode);
            }
            temp
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let episodes = List::new(episodes)
        .block(Block::default().borders(Borders::ALL).title("Episodes"))
        .highlight_style(
            Style::default()
                .bg(app.config.colors().highlight)
                .add_modifier(Modifier::BOLD),
        );

    let help = build_help(
        &app.focused_window,
        &app.insert_popup.state,
        &app.insert_episode_popup.state,
        app.config.colors().highlight_dark,
    );

    // We can now render the item list
    frame.render_stateful_widget(items, main_chunks[0], &mut app.list_state.shows_state);
    frame.render_stateful_widget(
        episodes,
        main_chunks[1],
        &mut app.list_state.episodes_state,
    );
    frame.render_widget(help, chunks[1]);
}

fn append_extra_info(
    new_episode: &mut ListItem<'_>,
    space: u16,
    episode: &Episode,
    episode_display_name: String,
    style: Style,
) {
    if !episode.recap && !episode.filler {
        return
    }
    let recap_text = "RECAP";
    let filler_text = "FILLER";
    let text = if episode.recap && episode.filler {
        format!("{recap_text}/{filler_text}")
    } else if episode.recap {
        recap_text.to_string()
    } else {
        filler_text.to_string()
    };
    let trunc_symbol = "... ";
    let trunc_symbol_len = u16::try_from(trunc_symbol.len()).unwrap_or_default();
    let episode_width = new_episode.width();
    let offset = u16::try_from(text.len()).unwrap_or_default() + u16::try_from(episode.number.checked_ilog10().unwrap_or(0) + 1).unwrap_or_default() + 3;
    if episode_width > (space - offset - trunc_symbol_len + 3).into() {
        let mut trunc_episode_display_name = episode_display_name;
        trunc_episode_display_name.truncate((space - offset - trunc_symbol_len).into());
        trunc_episode_display_name += trunc_symbol;
        trunc_episode_display_name += text.as_str();
        *new_episode = ListItem::new(format!("{} {}", episode.number, trunc_episode_display_name))
            .style(style);
    } else {
        let mut trunc_episode_display_name =
            format!("{:1$}", episode_display_name, (space - offset).into());
        trunc_episode_display_name += text.as_str();
        *new_episode = ListItem::new(format!("{} {}", episode.number, trunc_episode_display_name))
            .style(style);
    }
}

struct HelpItem {
    text: &'static str,
    key: &'static str,
    text_style: Style,
    key_style: Style,
}

impl HelpItem {
    fn new(text: &'static str, key: &'static str, highlight_color: Color) -> Self {
        let text_style = Style::default().bg(highlight_color);
        let key_style = text_style.add_modifier(Modifier::BOLD);
        Self {
            text,
            key,
            text_style,
            key_style,
        }
    }

    fn to_span<'a>(&self) -> Vec<Span<'a>> {
        vec![
            Span::styled(format!("{} ", self.text), self.text_style),
            Span::styled(format!("[{}]", self.key), self.key_style),
            Span::raw(" "),
        ]
    }
}

fn build_help<'a>(focused_window: &FocusedWindow, insert_state: &InsertState, insert_episode_state: &InsertState, highlight_color: Color) -> Paragraph<'a> {
    // Create help text at the bottom
    let navigation = HelpItem::new("Navigation", "ARROWS", highlight_color);
    let insert = HelpItem::new("Insert new show", "N", highlight_color);
    let delete = HelpItem::new("Delete the entry", "DEL", highlight_color);
    let close_window = HelpItem::new("Close the window", "ESC", highlight_color);
    let exit_inputting = HelpItem::new("Stop inputting", "ESC", highlight_color);
    let start_inputting = HelpItem::new("Start inputting", "E", highlight_color);
    let confirm = HelpItem::new("Confirm", "ENTER", highlight_color);
    let login = HelpItem::new("Login to MAL", "L", highlight_color);
    let insert_episode = HelpItem::new("Progress", "< >", highlight_color);
    let progress = HelpItem::new("Add episode manually", "E", highlight_color);
    let quit = HelpItem::new("Quit", "Q", highlight_color);

    let mut information = Vec::new();
    match focused_window {
        FocusedWindow::MainMenu => {
            information.extend(navigation.to_span());
            information.extend(insert.to_span());
            information.extend(delete.to_span());
            information.extend(login.to_span());
            information.extend(insert_episode.to_span());
            information.extend(progress.to_span());
            information.extend(quit.to_span());
        }
        FocusedWindow::InsertPopup => {
            information.extend(navigation.to_span());
            match insert_state {
                InsertState::Inputting | InsertState::Next => {
                    information.extend(confirm.to_span());
                    information.extend(exit_inputting.to_span());
                }
                _ => {
                    information.extend(start_inputting.to_span());
                    information.extend(close_window.to_span());
                }
            }
        }
        FocusedWindow::InsertEpisodePopup => {
            information.extend(navigation.to_span());
            match insert_episode_state {
                InsertState::Inputting | InsertState::Next => {
                    information.extend(confirm.to_span());
                    information.extend(exit_inputting.to_span());
                }
                _ => {
                    information.extend(start_inputting.to_span());
                    information.extend(close_window.to_span());
                }
            }
        }
        FocusedWindow::Login => {
            information.extend(close_window.to_span());
        }
        FocusedWindow::TitleSelection => {
            information.extend(navigation.to_span());
            information.extend(close_window.to_span());
        }
        FocusedWindow::EpisodeMismatch | FocusedWindow::Error => {
            information.extend(confirm.to_span());
            information.extend(close_window.to_span());
        },
    };

    Paragraph::new(Line::from(information))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_item() {
        let highlight_color = Color::Rgb(0, 0, 0);
        let test_item = HelpItem::new("Testing", "THINGS", highlight_color);

        let text_style = Style::default().bg(highlight_color);
        let key_style = text_style.add_modifier(Modifier::BOLD);
        let expected_span = vec![
            Span::styled("Testing ", text_style),
            Span::styled("[THINGS]", key_style),
            Span::raw(" "),
        ];

        assert_eq!(test_item.to_span(), expected_span);
    }

    #[test]
    fn append_extra_info_recap_filler_trunc() {
        let space = 29; // +2 because of the border
        let episode = create_test_episode(true, true);
        let episode_display_name = episode.title.clone();
        let style = Style::default();
        let mut new_episode = ListItem::new(format!("{} {}", episode.number, episode_display_name)).style(style);

        append_extra_info(
            &mut new_episode,
            space,
            &episode,
            episode_display_name,
            style,
        );

        assert_eq!(
            new_episode,
            ListItem::new("1 Test Epis... RECAP/FILLER")
                .style(style)
        );
    }

    #[test]
    fn append_extra_info_recap_filler() {
        let space = 40; // +2 because of the border
        let episode = create_test_episode(true, true);
        let episode_display_name = episode.title.clone();
        let style = Style::default();
        let mut new_episode = ListItem::new(format!("{} {}", episode.number, episode_display_name)).style(style);

        append_extra_info(
            &mut new_episode,
            space,
            &episode,
            episode_display_name,
            style,
        );

        assert_eq!(
            new_episode,
            ListItem::new("1 Test Episode            RECAP/FILLER")
                .style(style)
        );
    }

    #[test]
    fn append_extra_info_filler_trunc() {
        let space = 24; // +2 because of the border
        let episode = create_test_episode(false, true);
        let episode_display_name = episode.title.clone();
        let style = Style::default();
        let mut new_episode = ListItem::new(format!("{} {}", episode.number, episode_display_name)).style(style);

        append_extra_info(
            &mut new_episode,
            space,
            &episode,
            episode_display_name,
            style,
        );

        assert_eq!(
            new_episode,
            ListItem::new("1 Test Episo... FILLER")
                .style(style)
        );
    }

    #[test]
    fn append_extra_info_filler() {
        let space = 40; // +2 because of the border
        let episode = create_test_episode(false, true);
        let episode_display_name = episode.title.clone();
        let style = Style::default();
        let mut new_episode = ListItem::new(format!("{} {}", episode.number, episode_display_name)).style(style);

        append_extra_info(
            &mut new_episode,
            space,
            &episode,
            episode_display_name,
            style,
        );

        assert_eq!(
            new_episode,
            ListItem::new("1 Test Episode                  FILLER")
                .style(style)
        );
    }

    #[test]
    fn append_extra_info_long_number() {
        let space = 24; // +2 because of the border
        let mut episode = create_test_episode(false, true);
        episode.number = 420;
        let episode_display_name = episode.title.clone();
        let style = Style::default();
        let mut new_episode = ListItem::new(format!("{} {}", episode.number, episode_display_name)).style(style);

        append_extra_info(
            &mut new_episode,
            space,
            &episode,
            episode_display_name,
            style,
        );

        assert_eq!(
            new_episode,
            ListItem::new("420 Test Epi... FILLER")
                .style(style)
        );
    }

    #[test]
    fn list_first_selection() {
        let count = 12;
        let mut list = generate_test_stateful_list(count);
        assert_eq!(list.selected_local_id, 0, "Check initial state");
        // first move doesn't change anything beside selecting an element
        // because at the start nothing is selected by default
        list.move_show_selection(&SelectionDirection::Next);
        assert_eq!(list.selected_local_id, 1, "Check first selection");
    }

    #[test]
    fn list_wrap_to_end() {
        let count = 12;
        let mut list = generate_test_stateful_list(count);
        list.move_show_selection(&SelectionDirection::Next);
        list.move_show_selection(&SelectionDirection::Previous);
        assert_eq!(list.selected_local_id, 12, "Wrapping around the list");
    }

    #[test]
    fn list_wrap_to_start() {
        let count = 12;
        let mut list = generate_test_stateful_list(count);
        list.move_show_selection(&SelectionDirection::Next);
        for _ in 1..=count {
            list.move_show_selection(&SelectionDirection::Next);
        }
        assert_eq!(list.selected_local_id, 1, "Wrapping around the list");
    }

    #[test]
    fn list_select_next() {
        let count = 12;
        let mut list = generate_test_stateful_list(count);
        list.move_show_selection(&SelectionDirection::Next);
        list.move_show_selection(&SelectionDirection::Next);
        assert_eq!(list.selected_local_id, 2, "Wrapping around the list");
    }

    #[test]
    fn list_selected_show_none() {
        let count = 12;
        let list = generate_test_stateful_list(count);
        let show = list.selected_show();
        assert!(show.is_none());
    }

    #[test]
    fn list_selected_show() {
        let count = 12;
        let mut list = generate_test_stateful_list(count);
        for _ in 1..=5 {
            list.move_show_selection(&SelectionDirection::Next);
        }
        let show = list.selected_show().expect("5th show");
        assert_eq!(show.local_id, 5);
        assert_eq!(show.title, "Test Show 5");
    }

    fn create_test_episode(recap: bool, filler: bool) -> Episode {
        Episode { 
            title: String::from("Test Episode"), 
            number: 1, 
            path: PathBuf::from("/path/just/for/testing.mp4"), 
            file_deleted: false, 
            recap, 
            filler 
        }
    }

    fn generate_test_stateful_list(count: i64) -> StatefulList {
        StatefulList { 
            shows_state: ListState::default(), 
            episodes_state: ListState::default(), 
            selecting_episode: false, 
            selected_local_id: 0, 
            list_cache: generate_test_shows(count),
        }
    }

    fn generate_test_episodes(count: i64) -> Vec<Episode> {
        let mut episodes = Vec::new();
        for i in 1..=count {
            let episode = Episode { 
                title: format!("Test Episode {i}"),
                number: i, 
                path: PathBuf::from("/path/just/for/testing.mp4"), 
                file_deleted: false, 
                recap: false, 
                filler: false 
            };
            episodes.push(episode);
        }
        episodes
    }

    fn generate_test_shows(count: i64) -> Vec<Show> {
        let mut shows = Vec::new();
        for i in 1..=count {
            let show = Show {
                local_id: i,
                title: format!("Test Show {i}"),
                service_id: 100 + i,
                episodes: generate_test_episodes(count),
                progress: i%4,
            };
            shows.push(show);
        }
        shows
    }
}