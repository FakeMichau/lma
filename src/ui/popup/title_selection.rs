use lma::ServiceTitle;
use ratatui::{
    backend::Backend,
    layout::Margin,
    widgets::{ListState, ListItem, List, Block, Borders, Clear},
    Frame, style::{Style, Modifier, Color},
};

use crate::{ui::SelectionDirection, app::App};

#[derive(Default)]
pub(crate) struct TitlesPopup {
    pub(crate) state: ListState,
    pub(crate) service_titles: Vec<ServiceTitle>,
}

impl TitlesPopup {
    pub(crate) fn with_items(titles: Vec<ServiceTitle>) -> TitlesPopup {
        TitlesPopup {
            state: ListState::default(),
            service_titles: titles,
        }
    }

    pub(crate) fn move_selection(&mut self, direction: SelectionDirection) {
        let i = self.select_element(
            self.service_titles.len(), 
            self.state.selected(), 
            direction
        );
        self.state.select(Some(i))
    }

    pub(crate) fn select(&self) {

    }

    fn select_element(
        &mut self,
        list_length: usize,
        selected_element: Option<usize>,
        direction: SelectionDirection,
    ) -> usize {
        match selected_element {
            Some(i) => match direction {
                SelectionDirection::Next => (i + 1) % list_length,
                SelectionDirection::Previous => (list_length + i - 1) % list_length,
            },
            None => 0,
        }
    }
}

use super::centered_rect;

pub(crate) fn build<B: Backend>(frame: &mut Frame<B>, app: &mut App) {
    let area = centered_rect(70, 70, frame.size());
    let list_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    let items: Vec<_> = app
        .titles_popup
        .service_titles
        .iter()
        .map(|service_title| ListItem::new(format!("{}", service_title.title)).style(Style::default()))
        .collect();

    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(items, list_area, &mut app.titles_popup.state);
}