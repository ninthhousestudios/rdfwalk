use ratatui::style::Color;

pub fn cell_color(cell: &Option<oxrdf::Term>) -> Color {
    match cell {
        Some(oxrdf::Term::NamedNode(_)) => Color::Yellow,
        Some(oxrdf::Term::Literal(_)) => Color::Green,
        #[cfg(feature = "rdf-star")]
        Some(oxrdf::Term::Triple(_)) => Color::Magenta,
        _ => Color::Gray,
    }
}
