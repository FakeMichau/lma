use std::{process::{Command, Stdio}, error::Error};

use lma::{AnimeList, Show};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, text::{Span, Line},
};

use super::{SelectionDirection, FocusedWindow, popup::insert_show::InsertState};
use crate::app::App;

pub(crate) struct StatefulList {
    pub(crate) state: ListState,
    pub(crate) episodes_state: EpisodesState,
    pub(crate) items: AnimeList,
    pub(crate) selected_id: i64,
    list_cache: Vec<Show>,
}

pub(crate) struct EpisodesState {
    pub(crate) list_state: ListState,
    pub(crate) selection_enabled: bool,
}

impl StatefulList {
    pub(crate) fn with_items(shows: AnimeList) -> StatefulList {
        let list_cache = shows.get_list().unwrap();
        StatefulList {
            state: ListState::default(),
            episodes_state: EpisodesState {
                list_state: ListState::default(),
                selection_enabled: false,
            },
            items: shows,
            selected_id: 0,
            list_cache,
        }
    }

    pub(crate) fn delete(&mut self) -> Result<(), Box<dyn Error>> {
        if self.episodes_state.selection_enabled {
            // todo: delete just an episode
        } else {
            self.items.remove_entry(self.selected_id)?;
            self.update_cache();
            self.update_selected_id(self.state.selected().unwrap_or_default());
        }
        Ok(())
    }

    pub(crate) fn move_selection(&mut self, direction: SelectionDirection) {
        if self.episodes_state.selection_enabled {
            self.move_episode_selection(direction);
        } else {
            self.update_cache();
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

    fn move_episode_selection(&mut self, direction: SelectionDirection) {
        let selected_show = self
            .list_cache
            .get(self.state.selected().unwrap_or_default())
            .unwrap();
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
                .unwrap()
                .path;
            if cfg!(target_os = "linux") {
                _ = Command::new("xdg-open")
                    .arg(path)
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .spawn();
            }
        } else {
            match self
                .list_cache
                .get(self.state.selected().unwrap_or_default())
            {
                Some(show) => {
                    if show.episodes.len() > 0 {
                        self.episodes_state.list_state.select(Some(0));
                        self.episodes_state.selection_enabled = true;
                    }
                }
                None => {}
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
        match selected_element {
            Some(i) => match direction {
                SelectionDirection::Next => (i + 1) % list_length,
                SelectionDirection::Previous => (list_length + i - 1) % list_length,
            },
            None => 0,
        }
    }

    fn update_cache(&mut self) {
        self.list_cache = self.items.get_list().unwrap();
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
        .shows
        .items
        .get_list()
        .unwrap()
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
        )
        .highlight_symbol(">> ");

    // Iterate through all elements in the `items` app
    let episodes: Vec<ListItem> = app
        .shows
        .items
        .get_list()
        .unwrap()
        .iter()
        .filter(|show| show.id == app.shows.selected_id)
        .flat_map(|show| {
            let mut temp: Vec<ListItem> = Vec::new();
            for episode in &show.episodes {
                temp.push(
                    ListItem::new(format!("{} {}", episode.number, episode.path))
                        .style(Style::default()),
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
        )
        .highlight_symbol(">> ");

    let help = build_help(&app.focused_window, &app.insert_popup.state);

    // We can now render the item list
    frame.render_stateful_widget(items, main_chunks[0], &mut app.shows.state);
    frame.render_stateful_widget(
        episodes,
        main_chunks[1],
        &mut app.shows.episodes_state.list_state,
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
    let quit = HelpItem::new("Quit", "Q");

    let mut information = Vec::new();
    match focused_window {
        FocusedWindow::MainMenu => {
            information.extend(navigation.to_span());
            information.extend(insert.to_span());
            information.extend(delete.to_span());
            information.extend(login.to_span());
            information.extend(quit.to_span());
        },
        FocusedWindow::InsertPopup => {
            information.extend(navigation.to_span());
            match insert_state {
                InsertState::Inputting | InsertState::Next => {
                    information.extend(confirm.to_span());
                    information.extend(exit_inputting.to_span());
                },
                _ => {
                    information.extend(start_inputting.to_span());
                    information.extend(close_window.to_span());
                }
            }
        },
        FocusedWindow::Login => {
            information.extend(close_window.to_span());
        },
        FocusedWindow::TitleSelection => {
            information.extend(navigation.to_span());
            information.extend(close_window.to_span());
        },
    };

    Paragraph::new(Line::from(information))
}
