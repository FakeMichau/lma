use super::{get_inner_layout, render_scrollbar, try_to_scroll_title, HeaderType, Table};
use crate::app::App;
use crate::config::TermColors;
use lma_lib::{Episode, Service, Show};
use ratatui::backend::Backend;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, TableState};
use ratatui::widgets::{Cell, Row};
use ratatui::Frame;

pub fn render<B: Backend, T: Service>(app: &mut App<T>, area: Rect, frame: &mut Frame<'_, B>) {
    let mut header = app.config.headers().episodes.clone();

    let (table_area, scrollbar_area) = get_inner_layout(area);

    let selected_show = app.list_state.selected_show().cloned();
    #[allow(clippy::cast_precision_loss)]
    let average_episode_score = if app.config.relative_episode_score() {
        // TODO: fix episodes without a score skewing the average
        selected_show
            .clone()
            .map(|show| {
                show.episodes.iter().map(|e| e.score).sum::<f32>() / show.episodes.len() as f32
            })
            .filter(|avg| avg > &0.0)
    } else {
        None
    };
    if average_episode_score.is_none() && app.config.relative_episode_score() {
        if let Some(pos) = header
            .iter()
            .position(|x| matches!(x, HeaderType::Score(_)))
        {
            header.remove(pos);
        }
    }
    let episodes: Vec<Row> = selected_show
        .into_iter()
        .flat_map(|show| {
            let mut temp: Vec<Row> = Vec::new();
            let selected_episode_number =
                get_selected_episode_number(&app.list_state.episodes_state, &show);
            for episode in show.episodes {
                let mut episode_display_name =
                    get_display_name(&episode, app.config.path_instead_of_title());

                if app.list_state.selecting_episode
                    && selected_episode_number == Some(episode.number)
                {
                    try_to_scroll_title(
                        table_area.width,
                        &header,
                        &mut app.list_state.scroll_progress,
                        &mut episode_display_name,
                    );
                }
                let style = get_episode_style(&episode, show.progress, app.config.colors());
                let cells = generate_episode_cells(
                    &episode,
                    &header,
                    style,
                    &episode_display_name,
                    average_episode_score,
                    app.config.colors(),
                );
                let new_episode = Row::new(cells);
                temp.push(new_episode);
            }
            temp
        })
        .collect();

    let border = generate_border(average_episode_score, app);
    frame.render_widget(border, area);

    render_scrollbar(
        scrollbar_area,
        frame,
        episodes.len(),
        app.config.colors(),
        app.list_state.episodes_state.offset(),
    );

    Table::new(
        &mut app.list_state.episodes_state,
        episodes,
        &header,
        table_area,
    )
    .render(frame, app.config.colors());
}

fn generate_border<T: Service>(average_episode_score: Option<f32>, app: &App<T>) -> Block<'_> {
    let extra_title =
        average_episode_score.map_or_else(String::new, |avg| format!(" - Average score: {avg:.2}"));
    Block::default()
        .borders(Borders::ALL)
        .title(format!("Episodes{extra_title}"))
        .border_style(if app.list_state.selecting_episode {
            Style::default().fg(app.config.colors().highlight)
        } else {
            Style::default()
        })
}

fn generate_episode_cells<'a>(
    episode: &Episode,
    header: &[HeaderType],
    style: Style,
    episode_display_name: &str,
    average_episode_score: Option<f32>,
    colors: &TermColors,
) -> Vec<Cell<'a>> {
    header
        .iter()
        .map(|column| match column {
            HeaderType::Number(width) => Cell::from(format!(
                "{:>1$}",
                episode.number.to_string(),
                usize::from(*width)
            ))
            .style(style),
            HeaderType::Title => Cell::from(episode_display_name.to_string()).style(style),
            HeaderType::Extra(_) => Cell::from(format!(
                " {} {} ",
                if episode.filler { "F" } else { "" },
                if episode.recap { "R" } else { "" }
            ))
            .style(style),
            HeaderType::Score(width) => average_episode_score.map_or_else(
                || Cell::from(format!("{:>1$.2}", episode.score, usize::from(*width))).style(style),
                |avg| {
                    let diff = episode.score - avg;
                    let style = if diff.is_sign_negative() {
                        style.fg(colors.text_deleted)
                    } else {
                        style
                    };
                    Cell::from(format!("{:>1$.2}", diff, usize::from(*width))).style(style)
                },
            ),
        })
        .collect::<Vec<_>>()
}

fn get_episode_style(episode: &Episode, progress: i64, colors: &TermColors) -> Style {
    let mut style = Style::default();
    if episode.number <= progress {
        style = style.fg(colors.text).add_modifier(Modifier::DIM);
    }
    if episode.file_deleted {
        style = style
            .fg(colors.text_deleted)
            .add_modifier(Modifier::CROSSED_OUT | Modifier::DIM);
    }
    style
}

fn get_selected_episode_number(episode_state: &TableState, show: &Show) -> Option<i64> {
    episode_state
        .selected()
        .and_then(|index| show.episodes.get(index))
        .map(|e| e.number)
}

fn get_display_name(episode: &Episode, use_path: bool) -> String {
    if episode.title.is_empty() || use_path {
        episode
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into()
    } else {
        episode.title.clone()
    }
}
