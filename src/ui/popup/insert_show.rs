use tui::{
    backend::Backend,
    widgets::{Block, Borders, Clear},
    Frame,
};
use super::centered_rect;

pub(crate) fn build_creation_popup<B: Backend>(frame: &mut Frame<B>, title: &str) {
    let block = Block::default().title(title).borders(Borders::ALL);
    let area = centered_rect(70, 70, frame.size());
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
}
