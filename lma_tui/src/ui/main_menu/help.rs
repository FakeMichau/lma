use crate::app::App;
use crate::config::KeyBinds;
use crate::ui::popup::insert_show::InsertState;
use crate::ui::FocusedWindow;
use crossterm::event::KeyCode;
use lma_lib::Service;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::{text::Line, Frame};

pub fn render<T: Service>(app: &App<T>, area: Rect, frame: &mut Frame) {
    let help = build_help(
        &app.focused_window,
        &app.insert_popup.state,
        &app.insert_episode_popup.state,
        app.config.colors.highlight_dark,
        &app.config.key_binds,
    );
    frame.render_widget(help, area);
}

fn build_help<'a>(
    focused_window: &FocusedWindow,
    insert_show_state: &InsertState,
    insert_episode_state: &InsertState,
    bg_color: Color,
    key_binds: &KeyBinds,
) -> Paragraph<'a> {
    let navigation = HelpItem::new("Navigation", &Action::Navigation, key_binds, bg_color);
    let insert = HelpItem::new("Insert new show", &Action::NewShow, key_binds, bg_color);
    let delete = HelpItem::new("Delete the entry", &Action::Delete, key_binds, bg_color);
    let go_back = HelpItem::new("Go back", &Action::Close, key_binds, bg_color);
    let close_window = HelpItem::new("Close the window", &Action::Close, key_binds, bg_color);
    let exit_input = HelpItem::new("Stop inputting", &Action::Close, key_binds, bg_color);
    let start_input = HelpItem::new("Start inputting", &Action::EnterInput, key_binds, bg_color);
    let confirm = HelpItem::new("Confirm", &Action::Confirmation, key_binds, bg_color);
    let login = HelpItem::new("Login", &Action::Login, key_binds, bg_color);
    let progress = HelpItem::new("Progress", &Action::Progress, key_binds, bg_color);
    let insert_episode = HelpItem::new("Add episode", &Action::NewEpisode, key_binds, bg_color);
    let quit = HelpItem::new("Quit", &Action::Quit, key_binds, bg_color);

    let mut information = Vec::new();
    match focused_window {
        FocusedWindow::MainMenu => {
            information.extend(navigation);
            information.extend(insert);
            information.extend(delete);
            information.extend(login);
            information.extend(insert_episode);
            information.extend(progress);
            information.extend(quit);
        }
        FocusedWindow::InsertPopup => {
            information.extend(navigation);
            match insert_show_state {
                InsertState::Inputting | InsertState::Next => {
                    information.extend(confirm);
                    information.extend(exit_input);
                }
                _ => {
                    information.extend(start_input);
                    information.extend(close_window);
                }
            }
        }
        FocusedWindow::InsertEpisodePopup => {
            information.extend(navigation);
            match insert_episode_state {
                InsertState::Inputting | InsertState::Next => {
                    information.extend(confirm);
                    information.extend(exit_input);
                }
                _ => {
                    information.extend(start_input);
                    information.extend(close_window);
                }
            }
        }
        FocusedWindow::Login => {
            information.extend(close_window);
        }
        FocusedWindow::FirstSetup => {
            information.extend(navigation);
            information.extend(go_back);
            information.extend(confirm);
        }
        FocusedWindow::TitleSelection => {
            information.extend(navigation);
            information.extend(close_window);
        }
        FocusedWindow::EpisodeMismatch | FocusedWindow::Error => {
            information.extend(confirm);
            information.extend(close_window);
        }
    };

    Paragraph::new(Line::from(information))
}

enum Action {
    Navigation,
    Progress,
    Confirmation,
    Close,
    Delete,
    Quit,
    EnterInput,
    NewShow,
    NewEpisode,
    Login,
}

struct HelpItem<'a> {
    text: &'a str,
    key: String,
    text_style: Style,
    key_style: Style,
}

impl<'a> HelpItem<'a> {
    fn new(text: &'a str, name: &Action, key_binds: &KeyBinds, highlight_color: Color) -> Self {
        let text_style = Style::default().bg(highlight_color);
        let key_style = text_style.add_modifier(Modifier::BOLD);
        let key = key_to_abbr(key_binds, name);
        Self {
            text,
            key,
            text_style,
            key_style,
        }
    }
}

impl<'a> IntoIterator for HelpItem<'a> {
    type Item = Span<'a>;
    type IntoIter = <Vec<Span<'a>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            Span::styled(format!("{} ", self.text), self.text_style),
            Span::styled(format!("[{}]", self.key), self.key_style),
            Span::raw(" "),
        ]
        .into_iter()
    }
}

fn key_to_abbr(key: &KeyBinds, name: &Action) -> String {
    match name {
        Action::Navigation => {
            if key.move_up == KeyCode::Up
                && key.move_down == KeyCode::Down
                && key.backwards == KeyCode::Left
                && key.forwards == KeyCode::Right
            {
                String::from("ARROWS")
            } else {
                format!(
                    "{}{}{}{}",
                    keycode_to_key(key.move_up),
                    keycode_to_key(key.move_down),
                    keycode_to_key(key.backwards),
                    keycode_to_key(key.forwards),
                )
            }
        }
        Action::Progress => {
            if key.progress_inc == KeyCode::Char('.') && key.progress_dec == KeyCode::Char(',') {
                String::from("< >")
            } else {
                format!(
                    "{} {}",
                    keycode_to_key(key.progress_inc),
                    keycode_to_key(key.progress_dec),
                )
            }
        }
        Action::Confirmation => keycode_to_key(key.confirmation),
        Action::Close => keycode_to_key(key.close),
        Action::Delete => keycode_to_key(key.delete),
        Action::Quit => keycode_to_key(key.quit),
        Action::EnterInput => keycode_to_key(key.enter_inputting),
        Action::NewShow => keycode_to_key(key.new_show),
        Action::NewEpisode => keycode_to_key(key.new_episode),
        Action::Login => keycode_to_key(key.login),
    }
}

fn keycode_to_key(keycode: KeyCode) -> String {
    match keycode {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Char(c) => c.to_uppercase().to_string(),
        KeyCode::Null => "Null".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::CapsLock => "CapsLock".to_string(),
        KeyCode::ScrollLock => "ScrollLock".to_string(),
        KeyCode::NumLock => "NumLock".to_string(),
        KeyCode::PrintScreen => "PrintScreen".to_string(),
        KeyCode::Pause => "Pause".to_string(),
        KeyCode::Menu => "Menu".to_string(),
        KeyCode::KeypadBegin => "KeypadBegin".to_string(),
        KeyCode::Media(m) => format!("Media({m:?})"),
        KeyCode::Modifier(m) => format!("Modifier({m:?})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_item() {
        let highlight_color = Color::Rgb(0, 0, 0);
        let key_binds = KeyBinds {
            quit: KeyCode::Char('q'),
            ..Default::default()
        };
        let mut test_item =
            HelpItem::new("Testing", &Action::Quit, &key_binds, highlight_color).into_iter();

        let text_style = Style::default().bg(highlight_color);
        let key_style = text_style.add_modifier(Modifier::BOLD);
        let mut expected_span = vec![
            Span::styled("Testing ", text_style),
            Span::styled("[Q]", key_style),
            Span::raw(" "),
        ]
        .into_iter();
        assert_eq!(test_item.next(), expected_span.next());
        assert_eq!(test_item.next(), expected_span.next());
        assert_eq!(test_item.next(), expected_span.next());
    }
}
