use std::error::Error;
use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use tokio::runtime::Runtime;
use lma_lib::{AnimeList,Service};
use crate::app;
use crate::config::Config;
use crate::ui::{FocusedWindow, SelectionDirection, ui};
use crate::ui::main_menu::StatefulList;
use crate::ui::popup::first_setup::SetupPopup;
use crate::ui::popup::insert_episode::InsertEpisodePopup;
use crate::ui::popup::episode_mismatch::MismatchPopup;
use crate::ui::popup::title_selection::TitlesPopup;
use crate::ui::popup::insert_show::{InsertPopup, InsertState};

pub struct App<T: Service> {
    pub focused_window: FocusedWindow,
    pub insert_popup: InsertPopup,
    pub insert_episode_popup: InsertEpisodePopup,
    pub titles_popup: TitlesPopup,
    pub mismatch_popup: MismatchPopup,
    pub first_setup_popup: SetupPopup,
    pub list_state: StatefulList,
    pub anime_list: AnimeList<T>,
    pub config: Config,
    error: String,
}

impl<T: Service + Send> App<T> {
    pub fn build(rt: &Runtime, config: Config) -> Result<Self, String> {
        let service = rt.block_on(lma_lib::Service::new(config.data_dir().clone()))?;
        let anime_list = lma_lib::create(service, config.data_dir(), config.title_sort())?;
        Ok(Self {
            list_state: StatefulList::new(&anime_list)?,
            focused_window: FocusedWindow::MainMenu,
            insert_popup: InsertPopup::default(),
            insert_episode_popup: InsertEpisodePopup::default(),
            titles_popup: TitlesPopup::default(),
            mismatch_popup: MismatchPopup::default(),
            first_setup_popup: SetupPopup::new(),
            anime_list,
            config,
            error: String::new(),
        })
    }

    fn handle_login_popup<B: Backend>(
        &mut self,
        rt: &Runtime,
        terminal: &mut Terminal<B>,
    ) -> Result<(), String> {
        rt.block_on(self.anime_list.service.auth());
        self.focused_window = FocusedWindow::Login;
        terminal
            .draw(|f| ui(f, self, rt))
            .map_err(|err| err.to_string())?;
        if !self.anime_list.service.is_logged_in() {
            rt.block_on(self.anime_list.service.login())?; // freezes the app as it waits
            self.focused_window = FocusedWindow::MainMenu;
            terminal
                .draw(|f| ui(f, self, rt))
                .map_err(|err| err.to_string())?;
            self.focused_window = FocusedWindow::Login;
        }
        Ok(())
    }

    fn fill_with_api_data(&mut self) {
        self.insert_popup.service_id = i64::from(
            self.titles_popup
                .selected_show()
                .map(|show| show.service_id)
                .unwrap_or_default(),
        );
        self.insert_popup.state = InsertState::Next;
        self.focused_window = FocusedWindow::InsertPopup;
    }

    pub fn set_error(&mut self, error: String) {
        if self.error.is_empty() {
            self.error = error;
        }
    }

    pub fn error(&self) -> &str {
        self.error.as_ref()
    }
}

pub fn run<B: Backend, T: Service + Send>(
    terminal: &mut Terminal<B>,
    mut app: app::App<T>,
    tick_rate: Duration,
    rt: &Runtime,
) -> Result<(), Box<dyn Error>> {
    if !app.config.config_file_path().exists() {
        app.focused_window = FocusedWindow::FirstSetup;
    } else if app.config.data_dir().join("tokens").exists() {
        if app.config.update_progress_on_start() {
            println!("Updating your progress - please wait");
            app.anime_list.update_progress(rt)?;
        }
    } else {
        app.focused_window = FocusedWindow::Login;
        app.handle_login_popup(rt, terminal)?;
    }
    let mut last_tick = Instant::now();
    terminal.clear()?;
    loop {
        terminal.draw(|f| ui(f, &mut app, rt))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.focused_window {
                        FocusedWindow::MainMenu => {
                            match handle_main_menu_key(key, &mut app, rt, terminal) {
                                Ok(ok) => {
                                    if ok.is_none() {
                                        return Ok(());
                                    }
                                },
                                Err(err) => {
                                    app.set_error(err);
                                },
                            }
                        }
                        FocusedWindow::FirstSetup => {
                            if handle_first_setup_key(key, &mut app)?.is_none() {
                                return Ok(());
                            }
                        }
                        FocusedWindow::Login => handle_login_key(key, &mut app),
                        FocusedWindow::InsertPopup => handle_insert_popup_key(&mut app, key),
                        FocusedWindow::InsertEpisodePopup => handle_insert_episode_popup_key(&mut app, key),
                        FocusedWindow::TitleSelection => handle_title_selection_key(key, &mut app),
                        FocusedWindow::EpisodeMismatch => handle_mismatch_popup_key(key, &mut app),
                        FocusedWindow::Error => handle_error_key(key, &mut app),
                    }
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
) -> Result<Option<bool>, String> {
    let key_binds = app.config.key_binds();
    if key.code == key_binds.quit { 
        return Ok(None);
    } else if key.code == key_binds.move_down {
        app.list_state
            .move_selection(&SelectionDirection::Next, &app.anime_list)?;
    } else if key.code == key_binds.move_up {
        app.list_state
            .move_selection(&SelectionDirection::Previous, &app.anime_list)?;
    } else if key.code == key_binds.progress_inc {
        app.list_state
            .move_progress(&SelectionDirection::Next, &mut app.anime_list, rt)?;
    } else if key.code == key_binds.progress_dec {
        app.list_state
            .move_progress(&SelectionDirection::Previous, &mut app.anime_list, rt)?;
    } else if key.code == key_binds.forwards || key.code == key_binds.confirmation {
        app.list_state.select()?;
    } else if key.code == key_binds.backwards {
        app.list_state.unselect();
    } else if key.code == key_binds.delete {
        app.list_state.delete(&app.anime_list)?;
    } else if key.code == key_binds.new_show {
        app.focused_window = FocusedWindow::InsertPopup;
        app.insert_popup.state = InsertState::Inputting;
    } else if key.code == key_binds.new_episode && app.list_state.selected_show().is_some() {
        app.focused_window = FocusedWindow::InsertEpisodePopup;
        app.insert_episode_popup.state = InsertState::Inputting;
    } else if key.code == key_binds.login {
        app.handle_login_popup(rt, terminal)?;
        app.anime_list.update_progress(rt)?;
    }
    Ok(Some(true))
}

fn handle_login_key<T: Service + Send>(key: event::KeyEvent, app: &mut App<T>) {
    if key.code == app.config.key_binds().close {
        app.focused_window = FocusedWindow::MainMenu;
        if let Err(err) = app.list_state.update_cache(&app.anime_list) {
            app.set_error(err);
        };
    }
}

fn handle_first_setup_key<T: Service + Send>(
    key: event::KeyEvent,
    app: &mut App<T>,
) -> Result<Option<bool>, String> {
    let key_binds = app.config.key_binds();
    if key.code == key_binds.confirmation {
        if app.first_setup_popup.next_page() {
            app.first_setup_popup.reset();
            let selected_service = app.first_setup_popup.selected_service();
            app.config.create_personalized(selected_service.clone())?;
            return Ok(None);
        }
    } else if key.code == key_binds.close {
        app.first_setup_popup.previous_page();
    } else if key.code == key_binds.move_down {
        app.first_setup_popup.move_selection(&SelectionDirection::Next);
    } else if key.code == key_binds.move_up {
        app.first_setup_popup.move_selection(&SelectionDirection::Previous);
    }
    Ok(Some(true))
}

fn handle_error_key<T: Service>(key: event::KeyEvent, app: &mut App<T>) {
    let key_binds = app.config.key_binds();
    if key.code == key_binds.close || key.code == key_binds.confirmation {
        app.error = String::new();
        app.focused_window = FocusedWindow::MainMenu;
    }
}

fn handle_insert_popup_key<T: Service>(app: &mut App<T>, key: event::KeyEvent) {
    let key_binds = app.config.key_binds();
    match app.insert_popup.state {
        InsertState::Inputting => {
            match key.code {
                KeyCode::Char(c) => app.insert_popup.data.push(c),
                KeyCode::Backspace => _ = app.insert_popup.data.pop(),
                _ => {
                    if key.code == key_binds.close {
                        app.insert_popup.state = InsertState::None;
                    } else if key.code == key_binds.confirmation {
                        app.insert_popup.state = if app
                            .insert_popup
                            .move_line_selection(&SelectionDirection::Next)
                        {
                            InsertState::Save
                        } else {
                            InsertState::Next
                        };
                    } else if key.code == key_binds.move_down {
                        app.insert_popup
                            .move_line_selection(&SelectionDirection::Next);
                        app.insert_popup.state = InsertState::Next;
                    } else if key.code == key_binds.move_up {
                        app.insert_popup
                            .move_line_selection(&SelectionDirection::Previous);
                        app.insert_popup.state = InsertState::Next;
                    }
                }
            }
        }
        _ => {
            if key.code == key_binds.close {
                app.focused_window = FocusedWindow::MainMenu;
                app.insert_popup = InsertPopup::default();
            } else if key.code == key_binds.enter_inputting {
                app.insert_popup.state = InsertState::Inputting;
            } else if key.code == key_binds.move_down {
                _ = app
                    .insert_popup
                    .move_line_selection(&SelectionDirection::Next);
            } else if key.code == key_binds.move_up {
                _ = app
                    .insert_popup
                    .move_line_selection(&SelectionDirection::Previous);
            }
        }
    }
}

fn handle_insert_episode_popup_key<T: Service>(app: &mut App<T>, key: event::KeyEvent) {
    let key_binds = app.config.key_binds();
    match app.insert_episode_popup.state {
        InsertState::Inputting => {
            if key.code == key_binds.close {
                app.insert_episode_popup.state = InsertState::None;
            } else if key.code == key_binds.confirmation {
                app.insert_episode_popup.state = InsertState::Save;
            }
            match key.code {
                KeyCode::Char(c) => app.insert_episode_popup.data.push(c),
                KeyCode::Backspace => _ = app.insert_episode_popup.data.pop(),
                _ => {}
            }
        },
        _ => {
            if key.code == key_binds.close {
                app.focused_window = FocusedWindow::MainMenu;
                app.insert_episode_popup = InsertEpisodePopup::default();
            } else if key.code == key_binds.enter_inputting {
                app.insert_episode_popup.state = InsertState::Inputting;
            }
        }
    }
}

fn handle_title_selection_key<T: Service + Send>(key: event::KeyEvent, app: &mut App<T>) {
    let key_binds = app.config.key_binds();
    if key.code == key_binds.move_down {
        app.titles_popup.move_selection(&SelectionDirection::Next);
    } else if key.code == key_binds.move_up {
        app.titles_popup
            .move_selection(&SelectionDirection::Previous);
    } else if key.code == key_binds.confirmation {
        app.fill_with_api_data();
    } else if key.code == key_binds.close {
        app.focused_window = FocusedWindow::InsertPopup;
    }
}

fn handle_mismatch_popup_key<T: Service + Send>(key: event::KeyEvent, app: &mut App<T>) {
    let key_binds = app.config.key_binds();
    if key.code == key_binds.close {
        app.focused_window = FocusedWindow::InsertPopup;
    } else if key.code == key_binds.confirmation {
        match app.mismatch_popup.save::<T>(&app.insert_popup.path) {
            Ok(episodes) => app.insert_popup.episodes = episodes,
            Err(err) => app.set_error(err),
        };
        app.focused_window = FocusedWindow::InsertPopup;
    }
    match key.code {
        KeyCode::Char(c) => app.mismatch_popup.owned_episodes.push(c),
        KeyCode::Backspace => _ = app.mismatch_popup.owned_episodes.pop(),
        _ => {}
    }
}
