use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lma;
use std::{
    error::Error,
    io,
    time::{Duration, Instant}, collections::HashMap,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame, Terminal,
};

struct StatefulList {
    state: ListState,
    episodes_state: (i64, ListState, bool),
    items: HashMap<i64, lma::Show>,
}

impl StatefulList
{
    fn with_items(items: HashMap<i64, lma::Show>) -> StatefulList {
        StatefulList {
            state: ListState::default(),
            episodes_state: (0, ListState::default(), false),
            items,
        }
    }

    fn next(&mut self) {
        if self.episodes_state.2 {
            return self.next_episode()
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len().checked_sub(1).unwrap_or_default() {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.episodes_state.0 = i as i64;
    }

    fn previous(&mut self) {
        if self.episodes_state.2 {
            return self.previous_episode()
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len().checked_sub(1).unwrap_or_default()
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.episodes_state.0 = i as i64;
    }

    fn next_episode(&mut self) {
        let i = match self.episodes_state.1.selected() {
            Some(i) => {
                if i >= self.items.get(&(self.state.selected().unwrap() as i64)).unwrap().episodes.len().checked_sub(1).unwrap_or_default() {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.episodes_state.1.select(Some(i));
    }

    fn previous_episode(&mut self) {
        let i = match self.episodes_state.1.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.get(&(self.state.selected().unwrap() as i64)).unwrap().episodes.len().checked_sub(1).unwrap_or_default()
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.episodes_state.1.select(Some(i));
    }

    fn select(&mut self) {
        match self.items.get(&(self.state.selected().unwrap() as i64)) {
            Some(show) => {
                if show.episodes.len() > 0 {
                    self.episodes_state.1.select(Some(0));
                    self.episodes_state.2 = true;
                }
            },
            None => {},
        }
    }
    fn unselect(&mut self) {
        self.episodes_state.1.select(None);
        self.episodes_state.2 = false;
    }
}

struct App {
    items: StatefulList,
}

impl<'a> App {
    fn new() -> App {
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

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    app.items.state.select(Some(0));
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.items.next(),
                    KeyCode::Up => app.items.previous(),
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

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|(id, _)| {
            ListItem::new(format!("{}", id)).style(Style::default())
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        // Iterate through all elements in the `items` app and append some debug text to it.
    let episodes: Vec<ListItem> = app
        .items
        .items
        .iter()
        .filter(|(id, _)| **id == app.items.episodes_state.0)
        .flat_map(|(_, show)| {
            let mut temp: Vec<ListItem> = Vec::new();
            for (episode_number, path) in &show.episodes {
                temp.push(ListItem::new(format!("{} {}", episode_number, path)).style(Style::default()));
            }
            temp
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let episodes = List::new(episodes)
        .block(Block::default().borders(Borders::ALL).title("Episodes"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");


    // We can now render the item list
    f.render_stateful_widget(items, chunks[0], &mut app.items.state);
    f.render_stateful_widget(episodes, chunks[1], &mut app.items.episodes_state.1);
}
