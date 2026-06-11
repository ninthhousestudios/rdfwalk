use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::widgets;
use crate::app::App;
use crate::util::*;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    let cursor = if app.search_mode_input {
        Some(app.search_cursor)
    } else {
        None
    };
    widgets::text_input(
        f,
        &app.search_input,
        cursor,
        app.search_mode_input,
        " Search literals ",
        chunks[0],
    );

    let result_block = Block::default()
        .title(" Results (resource  │  property  │  matched value) ")
        .borders(Borders::ALL)
        .border_style(if !app.search_mode_input {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let inner = result_block.inner(chunks[1]);
    f.render_widget(result_block, chunks[1]);

    let col_w = (inner.width as usize).saturating_sub(6) / 3;
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let res = string::truncate(app.display.display_focus(&r.resource), col_w);
            let prop = string::truncate(app.display.display_node(&r.property), col_w);
            let val = string::truncate(string::sanitize(&r.matched_value), col_w);
            let line = Line::from(vec![
                Span::styled(
                    format!("{:<width$}", res, width = col_w),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" │ "),
                Span::styled(
                    format!("{:<width$}", prop, width = col_w),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" │ "),
                Span::styled(val, Style::default().fg(Color::Green)),
            ]);
            let selected = i == app.search_selection && !app.search_mode_input;
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
    if !app.search_mode_input {
        state.select(Some(app.search_selection));
    }
    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, inner, &mut state);
}
