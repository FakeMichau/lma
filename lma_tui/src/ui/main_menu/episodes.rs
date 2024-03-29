use super::{try_to_scroll_title, HeaderType, Selection, Table};
use crate::app::App;
use crate::config::TermColors;
use lma_lib::{Episode, Service, Show};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, TableState};
use ratatui::widgets::{Cell, Row};
use ratatui::Frame;

pub fn render<T: Service>(app: &mut App<T>, area: Rect, frame: &mut Frame) {
    let mut header = app.config.headers.episodes.clone();

    let table_area = area.inner(&Margin::new(1, 1));
    app.list_state.last_height = table_area.height;

    let selected_show = app.list_state.selected_show().cloned();
    #[allow(clippy::cast_precision_loss)]
    let average_episode_score = if app.config.relative_episode_score {
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
    if average_episode_score.is_none() && app.config.relative_episode_score {
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
            let selected_episode_number =
                get_selected_episode_number(&app.list_state.episodes_state, &show);

            show.episodes
                .into_iter()
                .map(|episode| {
                    let mut episode_display_name =
                        get_display_name(&episode, app.config.path_instead_of_title);

                    if app.list_state.selection == Selection::Episode
                        && selected_episode_number == Some(episode.number)
                    {
                        try_to_scroll_title(
                            table_area.width,
                            &header,
                            &mut app.list_state.scroll_progress,
                            &mut episode_display_name,
                        );
                    }
                    let cells = generate_cells(
                        &episode,
                        &header,
                        get_style(&episode, show.progress, &app.config.colors),
                        &episode_display_name,
                        average_episode_score,
                        &app.config.colors,
                    );
                    Row::new(cells)
                })
                .collect::<Vec<Row>>()
        })
        .collect();

    let border = generate_border(average_episode_score, app);
    frame.render_widget(border, area);

    Table::new(
        &mut app.list_state.episodes_state,
        episodes,
        &header,
        table_area,
    )
    .render(frame, &app.config.colors);
}

fn generate_border<T: Service>(average_episode_score: Option<f32>, app: &App<T>) -> Block<'_> {
    let extra_title =
        average_episode_score.map_or_else(String::new, |avg| format!(" - Average score: {avg:.2}"));
    Block::default()
        .borders(Borders::ALL)
        .title(format!("Episodes{extra_title}"))
        .border_style(if app.list_state.selection == Selection::Episode {
            Style::default().fg(app.config.colors.highlight)
        } else {
            Style::default()
        })
}

fn generate_cells<'a>(
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

fn get_style(episode: &Episode, progress: usize, colors: &TermColors) -> Style {
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

fn get_selected_episode_number(episode_state: &TableState, show: &Show) -> Option<usize> {
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
