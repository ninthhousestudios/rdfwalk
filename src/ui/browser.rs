use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::{App, model::BrowserItem};
use crate::util::*;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    // Collect rdf:type values from outgoing links already in browser_items
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let type_labels: Vec<String> = app
        .browser_items
        .iter()
        .filter_map(|item| {
            if let BrowserItem::OutgoingLink { prop, target } = item {
                if prop.as_str() == RDF_TYPE {
                    Some(app.display.display_focus(target))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Address bar: left = bookmark marker + label + URI, right = types flush to right edge
    let inner_width = chunks[0].width.saturating_sub(2) as usize; // minus borders
    let addr_line = if let Some(d) = &app.browser_data {
        let label = app.display.display_focus(&d.focus);
        let bookmarked = d
            .focus
            .as_named_node()
            .is_some_and(|uri| app.is_bookmarked(uri));

        // Build left spans and measure their total char width
        let star_str = if bookmarked { " ★ " } else { "   " };
        let (left_spans, left_len) = if let Some(uri) = d.focus.as_named_node() {
            let raw_uri = uri.as_str();
            if label == format!("<{}>", raw_uri) {
                let s = format!("{}{}", star_str, raw_uri);
                let len = s.chars().count();
                (
                    vec![
                        Span::styled(star_str.to_string(), Style::default().fg(Color::Yellow)),
                        Span::styled(raw_uri.to_string(), Style::default().fg(Color::Yellow)),
                    ],
                    len,
                )
            } else {
                let raw = format!("<{}>", raw_uri);
                let len =
                    star_str.chars().count() + label.chars().count() + 1 + raw.chars().count();
                (
                    vec![
                        Span::styled(star_str.to_string(), Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{} ", label),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(raw, Style::default().fg(Color::DarkGray)),
                    ],
                    len,
                )
            }
        } else {
            let s = format!("{}{}", star_str, label);
            let len = s.chars().count();
            (
                vec![
                    Span::styled(star_str.to_string(), Style::default().fg(Color::Yellow)),
                    Span::styled(label, Style::default().fg(Color::Yellow)),
                ],
                len,
            )
        };

        // Right: types joined, padded to the right edge
        let mut spans = left_spans;
        if !type_labels.is_empty() {
            let type_str = type_labels.join("  ");
            let type_len = type_str.chars().count();
            let pad = inner_width.saturating_sub(left_len + type_len);
            spans.push(Span::raw(" ".repeat(pad)));
            spans.push(Span::styled(type_str, Style::default().fg(Color::Green)));
        }
        Line::from(spans)
    } else {
        Line::from(Span::raw(" "))
    };

    let addr_block = Block::default().title(" Resource ").borders(Borders::ALL);
    f.render_widget(Paragraph::new(addr_line).block(addr_block), chunks[0]);

    // Four separate section lists
    let offsets = app.browser_section_offsets;
    let total = app.browser_items.len();
    let section_counts = [
        offsets[1],
        offsets[2] - offsets[1],
        offsets[3] - offsets[2],
        total.saturating_sub(offsets[3]),
    ];
    let section_names = [
        "Literal Properties",
        "Outgoing Links",
        "Incoming Links",
        "As Predicate",
    ];

    let constraints: Vec<Constraint> = section_counts
        .iter()
        .map(|&n| {
            if n > 0 {
                Constraint::Min(3)
            } else {
                Constraint::Length(0)
            }
        })
        .collect();

    let section_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(chunks[1]);

    for idx in 0..4 {
        let count = section_counts[idx];
        if count == 0 {
            continue;
        }
        let start = offsets[idx];
        let block = Block::default()
            .title(format!(" {} ", section_names[idx]))
            .borders(Borders::ALL);
        let inner = block.inner(section_areas[idx]);
        f.render_widget(block, section_areas[idx]);

        let avail_width = inner.width as usize;
        let items: Vec<ListItem> = app.browser_items[start..start + count]
            .iter()
            .enumerate()
            .map(|(j, item)| {
                let selected = (start + j) == app.browser_selection;
                let line = item_line(item, app, avail_width);
                let style = if selected {
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(line).style(style)
            })
            .collect();

        let local_sel = if app.browser_selection >= start && app.browser_selection < start + count {
            Some(app.browser_selection - start)
        } else {
            None
        };
        let mut state = ListState::default();
        state.select(local_sel);
        let list = List::new(items).highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list, inner, &mut state);
    }

    // Triple line at the bottom of the browser area
    let triple = current_triple(app);
    f.render_widget(Paragraph::new(triple), chunks[2]);
}

fn current_triple(app: &App) -> Line<'static> {
    let dim = Style::default().fg(Color::DarkGray);
    match app.current_triple_sparql() {
        Some(text) => Line::from(Span::styled(format!("{}", text), dim)),
        None => Line::from(""),
    }
}

fn item_line<'a>(item: &BrowserItem, app: &App, avail_width: usize) -> Line<'a> {
    const PROP_COL: usize = 40;
    const TYPE_COL: usize = 20;
    const ARROW: usize = 3; // Arrow prefix "→  " or "←  " occupies 3 chars; property name gets the rest of PROP_COL

    match item {
        BrowserItem::LiteralProp { prop, value } => {
            let p = string::truncate(app.display.display_node(prop), PROP_COL - ARROW);
            // 4 (indent+arrow) + (PROP_COL-ARROW) + 3 (" = ") + TYPE_COL
            let value_col = avail_width.saturating_sub(4 + (PROP_COL - ARROW) + 3 + TYPE_COL);
            let (raw_val, suffix) = app.display.literal_parts(value);
            let v = string::truncate(string::sanitize(&raw_val), value_col);
            let type_str = string::truncate(suffix.unwrap_or_default(), TYPE_COL);
            Line::from(vec![
                Span::styled("  → ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:<width$}", p, width = PROP_COL - ARROW),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(" = ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<width$}", v, width = value_col)),
                Span::styled(type_str, Style::default().fg(Color::DarkGray)),
            ])
        }
        BrowserItem::OutgoingLink { prop, target } => {
            // → predicate → target
            let p = string::truncate(app.display.display_node(prop), PROP_COL - ARROW);
            let t = app.display.display_focus(target);
            Line::from(vec![
                Span::styled("  → ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:<width$}", p, width = PROP_COL - ARROW),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(" → ", Style::default().fg(Color::DarkGray)),
                Span::styled(t, Style::default().fg(Color::Yellow)),
            ])
        }
        BrowserItem::IncomingLink { prop, source } => {
            // ← predicate ← source
            let p = string::truncate(app.display.display_node(prop), PROP_COL - ARROW);
            let s = app.display.display_focus(source);
            Line::from(vec![
                Span::styled("  ← ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:<width$}", p, width = PROP_COL - ARROW),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(" ← ", Style::default().fg(Color::DarkGray)),
                Span::styled(s, Style::default().fg(Color::Yellow)),
            ])
        }
        BrowserItem::AsPredicateRow { subject, object } => {
            let s = string::truncate(app.display.display_focus(subject), PROP_COL - ARROW);
            let o = app.display.display_term(object);
            Line::from(vec![
                Span::styled("  ◆ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("  {:<width$}", s, width = PROP_COL - ARROW),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(" ◆ ", Style::default().fg(Color::DarkGray)),
                Span::styled(o, Style::default().fg(Color::Green)),
            ])
        }
    }
}
