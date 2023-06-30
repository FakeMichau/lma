use std::path::PathBuf;
use crossterm::event::KeyCode;
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Clear};
use ratatui::{Frame, text::{Span, Line}};
use ratatui::widgets::{Row, Table as TableWidget, TableState, Cell};
use tokio::runtime::Runtime;
use lma_lib::{AnimeList, Show, Service};
use crate::app::App;
use crate::config::{KeyBinds, TermColors};
use super::{SelectionDirection, FocusedWindow, popup::insert_show::InsertState};

pub struct StatefulList {
    shows_state: TableState,
    episodes_state: TableState,
    selecting_episode: bool,
    selected_local_id: i64,
    list_cache: Vec<Show>,
    scroll_progress: u32
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
            scroll_progress: 0
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

    pub fn select(&mut self) -> Result<(), String>{
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

    render_shows(app, main_chunks[0], frame);
    render_episodes(app, main_chunks[1], frame);
    render_help(app, chunks[1], frame);    
}

fn render_help<B: Backend, T: Service>(app: &mut App<T>, area: Rect, frame: &mut Frame<B>) {
    let help = build_help(
        &app.focused_window,
        &app.insert_popup.state,
        &app.insert_episode_popup.state,
        app.config.colors().highlight_dark,
        app.config.key_binds(),
    );
    frame.render_widget(help, area);
}

fn render_shows<B: Backend, T: Service>(app: &mut App<T>, area: Rect, frame: &mut Frame<B>) {
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    let inner_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([ Constraint::Percentage(100), Constraint::Min(1) ].as_ref())
        .split(inner_area);

    let mut scroll_progress: usize = app.list_state.scroll_progress.try_into().unwrap();
    let shows: Vec<_> = app
        .list_state
        .list_cache
        .iter()
        .map(|show| {
            let title = if app.list_state.selected_show().map(|selected_show| selected_show.local_id).unwrap_or_default() == show.local_id 
                && !app.list_state.selecting_episode
            {
                let full_title = show.title.clone();
                let space = usize::from(inner_layout[0].width); // without border and scrollbar
                scroll_text(full_title, space, &mut scroll_progress)
            } else {
                show.title.clone()
            };

            let style = Style::default().fg(app.config.colors().text);
            Row::new(vec![
                Cell::from(title).style(style),
            ])
        })
        .collect();
    app.list_state.scroll_progress = scroll_progress.try_into().unwrap();
    
    let show_count = shows.len();

    let border = Block::default().borders(Borders::ALL).title("Shows").border_style(
        if app.list_state.selecting_episode || app.list_state.selected_show().is_none() {
            Style::default()
        } else {
            Style::default().fg(app.config.colors().highlight)
        }
    );

    frame.render_widget(border, area);

    let header: Vec<HeaderType> = vec![
        HeaderType::title(),
    ];

    Table::new(&mut app.list_state.shows_state, shows, header, inner_layout[0])
        .render(frame, app.config.colors());
    
    render_scrollbar(inner_layout[1], frame, show_count, app, app.list_state.shows_state.offset());
}

fn render_episodes<B: Backend, T: Service>(app: &mut App<T>, area: Rect, frame: &mut Frame<'_, B>) {
    let header: Vec<HeaderType> = vec![
        HeaderType::number(),
        HeaderType::title(),
        HeaderType::extra(),
    ];
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    let inner_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [ Constraint::Percentage(100), Constraint::Min(1) ]
            .as_ref(),
        )
        .split(inner_area);
    
    let episodes: Vec<Row> = app
        .list_state
        .list_cache
        .iter()
        .filter(|show| show.local_id == app.list_state.selected_local_id)
        .flat_map(|show| {
            let mut temp: Vec<Row> = Vec::new();
            for episode in &show.episodes {
                let mut style = Style::default();
                if episode.number <= show.progress {
                    style = style.fg(app.config.colors().text).add_modifier(Modifier::DIM);
                }
                if episode.file_deleted {
                    style = style.fg(app.config.colors().text_deleted).add_modifier(Modifier::CROSSED_OUT | Modifier::DIM);
                }
                let mut episode_display_name = if episode.title.is_empty() || app.config.path_instead_of_title() {
                    episode
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into()
                } else {
                    episode.title.clone()
                };
                let selected_episode = app.list_state.episodes_state
                    .selected()
                    .and_then(|index| show.episodes.get(index));
                if !episode.filler 
                    && !episode.recap 
                    && app.list_state.selecting_episode 
                    && selected_episode.expect("Is selecting_episode").number == episode.number
                {
                    let space = usize::from(inner_layout[0].width - header.sum_consts() - 2);
                    let mut scroll_progress: usize =
                        app.list_state.scroll_progress.try_into().unwrap();
                    episode_display_name =
                        scroll_text(episode_display_name, space, &mut scroll_progress);
                    app.list_state.scroll_progress = scroll_progress.try_into().unwrap();
                }
                
                let new_episode = Row::new(vec![
                    Cell::from(episode.number.to_string()).style(style),
                    Cell::from(episode_display_name.to_string()).style(style),
                    Cell::from(
                        format!(" {} {} ", if episode.filler {"F"} else {""}, if episode.recap {"R"} else {""})
                    ).style(style),
                ]);
                temp.push(new_episode);
            }
            temp
        })
        .collect();
    let episode_count = episodes.len();
    
    let border = Block::default().borders(Borders::ALL).title("Episodes").border_style(
        if app.list_state.selecting_episode {
            Style::default().fg(app.config.colors().highlight)
        } else {
            Style::default()
        }
    );
    frame.render_widget(border, area);

    Table::new(&mut app.list_state.episodes_state, episodes, header, inner_layout[0])
        .render(frame, app.config.colors());
    
    render_scrollbar(inner_layout[1], frame, episode_count, app, app.list_state.episodes_state.offset());
}

enum HeaderType {
    Number(u16),
    Title,
    Score(u16),
    Extra(u16),
}

impl HeaderType {
    const fn get_width(&self) -> Option<u16> {
        match self {
            Self::Title => None,
            Self::Number(width) | Self::Score(width) | Self::Extra(width) => Some(*width),
        }
    }
    const fn title() -> Self {
        Self::Title
    }
    const fn number() -> Self {
        Self::Number(3)
    }
    const fn score() -> Self {
        Self::Score(5)
    }
    const fn extra() -> Self {
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
                HeaderType::Title => {
                    TableHeaderItem::new(const_align("Title", space - space_of_consts), Constraint::Percentage(100))
                }
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
            .map(|header_type| header_type.get_width().map_or(0, |width| width))
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
    header: Vec<HeaderType>,
    area: Rect,
}

impl<'a> Table<'a> {
    fn new(
        state: &'a mut TableState,
        items: Vec<Row<'a>>,
        header: Vec<HeaderType>,
        area: Rect
    ) -> Self {
        Self { state, items: Some(items), header, area }
    }

    fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>, colors: &TermColors) {
        const COLUMN_SPACING: u16 = 1;
        let mut header_text = Vec::new();
        let mut header_constraint = Vec::new();
        let aligned_header = &self.header.align(self.area.width);
        for header_item in aligned_header {
            header_text.push(header_item.text.clone());
            header_constraint.push(header_item.constraint);
        }
        if header_text.len() > 1 {
            header_constraint.push(Constraint::Min(COLUMN_SPACING));
        }
        let widget = TableWidget::new(self.items.take().unwrap_or_default())
            .header(Row::new(header_text).style(Style::default().fg(colors.secondary)))
            .widths(&header_constraint)
            .column_spacing(COLUMN_SPACING)
            .highlight_style(Style::default().fg(colors.highlight).add_modifier(Modifier::BOLD));
        frame.render_stateful_widget(widget, self.area, self.state);
    }
}

fn render_scrollbar<B: Backend, T: Service>(area: Rect, frame: &mut Frame<B>, entry_count: usize, app: &mut App<T>, offset: usize) {
    frame.render_widget(Clear, area);
    if entry_count > area.height.into() {
        let area = get_scroll_bar(offset, area, entry_count);
        let progress = Block::default().style(Style::default().bg(app.config.colors().text));
        frame.render_widget(progress, area);
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn get_scroll_bar(offset: usize, scrollbar_area: Rect, episode_count: usize) -> Rect {
    let float_skipped_entries = offset as f64;
    let float_height =  f64::from(scrollbar_area.height);
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
            i if max_offset + WAIT_OFFSET < i && i <= max_offset + 2 * WAIT_OFFSET + 1 => max_offset,
            _ => {
                *scroll_progress = 0; 
                0
            }
        };
        *scroll_progress += 1;
        let trimmed = &full_title[offset..offset+space];
        String::from(trimmed)
    } else {
        full_title
    }
}

enum Function {
    Navigation,
    Progress,
    Confirmation,
    Close,
    Delete,
    Quit,
    EnterInputting,
    NewShow,
    NewEpisode,
    Login,
}

struct HelpItem<'a> {
    text: &'a str,
    key: String,
    text_style: Style,
    key_style: Style,
}

impl<'a> HelpItem<'a> {
    fn new(text: &'a str, name: &Function, key_binds: &KeyBinds, highlight_color: Color) -> Self {
        let text_style = Style::default().bg(highlight_color);
        let key_style = text_style.add_modifier(Modifier::BOLD);
        let key = key_to_abbr(key_binds, name);
        Self {
            text,
            key,
            text_style,
            key_style,
        }
    }

    fn to_span<'b>(&self) -> Vec<Span<'b>> {
        vec![
            Span::styled(format!("{} ", self.text), self.text_style),
            Span::styled(format!("[{}]", self.key), self.key_style),
            Span::raw(" "),
        ]
    }
}

fn build_help<'a>(
    focused_window: &FocusedWindow,
    insert_state: &InsertState,
    insert_episode_state: &InsertState,
    highlight_color: Color,
    key_binds: &KeyBinds,
) -> Paragraph<'a> {
    // Create help text at the bottom
    let navigation = HelpItem::new("Navigation", &Function::Navigation, key_binds, highlight_color);
    let insert = HelpItem::new("Insert new show", &Function::NewShow, key_binds, highlight_color);
    let delete = HelpItem::new("Delete the entry", &Function::Delete, key_binds, highlight_color);
    let go_back = HelpItem::new("Go back", &Function::Close, key_binds, highlight_color);
    let close_window = HelpItem::new("Close the window", &Function::Close, key_binds, highlight_color);
    let exit_inputting = HelpItem::new("Stop inputting", &Function::Close, key_binds, highlight_color);
    let start_inputting = HelpItem::new("Start inputting", &Function::EnterInputting, key_binds, highlight_color);
    let confirm = HelpItem::new("Confirm", &Function::Confirmation, key_binds, highlight_color);
    let login = HelpItem::new("Login", &Function::Login, key_binds, highlight_color);
    let progress = HelpItem::new("Progress", &Function::Progress, key_binds, highlight_color);
    let insert_episode = HelpItem::new("Add episode manually", &Function::NewEpisode, key_binds, highlight_color);
    let quit = HelpItem::new("Quit", &Function::Quit, key_binds, highlight_color);    

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
        FocusedWindow::FirstSetup => {
            information.extend(navigation.to_span());
            information.extend(go_back.to_span());
            information.extend(confirm.to_span());
        }
        FocusedWindow::TitleSelection => {
            information.extend(navigation.to_span());
            information.extend(close_window.to_span());
        }
        FocusedWindow::EpisodeMismatch | FocusedWindow::Error => {
            information.extend(confirm.to_span());
            information.extend(close_window.to_span());
        }
    };

    Paragraph::new(Line::from(information))
}

fn key_to_abbr(key: &KeyBinds, name: &Function) -> String {
    match name {
        Function::Navigation => {
            if key.move_up == KeyCode::Up
                && key.move_down == KeyCode::Down
                && key.backwards == KeyCode::Left
                && key.forwards == KeyCode::Right
            {
                String::from("ARROWS")
            } else {
                format!(
                    "{}{}{}{}",
                    keycode_to_key(key.move_up),
                    keycode_to_key(key.move_down),
                    keycode_to_key(key.backwards),
                    keycode_to_key(key.forwards),
                )
            }
        }
        Function::Progress => {
            if key.progress_inc == KeyCode::Char('.') && key.progress_dec == KeyCode::Char(',') {
                String::from("< >")
            } else {
                format!(
                    "{} {}",
                    keycode_to_key(key.progress_inc),
                    keycode_to_key(key.progress_dec),
                )
            }
        }
        Function::Confirmation => keycode_to_key(key.confirmation),
        Function::Close => keycode_to_key(key.close),
        Function::Delete => keycode_to_key(key.delete),
        Function::Quit => keycode_to_key(key.quit),
        Function::EnterInputting => keycode_to_key(key.enter_inputting),
        Function::NewShow => keycode_to_key(key.new_show),
        Function::NewEpisode => keycode_to_key(key.new_episode),
        Function::Login => keycode_to_key(key.login),
    }
}

fn keycode_to_key(keycode: KeyCode) -> String {
    match keycode {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Char(c) => c.to_uppercase().to_string(),
        KeyCode::Null => "Null".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::CapsLock => "CapsLock".to_string(),
        KeyCode::ScrollLock => "ScrollLock".to_string(),
        KeyCode::NumLock => "NumLock".to_string(),
        KeyCode::PrintScreen => "PrintScreen".to_string(),
        KeyCode::Pause => "Pause".to_string(),
        KeyCode::Menu => "Menu".to_string(),
        KeyCode::KeypadBegin => "KeypadBegin".to_string(),
        KeyCode::Media(m) => format!("Media({m:?})"),
        KeyCode::Modifier(m) => format!("Modifier({m:?})"),
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_item() {
        let highlight_color = Color::Rgb(0, 0, 0);
        let key_binds = KeyBinds {
            quit: KeyCode::Char('q'),
            ..Default::default()
        };
        let test_item = HelpItem::new("Testing", &Function::Quit, &key_binds, highlight_color);

        let text_style = Style::default().bg(highlight_color);
        let key_style = text_style.add_modifier(Modifier::BOLD);
        let expected_span = vec![
            Span::styled("Testing ", text_style),
            Span::styled("[Q]", key_style),
            Span::raw(" "),
        ];

        assert_eq!(test_item.to_span(), expected_span);
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

    fn generate_test_stateful_list(count: i64) -> StatefulList {
        StatefulList {
            shows_state: TableState::default(),
            episodes_state: TableState::default(),
            selecting_episode: false,
            selected_local_id: 0,
            list_cache: generate_test_shows(count),
            scroll_progress: 0
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
