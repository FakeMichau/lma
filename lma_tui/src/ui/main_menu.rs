mod shows;
mod episodes;
mod help;
use super::SelectionDirection;
use crate::app::App;
use crate::config::TermColors;
use lma_lib::{AnimeList, Service, Show};
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect, Margin};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear};
use ratatui::widgets::{Row, Table as TableWidget, TableState};
use ratatui::Frame;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct StatefulList {
    shows_state: TableState,
    episodes_state: TableState,
    selecting_episode: bool,
    selected_local_id: i64,
    list_cache: Vec<Show>,
    scroll_progress: u32,
}

impl StatefulList {
    pub fn new<T: Service>(shows: &AnimeList<T>) -> Result<Self, String> {
        let list_cache = shows.get_list().map_err(|e| e.to_string())?;
        Ok(Self {
            shows_state: TableState::default(),
            selecting_episode: false,
            episodes_state: TableState::default(),
            selected_local_id: 0,
            list_cache,
            scroll_progress: 0,
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

    pub fn move_selection<T: Service>(
        &mut self,
        direction: &SelectionDirection,
        shows: &AnimeList<T>,
    ) -> Result<(), String> {
        self.scroll_progress = 0;
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
        let i = super::select_element(episodes_len, self.episodes_state.selected(), direction);
        self.episodes_state.select(Some(i));
    }

    fn move_show_selection(&mut self, direction: &SelectionDirection) {
        let i = super::select_element(
            self.list_cache.len(),
            self.shows_state.selected(),
            direction,
        );
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

    pub fn move_progress<T: Service>(
        &mut self,
        direction: &SelectionDirection,
        shows: &mut AnimeList<T>,
        rt: &Runtime,
    ) -> Result<(), String> {
        let Some(selected_show) = self.selected_show() else {
            return Ok(())
        };
        let offset = match direction {
            SelectionDirection::Next => 1,
            SelectionDirection::Previous => -1,
        };
        let progress = u32::try_from(selected_show.progress + offset).unwrap_or_default();
        let actual_progress = rt.block_on(shows.service.set_progress(
            u32::try_from(selected_show.service_id).map_err(|e| e.to_string())?,
            progress,
        ))?;
        shows
            .set_progress(selected_show.local_id, i64::from(actual_progress))
            .expect("Set local progress");
        self.update_cache(shows)?;
        Ok(())
    }

    pub fn select(&mut self) -> Result<(), String> {
        if self.selecting_episode {
            // navigating inside the episodes tab
            let selected_episode = self.episodes_state.selected().unwrap_or_default();

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

            if path.exists() {
                open::that(path).map_err(|err| err.to_string())?;
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
        Ok(())
    }
    pub fn unselect(&mut self) {
        self.episodes_state.select(None);
        self.selecting_episode = false;
    }

    pub fn update_cache<T: Service>(&mut self, shows: &AnimeList<T>) -> Result<(), String> {
        self.list_cache = shows.get_list().map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn render<B: Backend, T: Service>(frame: &mut Frame<'_, B>, app: &mut App<T>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Max(1)].as_ref())
        .split(frame.size());

    // Split the bigger chunk into halves
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
        .split(chunks[0]);

    shows::render(app, main_chunks[0], frame);
    episodes::render(app, main_chunks[1], frame);
    help::render(app, chunks[1], frame);
}

fn try_to_scroll_title(
    width: u16,
    header: &Vec<HeaderType>,
    scroll_progress_u32: &mut u32,
    episode_display_name: &mut String,
) {
    let space = usize::from(width - header.sum_consts());
    let mut scroll_progress: usize = (*scroll_progress_u32).try_into().unwrap();
    *episode_display_name = scroll_text(episode_display_name.clone(), space, &mut scroll_progress);
    scroll_progress_u32.clone_from(&u32::try_from(scroll_progress).unwrap_or_default());
}

fn get_inner_layout(area: Rect) -> (Rect, Rect) {
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    let inner_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100), Constraint::Min(1)].as_ref())
        .split(inner_area);
    (inner_layout[0], inner_layout[1])
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum HeaderType {
    Number(u16),
    Title,
    Score(u16),
    Extra(u16),
}

impl HeaderType {
    const fn get_width(self) -> Option<u16> {
        match self {
            Self::Title => None,
            Self::Number(width) | Self::Score(width) | Self::Extra(width) => Some(width),
        }
    }
    pub const fn title() -> Self {
        Self::Title
    }
    pub const fn number() -> Self {
        Self::Number(3)
    }
    pub const fn score() -> Self {
        Self::Score(5)
    }
    pub const fn extra() -> Self {
        Self::Extra(5)
    }
}

trait HeaderAlign {
    fn align(&self, space: u16) -> Vec<TableHeaderItem>;
    fn sum_consts(&self) -> u16;
}

impl HeaderAlign for Vec<HeaderType> {
    fn align<'a>(&self, space: u16) -> Vec<TableHeaderItem> {
        let space_of_consts: u16 = self.sum_consts();
        self.iter()
            .map(|header_type| match header_type {
                HeaderType::Number(width) => {
                    TableHeaderItem::new(const_align("#", *width), Constraint::Min(*width))
                }
                HeaderType::Title => TableHeaderItem::new(
                    const_align("Title", space - space_of_consts),
                    Constraint::Percentage(100),
                ),
                HeaderType::Score(width) => {
                    TableHeaderItem::new(const_align("Score", *width), Constraint::Min(*width))
                }
                HeaderType::Extra(width) => {
                    TableHeaderItem::new(const_align("Extra", *width), Constraint::Min(*width))
                }
            })
            .collect()
    }
    fn sum_consts(&self) -> u16 {
        self.iter()
            .map(|header_type| header_type.get_width().map_or(0, |width| width + 1))
            .sum()
    }
}
fn const_align(text: &str, width: u16) -> String {
    format!("{:^1$}", text, usize::from(width))
}

struct TableHeaderItem {
    text: String,
    constraint: Constraint,
}

impl TableHeaderItem {
    const fn new(text: String, constraint: Constraint) -> Self {
        Self { text, constraint }
    }
}

struct Table<'a> {
    state: &'a mut TableState,
    items: Option<Vec<Row<'a>>>,
    header: &'a Vec<HeaderType>,
    area: Rect,
}

impl<'a> Table<'a> {
    fn new(
        state: &'a mut TableState,
        items: Vec<Row<'a>>,
        header: &'a Vec<HeaderType>,
        area: Rect,
    ) -> Self {
        Self {
            state,
            items: Some(items),
            header,
            area,
        }
    }

    fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>, colors: &TermColors) {
        const COLUMN_SPACING: u16 = 1;
        let mut header_text = Vec::new();
        let mut header_constraint = Vec::new();
        let aligned_header: Vec<TableHeaderItem> = self.header.align(self.area.width);
        for header_item in aligned_header {
            header_text.push(header_item.text);
            header_constraint.push(header_item.constraint);
        }
        let column_count = header_text.len();
        if let Some(title_pos) = header_constraint
            .iter()
            .position(|constraint| constraint == &Constraint::Percentage(100))
        {
            let position_from_end = u16::try_from(column_count - title_pos - 1).unwrap_or_default();
            header_constraint.push(Constraint::Min(position_from_end));
        }
        let widget = TableWidget::new(self.items.take().unwrap_or_default())
            .header(Row::new(header_text).style(Style::default().fg(colors.secondary)))
            .widths(&header_constraint)
            .column_spacing(COLUMN_SPACING)
            .highlight_style(
                Style::default()
                    .fg(colors.highlight)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(widget, self.area, self.state);
    }
}

fn render_scrollbar<B: Backend>(
    area: Rect,
    frame: &mut Frame<B>,
    entry_count: usize,
    colors: &TermColors,
    offset: usize,
) {
    frame.render_widget(Clear, area);
    if entry_count > area.height.into() {
        let area = get_scroll_bar(offset, area, entry_count);
        let progress = Block::default().style(Style::default().bg(colors.text));
        frame.render_widget(progress, area);
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn get_scroll_bar(offset: usize, scrollbar_area: Rect, episode_count: usize) -> Rect {
    let float_skipped_entries = offset as f64;
    let float_height = f64::from(scrollbar_area.height);
    let float_episode_count = episode_count as f64;
    let float_bar_height = float_height * float_height / float_episode_count;
    let max_y = float_height - float_bar_height;
    let float_y = (float_skipped_entries / float_episode_count * float_height).clamp(0.0, max_y);
    Rect {
        x: scrollbar_area.x,
        y: float_y as u16 + scrollbar_area.y,
        width: scrollbar_area.width,
        height: float_bar_height.ceil() as u16,
    }
}

fn scroll_text(full_title: String, space: usize, scroll_progress: &mut usize) -> String {
    if full_title.len() > space {
        const WAIT_OFFSET: usize = 3;
        let max_offset = full_title.len() - space;
        let offset = match *scroll_progress {
            ..=WAIT_OFFSET => 0,
            i if WAIT_OFFSET <= i && i <= WAIT_OFFSET + max_offset => i - WAIT_OFFSET - 1,
            i if max_offset + WAIT_OFFSET < i && i <= max_offset + 2 * WAIT_OFFSET + 1 => {
                max_offset
            }
            _ => {
                *scroll_progress = 0;
                0
            }
        };
        *scroll_progress += 1;
        let trimmed = &full_title[offset..offset + space];
        String::from(trimmed)
    } else {
        full_title
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn generate_test_stateful_list(count: i64) -> StatefulList {
        StatefulList {
            shows_state: TableState::default(),
            episodes_state: TableState::default(),
            selecting_episode: false,
            selected_local_id: 0,
            list_cache: generate_test_shows(count),
            scroll_progress: 0,
        }
    }

    use lma_lib::Episode;
    fn generate_test_episodes(count: i64) -> Vec<Episode> {
        let mut episodes = Vec::new();
        for i in 1..=count {
            let episode = Episode {
                title: format!("Test Episode {i}"),
                number: i,
                path: PathBuf::from("/path/just/for/testing.mp4"),
                file_deleted: false,
                score: 0.0,
                recap: false,
                filler: false,
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
                progress: i % 4,
            };
            shows.push(show);
        }
        shows
    }
}
