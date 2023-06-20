use std::collections::BTreeSet;
use std::path::PathBuf;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Margin, Layout, Direction, Constraint};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use lma::{AnimeList, Episode, Service};
use crate::app::App;

#[derive(Default)]
pub struct MismatchPopup {
    episodes_count: u32,
    video_files_count: u32,
    pub owned_episodes: String,
}

impl MismatchPopup {
    pub const fn new(episodes_count: u32, video_files_count: u32) -> Self {
        Self {
            episodes_count,
            video_files_count,
            owned_episodes: String::new(),
        }
    }
    pub fn save<T: Service>(&self, path: &PathBuf) -> Vec<Episode> {
        let episodes = self.parse_owned();
        let mut episodes_iter = episodes.into_iter();
        AnimeList::<T>::get_video_file_paths(path)
            .unwrap_or_default()
            .into_iter()
            .map(|path| Episode {
                number: episodes_iter
                    .next()
                    .expect("Number of episodes doesn't line up")
                    .into(),
                path: path.clone(),
                title: String::new(),
                file_deleted: !path.exists(),
                recap: false,
                filler: false,
            })
            .collect()
    }


    fn parse_owned(&self) -> BTreeSet<u32> {
        self.owned_episodes
            .split(',')
            .flat_map(|slice| {
                let mut episodes_from_slice = Vec::new();
                if slice.contains('-') {
                    let mut range = slice.split('-');
                    let min = range
                        .next()
                        .expect("Minimum value from the range")
                        .trim()
                        .parse::<u32>()
                        .expect("Value from range must be a number");
                    let max = range
                        .next()
                        .expect("Maximum value from the range")
                        .trim()
                        .parse::<u32>()
                        .expect("Value from range must be a number");
                    (min..=max).for_each(|value| episodes_from_slice.push(value));
                } else {
                    let episode = slice.trim();
                    if !episode.is_empty() {
                        episodes_from_slice
                            .push(episode.parse::<u32>().expect("Episode must be a number"));
                    }
                }
                episodes_from_slice
            })
            .collect()
    }
}

use super::centered_rect;

pub fn build<B: Backend, T: Service>(frame: &mut Frame<B>, app: &mut App<T>) {
    let area = centered_rect(70, 70, frame.size());
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(inner_area);

    let mismatch_info = vec![
        Line::from(vec![
            Span::raw("Expected number of episodes: "),
            Span::styled(
                app.mismatch_popup.episodes_count.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Number of found video files: "),
            Span::styled(
                app.mismatch_popup.video_files_count.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Type out exact episodes you have, e.g. 8,10-12"),
        ]),
    ];

    let user_input = Line::from(
        Span::raw(app.mismatch_popup.owned_episodes.clone())
    );

    frame.set_cursor(
        main_chunks[1].x + u16::try_from(user_input.width()).unwrap(),
        main_chunks[1].y,
    );

    let block = Block::default()
        .title("Episodes mismatch")
        .borders(Borders::ALL);
    let mismatch_info = Paragraph::new(mismatch_info)
        .alignment(Alignment::Center);
    let user_input = Paragraph::new(user_input);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(mismatch_info, main_chunks[0]);
    frame.render_widget(user_input, main_chunks[1]);
}
