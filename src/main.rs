use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lma::{self, Show};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
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
    episodes_state: EpisodesState,
    shows: Vec<(i64, Show)>,
}

struct EpisodesState {
    selected_id: i64,
    list_state: ListState,
    selection_enabled: bool,
}

#[derive(PartialEq)]
enum DirectionSelection {
    Next,
    Previous,
}

impl StatefulList {
    fn with_items(items: Vec<(i64, Show)>) -> StatefulList {
        StatefulList {
            state: ListState::default(),
            episodes_state: EpisodesState {
                selected_id: 0,
                list_state: ListState::default(),
                selection_enabled: false,
            },
            shows: items,
        }
    }

    fn move_selection(&mut self, direction: DirectionSelection) {
        if self.episodes_state.selection_enabled {
            self.move_episode_selection(direction);
        } else {
            let i = self.select_element(
                self.shows.len(), 
                self.state.selected(), 
                direction
            );
            self.state.select(Some(i));
            let selected_id = self.shows.get(i).unwrap().0;
            self.episodes_state.selected_id = selected_id;
        }
    }

    fn move_episode_selection(&mut self, direction: DirectionSelection) {
        let selected_show = self.shows.get(self.state.selected().unwrap()).unwrap();
        let episodes_len = selected_show.1.episodes.len();
        let i = self.select_element(
            episodes_len,
            self.episodes_state.list_state.selected(),
            direction,
        );
        self.episodes_state.list_state.select(Some(i));
    }

    fn select(&mut self) {
        match self.shows.get(self.state.selected().unwrap_or_default()) {
            Some(show) => {
                if show.1.episodes.len() > 0 {
                    self.episodes_state.list_state.select(Some(0));
                    self.episodes_state.selection_enabled = true;
                }
            }
            None => {}
        }
    }
    fn unselect(&mut self) {
        self.episodes_state.list_state.select(None);
        self.episodes_state.selection_enabled = false;
    }

    fn select_element(
        &mut self,
        list_length: usize,
        selected_element: Option<usize>,
        direction: DirectionSelection,
    ) -> usize {
        match direction {
            DirectionSelection::Next => match selected_element {
                Some(i) => {
                    if i >= list_length.checked_sub(1).unwrap_or_default() {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            },
            DirectionSelection::Previous => match selected_element {
                Some(i) => {
                    if i == 0 {
                        list_length.checked_sub(1).unwrap_or_default()
                    } else {
                        i - 1
                    }
                }
                None => 0,
            },
        }
    }
}

struct App {
    items: StatefulList,
}

impl App {
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
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.items.move_selection(DirectionSelection::Next),
                    KeyCode::Up => app.items.move_selection(DirectionSelection::Previous),
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
        .split(f.size());
    // Split the bigger chunk into halves
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    let items: Vec<_> = app
        .items
        .shows
        .iter()
        .map(|(_, show)| ListItem::new(format!("{}", show.title)).style(Style::default()))
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

    // Iterate through all elements in the `items` app
    let episodes: Vec<ListItem> = app
        .items
        .shows
        .iter()
        .filter(|(id, _)| *id == app.items.episodes_state.selected_id)
        .flat_map(|(_, show)| {
            let mut temp: Vec<ListItem> = Vec::new();
            for (episode_number, path) in &show.episodes {
                temp.push(
                    ListItem::new(format!("{} {}", episode_number, path)).style(Style::default()),
                );
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

    // Create help text at the bottom
    let help = Block::default().title("Help").borders(Borders::ALL);

    // We can now render the item list
    f.render_stateful_widget(items, main_chunks[0], &mut app.items.state);
    f.render_stateful_widget(episodes, main_chunks[1], &mut app.items.episodes_state.list_state);
    f.render_widget(help, chunks[1]);
}
