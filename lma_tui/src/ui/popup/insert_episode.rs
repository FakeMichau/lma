use ratatui::backend::Backend;
use ratatui::layout::{Margin, Layout, Direction, Constraint, Alignment};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use tokio::runtime::Runtime;
use lma_lib::{Episode, Service, is_video_file};
use super::insert_show::InsertState;
use super::{centered_rect, insert_show};
use crate::app::App;
use crate::ui::{FocusedWindow, SelectionDirection};

#[derive(Default)]
pub struct InsertEpisodePopup {
    pub state: InsertState,
    pub data: String,
    pub episode: Episode,
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

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Percentage(85)].as_ref())
        .split(text_area);
    let title_area = main_chunks[0];
    let text_area = main_chunks[1];

    match app.insert_episode_popup.state {
        InsertState::Inputting => handle_inputting_state(app),
        InsertState::Save => handle_save_state(app, rt)?,
        InsertState::None => {}
        InsertState::Next => todo!(),
    }
    let title = app
        .list_state
        .selected_show()
        .map(|show| show.title.clone())
        .unwrap_or_default();
    let title_line = vec![
        Line::from(vec![
            Span::raw("Adding episodes to:"),
        ]),
        Line::from(vec![
            Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        ])
    ];

    let input_form = Line::from(vec![
        Span::raw("Path to the episode: "),
        Span::raw(app.insert_episode_popup.episode.path.to_string_lossy()),
    ]);
    if app.insert_episode_popup.state == InsertState::Inputting {
        frame.set_cursor(
            text_area.x + u16::try_from(input_form.width()).unwrap_or_default(),
            text_area.y + u16::try_from(0).unwrap_or_default(),
        );
    }

    let block = Block::default()
        .title("Insert episode")
        .borders(Borders::ALL);
    // .wrap(Wrap { trim: true }); messes up the cursor position
    let title = Paragraph::new(title_line)
        .alignment(Alignment::Center)
        .wrap( Wrap { trim: true } );
    let form = Paragraph::new(input_form);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(title, title_area);
    frame.render_widget(form, text_area);
    Ok(())
}

fn handle_inputting_state<T: Service>(app: &mut App<T>) {
    app.insert_episode_popup.episode.path = app.insert_episode_popup.data.clone().into();
}

fn handle_save_state<T: Service + Send>(app: &mut App<T>, rt: &Runtime) -> Result<(), String> {
    if is_video_file(&app.insert_episode_popup.episode.path) {
        if let Some(show) = app.list_state.selected_show() {
            // always append episodes
            let last_episode_number = show
                .episodes
                .iter()
                .last()
                .map(|last_show| last_show.number)
                .unwrap_or_default();
            app.insert_episode_popup.episode.number = last_episode_number + 1;
            insert_episode(rt, app, show.local_id, show.service_id)?;
            app.insert_episode_popup.state = InsertState::None;
            app.insert_episode_popup = InsertEpisodePopup::default();
            app.list_state.update_cache(&app.anime_list)?;
            app.focused_window = FocusedWindow::MainMenu;
            // to update episodes list
            app.list_state
                .move_selection(&SelectionDirection::Next, &app.anime_list)?;
        } else {
            // show not selected, shouldn't be in this state anyway
            app.insert_episode_popup.state = InsertState::None;
        }
    } else {
        // not a video file, give use a message?
        app.insert_episode_popup.state = InsertState::None;
    }
    Ok(())
}

fn insert_episode<T: Service + Send>(
    rt: &Runtime,
    app: &mut App<T>,
    local_id: i64,
    service_id: i64,
) -> Result<(), String> {
    // service_id is fine because hashmap can be empty here
    let episodes_details_hash = rt.block_on(insert_show::get_episodes_info(
        &mut app.anime_list.service,
        u32::try_from(service_id).map_err(|e| e.to_string())?,
    ))?;
    let episode = &app.insert_episode_popup.episode;
    let potential_title =
        episodes_details_hash.get(&u32::try_from(episode.number).map_err(|e| e.to_string())?);
    let (title, recap, filler) = potential_title
        .unwrap_or(&(String::new(), false, false))
        .clone();

    if let Err(why) = app.anime_list.add_episode(
        local_id,
        episode.number,
        &episode.path.to_string_lossy(),
        &title,
        insert_show::generate_extra_info(recap, filler),
    ) {
        eprintln!("{why}");
    }
    Ok(())
}