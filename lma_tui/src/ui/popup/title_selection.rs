use crate::{
    app::App,
    ui::{self, SelectionDirection},
};
use lma_lib::{Service, ServiceTitle};
use ratatui::backend::Backend;
use ratatui::layout::Margin;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;

#[derive(Default)]
pub struct TitlesPopup {
    pub state: ListState,
    service_titles: Vec<ServiceTitle>,
}

impl TitlesPopup {
    pub fn new(titles: Vec<ServiceTitle>) -> Self {
        let mut default_state = ListState::default();
        // first item always selected when possible
        if !titles.is_empty() {
            default_state.select(Some(0));
        }
        Self {
            state: default_state,
            service_titles: titles,
        }
    }

    pub fn move_selection(&mut self, direction: &SelectionDirection) {
        let i = ui::select_element(self.service_titles.len(), self.state.selected(), direction);
        self.state.select(Some(i));
    }

    pub fn selected_show(&mut self) -> Option<&ServiceTitle> {
        if self.service_titles.is_empty() {
            None
        } else {
            let index = self.state.selected().unwrap_or_default();
            self.service_titles.get(index)
        }
    }
}

use super::centered_rect;

pub fn build<B: Backend, T: Service>(frame: &mut Frame<B>, app: &mut App<T>) {
    let area = centered_rect(70, 70, frame.size());
    let list_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    let items: Vec<_> = app
        .titles_popup
        .service_titles
        .iter()
        .map(|service_title| {
            ListItem::new(service_title.title.clone())
                .style(Style::default().fg(app.config.colors.text))
        })
        .collect();

    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .fg(app.config.colors.highlight)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(Clear, area);
    frame.render_stateful_widget(items, list_area, &mut app.titles_popup.state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_selection() {
        let mut list = generate_title_popup(6);
        assert!(list.selected_show().is_some());
    }

    #[test]
    fn empty_first_selection() {
        let mut list = TitlesPopup::new(Vec::new());
        assert!(list.selected_show().is_none());
    }

    #[test]
    fn empty_move_selection() {
        let mut list = TitlesPopup::new(Vec::new());
        list.move_selection(&SelectionDirection::Previous);
        list.move_selection(&SelectionDirection::Previous);
        list.move_selection(&SelectionDirection::Next);
        list.move_selection(&SelectionDirection::Next);
        assert!(list.selected_show().is_none());
    }

    #[test]
    fn wrap_to_end() {
        let mut list = generate_title_popup(6);
        list.move_selection(&SelectionDirection::Previous);
        let selected_show = list.selected_show().expect("6th show");
        assert_eq!(selected_show.service_id, 6);
    }

    #[test]
    fn wrap_to_start() {
        let mut list = generate_title_popup(6);
        for _ in 1..=6 {
            list.move_selection(&SelectionDirection::Next);
        }
        let selected_show = list.selected_show().expect("First show");
        assert_eq!(selected_show.service_id, 1);
    }

    fn generate_title_popup(count: usize) -> TitlesPopup {
        TitlesPopup::new(generate_title_services(count))
    }

    fn generate_title_services(count: usize) -> Vec<ServiceTitle> {
        let mut service_titles = Vec::new();
        for i in 1..=count {
            service_titles.push(ServiceTitle {
                service_id: i,
                title: format!("Test title {i}"),
            });
        }
        service_titles
    }
}
