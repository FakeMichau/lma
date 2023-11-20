use super::centered_rect;
use crate::app::App;
use lma_lib::Service;
use ratatui::layout::{Alignment, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub fn build<T: Service>(frame: &mut Frame, app: &mut App<T>) {
    let area = centered_rect(70, 70, frame.size());
    let text_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    let login_info = app.anime_list.service.get_url().map_or_else(
        || {
            vec![Line::from(Span::styled(
                "You are already logged in",
                Style::default().fg(Color::Green),
            ))]
        },
        |url| {
            _ = open::that(url.clone());
            vec![
                Line::from(Span::raw("Login using the link below")),
                Line::from(Span::styled(
                    url,
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ]
        },
    );
    let block = Block::default().title("Login").borders(Borders::ALL);
    let form = Paragraph::new(login_info)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area);
}
