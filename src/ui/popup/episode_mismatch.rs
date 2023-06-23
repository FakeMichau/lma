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
    pub fn save<T: Service>(&self, path: &PathBuf) -> Result<Vec<Episode>, String> {
        let episodes = self.parse_owned()?;
        let mut episodes_iter = episodes.into_iter();
        Ok(AnimeList::<T>::get_video_file_paths(path)
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
            .collect())
    }

    fn parse_owned(&self) -> Result<BTreeSet<u32>, String> {
        self.owned_episodes
            .split(',')
            .map(|slice| {
                let mut episodes_from_slice = Vec::new();
                if slice.contains('-') {
                    let mut range = slice.split('-');
                    let min = range
                        .next()
                        .and_then(|str| if str.is_empty() {None} else {Some(str)})
                        .ok_or("Can't find minimum value from the range")?
                        .trim()
                        .parse::<u32>()
                        .map_err(|err| format!("Value from range must be a number: {err}"))?;
                    let max = range
                        .next()
                        .and_then(|str| if str.is_empty() {None} else {Some(str)})
                        .ok_or("Can't find maximum value from the range")?
                        .trim()
                        .parse::<u32>()
                        .map_err(|err| format!("Value from range must be a number: {err}"))?;
                    (min..=max).for_each(|value| episodes_from_slice.push(value));
                } else {
                    let episode = slice.trim();
                    if !episode.is_empty() {
                        episodes_from_slice
                            .push(episode.parse::<u32>()
                                .map_err(|err| format!("Episode must be a number: {err}"))?);
                    }
                }
                Ok(episodes_from_slice)
            })
            .flat_map(|vec| match vec {
                Ok(vec) => vec.into_iter().map(Ok).collect(),
                Err(er) => vec![Err(er)],
            })
            .collect::<Result<BTreeSet<u32>, String>>()
    }
}

pub fn build<B: Backend, T: Service>(frame: &mut Frame<B>, app: &mut App<T>) {
    let area = super::centered_rect(70, 70, frame.size());
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
        main_chunks[1].x + u16::try_from(user_input.width()).unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_individual() {
        let mut mismatch_popup = MismatchPopup::new(15, 14);

        mismatch_popup.owned_episodes = String::from("1, 3, 5, 7");
        let result = mismatch_popup.parse_owned();
        assert!(result.is_ok());
        let parsed_owned = result.unwrap();
        let expected_owned: BTreeSet<u32> = vec![1, 3, 5, 7].into_iter().collect();
        assert_eq!(parsed_owned, expected_owned);
    }

    #[test]
    fn parse_range() {
        let mut mismatch_popup = MismatchPopup::new(15, 14);

        mismatch_popup.owned_episodes = String::from("1-5, 8-10");
        let result = mismatch_popup.parse_owned();
        assert!(result.is_ok());
        let parsed_owned = result.unwrap();
        let expected_owned: BTreeSet<u32> = vec![1, 2, 3, 4, 5, 8, 9, 10].into_iter().collect();
        assert_eq!(parsed_owned, expected_owned);
    }

    #[test]
    fn parse_combo() {
        let mut mismatch_popup = MismatchPopup::new(15, 14);

        mismatch_popup.owned_episodes = String::from("1-4, 6, 8-10, 12");
        let result = mismatch_popup.parse_owned();
        assert!(result.is_ok());
        let parsed_owned = result.unwrap();
        let expected_owned: BTreeSet<u32> = vec![1, 2, 3, 4, 6, 8, 9, 10, 12].into_iter().collect();
        assert_eq!(parsed_owned, expected_owned);
    }

    #[test]
    fn parse_range_no_max() {
        let mut mismatch_popup = MismatchPopup::new(15, 14);

        mismatch_popup.owned_episodes = String::from("1-");
        let result = mismatch_popup.parse_owned();
        assert!(result.is_err());
        let error = result.err().unwrap();
        assert_eq!(error, "Can't find maximum value from the range");
    }

    #[test]
    fn parse_range_no_min() {
        let mut mismatch_popup = MismatchPopup::new(15, 14);

        mismatch_popup.owned_episodes = String::from("-5");
        let result = mismatch_popup.parse_owned();
        assert!(result.is_err());
        let error = result.err().unwrap();
        assert_eq!(error, "Can't find minimum value from the range");
    }

    #[test]
    fn parse_range_nan() {
        let mut mismatch_popup = MismatchPopup::new(15, 14);

        mismatch_popup.owned_episodes = String::from("3-sd");
        let result = mismatch_popup.parse_owned();
        assert!(result.is_err());
        let error = result.err().unwrap();
        assert_eq!(error, "Value from range must be a number: invalid digit found in string");
    }

    #[test]
    fn parse_individual_nan() {
        let mut mismatch_popup = MismatchPopup::new(15, 14);

        mismatch_popup.owned_episodes = String::from("1, abc, 3");
        let result = mismatch_popup.parse_owned();
        assert!(result.is_err());
        let error = result.err().unwrap();
        assert_eq!(error, "Episode must be a number: invalid digit found in string");
    }
}