use super::{get_inner_layout, render_scrollbar, try_to_scroll_title, HeaderType, Table};
use crate::app::App;
use crate::config::TermColors;
use lma_lib::{Service, Show};
use ratatui::backend::Backend;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::widgets::{Cell, Row};
use ratatui::Frame;

pub fn render<B: Backend, T: Service>(app: &mut App<T>, area: Rect, frame: &mut Frame<B>) {
    let header = &app.config.headers.shows;

    let (table_area, scrollbar_area) = get_inner_layout(area);

    let selected_show_id = get_selected_show_id(app);
    let shows: Vec<Row> = app
        .list_state
        .list_cache
        .iter()
        .map(|show| {
            let mut title = show.title.clone();
            if selected_show_id == Some(show.local_id) && !app.list_state.selecting_episode {
                try_to_scroll_title(
                    table_area.width,
                    header,
                    &mut app.list_state.scroll_progress,
                    &mut title,
                );
            }
            let style = get_style(show, &app.config.colors);
            let cells = generate_cells(show, header, style, &title);
            Row::new(cells)
        })
        .collect();

    let border = generate_border(app);

    frame.render_widget(border, area);

    let shows_number = shows.len();
    Table::new(&mut app.list_state.shows_state, shows, header, table_area)
        .render(frame, &app.config.colors);

    render_scrollbar(
        scrollbar_area,
        frame,
        shows_number,
        &app.config.colors,
        app.list_state.shows_state.offset(),
    );
}

fn get_style(show: &Show, colors: &TermColors) -> Style {
    let mut style = Style::default().fg(colors.text);
    if show.progress >= show.episodes.len() {
        style = style.add_modifier(Modifier::DIM);
    }
    style
}

fn get_selected_show_id<T: Service>(app: &App<T>) -> Option<usize> {
    app.list_state
        .selected_show()
        .map(|selected_show| selected_show.local_id)
}

fn generate_border<T: Service>(app: &App<T>) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title("Shows")
        .border_style(
            if app.list_state.selecting_episode || app.list_state.selected_show().is_none() {
                Style::default()
            } else {
                Style::default().fg(app.config.colors.highlight)
            },
        )
}

fn generate_cells<'a>(
    show: &'a Show,
    header: &[HeaderType],
    style: Style,
    title: &str,
) -> Vec<Cell<'a>> {
    header
        .iter()
        .map(|column| match column {
            HeaderType::Number(width) => Cell::from(format!(
                "{:>1$}",
                show.local_id.to_string(),
                usize::from(*width)
            ))
            .style(style),
            HeaderType::Title => Cell::from(String::from(title)).style(style),
            _ => Cell::from(""),
        })
        .collect::<Vec<_>>()
}
