use ratatui::backend::Backend;
use ratatui::layout::{Margin, Alignment};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use lma::Service;
use crate::app::App;
use super::centered_rect;

pub fn build<B: Backend, T: Service + Send>(frame: &mut Frame<B>, app: &mut App<T>) {
    let area = centered_rect(70, 70, frame.size());
    let text_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    let error = vec![
        Line::from(Span::styled(
            "Encountered an error",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(app.error())),
    ];
    let block = Block::default().title("Error").borders(Borders::ALL);
    let form = Paragraph::new(error)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(form, text_area);
}
