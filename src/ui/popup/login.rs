use crate::app::App;
use ratatui::{
    backend::Backend,
    layout::{Margin, Alignment},
    style::{Modifier, Style, Color},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::centered_rect;

pub(crate) fn build<B: Backend>(frame: &mut Frame<'_, B>, app: &mut App) {
    let area = centered_rect(70, 70, frame.size());
    let text_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    let login_info = if let Some(url) = app.anime_list.service.get_url() {
        vec![
            Line::from(Span::raw("Login using the link below")),
            Line::from(Span::styled(
                url,
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "You are already logged in",
            Style::default().fg(Color::Green)
        ))]
    };
    let block = Block::default().title("Login").borders(Borders::ALL);
    let form = Paragraph::new(login_info)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area)
}
