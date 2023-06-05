mod app;
mod ui;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io::{self, Stdout},
    process,
    time::Duration,
};

use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;

    let tick_rate = Duration::from_millis(250);
    let app = app::App::build().await.unwrap_or_else(|why| {
        eprintln!("App couldn't be build: {why}");
        process::exit(1)
    });
    let run_result = app::run(&mut terminal, app, tick_rate).await;

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
