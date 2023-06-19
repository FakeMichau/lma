mod app;
mod ui;
mod config;
use config::Config;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lma::MAL;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io::{self, Stdout},
    process,
    time::Duration,
};

use tokio;

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;

    let tick_rate = Duration::from_millis(250);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    
    let config = Config::default();
    let run_result = if config.service().to_ascii_lowercase() == "mal" {
        let app = app::App::<MAL>::build(&rt, config).unwrap_or_else(|why| {
            eprintln!("App couldn't be build: {why}");
            process::exit(1)
        });
        app::run(&mut terminal, app, tick_rate, rt)
    } else {
        let app = app::App::<MAL>::build(&rt, config).unwrap_or_else(|why| {
            eprintln!("App couldn't be build: {why}");
            process::exit(1)
        });
        app::run(&mut terminal, app, tick_rate, rt)
    };

    restore_terminal(&mut terminal)?;

    if let Err(why) = run_result {
        eprintln!("{:?}", why)
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
