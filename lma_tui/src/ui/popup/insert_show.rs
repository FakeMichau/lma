use super::{centered_rect, episode_mismatch::MismatchPopup, title_selection::TitlesPopup};
use crate::app::App;
use crate::ui::{FocusedWindow, SelectionDirection};
use lma_lib::{AnimeList, Episode, Service, ServiceType};
use ratatui::backend::Backend;
use ratatui::layout::Margin;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::runtime::Runtime;

#[derive(Default)]
pub struct InsertPopup {
    pub path: PathBuf,
    title: String,
    pub service_id: usize,
    episode_count: usize,
    pub state: InsertState,
    pub data: String,
    pub episodes: Vec<Episode>,
    selected_line: usize,
}

#[derive(Default, PartialEq, Eq)]
pub enum InsertState {
    #[default]
    None,
    Inputting,
    Next,
    Save,
}

// path, title, sync_service_id, episode_count
const ENTRY_COUNT: usize = 4;
impl InsertPopup {
    pub const fn current_line(&self) -> usize {
        self.selected_line
    }
    // return true on return to the beginning
    pub fn move_line_selection(&mut self, direction: &SelectionDirection) -> bool {
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

pub fn build<B: Backend, T: Service + Send>(
    frame: &mut Frame<B>,
    app: &mut App<T>,
    rt: &Runtime,
) -> Result<(), String> {
    let area = centered_rect(70, 70, frame.size());
    let text_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    match app.insert_popup.state {
        InsertState::Inputting => handle_inputting_state(app),
        InsertState::Next => handle_next_state(app, rt)?,
        InsertState::Save => handle_save_state(app, rt)?,
        InsertState::None => {}
    }

    let input_form = vec![
        Line::from(vec![
            Span::raw("Path to the folder: "),
            Span::raw(app.insert_popup.path.to_string_lossy()),
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
                + u16::try_from(
                    input_form
                        .get(app.insert_popup.current_line())
                        .map(Line::width)
                        .unwrap_or_default(),
                )
                .unwrap_or_default(),
            text_area.y + u16::try_from(app.insert_popup.current_line()).unwrap_or_default(),
        );
    }

    let block = Block::default().title("Insert show").borders(Borders::ALL);
    // .wrap(Wrap { trim: true }); messes up the cursor position
    let form = Paragraph::new(input_form);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area);
    Ok(())
}

fn handle_inputting_state<T: Service>(app: &mut App<T>) {
    match app.insert_popup.current_line() {
        0 => app.insert_popup.path = app.insert_popup.data.clone().into(),
        1 => app.insert_popup.title = app.insert_popup.data.clone(),
        2 => app.insert_popup.service_id = parse_number(&mut app.insert_popup.data),
        3 => app.insert_popup.episode_count = parse_number(&mut app.insert_popup.data),
        _ => {}
    }
}

fn handle_next_state<T: Service>(app: &mut App<T>, rt: &Runtime) -> Result<(), String> {
    match app.insert_popup.current_line() {
        1 if !app.insert_popup.path.to_string_lossy().is_empty()
            && app.insert_popup.title.is_empty() =>
        {
            // when there's a path and no title yet
            handle_second_line(app)
        }
        2 if !app.insert_popup.title.is_empty()
            && app.insert_popup.service_id == 0
            && app.anime_list.service.get_service_type() != ServiceType::Local =>
        {
            // when there's title and no service id
            handle_third_line(app, rt)
        }
        3 if (app.insert_popup.service_id != 0
            || app.anime_list.service.get_service_type() == ServiceType::Local)
            && app.insert_popup.episode_count == 0
            && !app.insert_popup.path.to_string_lossy().is_empty() =>
        {
            // when there's service id and path but no episodes
            handle_forth_line(app, rt)
        }
        _ => Ok(()),
    }?;
    app.insert_popup.data = match app.insert_popup.current_line() {
        0 if !app.insert_popup.path.to_string_lossy().is_empty() => {
            app.insert_popup.path.to_string_lossy().into()
        }
        1 if !app.insert_popup.title.is_empty() => app.insert_popup.title.clone(),
        2 if !app.insert_popup.service_id != 0 => app.insert_popup.service_id.to_string(),
        3 if !app.insert_popup.episode_count != 0 => app.insert_popup.episode_count.to_string(),
        _ => String::new(),
    };
    app.insert_popup.state = InsertState::Inputting;
    Ok(())
}

fn handle_second_line<T: Service>(app: &mut App<T>) -> Result<(), String> {
    // trim path
    let matches: &[_] = &['"', '\''];
    app.insert_popup.path =
        app.insert_popup
            .path
            .to_str()
            .map_or(app.insert_popup.path.clone(), |str| {
                if str.starts_with('\"') | str.starts_with('\'') {
                    PathBuf::from(str.trim_matches(matches))
                } else {
                    PathBuf::from(str)
                }
            });
    app.insert_popup.title = app.anime_list.guess_shows_title(&app.insert_popup.path)?;
    Ok(())
}

fn handle_third_line<T: Service>(app: &mut App<T>, rt: &Runtime) -> Result<(), String> {
    // create a popup to select the exact show from a sync service
    let items = rt.block_on(async {
        app.anime_list
            .service
            .search_title(&app.insert_popup.title)
            .await
    })?;
    app.titles_popup = TitlesPopup::new(items);
    app.focused_window = FocusedWindow::TitleSelection;
    Ok(())
}

fn handle_forth_line<T: Service>(app: &mut App<T>, rt: &Runtime) -> Result<(), String> {
    if app.config.autofill_title {
        fill_title(app, rt)?;
    }
    // compare number of video files with the retrieved number of episodes
    let episode_count = rt
        .block_on(
            app.anime_list
                .service
                .get_episode_count(app.insert_popup.service_id),
        )?
        .unwrap_or_default();
    let video_files_count = app
        .anime_list
        .count_video_files(&app.insert_popup.path)
        .unwrap_or_default();

    app.insert_popup.episode_count = episode_count;
    if episode_count == video_files_count
        || app.anime_list.service.get_service_type() == ServiceType::Local
    {
        app.insert_popup.episodes = AnimeList::<T>::get_video_file_paths(&app.insert_popup.path)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(k, path)| Episode {
                number: k + 1,
                path: path.clone(),
                title: String::new(),
                file_deleted: !path.exists(),
                score: 0.0,
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
    Ok(())
}

fn fill_title<T: Service>(app: &mut App<T>, rt: &Runtime) -> Result<(), String> {
    let mut title = if app.config.english_show_titles {
        let titles = rt.block_on(
            app.anime_list
                .service
                .get_alternative_titles(app.insert_popup.service_id),
        )?;
        let title_languages = titles.map(|options| options.languages).unwrap_or_default();
        title_languages.get("en").cloned()
    } else {
        None
    };
    if title.is_none() {
        title = Some(
            rt.block_on(
                app.anime_list
                    .service
                    .get_title(app.insert_popup.service_id),
            )?,
        );
    }
    if app.anime_list.service.get_service_type() != ServiceType::Local {
        app.insert_popup.title = title.expect("Has to be set at this point");
    }
    Ok(())
}

fn handle_save_state<T: Service + Send>(app: &mut App<T>, rt: &Runtime) -> Result<(), String> {
    match app
        .anime_list
        .add_show(&app.insert_popup.title, app.insert_popup.service_id, 0)
    {
        Ok(local_id) => {
            insert_episodes(rt, app, local_id)?;
            rt.block_on(async {
                app.anime_list
                    .service
                    .init_show(app.insert_popup.service_id)
                    .await
            })
        }
        Err(why) => {
            if why.contains("constraint failed") {
                // show with this sync_service_id or title already exists
                // get local_id of the show with the same title
                if let Ok(local_id) = app.anime_list.get_local_show_id(&app.insert_popup.title) {
                    insert_episodes(rt, app, local_id)?;
                }
                // don't do anything more if can't get the id by title
            } else {
                eprintln!("{why}");
            }
            Ok(())
        }
    }?;
    app.insert_popup.state = InsertState::None;
    app.insert_popup = InsertPopup::default();
    app.list_state.update_cache(&app.anime_list)?;
    app.focused_window = FocusedWindow::MainMenu;
    // to update episodes list
    app.list_state
        .move_selection(&SelectionDirection::Next, &app.anime_list)?;
    Ok(())
}

fn insert_episodes<T: Service + Send>(
    rt: &Runtime,
    app: &mut App<T>,
    local_id: usize,
) -> Result<(), String> {
    // service_id is fine because hashmap can be empty here
    let episodes_details_hash = rt.block_on(get_episodes_info(
        &mut app.anime_list.service,
        app.insert_popup.service_id,
        app.config.precise_score,
    ))?;
    // surely I can be smarter about it
    let episode_offset = if app.anime_list.service.get_service_type() == ServiceType::Local {
        app.anime_list
            .get_list()
            .map(|shows| {
                shows
                    .iter()
                    .find(|show| show.local_id == local_id)
                    .map(|show| show.episodes.len())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    } else {
        0
    };
    app.insert_popup.episodes.iter().for_each(|episode| {
        let details = episodes_details_hash
            .get(&episode.number)
            .cloned()
            .unwrap_or_default();
        if let Err(why) = app.anime_list.add_episode(
            local_id,
            episode.number + episode_offset,
            &episode.path.to_string_lossy(),
            &details.title,
            generate_extra_info(details.recap, details.filler),
            details.score.unwrap_or_default(),
        ) {
            eprintln!("{why}");
        }
    });
    Ok(())
}

#[derive(Default, Clone)]
pub struct EpisodeDetails {
    pub title: String,
    pub recap: bool,
    pub filler: bool,
    pub score: Option<f32>,
}

pub async fn get_episodes_info<T: Service + Send>(
    service: &mut T,
    id: usize,
    precise_score: bool,
) -> Result<HashMap<usize, EpisodeDetails>, String> {
    let episodes_details = service.get_episodes(id, precise_score).await?;
    Ok(episodes_details
        .iter()
        .map(|episode| {
            (
                episode.number.unwrap_or_default(),
                (EpisodeDetails {
                    title: episode.title.clone().unwrap_or_default(),
                    recap: episode.recap.unwrap_or_default(),
                    filler: episode.filler.unwrap_or_default(),
                    score: episode.score,
                }),
            )
        })
        .collect())
}

pub const fn generate_extra_info(recap: bool, filler: bool) -> usize {
    let mut extra_info: usize = 0;
    if recap {
        extra_info |= 1 << 0;
    }
    if filler {
        extra_info |= 1 << 1;
    }
    extra_info
}

/// Clears the string on an invalid number
fn parse_number(str: &mut String) -> usize {
    str.trim().parse().map_or_else(
        |_| {
            *str = String::new();
            0
        },
        |number| number,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_line() {
        let insert_popup = InsertPopup::default();
        assert_eq!(insert_popup.current_line(), insert_popup.selected_line);
    }

    #[test]
    fn first_selection() {
        let insert_popup = InsertPopup::default();
        assert_eq!(insert_popup.current_line(), 0);
    }

    #[test]
    fn wrap_to_end() {
        let mut insert_popup = InsertPopup {
            state: InsertState::Inputting,
            ..Default::default()
        };
        insert_popup.move_line_selection(&SelectionDirection::Previous);
        assert_eq!(insert_popup.current_line(), 3);
    }

    #[test]
    fn wrap_to_start() {
        let mut insert_popup = InsertPopup {
            state: InsertState::Inputting,
            ..Default::default()
        };
        for _ in 1..=4 {
            insert_popup.move_line_selection(&SelectionDirection::Next);
        }
        assert_eq!(insert_popup.current_line(), 0);
    }

    #[test]
    fn test_generate_extra_info() {
        let result = generate_extra_info(false, false);
        assert_eq!(result, 0);

        let result = generate_extra_info(true, false);
        assert_eq!(result, 1);

        let result = generate_extra_info(false, true);
        assert_eq!(result, 2);

        let result = generate_extra_info(true, true);
        assert_eq!(result, 3);
    }

    #[test]
    fn valid_parse_number() {
        let mut input = "123".to_owned();
        let result = parse_number(&mut input);
        assert_eq!(result, 123);
        assert_eq!(input, "123");
    }

    #[test]
    fn invalid_parse_number() {
        let mut input = "abc".to_owned();
        let result = parse_number(&mut input);
        assert_eq!(result, 0);
        assert_eq!(input, "");
    }
}
