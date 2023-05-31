use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
pub(crate) mod interactions;
use crate::app;

pub(crate) fn ui<B: Backend>(f: &mut Frame<B>, app: &mut app::App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
        .split(f.size());
    
    // Split the bigger chunk into halves
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    let items: Vec<_> = app
        .items
        .shows
        .iter()
        .map(|(_, show)| ListItem::new(format!("{}", show.title)).style(Style::default()))
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Iterate through all elements in the `items` app
    let episodes: Vec<ListItem> = app
        .items
        .shows
        .iter()
        .filter(|(id, _)| *id == app.items.episodes_state.selected_id)
        .flat_map(|(_, show)| {
            let mut temp: Vec<ListItem> = Vec::new();
            for (episode_number, path) in &show.episodes {
                temp.push(
                    ListItem::new(format!("{} {}", episode_number, path)).style(Style::default()),
                );
            }
            temp
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let episodes = List::new(episodes)
        .block(Block::default().borders(Borders::ALL).title("Episodes"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Create help text at the bottom
    let help = Block::default().title("Help").borders(Borders::ALL);

    // We can now render the item list
    f.render_stateful_widget(items, main_chunks[0], &mut app.items.state);
    f.render_stateful_widget(episodes, main_chunks[1], &mut app.items.episodes_state.list_state);
    f.render_widget(help, chunks[1]);
}
