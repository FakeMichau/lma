use super::{centered_rect, title_selection::TitlesPopup, episode_mismatch::MismatchPopup};
use crate::{
    app::App,
    ui::{FocusedWindow, SelectionDirection},
};

use lma::{Episode, AnimeList, Service};
use std::collections::HashMap;
use ratatui::{
    backend::Backend,
    layout::Margin,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tokio::runtime::Runtime;

#[derive(Default)]
pub(crate) struct InsertPopup {
    pub(crate) path: String,
    title: String,
    pub(crate) service_id: i64,
    episode_count: i64,
    pub(crate) state: InsertState,
    pub(crate) data: String,
    pub(crate) episodes: Vec<Episode>,
    selected_line: usize,
}

#[derive(PartialEq)]
pub(crate) enum InsertState {
    None,
    Inputting,
    Next,
    Save,
}

impl Default for InsertState {
    fn default() -> Self {
        InsertState::None
    }
}

// path, title, sync_service_id, episode_count
const ENTRY_COUNT: usize = 4;
impl InsertPopup {
    pub(crate) fn current_line(&self) -> usize {
        self.selected_line
    }
    // return true on return to the beginning
    pub(crate) fn move_line_selection(&mut self, direction: SelectionDirection) -> bool {
        if self.state != InsertState::Inputting {
            return false;
        }
        match direction {
            SelectionDirection::Next => {
                self.selected_line = (self.selected_line + 1) % ENTRY_COUNT;
            }
            SelectionDirection::Previous => {
                self.selected_line = (self.selected_line + ENTRY_COUNT - 1) % ENTRY_COUNT;
            }
        }
        self.selected_line == 0
    }
}

pub(crate) fn build<B: Backend, T: Service>(frame: &mut Frame<B>, app: &mut App<T>, rt: &Runtime) {
    let area = centered_rect(70, 70, frame.size());
    let text_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    match app.insert_popup.state {
        InsertState::Inputting => handle_inputting_state(app),
        InsertState::Next => handle_next_state(app, rt),
        InsertState::Save => handle_save_state(app, rt),
        _ => {}
    }

    let input_form = vec![
        Line::from(vec![
            Span::raw("Path to the folder: "),
            Span::raw(app.insert_popup.path.clone()),
        ]),
        Line::from(vec![
            Span::raw("Show's title: "),
            Span::raw(app.insert_popup.title.clone()),
        ]),
        Line::from(vec![
            Span::raw("Sync service ID: "),
            Span::raw(app.insert_popup.service_id.to_string()),
        ]),
        Line::from(vec![
            Span::raw("Number of episodes: "),
            Span::raw(app.insert_popup.episode_count.to_string()),
        ]),
    ];
    if app.insert_popup.state == InsertState::Inputting {
        frame.set_cursor(
            text_area.x
                + input_form
                    .get(app.insert_popup.current_line())
                    .unwrap()
                    .width() as u16,
            text_area.y + app.insert_popup.current_line() as u16,
        );
    }

    let block = Block::default().title("Insert show").borders(Borders::ALL);
    // .wrap(Wrap { trim: true }); messes up the cursor position
    let form = Paragraph::new(input_form);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area)
}

fn handle_inputting_state<T: Service>(app: &mut App<T>) {
    match app.insert_popup.current_line() {
        0 => app.insert_popup.path = app.insert_popup.data.clone(),
        1 => app.insert_popup.title = app.insert_popup.data.clone(),
        2 => app.insert_popup.service_id = parse_number(&mut app.insert_popup.data),
        3 => app.insert_popup.episode_count = parse_number(&mut app.insert_popup.data),
        _ => {}
    }
}

fn handle_next_state<T: Service>(app: &mut App<T>, rt: &Runtime) {
    match app.insert_popup.current_line() {
        // after going to the next line, when data in the previous one is present
        1 if !app.insert_popup.path.is_empty() && app.insert_popup.title.is_empty() => {
            // sanitize user input
            app.insert_popup.title = app
                .anime_list
                .guess_shows_title(&app.insert_popup.path)
                .unwrap_or_default();
        }
        2 if !app.insert_popup.title.is_empty() && app.insert_popup.service_id == 0 => {
            // create a popup to select the exact show from a sync service
            let items: Vec<_> = rt.block_on(async { 
                app.anime_list
                    .service
                    .search_title(&app.insert_popup.title)
                    .await 
            });
            app.titles_popup = TitlesPopup::new(items);
            app.titles_popup.state.select(Some(0));
            app.focused_window = FocusedWindow::TitleSelection
        }
        3 if app.insert_popup.service_id != 0
            && app.insert_popup.episode_count == 0
            && !app.insert_popup.path.is_empty() =>
        {
            let title = rt.block_on(app.anime_list.service.get_title(app.insert_popup.service_id as u32));
            app.insert_popup.title = title; // make it a config?
            // compare number of video files with the retrieved number of episodes
            let episode_count = rt.block_on(
                app.anime_list
                    .service
                    .get_episode_count(app.insert_popup.service_id as u32)
                )
                .unwrap_or_default();
            let video_files_count = app
                .anime_list
                .count_video_files(&app.insert_popup.path)
                .unwrap_or_default() as u32;

            app.insert_popup.episode_count = episode_count.into();
            if episode_count == video_files_count {
                app.insert_popup.episodes = AnimeList::<T>::get_video_file_paths(&app.insert_popup.path)
                    .unwrap_or_default()
                    .into_iter()
                    .enumerate()
                    .map(|(k, path)| Episode {
                        number: k as i64 + 1,
                        path: path.clone(),
                        title: String::new(),
                        file_deleted: !path.exists(),
                        recap: false,
                        filler: false,
                    })
                    .collect();
            } else if episode_count > video_files_count {
                app.mismatch_popup = MismatchPopup::new(episode_count, video_files_count);
                app.focused_window = FocusedWindow::EpisodeMismatch;
            } else {
                // more files locally than expected
                app.insert_popup.episode_count = 0;
            }
        }
        _ => {}
    };
    app.insert_popup.data = match app.insert_popup.current_line() {
        0 if !app.insert_popup.path.is_empty() => app.insert_popup.path.clone(),
        1 if !app.insert_popup.title.is_empty() => app.insert_popup.title.clone(),
        2 if !app.insert_popup.service_id != 0 => app.insert_popup.service_id.to_string(),
        3 if !app.insert_popup.episode_count != 0 => app.insert_popup.episode_count.to_string(),
        _ => String::new(),
    };
    app.insert_popup.state = InsertState::Inputting;
}

fn handle_save_state<T: Service>(app: &mut App<T>, rt: &Runtime) {
    match app.anime_list.add_show(
        &app.insert_popup.title,
        app.insert_popup.service_id,
        0,
    ) {
        Ok(local_id) => {
            insert_episodes(rt, app, local_id);
            rt.block_on(async {
                app.anime_list
                    .service
                    .init_show(app.insert_popup.service_id as u32)
                    .await
            });
        },
        Err(why) => {
            if why.contains("constraint failed") {
                // show with this sync_service_id or title already exists
                // get local_id of the show with the same title
                if let Ok(local_id) = app.anime_list.get_local_show_id(&app.insert_popup.title) {
                    insert_episodes(rt, app, local_id);
                }
                // don't do anything more if can't get the id by title
            } else {
                eprintln!("{}", why);
            }
        },
    }
    app.insert_popup.state = InsertState::None;
    app.insert_popup = InsertPopup::default();
    app.list_state.update_cache(&app.anime_list);
    app.focused_window = FocusedWindow::MainMenu;
}

fn insert_episodes<T: Service>(rt: &Runtime, app: &mut App<T>, local_id: i64) {
    // service_id is fine because hashmap can be empty here
    let episodes_details_hash = rt.block_on(
        get_episodes_info(&mut app.anime_list.service, app.insert_popup.service_id as u32)
    );
    app.insert_popup.episodes.iter().for_each(|episode| {
        let potential_title = episodes_details_hash.get(&(episode.number as u32));
        let (title, recap, filler) = potential_title.unwrap_or(&(String::new(), false, false)).clone();

        if let Err(why) = app.anime_list.add_episode(
            local_id,
            episode.number,
            &episode.path.to_string_lossy().to_string(),
            &title,
            generate_extra_info(recap, filler)
        ) {
            eprintln!("{}", why);
        }
    });
}

async fn get_episodes_info<T: Service>(service: &mut T, id: u32) -> HashMap<u32, (String, bool, bool)> {
    let episodes_details = service.get_episodes(id).await.unwrap_or(Vec::new());
    episodes_details
        .iter()
        .map(|episode| {
            (
                episode.mal_id.unwrap_or_default(),
                (
                    episode.title.clone().unwrap_or_default(),
                    episode.recap.clone().unwrap_or_default(),
                    episode.filler.clone().unwrap_or_default(),
                ),
            )
        })
        .collect()
}

fn generate_extra_info(recap: bool, filler: bool) -> i64 {
    let mut extra_info: i64 = 0;
    if recap {
        extra_info |= 1 << 0;
    }
    if filler {
        extra_info |= 1 << 1;
    }
    extra_info
}

fn parse_number(str: &mut String) -> i64 {
    if let Ok(number) = str.trim().parse() {
        number
    } else {
        *str = String::new();
        0
    }
}
