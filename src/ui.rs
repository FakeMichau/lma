use lma::Service;
use ratatui::{backend::Backend, Frame};
use tokio::runtime::Runtime;
pub(crate) mod main_menu;
pub(crate) mod popup;
use crate::app;

#[derive(PartialEq)]
pub(crate) enum FocusedWindow {
    MainMenu,
    InsertPopup,
    Login,
    TitleSelection,
    EpisodeMismatch,
}

#[derive(PartialEq)]
pub(crate) enum SelectionDirection {
    Next,
    Previous,
}

pub(crate) fn ui<B: Backend, T: Service>(frame: &mut Frame<B>, mut app: &mut app::App<T>, rt: &Runtime) {
    main_menu::build(frame, &mut app);

    match app.focused_window {
        FocusedWindow::InsertPopup => popup::insert_show::build(frame, &mut app, &rt),
        FocusedWindow::Login => popup::login::build(frame, &mut app),
        FocusedWindow::TitleSelection => popup::title_selection::build(frame, &mut app),
        FocusedWindow::EpisodeMismatch => popup::episode_mismatch::build(frame, &mut app),
        _ => {}
    }
}
