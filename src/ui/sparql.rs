use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use super::widgets;
use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(3)])
        .split(area);

    let cursor = if app.sparql_mode_input {
        Some(app.sparql_cursor)
    } else {
        None
    };
    widgets::text_input(
        f,
        &app.sparql_input,
        cursor,
        app.sparql_mode_input,
        " SPARQL Query ",
        chunks[0],
    );

    let vars_title = app
        .sparql_result
        .as_ref()
        .map(|r| {
            let names: Vec<String> = r.variables.iter().map(|v| format!("?{}", v)).collect();
            format!(" Results: {} ", names.join("  "))
        })
        .unwrap_or_else(|| " Results ".into());

    // Show error panel when the last query failed
    if let Some(ref err) = app.sparql_error {
        let block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));
        let inner = block.inner(chunks[1]);
        f.render_widget(block, chunks[1]);
        let para = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: false });
        f.render_widget(para, inner);
        return;
    }

    let result_block = Block::default()
        .title(vars_title)
        .borders(Borders::ALL)
        .border_style(if !app.sparql_mode_input {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let inner = result_block.inner(chunks[1]);
    f.render_widget(result_block, chunks[1]);

    let items: Vec<ListItem> = app
        .sparql_result
        .as_ref()
        .map(|r| {
            let ncols = r.variables.len().max(1);
            let col_w = (inner.width as usize).saturating_sub(ncols.saturating_sub(1) * 3) / ncols;
            r.rows
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let spans: Vec<Span> = row
                        .iter()
                        .enumerate()
                        .flat_map(|(j, cell)| {
                            let val = cell
                                .as_ref()
                                .map(|t| app.display.display_term_plain(t))
                                .unwrap_or_default();
                            let color = super::util::cell_color(cell);
                            let padded = format!("{:<width$}", val, width = col_w);
                            if j + 1 < ncols {
                                vec![
                                    Span::styled(padded, Style::default().fg(color)),
                                    Span::raw(" │ "),
                                ]
                            } else {
                                vec![Span::styled(padded, Style::default().fg(color))]
                            }
                        })
                        .collect();
                    let selected = i == app.sparql_selection && !app.sparql_mode_input;
                    ListItem::new(Line::from(spans)).style(if selected {
                        Style::default()
                            .bg(Color::Blue)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut state = ListState::default();
    if !app.sparql_mode_input {
        state.select(Some(app.sparql_selection));
    }
    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, inner, &mut state);
}
