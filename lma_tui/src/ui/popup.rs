#[allow(clippy::module_name_repetitions)]
pub mod insert_episode;
pub mod insert_show;
pub mod login;
pub mod title_selection;
pub mod episode_mismatch;
pub mod error;
pub mod first_setup;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_rect_calculation() {
        let main_rect = Rect::new(0, 0, 128, 128);
        let popup = centered_rect(50, 50, main_rect);
        assert_eq!(popup.width, 64, "Tests width of the popup");
        assert_eq!(popup.height, 64, "Tests height of the popup");
        assert_eq!(popup.x, 32, "Tests x position of the popup");
        assert_eq!(popup.y, 32, "Tests y position of the popup");
    }
}