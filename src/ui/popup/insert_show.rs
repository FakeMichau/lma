use crate::app::App;
use tui::{
    backend::Backend,
    layout::Margin,
    text::{Span, Spans},
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

    if app.input.inputting {
        if app.input.confirmation {
            app.input.data = if app.input.current_line() == 0 && !app.insert_popup.path.is_empty() {
                app.insert_popup.path.clone()
            } else if app.input.current_line() == 1 && !app.insert_popup.title.is_empty() {
                app.insert_popup.title.clone()
            } else if app.input.current_line() == 2 && !app.insert_popup.sync_service_id != 0 {
                app.insert_popup.sync_service_id.to_string()
            } else if app.input.current_line() == 3 && !app.insert_popup.episode_count != 0 {
                app.insert_popup.episode_count.to_string()
            } else {
                String::new()
            };
            app.input.confirmation = false;
        }
        match app.input.current_line() {
            0 => app.insert_popup.path = app.input.data.clone(),
            1 => app.insert_popup.title = app.input.data.clone(),
            2 => app.insert_popup.sync_service_id = parse_number(&mut app.input.data),
            3 => app.insert_popup.episode_count = parse_number(&mut app.input.data),
            _ => {}
        }
    }
    let input_form = vec![
        Spans::from(vec![
            Span::raw("Path to the folder: "),
            Span::raw(app.insert_popup.path.clone()),
        ]),
        Spans::from(vec![
            Span::raw("Show's title: "),
            Span::raw(app.insert_popup.title.clone()),
        ]),
        Spans::from(vec![
            Span::raw("Sync service ID: "),
            Span::raw(app.insert_popup.sync_service_id.to_string()),
        ]),
        Spans::from(vec![
            Span::raw("Number of episodes: "),
            Span::raw(app.insert_popup.episode_count.to_string()),
        ]),
    ];
    if app.input.inputting {
        frame.set_cursor(
            text_area.x + input_form.get(app.input.current_line()).unwrap().width() as u16,
            text_area.y + app.input.current_line() as u16,
        );
    }

    let block = Block::default().title("Insert show").borders(Borders::ALL);
    let form = Paragraph::new(input_form);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area)
}
