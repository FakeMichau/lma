use lma::{AnimeList, Show};
use tui::widgets::ListState;

pub(crate) struct StatefulList {
    pub(crate) state: ListState,
    pub(crate) episodes_state: EpisodesState,
    pub(crate) shows: AnimeList,
    list_cache: Vec<Show>
}

pub(crate) struct EpisodesState {
    pub(crate) selected_id: i64,
    pub(crate) list_state: ListState,
    pub(crate) selection_enabled: bool,
}

#[derive(PartialEq)]
pub(crate) enum Direction {
    Next,
    Previous,
}

impl StatefulList {
    pub(crate) fn with_items(shows: AnimeList) -> StatefulList {
        let list_cache = shows.get_list().unwrap();
        StatefulList {
            state: ListState::default(),
            episodes_state: EpisodesState {
                selected_id: 0,
                list_state: ListState::default(),
                selection_enabled: false,
            },
            shows,
            list_cache,
        }
    }

    pub(crate) fn move_selection(&mut self, direction: Direction) {
        if self.episodes_state.selection_enabled {
            self.move_episode_selection(direction);
        } else {
            let i = self.select_element(
                self.list_cache.len(), 
                self.state.selected(), 
                direction
            );
            self.state.select(Some(i));
            let selected_id = if let Some(show) = self.list_cache.get(i) {
                show.id
            } else {
                0
            };
            self.episodes_state.selected_id = selected_id;
        }
    }

    fn move_episode_selection(&mut self, direction: Direction) {
        let selected_show = self.list_cache.get(self.state.selected().unwrap_or_default()).unwrap();
        let episodes_len = selected_show.episodes.len();
        let i = self.select_element(
            episodes_len,
            self.episodes_state.list_state.selected(),
            direction,
        );
        self.episodes_state.list_state.select(Some(i));
    }

    pub(crate) fn select(&mut self) {
        match self.list_cache.get(self.state.selected().unwrap_or_default()) {
            Some(show) => {
                if show.episodes.len() > 0 {
                    self.episodes_state.list_state.select(Some(0));
                    self.episodes_state.selection_enabled = true;
                }
            }
            None => {}
        }
        self.update_cache();
    }
    pub(crate) fn unselect(&mut self) {
        self.episodes_state.list_state.select(None);
        self.episodes_state.selection_enabled = false;
    }

    fn select_element(
        &mut self,
        list_length: usize,
        selected_element: Option<usize>,
        direction: Direction,
    ) -> usize {
        match direction {
            Direction::Next => match selected_element {
                Some(i) => {
                    if i >= list_length.checked_sub(1).unwrap_or_default() {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            },
            Direction::Previous => match selected_element {
                Some(i) => {
                    if i == 0 {
                        list_length.checked_sub(1).unwrap_or_default()
                    } else {
                        i - 1
                    }
                }
                None => 0,
            },
        }
    }

    fn update_cache(&mut self) {
        self.list_cache = self.shows.get_list().unwrap();
    }
}