use crate::app::App;
use crate::ui::popup::insert_episode::InsertEpisodePopup;
use crate::ui::popup::insert_show::{InsertPopup, InsertState};
use crate::ui::{FocusedWindow, SelectionDirection};
use crossterm::event::{self, KeyCode};
use lma_lib::Service;
use ratatui::{backend::Backend, Terminal};
use tokio::runtime::Runtime;

pub fn main_menu<B: Backend, T: Service>(
    key: event::KeyEvent,
    app: &mut App<T>,
    rt: &Runtime,
    terminal: &mut Terminal<B>,
) -> Result<Option<bool>, String> {
    let key_binds = &app.config.key_binds;
    if key.code == key_binds.quit {
        return Ok(None);
    } else if key.code == key_binds.move_down {
        rt.block_on(
            app.list_state
                .move_selection(&SelectionDirection::Next, &app.anime_list),
        )?;
    } else if key.code == key_binds.move_up {
        rt.block_on(
            app.list_state
                .move_selection(&SelectionDirection::Previous, &app.anime_list),
        )?;
    } else if key.code == key_binds.progress_inc {
        rt.block_on(
            app.list_state
                .move_progress(&SelectionDirection::Next, &mut app.anime_list),
        )?;
    } else if key.code == key_binds.progress_dec {
        rt.block_on(
            app.list_state
                .move_progress(&SelectionDirection::Previous, &mut app.anime_list),
        )?;
    } else if key.code == key_binds.forwards || key.code == key_binds.confirmation {
        app.list_state.select(app.list_state.last_height)?;
    } else if key.code == key_binds.backwards || key.code == key_binds.close {
        app.list_state.unselect();
    } else if key.code == key_binds.delete {
        rt.block_on(app.list_state.delete(&app.anime_list))?;
    } else if key.code == key_binds.new_show {
        app.focused_window = FocusedWindow::InsertPopup;
        app.insert_popup.state = InsertState::Inputting;
    } else if key.code == key_binds.new_episode && app.list_state.selected_show().is_some() {
        app.focused_window = FocusedWindow::InsertEpisodePopup;
        app.insert_episode_popup.state = InsertState::Inputting;
    } else if key.code == key_binds.login {
        app.handle_login(rt, terminal)?;
        rt.block_on(app.anime_list.update_progress())?;
    }
    Ok(Some(true))
}

pub fn login<T: Service>(key: event::KeyEvent, app: &mut App<T>, rt: &Runtime) {
    if key.code == app.config.key_binds.close {
        app.focused_window = FocusedWindow::MainMenu;
        if let Err(err) = rt.block_on(app.list_state.update_cache(&app.anime_list)) {
            app.set_error(err);
        };
    }
}

pub fn first_setup<T: Service>(
    key: event::KeyEvent,
    app: &mut App<T>,
) -> Result<Option<bool>, String> {
    let key_binds = &app.config.key_binds;
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
        app.first_setup_popup
            .move_selection(&SelectionDirection::Next);
    } else if key.code == key_binds.move_up {
        app.first_setup_popup
            .move_selection(&SelectionDirection::Previous);
    }
    Ok(Some(true))
}

pub fn error<T: Service>(key: event::KeyEvent, app: &mut App<T>) {
    let key_binds = &app.config.key_binds;
    if key.code == key_binds.close || key.code == key_binds.confirmation {
        app.set_error(String::new());
        app.focused_window = FocusedWindow::MainMenu;
    }
}

pub fn insert_popup<T: Service>(app: &mut App<T>, key: event::KeyEvent) {
    let key_binds = &app.config.key_binds;
    match app.insert_popup.state {
        InsertState::Inputting => match key.code {
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
        },
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

pub fn insert_episode_popup<T: Service>(app: &mut App<T>, key: event::KeyEvent) {
    let key_binds = &app.config.key_binds;
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
        }
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

pub fn title_selection<T: Service>(key: event::KeyEvent, app: &mut App<T>) {
    let key_binds = &app.config.key_binds;
    if key.code == key_binds.move_down {
        app.titles_popup.move_selection(&SelectionDirection::Next);
    } else if key.code == key_binds.move_up {
        app.titles_popup
            .move_selection(&SelectionDirection::Previous);
    } else if key.code == key_binds.confirmation {
        app.insert_popup.service_id = app
            .titles_popup
            .selected_show()
            .map(|show| show.service_id)
            .unwrap_or_default();
        app.insert_popup.state = InsertState::Next;
        app.focused_window = FocusedWindow::InsertPopup;
    } else if key.code == key_binds.close {
        app.focused_window = FocusedWindow::InsertPopup;
    }
}

pub fn mismatch_popup<T: Service>(key: event::KeyEvent, app: &mut App<T>) {
    let key_binds = &app.config.key_binds;
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
