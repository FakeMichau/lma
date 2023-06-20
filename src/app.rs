use std::error::Error;
use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{backend::Backend, Terminal};
use tokio::runtime::Runtime;
use lma::{AnimeList,Service};
use crate::app;
use crate::config::Config;
use crate::ui::{FocusedWindow, SelectionDirection, ui};
use crate::ui::main_menu::StatefulList;
use crate::ui::popup::episode_mismatch::MismatchPopup;
use crate::ui::popup::title_selection::TitlesPopup;
use crate::ui::popup::insert_show::{InsertPopup, InsertState};

pub struct App<T: Service> {
    pub focused_window: FocusedWindow,
    pub insert_popup: InsertPopup,
    pub titles_popup: TitlesPopup,
    pub mismatch_popup: MismatchPopup,
    pub list_state: StatefulList,
    pub anime_list: AnimeList<T>,
    pub config: Config,
}

impl<T: Service + Send> App<T> {
    pub fn build(rt: &Runtime, config: Config) -> Self {
        let service = rt.block_on(lma::Service::new(config.data_dir().clone()));
        let anime_list = lma::create(service, config.data_dir());
        Self {
            list_state: StatefulList::new(&anime_list),
            focused_window: FocusedWindow::MainMenu,
            insert_popup: InsertPopup::default(),
            titles_popup: TitlesPopup::default(),
            mismatch_popup: MismatchPopup::default(),
            anime_list,
            config,
        }
    }

    fn handle_login_popup<B: Backend>(
        &mut self,
        rt: &Runtime,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Box<dyn Error>> {
        rt.block_on(self.anime_list.service.auth());
        self.focused_window = FocusedWindow::Login;
        terminal.draw(|f| ui(f, self, rt))?;
        if !self.anime_list.service.is_logged_in() {
            rt.block_on(self.anime_list.service.login()); // freezes the app as it waits
            self.focused_window = FocusedWindow::MainMenu;
            terminal.draw(|f| ui(f, self, rt)).unwrap();
            self.focused_window = FocusedWindow::Login;
        }
        Ok(())
    }

    fn fill_with_api_data(&mut self) {
        let selected_show = self.titles_popup.selected_show();
        self.insert_popup.service_id = i64::from(selected_show.service_id);
        self.insert_popup.state = InsertState::Next;
        self.focused_window = FocusedWindow::InsertPopup;
    }
}

pub fn run<B: Backend, T: Service + Send>(
    terminal: &mut Terminal<B>,
    mut app: app::App<T>,
    tick_rate: Duration,
    rt: &Runtime,
) -> Result<(), Box<dyn Error>> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app, rt))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.focused_window {
                    FocusedWindow::MainMenu => {
                        if handle_main_menu_key(key, &mut app, rt, terminal)?.is_none() {
                            return Ok(());
                        }
                    }
                    FocusedWindow::Login => handle_login_key(key, &mut app),
                    FocusedWindow::InsertPopup => handle_insert_popup_key(&mut app, key),
                    FocusedWindow::TitleSelection => handle_title_selection_key(key, &mut app),
                    FocusedWindow::EpisodeMismatch => handle_mismatch_popup_key(key, &mut app),
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn handle_main_menu_key<B: Backend, T: Service + Send>(
    key: event::KeyEvent,
    app: &mut App<T>,
    rt: &Runtime,
    terminal: &mut Terminal<B>,
) -> Result<Option<bool>, Box<dyn Error>> {
    match key.code {
        KeyCode::Char('q') => return Ok(None),
        KeyCode::Down => app.list_state.move_selection(&SelectionDirection::Next, &app.anime_list),
        KeyCode::Up => app.list_state.move_selection(&SelectionDirection::Previous, &app.anime_list),
        KeyCode::Char('.') => app.list_state.move_progress(&SelectionDirection::Next, &mut app.anime_list, rt),
        KeyCode::Char(',') => app.list_state.move_progress(&SelectionDirection::Previous, &mut app.anime_list, rt),
        KeyCode::Right | KeyCode::Enter => app.list_state.select(),
        KeyCode::Left => app.list_state.unselect(),
        KeyCode::Delete => app.list_state.delete(&app.anime_list)?,
        KeyCode::Char('n') => {
            app.focused_window = FocusedWindow::InsertPopup;
            app.insert_popup.state = InsertState::Inputting;
        }
        KeyCode::Char('l') => {
            app.handle_login_popup(rt, terminal)?;
            app.anime_list.update_progress(rt);
        }
        _ => {}
    }
    Ok(Some(true))
}

#[allow(clippy::single_match)]
fn handle_login_key<T: Service>(key: event::KeyEvent, app: &mut App<T>) {
    match key.code {
        KeyCode::Esc => {
            app.focused_window = FocusedWindow::MainMenu;
            app.list_state.update_cache(&app.anime_list);
        },
        _ => {}
    }
}

fn handle_insert_popup_key<T: Service>(app: &mut App<T>, key: event::KeyEvent) {
    match app.insert_popup.state {
        InsertState::Inputting => match key.code {
            KeyCode::Char(c) => app.insert_popup.data.push(c),
            KeyCode::Backspace => _ = app.insert_popup.data.pop(),
            KeyCode::Esc => app.insert_popup.state = InsertState::None,
            KeyCode::Enter => {
                app.insert_popup.state = if app.insert_popup.move_line_selection(&SelectionDirection::Next) {
                    InsertState::Save
                } else {
                    InsertState::Next
                };
            }
            KeyCode::Down => {
                app.insert_popup.move_line_selection(&SelectionDirection::Next);
                app.insert_popup.state = InsertState::Next;
            }
            KeyCode::Up => {
                app.insert_popup.move_line_selection(&SelectionDirection::Previous);
                app.insert_popup.state = InsertState::Next;
            }
            _ => {}
        },
        _ => match key.code {
            KeyCode::Esc => {
                app.focused_window = FocusedWindow::MainMenu;
                app.insert_popup = InsertPopup::default();
            }
            KeyCode::Char('e') => app.insert_popup.state = InsertState::Inputting,
            KeyCode::Char('i') => app.insert_popup.state = InsertState::Save,
            KeyCode::Down => _=app.insert_popup.move_line_selection(&SelectionDirection::Next),
            KeyCode::Up => _=app.insert_popup.move_line_selection(&SelectionDirection::Previous),
            _ => {}
        }
    }
}

fn handle_title_selection_key<T: Service + Send>(key: event::KeyEvent, app: &mut App<T>) {
    match key.code {
        KeyCode::Down => app.titles_popup.move_selection(&SelectionDirection::Next),
        KeyCode::Up => app
            .titles_popup
            .move_selection(&SelectionDirection::Previous),
        KeyCode::Enter => app.fill_with_api_data(),
        KeyCode::Esc => app.focused_window = FocusedWindow::InsertPopup,
        _ => {}
    }
}

fn handle_mismatch_popup_key<T: Service + Send>(key: event::KeyEvent, app: &mut App<T>) {
    match key.code {
        KeyCode::Char(c) => app.mismatch_popup.owned_episodes.push(c),
        KeyCode::Backspace => _ = app.mismatch_popup.owned_episodes.pop(),
        KeyCode::Enter => {
            app.insert_popup.episodes = app.mismatch_popup.save::<T>(&app.insert_popup.path);
            app.focused_window = FocusedWindow::InsertPopup;
        },
        KeyCode::Esc => app.focused_window = FocusedWindow::InsertPopup,
        _ => {}
    }
}
