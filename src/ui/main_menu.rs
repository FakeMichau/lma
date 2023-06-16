use std::{process::{Command, Stdio}, error::Error};

use lma::{AnimeList, Show};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, text::{Span, Line},
};
use tokio::runtime::Runtime;

use super::{SelectionDirection, FocusedWindow, popup::insert_show::InsertState};
use crate::app::App;

pub(crate) struct StatefulList {
    state: ListState,
    episodes_state: EpisodesState,
    selected_id: i64,
    list_cache: Vec<Show>,
}

struct EpisodesState {
    list_state: ListState,
    selection_enabled: bool,
}

impl StatefulList {
    pub(crate) fn new(shows: &AnimeList) -> StatefulList {
        let list_cache = shows.get_list().unwrap();
        StatefulList {
            state: ListState::default(),
            episodes_state: EpisodesState {
                list_state: ListState::default(),
                selection_enabled: false,
            },
            selected_id: 0,
            list_cache,
        }
    }

    pub(crate) fn delete(&mut self, shows: &AnimeList) -> Result<(), Box<dyn Error>> {
        if self.episodes_state.selection_enabled {
            // todo: delete just an episode
        } else {
            shows.remove_entry(self.selected_id)?;
            self.update_cache(shows);
            self.update_selected_id(self.state.selected().unwrap_or_default());
        }
        Ok(())
    }

    pub(crate) fn move_selection(&mut self, direction: SelectionDirection, shows: &AnimeList, ) {
        if self.episodes_state.selection_enabled {
            self.move_episode_selection(direction);
        } else {
            self.update_cache(shows);
            let i = self.select_element(self.list_cache.len(), self.state.selected(), direction);
            self.state.select(Some(i));
            self.update_selected_id(i);
        }
    }

    fn update_selected_id(&mut self, index: usize) {
        self.selected_id = if let Some(show) = self.list_cache.get(index) {
            show.id
        } else {
            0
        };
    }

    pub(crate) fn selected_show(&self) -> Option<&Show> {
        self.list_cache
            .get(self.state.selected().unwrap_or_default())
    }

    pub(crate) fn move_progress(&mut self, direction: SelectionDirection, shows: &mut AnimeList, rt: &Runtime) {
        let selected_show = if let Some(selected_show) = self.selected_show() {
            selected_show
        } else {
            return
        };
        let offset = match direction {
            SelectionDirection::Next => 1,
            SelectionDirection::Previous => -1,
        };
        let progress = selected_show.progress + offset;
        shows.set_progress(selected_show.id, progress)
            .expect("Set local progress");
        rt.block_on(
            shows.service.set_progress(
                selected_show.sync_service_id as u32, 
                progress as u32
            )
        );
        self.update_cache(shows);
    }

    fn move_episode_selection(&mut self, direction: SelectionDirection) {
        let selected_show = if let Some(selected_show) = self.selected_show() {
            selected_show
        } else {
            return
        };
        let episodes_len = selected_show.episodes.len();
        let i = self.select_element(
            episodes_len,
            self.episodes_state.list_state.selected(),
            direction,
        );
        self.episodes_state.list_state.select(Some(i));
    }

    pub(crate) fn select(&mut self) {
        if self.episodes_state.selection_enabled {
            // navigating inside the episodes tab
            let selected_episode = self
                .episodes_state
                .list_state
                .selected()
                .unwrap_or_default();

            let path = &self
                .list_cache
                .iter()
                .filter(|show| show.id == self.selected_id)
                .next()
                .unwrap()
                .episodes
                .get(selected_episode)
                .map(|episode| {
                    episode.path.clone()
                })
                .unwrap_or_default();
            
            if path.exists() {
                if cfg!(target_os = "linux") {
                    _ = Command::new("xdg-open")
                        .arg(path)
                        .stderr(Stdio::null())
                        .stdout(Stdio::null())
                        .spawn();
                }
            }
        } else {
            if let Some(selected_id) = self.state.selected() {
                if let Some(show) = self.list_cache.get(selected_id) {
                    if !show.episodes.is_empty() {
                        let index = show
                            .episodes
                            .iter()
                            .position(|episode| episode.number == show.progress)
                            .map(|pos| (pos + 1) % show.episodes.len())
                            .unwrap_or(0);
            
                        self.episodes_state.list_state.select(Some(index));
                        self.episodes_state.selection_enabled = true;
                    }
                }
            }
            
        }
    }
    pub(crate) fn unselect(&mut self) {
        self.episodes_state.list_state.select(None);
        self.episodes_state.selection_enabled = false;
    }

    fn select_element(
        &mut self,
        list_length: usize,
        selected_element: Option<usize>,
        direction: SelectionDirection,
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

    pub(crate) fn update_cache(&mut self, shows: &AnimeList) {
        self.list_cache = shows.get_list().unwrap();
    }
}

pub(crate) fn build<B: Backend>(frame: &mut Frame<'_, B>, app: &mut App) {
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
        .map(|show| ListItem::new(format!("{}", show.title)).style(Style::default()))
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        );

    // Iterate through all elements in the `items` app
    let episodes: Vec<ListItem> = app
        .list_state
        .list_cache
        .iter()
        .filter(|show| show.id == app.list_state.selected_id)
        .flat_map(|show| {
            let mut temp: Vec<ListItem> = Vec::new();
            for episode in &show.episodes {
                let mut style = Style::default();
                if episode.number <= show.progress { 
                    style = style.fg(Color::Rgb(50, 50, 50))
                }
                if !episode.path.exists() {
                    style = style.fg(Color::Red)
                }
                // maybe make a config for that in the future
                let episode_display_name = if episode.title.is_empty() {
                    episode.path.file_name().unwrap_or_default().to_string_lossy().into()
                } else {
                    episode.title.clone()
                };

                temp.push(
                    ListItem::new(format!("{} {}", episode.number, episode_display_name)).style(style),
                );
            }
            temp
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let episodes = List::new(episodes)
        .block(Block::default().borders(Borders::ALL).title("Episodes"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        );

    let help = build_help(&app.focused_window, &app.insert_popup.state);

    // We can now render the item list
    frame.render_stateful_widget(items, main_chunks[0], &mut app.list_state.state);
    frame.render_stateful_widget(
        episodes,
        main_chunks[1],
        &mut app.list_state.episodes_state.list_state,
    );
    frame.render_widget(help, chunks[1]);
}

struct HelpItem {
    text: &'static str,
    key: &'static str,
    text_style: Style,
    key_style: Style,
}

impl HelpItem {
    fn new(text: &'static str, key: &'static str) -> Self {
        let text_style = Style::default().bg(Color::Rgb(0, 50, 0));
        let key_style = text_style.add_modifier(Modifier::BOLD);
        HelpItem {
            text,
            key,
            text_style,
            key_style,
        }
    }

    fn to_span<'a>(self) -> Vec<Span<'a>> {
        vec![
            Span::styled(format!("{} ", self.text), self.text_style),
            Span::styled(format!("[{}]", self.key), self.key_style),
            Span::raw(" "),
        ]
    }
}

fn build_help<'a>(focused_window: &FocusedWindow, insert_state: &InsertState) -> Paragraph<'a> {
    // Create help text at the bottom
    let navigation = HelpItem::new("Navigation", "ARROWS");
    let insert = HelpItem::new("Insert new show", "N");
    let delete = HelpItem::new("Delete the entry", "DEL");
    let close_window = HelpItem::new("Close the window", "ESC");
    let exit_inputting = HelpItem::new("Stop inputting", "ESC");
    let start_inputting = HelpItem::new("Start inputting", "E");
    let confirm = HelpItem::new("Confirm", "ENTER");
    let login = HelpItem::new("Login to MAL", "L");
    let progress = HelpItem::new("Progress", "< >");
    let quit = HelpItem::new("Quit", "Q");

    let mut information = Vec::new();
    match focused_window {
        FocusedWindow::MainMenu => {
            information.extend(navigation.to_span());
            information.extend(insert.to_span());
            information.extend(delete.to_span());
            information.extend(login.to_span());
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
        FocusedWindow::Login => {
            information.extend(close_window.to_span());
        }
        FocusedWindow::TitleSelection => {
            information.extend(navigation.to_span());
            information.extend(close_window.to_span());
        }
        FocusedWindow::EpisodeMismatch => {
            information.extend(confirm.to_span());
            information.extend(close_window.to_span());
        },
    };

    Paragraph::new(Line::from(information))
}
