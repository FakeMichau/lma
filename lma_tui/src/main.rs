mod app;
mod config;
mod handle_input;
mod ui;
use config::Config;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use lma_lib::{Local, MALClient, ServiceType, MAL};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::error::Error;
use std::io::{self, Stdout};
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;

    let tick_rate = Duration::from_millis(250);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let config = Config::default()?;
    let run_result = match config.service() {
        ServiceType::MAL => {
            let app = app::App::<MAL<MALClient>>::build(&rt, config)?;
            app::run(&mut terminal, app, tick_rate, &rt)
        }
        ServiceType::Local => {
            let app = app::App::<Local>::build(&rt, config)?;
            app::run(&mut terminal, app, tick_rate, &rt)
        }
    };

    restore_terminal(&mut terminal)?;

    if let Err(why) = run_result {
        eprintln!("{why:?}");
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    Ok(terminal.show_cursor()?)
}
