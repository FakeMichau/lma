pub mod main_menu;
pub mod popup;
use lma::Service;
use ratatui::{backend::Backend, Frame};
use tokio::runtime::Runtime;
use crate::app;
use self::popup::{insert_show::{InsertPopup, InsertState}, insert_episode::InsertEpisodePopup};

#[derive(PartialEq, Eq)]
pub enum FocusedWindow {
    MainMenu,
    InsertPopup,
    InsertEpisodePopup,
    Login,
    TitleSelection,
    EpisodeMismatch,
    Error,
}

#[derive(PartialEq, Eq)]
pub enum SelectionDirection {
    Next,
    Previous,
}

pub fn ui<B: Backend, T: Service + Send>(
    frame: &mut Frame<B>,
    app: &mut app::App<T>,
    rt: &Runtime,
) {
    let result: Result<(), String> = {
        // doesn't catch errors from main
        main_menu::build(frame, app);

        match app.focused_window {
            FocusedWindow::InsertEpisodePopup => popup::insert_episode::build(frame, app, rt),
            FocusedWindow::InsertPopup => popup::insert_show::build(frame, app, rt),
            FocusedWindow::Login => {
                popup::login::build(frame, app);
                Ok(())
            }
            FocusedWindow::TitleSelection => {
                popup::title_selection::build(frame, app);
                Ok(())
            }
            FocusedWindow::EpisodeMismatch => {
                popup::episode_mismatch::build(frame, app);
                Ok(())
            }
            // main menu is always drawn and error is drawn independently
            FocusedWindow::MainMenu | FocusedWindow::Error => Ok(()),
        }
    };
    app.set_error(result.err().unwrap_or_default());

    if !app.error().is_empty() {
        app.focused_window = FocusedWindow::Error;
        app.insert_popup = InsertPopup::default();
        app.insert_popup.state = InsertState::None;
        app.insert_episode_popup = InsertEpisodePopup::default();
        app.insert_episode_popup.state = InsertState::None;
        popup::error::build(frame, app);
    }
}
