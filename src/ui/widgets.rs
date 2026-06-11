use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Renders a text input box. When `cursor` is `Some(byte_offset)` the terminal
/// cursor is positioned at that offset inside the text (accounting for `\n`
/// and soft-wrapping at the inner widget width).
pub fn text_input(
    f: &mut Frame,
    text: &str,
    cursor: Option<usize>,
    active: bool,
    title: &str,
    area: Rect,
) {
    let border_style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let para = Paragraph::new(text)
        .block(block)
        .style(if active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        })
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);

    if let Some(byte_offset) = cursor {
        let inner_w = area.width.saturating_sub(2) as usize;
        let before = &text[..byte_offset.min(text.len())];
        let mut row = 0u16;
        let mut col = 0usize;
        for ch in before.chars() {
            if ch == '\n' {
                row += 1;
                col = 0;
            } else {
                col += 1;
                if col >= inner_w {
                    row += 1;
                    col = 0;
                }
            }
        }
        let cx = (area.x + 1 + col as u16).min(area.x + area.width.saturating_sub(2));
        let cy = (area.y + 1 + row).min(area.y + area.height.saturating_sub(2));
        f.set_cursor_position((cx, cy));
    }
}
