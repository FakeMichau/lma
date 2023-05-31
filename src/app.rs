use crate::app;
use crate::ui::{
    self,
    interactions::{Direction, StatefulList},
};
use crossterm::event::{self, Event, KeyCode};
use std::{
    io,
    time::{Duration, Instant},
};
use tui::{backend::Backend, Terminal};

pub(crate) struct App {
    pub(crate) items: StatefulList,
}

impl App {
    pub(crate) fn new() -> App {
        let anime_list = lma::create();
        let data = anime_list.get_list();
        let list = match data {
            Ok(result) => result,
            Err(why) => panic!("{}", why),
        };
        App {
            items: StatefulList::with_items(list),
        }
    }
}

pub(crate) fn run_app<B: Backend>(
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
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.items.move_selection(Direction::Next),
                    KeyCode::Up => app.items.move_selection(Direction::Previous),
                    KeyCode::Right => app.items.select(),
                    KeyCode::Left => app.items.unselect(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
