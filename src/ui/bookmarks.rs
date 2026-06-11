use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" Bookmarks ({}) ", app.bookmarks.len()))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = app
        .bookmarks
        .iter()
        .enumerate()
        .map(|(i, uri)| {
            let label = app.display.display_node(uri);
            let show_raw = label == format!("<{}>", uri.as_str());
            let line = if show_raw {
                Line::from(Span::styled(
                    format!("  {}", uri.as_str()),
                    Style::default().fg(Color::Yellow),
                ))
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("  {} ", label),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("<{}>", uri.as_str()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            };
            let selected = i == app.bookmarks_selection;
            ListItem::new(line).style(if selected {
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
        })
        .collect();

    let mut state = ListState::default();
    state.select(if app.bookmarks.is_empty() {
        None
    } else {
        Some(app.bookmarks_selection)
    });
    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, inner, &mut state);
}
