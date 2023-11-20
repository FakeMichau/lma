use super::centered_rect;
use crate::app::App;
use crate::ui::{self, SelectionDirection};
use lma_lib::{Service, ServiceType};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::env;
use std::path::Path;

pub struct SetupPopup {
    page: usize,
    service_list: ServiceList,
}

struct ServiceList {
    selected_service: ServiceType,
    state: ListState,
    services: Vec<String>,
}

impl SetupPopup {
    pub fn new() -> Self {
        Self {
            page: 0,
            service_list: ServiceList {
                selected_service: ServiceType::Local,
                state: ListState::default(),
                services: vec![String::from("Local"), String::from("MAL")],
            },
        }
    }

    /// Return true if setup is finised
    pub fn next_page(&mut self) -> bool {
        // limit page, move to enum?
        self.page += 1;
        let finised = self.page >= 3;
        if finised {
            self.update_selected_service();
        }
        finised
    }

    pub fn previous_page(&mut self) {
        self.page = self.page.checked_sub(1).unwrap_or_default();
    }

    pub fn move_selection(&mut self, direction: &SelectionDirection) {
        let i = ui::select_element(
            2, // hardcoded length
            self.service_list.state.selected(),
            direction,
        );
        self.service_list.state.select(Some(i));
    }

    fn update_selected_service(&mut self) {
        let index = self.service_list.state.selected().unwrap_or_default(); // defaults to first
        self.service_list.selected_service =
            match self.service_list.services.get(index).map(String::as_str) {
                Some("MAL") => ServiceType::MAL,
                Some("Local" | &_) | None => ServiceType::Local,
            }
    }

    pub fn reset(&mut self) {
        self.page = 0;
    }

    pub const fn page(&self) -> usize {
        self.page
    }

    pub const fn selected_service(&self) -> &ServiceType {
        &self.service_list.selected_service
    }
}

pub fn build<T: Service>(frame: &mut Frame, app: &mut App<T>) {
    let area = centered_rect(70, 70, frame.size());
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    let block = Block::default().title("First Setup").borders(Borders::ALL);

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    match app.first_setup_popup.page() {
        0 => render_first_page(frame, inner_area),
        1 => render_second_page(frame, inner_area, app),
        2 => render_third_page(frame, inner_area, &app.config.config_file_path),
        _ => {}
    };
}

fn get_vertical_middle(needed_space: u16, area: Rect) -> Rect {
    area.height
        .checked_sub(needed_space)
        .map_or_else(Rect::default, |space| {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(space / 2),
                        Constraint::Min(3),
                        Constraint::Percentage(space / 2),
                    ]
                    .as_ref(),
                )
                .split(area)[1]
        })
}

fn render_first_page(frame: &mut Frame, area: Rect) {
    const LINES_TO_FIT: u16 = 3;
    let middle = get_vertical_middle(LINES_TO_FIT, area);

    let content = vec![
        Line::from(Span::raw("Thanks for checking out the project")),
        Line::from(Span::raw("Before starting you need to set up few things")),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to continue"),
        ]),
    ];
    let form = Paragraph::new(content)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    frame.render_widget(form, middle);
}

fn render_second_page<T: Service>(frame: &mut Frame, area: Rect, app: &mut App<T>) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);
    let content = vec![
        Line::from(Span::raw("Select a service you want to be using")),
        Line::from(Span::raw(
            "Choose \"Local\" if you don't want to use any external services",
        )),
    ];
    let form = Paragraph::new(content)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    frame.render_widget(form, main_chunks[0]);

    let services: Vec<_> = app
        .first_setup_popup
        .service_list
        .services
        .clone()
        .into_iter()
        .map(|service| ListItem::new(service).style(Style::default().fg(app.config.colors.text)))
        .collect();

    let shows = List::new(services)
        .block(Block::default().borders(Borders::ALL).title("Services"))
        .highlight_style(
            Style::default()
                .fg(app.config.colors.highlight)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(
        shows,
        main_chunks[1],
        &mut app.first_setup_popup.service_list.state,
    );
}

fn render_third_page(frame: &mut Frame, area: Rect, config_path: &Path) {
    const LINES_TO_FIT: u16 = 6;
    let middle = get_vertical_middle(LINES_TO_FIT, area);
    let config_path = if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        env::current_dir().unwrap_or_default().join(config_path)
    };
    let content = vec![
        Line::from(Span::raw("That's everything")),
        Line::from(Span::raw("Relaunch the application for configs to apply")),
        Line::from(Span::raw("")),
        Line::from(Span::raw(
            "You can also edit your config manually after closing the app:",
        )),
        Line::from(Span::raw(config_path.to_string_lossy())),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to continue"),
        ]),
    ];
    let form = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(form, middle);
}
