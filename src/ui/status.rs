use crate::app::{App, View};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let view_hint = match app.view {
        View::Browser => {
            "[T]ypes  [S]PARQL  [F]ind  [M]arks  [b] Bookmark  [c] Copy triple  [Tab] Next section  [↑/↓] Navigate  [Enter] Open  [←/→] History  [Q]uit"
        }
        View::Types => "[S]PARQL  [F]ind  [M]arks  [↑/↓] Navigate  [Enter] Browse  [Q]uit",
        View::Sparql if app.sparql_mode_input => {
            "[Esc] Browser  [Enter] Run  [Tab] Results  [Ctrl+U] Clear  [Ctrl+C] Copy  [Ctrl+V] Paste"
        }
        View::Sparql => "[Tab] Input  [↑/↓] Navigate  [Enter] Open  [Esc/B] Browser  [Q]uit",
        View::Search if app.search_mode_input => "[Esc] Browser  [Enter] Search  [Tab] Results",
        View::Search => "[Tab] Input  [↑/↓] Navigate  [Enter] Browse  [Esc/B] Browser  [Q]uit",
        View::Bookmarks => {
            "[B]rowser  [T]ypes  [S]PARQL  [F]ind  [↑/↓] Navigate  [Enter] Browse  [Del] Remove  [Q]uit"
        }
    };
    let status = if app.status.is_empty() {
        view_hint.to_string()
    } else {
        format!("{}  |  {}", app.status, view_hint)
    };
    let p = Paragraph::new(status).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(p, area);
}
