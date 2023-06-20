use std::collections::HashMap;
use ratatui::backend::Backend;
use ratatui::layout::Margin;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tokio::runtime::Runtime;
use lma::{Episode, Service, is_video_file};
use super::centered_rect;
use crate::app::App;
use crate::ui::{FocusedWindow, SelectionDirection};

#[derive(Default)]
pub struct InsertEpisodePopup {
    pub state: InsertEpisodeState,
    pub data: String,
    pub episode: Episode,
}

#[derive(Default, PartialEq, Eq)]
pub enum InsertEpisodeState {
    #[default] None,
    Inputting,
    Save,
}

pub fn build<B: Backend, T: Service + Send>(frame: &mut Frame<B>, app: &mut App<T>, rt: &Runtime) {
    let area = centered_rect(70, 70, frame.size());
    let text_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    match app.insert_episode_popup.state {
        InsertEpisodeState::Inputting => handle_inputting_state(app),
        InsertEpisodeState::Save => handle_save_state(app, rt),
        InsertEpisodeState::None => {},
    }

    let input_form = Line::from(vec![
        Span::raw("Path to the episode: "),
        Span::raw(app.insert_episode_popup.episode.path.to_string_lossy()),
    ]);
    if app.insert_episode_popup.state == InsertEpisodeState::Inputting {
        frame.set_cursor(
            text_area.x
                + u16::try_from(input_form
                    .width())
                    .unwrap(),
            text_area.y + u16::try_from(0).unwrap(),
        );
    }

    let block = Block::default().title("Insert episode").borders(Borders::ALL);
    // .wrap(Wrap { trim: true }); messes up the cursor position
    let form = Paragraph::new(input_form);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area);
}

fn handle_inputting_state<T: Service>(app: &mut App<T>) {
    app.insert_episode_popup.episode.path = app.insert_episode_popup.data.clone().into();
}

fn handle_save_state<T: Service + Send>(app: &mut App<T>, rt: &Runtime) {
    if is_video_file(&app.insert_episode_popup.episode.path) {
        if let Some(show) = app.list_state.selected_show() {
            // always append episodes
            let last_episode_number = show.episodes.iter().last().map(|last_show| last_show.number).unwrap_or_default();
            app.insert_episode_popup.episode.number = last_episode_number + 1;
            insert_episode(rt, app, show.local_id, show.service_id);
            app.insert_episode_popup.state = InsertEpisodeState::None;
            app.insert_episode_popup = InsertEpisodePopup::default();
            app.list_state.update_cache(&app.anime_list);
            app.focused_window = FocusedWindow::MainMenu;
            // to update episodes list
            app.list_state.move_selection(&SelectionDirection::Next, &app.anime_list);
        } else {
            app.insert_episode_popup.state = InsertEpisodeState::None;
        }
    } else {
        app.insert_episode_popup.state = InsertEpisodeState::None;
    }
}

fn insert_episode<T: Service + Send>(rt: &Runtime, app: &mut App<T>, local_id: i64, service_id: i64) {
    // service_id is fine because hashmap can be empty here
    let episodes_details_hash = rt.block_on(
        get_episodes_info(&mut app.anime_list.service, u32::try_from(service_id).unwrap())
    );
    let episode = &app.insert_episode_popup.episode;
    let potential_title = episodes_details_hash.get(&u32::try_from(episode.number).unwrap());
    let (title, recap, filler) = potential_title.unwrap_or(&(String::new(), false, false)).clone();

    if let Err(why) = app.anime_list.add_episode(
        local_id,
        episode.number,
        &episode.path.to_string_lossy(),
        &title,
        generate_extra_info(recap, filler)
    ) {
        eprintln!("{why}");
    }
}

async fn get_episodes_info<T: Service + Send>(service: &mut T, id: u32) -> HashMap<u32, (String, bool, bool)> {
    let episodes_details = service.get_episodes(id).await;
    episodes_details
        .iter()
        .map(|episode| {
            (
                episode.number.unwrap_or_default(),
                (
                    episode.title.clone().unwrap_or_default(),
                    episode.recap.unwrap_or_default(),
                    episode.filler.unwrap_or_default(),
                ),
            )
        })
        .collect()
}

const fn generate_extra_info(recap: bool, filler: bool) -> i64 {
    let mut extra_info: i64 = 0;
    if recap {
        extra_info |= 1 << 0;
    }
    if filler {
        extra_info |= 1 << 1;
    }
    extra_info
}
