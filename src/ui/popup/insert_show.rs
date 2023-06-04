use crate::{app::{App, InsertState}, ui::FocusedWindow};
use ratatui::{
    backend::Backend,
    layout::Margin,
    text::{Span, Line},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;

pub(crate) fn build_creation_popup<B: Backend>(frame: &mut Frame<B>, app: &mut App) {
    let area = centered_rect(70, 70, frame.size());
    let text_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    fn parse_number(str: &mut String) -> i64 {
        if let Ok(number) = str.trim().parse() {
            number
        } else {
            *str = String::new();
            0
        }
    }

    match app.insert_popup.state {
        InsertState::Inputting => {
            match app.insert_popup.current_line() {
                0 => app.insert_popup.path = app.insert_popup.data.clone(),
                1 => app.insert_popup.title = app.insert_popup.data.clone(),
                2 => app.insert_popup.sync_service_id = parse_number(&mut app.insert_popup.data),
                3 => app.insert_popup.episode_count = parse_number(&mut app.insert_popup.data),
                _ => {}
            }
        },
        InsertState::Confirmation => {
            match app.insert_popup.current_line() {
                // after going to the next line, when data in the previous one is present
                1 if !app.insert_popup.path.is_empty() => {
                    // sanitize user input
                    app.insert_popup.title = app.shows.items.guess_shows_title(&app.insert_popup.path).unwrap_or_default();
                },
                2 if !app.insert_popup.title.is_empty() => {
                    // create a popup to select the exact show from a sync service
                },
                3 if !app.insert_popup.sync_service_id != 0 && !app.insert_popup.path.is_empty() => {
                    let video_files_count = app.shows.items.count_video_files(&app.insert_popup.path).unwrap_or_default();
                    app.insert_popup.episode_count = video_files_count.try_into().unwrap_or_default() // temporarily 
                    // compare number of video files with the retrieved number of episodes
                },
                _ => {}
            };
            app.insert_popup.data = match app.insert_popup.current_line() {
                0 if !app.insert_popup.path.is_empty() => app.insert_popup.path.clone(),
                1 if !app.insert_popup.title.is_empty() => app.insert_popup.title.clone(),
                2 if !app.insert_popup.sync_service_id != 0 => app.insert_popup.sync_service_id.to_string(),
                3 if !app.insert_popup.episode_count != 0=> app.insert_popup.episode_count.to_string(),
                _ => String::new(),
            };
            app.insert_popup.state = InsertState::Inputting;
        },
        InsertState::Save => {
            // temporarily as data retrieval from MAL isn't yet implemented
            // TODO: Don't allow for empty titles etc.
            if let Err(why) = app.shows.items.add_show(
                &app.insert_popup.title,
                app.insert_popup.sync_service_id,
                app.insert_popup.episode_count,
                0
            ) {
                eprintln!("{}", why);
            }
            app.focused_window = FocusedWindow::MainMenu; // close the popup
        },
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
            Span::raw(app.insert_popup.sync_service_id.to_string()),
        ]),
        Line::from(vec![
            Span::raw("Number of episodes: "),
            Span::raw(app.insert_popup.episode_count.to_string()),
        ]),
    ];
    if app.insert_popup.state == InsertState::Inputting {
        frame.set_cursor(
            text_area.x + input_form.get(app.insert_popup.current_line()).unwrap().width() as u16,
            text_area.y + app.insert_popup.current_line() as u16,
        );
    }

    let block = Block::default().title("Insert show").borders(Borders::ALL);
    let form = Paragraph::new(input_form);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area)
}
