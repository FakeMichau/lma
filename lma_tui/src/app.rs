use crate::app;
use crate::config::Config;
use crate::handle_input;
use crate::ui::main_menu::StatefulList;
use crate::ui::popup::episode_mismatch::MismatchPopup;
use crate::ui::popup::first_setup::SetupPopup;
use crate::ui::popup::insert_episode::InsertEpisodePopup;
use crate::ui::popup::insert_show::InsertPopup;
use crate::ui::popup::title_selection::TitlesPopup;
use crate::ui::{ui, FocusedWindow};
use crossterm::event::{self, Event, KeyEventKind};
use lma_lib::{AnimeList, Service};
use ratatui::{backend::Backend, Terminal};
use std::error::Error;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

pub struct App<T: Service> {
    pub focused_window: FocusedWindow,
    pub insert_popup: InsertPopup,
    pub insert_episode_popup: InsertEpisodePopup,
    pub titles_popup: TitlesPopup,
    pub mismatch_popup: MismatchPopup,
    pub first_setup_popup: SetupPopup,
    pub list_state: StatefulList,
    pub anime_list: AnimeList<T>,
    pub config: Config,
    error: String,
}

impl<T: Service> App<T> {
    pub async fn build(config: Config) -> Result<Self, String> {
        let service = lma_lib::Service::new(config.data_dir.clone()).await?;
        let anime_list = lma_lib::create(service, &config.data_dir, &config.title_sort).await?;
        Ok(Self {
            list_state: StatefulList::new(&anime_list).await?,
            focused_window: FocusedWindow::MainMenu,
            insert_popup: InsertPopup::default(),
            insert_episode_popup: InsertEpisodePopup::default(),
            titles_popup: TitlesPopup::default(),
            mismatch_popup: MismatchPopup::default(),
            first_setup_popup: SetupPopup::new(),
            anime_list,
            config,
            error: String::new(),
        })
    }

    pub fn handle_login<B: Backend>(
        &mut self,
        rt: &Runtime,
        terminal: &mut Terminal<B>,
    ) -> Result<(), String> {
        rt.block_on(self.anime_list.service.auth());
        self.focused_window = FocusedWindow::Login;
        terminal
            .draw(|f| ui(f, self, rt))
            .map_err(|err| err.to_string())?;
        if !self.anime_list.service.is_logged_in() {
            rt.block_on(self.anime_list.service.login())?; // freezes the app as it waits
            self.focused_window = FocusedWindow::MainMenu;
            terminal
                .draw(|f| ui(f, self, rt))
                .map_err(|err| err.to_string())?;
            self.focused_window = FocusedWindow::Login;
        }
        Ok(())
    }

    pub fn set_error(&mut self, error: String) {
        if self.error.is_empty() {
            self.error = error;
        }
    }

    pub fn clear_error(&mut self) {
        self.error = String::new();
    }

    pub fn error(&self) -> &str {
        self.error.as_ref()
    }
}

pub fn run<B: Backend, T: Service>(
    terminal: &mut Terminal<B>,
    mut app: app::App<T>,
    tick_rate: Duration,
    rt: &Runtime,
) -> Result<(), Box<dyn Error>> {
    if !app.config.config_file_path.exists() {
        app.focused_window = FocusedWindow::FirstSetup;
    } else if app.config.data_dir.join("tokens").exists() {
        if app.config.update_progress_on_start {
            println!("Updating your progress - please wait");
            rt.block_on(app.anime_list.update_progress())?;
        }
    } else {
        app.focused_window = FocusedWindow::Login;
        app.handle_login(rt, terminal)?;
    }
    let mut last_tick = Instant::now();
    terminal.clear()?;
    loop {
        terminal.draw(|f| ui(f, &mut app, rt))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.focused_window {
                        FocusedWindow::MainMenu => {
                            match handle_input::main_menu(key, &mut app, rt, terminal) {
                                Ok(ok) => {
                                    if ok.is_none() {
                                        return Ok(());
                                    }
                                }
                                Err(err) => {
                                    app.set_error(err);
                                }
                            }
                        }
                        FocusedWindow::FirstSetup => {
                            if handle_input::first_setup(key, &mut app)?.is_none() {
                                return Ok(());
                            }
                        }
                        FocusedWindow::Login => handle_input::login(key, &mut app, rt),
                        FocusedWindow::InsertPopup => handle_input::insert_popup(&mut app, key),
                        FocusedWindow::InsertEpisodePopup => {
                            handle_input::insert_episode_popup(&mut app, key);
                        }
                        FocusedWindow::TitleSelection => {
                            handle_input::title_selection(key, &mut app);
                        }
                        FocusedWindow::EpisodeMismatch => {
                            handle_input::mismatch_popup(key, &mut app);
                        }
                        FocusedWindow::Error => handle_input::error(key, &mut app),
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
