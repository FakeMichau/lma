use crate::app;
use crate::ui::{
    self,
    FocusedWindow,
    main_menu::{Direction, StatefulList},
};
use crossterm::event::{self, Event, KeyCode};
use std::error::Error;
use std::{
    io,
    time::{Duration, Instant},
};
use tui::{backend::Backend, Terminal};

pub(crate) struct App {
    pub(crate) items: StatefulList,
    pub(crate) focused_window: FocusedWindow,
    pub(crate) input: InputState,
}

#[derive(Default)]
pub(crate) struct InputState {
    pub(crate) inputting: bool,
    pub(crate) data: String,
    pub(crate) confirmation: bool,
}


impl App {
    pub(crate) fn build() -> Result<App, Box<dyn Error>> {
        let anime_list = lma::create();
        Ok(App {
            items: StatefulList::with_items(anime_list),
            focused_window: FocusedWindow::MainMenu,
            input: InputState::default(),
        })
    }

    fn generate_test_data(&self) -> bool {
        for i in 1..6 {
            _=self.items.shows.add_show(format!("Show {}", i).as_str(), 1000 + i, 12*(i%3)+1, 5*(i%3)+1);
            for e in 1..i+2 {
                _=self.items.shows.add_episode(i, e, format!("/path/to/episode{}.mp4", e).as_str());
            }
        }
        true
    }
}

pub(crate) fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: app::App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui::ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.focused_window {
                    FocusedWindow::MainMenu => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down => app.items.move_selection(Direction::Next),
                        KeyCode::Up => app.items.move_selection(Direction::Previous),
                        KeyCode::Right => app.items.select(),
                        KeyCode::Left => app.items.unselect(),
                        KeyCode::Char('n') => app.focused_window = FocusedWindow::InsertPopup,
                        KeyCode::Char('p') => debug_assert!(app.generate_test_data()),
                        _ => {}
                    },
                    FocusedWindow::InsertPopup => match app.input.inputting {
                        true => match key.code {
                            KeyCode::Enter => {
                                app.input.confirmation = true;
                            }
                            KeyCode::Char(c) => {
                                app.input.data.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input.data.pop();
                            }
                            KeyCode::Esc => {
                                app.input.inputting = false;
                            }
                            _ => {}
                        },
                        false => match key.code {
                            KeyCode::Char('n') => app.focused_window = FocusedWindow::MainMenu,
                            KeyCode::Char('e') => app.input.inputting = true,
                            _ => {}
                        },
                    }
                }
                
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
