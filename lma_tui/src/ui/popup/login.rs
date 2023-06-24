use ratatui::backend::Backend;
use ratatui::layout::{Margin, Alignment};
use ratatui::style::{Modifier, Style, Color};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use lma_lib::Service;
use crate::app::App;
use super::centered_rect;

pub fn build<B: Backend, T: Service>(frame: &mut Frame<B>, app: &mut App<T>) {
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
