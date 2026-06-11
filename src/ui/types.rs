use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref err) = app.types_error {
        let block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(err.as_str())
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let block = Block::default().title(" Types ").borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = app
        .types_list
        .iter()
        .enumerate()
        .map(|(i, uri)| {
            let label = app.display.display_node(uri);
            let mut li = ListItem::new(Line::from(Span::styled(
                format!("  {}", label),
                Style::default().fg(Color::Yellow),
            )));
            if i == app.types_selection {
                li = li.style(
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                );
            }
            li
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.types_selection));
    let list = List::new(items).highlight_style(Style::default().bg(Color::Blue));
    f.render_stateful_widget(list, inner, &mut state);
}
