use crate::app;
use crate::ui::{
    self,
    interactions::{Direction, StatefulList},
};
use crossterm::event::{self, Event, KeyCode};
use lma::AnimeList;
use std::error::Error;
use std::{
    io,
    time::{Duration, Instant},
};
use tui::{backend::Backend, Terminal};

pub(crate) struct App {
    pub(crate) items: StatefulList,
    anime_list: AnimeList
}

impl App {
    pub(crate) fn build() -> Result<App, Box<dyn Error>> {
        let anime_list = lma::create();
        let list = anime_list.get_list()?;
        Ok(App {
            items: StatefulList::with_items(list),
            anime_list
        })
    }

    fn generate_test_data(&self) -> bool {
        for i in 1..6 {
            _=self.anime_list.add_show(format!("Show {}", i).as_str(), 1000 + i, 12*(i%3)+1, 5*(i%3)+1);
            for e in 1..i+2 {
                _=self.anime_list.add_episode(i, e, format!("/path/to/episode{}.mp4", e).as_str());
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
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.items.move_selection(Direction::Next),
                    KeyCode::Up => app.items.move_selection(Direction::Previous),
                    KeyCode::Right => app.items.select(),
                    KeyCode::Left => app.items.unselect(),
                    KeyCode::Char('p') => debug_assert!(app.generate_test_data()),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
