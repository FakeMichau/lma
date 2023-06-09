use crate::app;
use crate::ui::popup::title_selection::TitlesPopup;
use crate::ui::{
    self,
    main_menu::StatefulList,
    popup::insert_show::{InsertPopup, InsertState},
    FocusedWindow, SelectionDirection,
};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{backend::Backend, Terminal};
use std::error::Error;
use std::{
    io,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

pub(crate) struct App {
    pub(crate) shows: StatefulList,
    pub(crate) focused_window: FocusedWindow,
    pub(crate) insert_popup: InsertPopup,
    pub(crate) titles_popup: TitlesPopup,
}

impl App {
    pub(crate) fn build(rt: &Runtime) -> Result<App, Box<dyn Error>> {
        let service = rt.block_on(async { lma::MAL::new().await });
        let anime_list = lma::create(service);
        Ok(App {
            shows: StatefulList::with_items(anime_list),
            focused_window: FocusedWindow::MainMenu,
            insert_popup: InsertPopup::default(),
            titles_popup: TitlesPopup::default(),
        })
    }

    fn generate_test_data(&self) -> bool {
        for i in 1..6 {
            _ = self.shows.items.add_show(
                format!("Show {}", i).as_str(),
                1000 + i,
                12 * (i % 3) + 1,
                5 * (i % 3) + 1,
            );
            for e in 1..i + 2 {
                _ = self.shows.items.add_episode(
                    i,
                    e,
                    format!("/path/to/episode{}.mp4", e).as_str(),
                );
            }
        }
        true
    }

    async fn handle_login_popup_async<B: Backend>(
        &mut self,
        rt: &Runtime,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        self.shows.items.service.auth().await;
        self.focused_window = FocusedWindow::Login;
        terminal.draw(|f| ui::ui(f, self, rt))?;
        if self.shows.items.service.get_url().is_some() {
            self.shows.items.service.login().await; // freezes the app as it waits
            self.focused_window = FocusedWindow::MainMenu;
            terminal.draw(|f| ui::ui(f, self, rt)).unwrap();
            self.focused_window = FocusedWindow::Login;
        }
        Ok(())
    }

    fn handle_login_popup<B: Backend>(
        &mut self,
        rt: &Runtime,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        rt.block_on(self.handle_login_popup_async(rt, terminal))?;
        Ok(())
    }

    fn fill_with_api_data(&mut self) {
        let selected_show = self.titles_popup.selected_show();
        self.insert_popup.sync_service_id = selected_show.id as i64;
        self.insert_popup.title = selected_show.title.to_owned(); // make it a config?
        self.insert_popup.state = InsertState::Next;
        self.focused_window = FocusedWindow::InsertPopup
    }
}

pub(crate) fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: app::App,
    tick_rate: Duration,
    rt: Runtime,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui::ui(f, &mut app, &rt))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.focused_window {
                    FocusedWindow::MainMenu => {
                        if handle_main_menu_key(key, &mut app, &rt, terminal)?.is_none() {
                            return Ok(());
                        }
                    }
                    FocusedWindow::Login => handle_login_key(key, &mut app),
                    FocusedWindow::InsertPopup => handle_insert_popup_key(&mut app, key),
                    FocusedWindow::TitleSelection => handle_title_selection_key(key, &mut app),
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn handle_main_menu_key<B: Backend>(
    key: event::KeyEvent,
    app: &mut App,
    rt: &Runtime,
    terminal: &mut Terminal<B>,
) -> Result<Option<bool>, io::Error> {
    match key.code {
        KeyCode::Char('q') => return Ok(None),
        KeyCode::Down => app.shows.move_selection(SelectionDirection::Next),
        KeyCode::Up => app.shows.move_selection(SelectionDirection::Previous),
        KeyCode::Right => app.shows.select(),
        KeyCode::Left => app.shows.unselect(),
        KeyCode::Char('n') => app.focused_window = FocusedWindow::InsertPopup,
        KeyCode::Char('l') => app.handle_login_popup(rt, terminal)?,
        KeyCode::Char('p') => debug_assert!(app.generate_test_data()),
        _ => {}
    }
    return Ok(Some(true));
}

fn handle_login_key(key: event::KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => app.focused_window = FocusedWindow::MainMenu,
        _ => {}
    }
}

fn handle_insert_popup_key(app: &mut App, key: event::KeyEvent) {
    match app.insert_popup.state {
        InsertState::Inputting => match key.code {
            KeyCode::Char(c) => app.insert_popup.data.push(c),
            KeyCode::Backspace => _ = app.insert_popup.data.pop(),
            KeyCode::Esc => app.insert_popup.state = InsertState::None,
            KeyCode::Enter => {
                app.insert_popup.state = if app.insert_popup.next_line() {
                    InsertState::Save
                } else {
                    InsertState::Next
                };
            }
            KeyCode::Down => {
                app.insert_popup.next_line();
                app.insert_popup.state = InsertState::Next;
            }
            KeyCode::Up => {
                app.insert_popup.previous_line();
                app.insert_popup.state = InsertState::Next;
            }
            _ => {}
        },
        _ => match key.code {
            KeyCode::Esc => {
                app.focused_window = FocusedWindow::MainMenu;
                app.insert_popup = InsertPopup::default()
            }
            KeyCode::Char('e') => app.insert_popup.state = InsertState::Inputting,
            KeyCode::Char('i') => app.insert_popup.state = InsertState::Save,
            KeyCode::Down => _ = app.insert_popup.next_line(),
            KeyCode::Up => app.insert_popup.previous_line(),
            _ => {}
        },
    }
}

fn handle_title_selection_key(key: event::KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Down => app.titles_popup.move_selection(SelectionDirection::Next),
        KeyCode::Up => app
            .titles_popup
            .move_selection(SelectionDirection::Previous),
        KeyCode::Enter => app.fill_with_api_data(),
        KeyCode::Esc => app.focused_window = FocusedWindow::InsertPopup,
        _ => {}
    }
}
