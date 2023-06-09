use std::process::{Command, Stdio};

use lma::{AnimeList, Show};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, text::{Span, Line},
};

use super::SelectionDirection;
use crate::app::App;

pub(crate) struct StatefulList {
    pub(crate) state: ListState,
    pub(crate) episodes_state: EpisodesState,
    pub(crate) items: AnimeList,
    list_cache: Vec<Show>,
}

pub(crate) struct EpisodesState {
    pub(crate) selected_show_id: i64,
    pub(crate) list_state: ListState,
    pub(crate) selection_enabled: bool,
}

impl StatefulList {
    pub(crate) fn with_items(shows: AnimeList) -> StatefulList {
        let list_cache = shows.get_list().unwrap();
        StatefulList {
            state: ListState::default(),
            episodes_state: EpisodesState {
                selected_show_id: 0,
                list_state: ListState::default(),
                selection_enabled: false,
            },
            items: shows,
            list_cache,
        }
    }

    pub(crate) fn move_selection(&mut self, direction: SelectionDirection) {
        if self.episodes_state.selection_enabled {
            self.move_episode_selection(direction);
        } else {
            self.update_cache();
            let i = self.select_element(self.list_cache.len(), self.state.selected(), direction);
            self.state.select(Some(i));
            let selected_id = if let Some(show) = self.list_cache.get(i) {
                show.id
            } else {
                0
            };
            self.episodes_state.selected_show_id = selected_id;
        }
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
                .get(self.episodes_state.selected_show_id as usize - 1)
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
        .filter(|show| show.id == app.shows.episodes_state.selected_show_id)
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

    // Create help text at the bottom
    let hint_text_style = Style::default().bg(Color::Rgb(0, 50, 0));
    let hint_key_style = hint_text_style.add_modifier(Modifier::BOLD);
    
    let information = Line::from(vec![
        Span::styled("Navigation ", hint_text_style),
        Span::styled("[ARROWS]", hint_key_style),
        Span::raw(" "),
        Span::styled("Insert new show ", hint_text_style),
        Span::styled("[N]", hint_key_style),
        Span::raw(" "),
        Span::styled("Login to MAL ", hint_text_style),
        Span::styled("[L]", hint_key_style),
        Span::raw(" "),
        Span::styled("Close a window ", hint_text_style),
        Span::styled("[ESC]", hint_key_style),
        Span::raw(" "),
        Span::styled("Quit ", hint_text_style),
        Span::styled("[Q]", hint_key_style),
    ]);
    let help = Paragraph::new(information);

    // We can now render the item list
    frame.render_stateful_widget(items, main_chunks[0], &mut app.shows.state);
    frame.render_stateful_widget(
        episodes,
        main_chunks[1],
        &mut app.shows.episodes_state.list_state,
    );
    frame.render_widget(help, chunks[1]);
}
