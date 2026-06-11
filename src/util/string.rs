pub fn truncate(s: String, max: usize) -> String {
    if s.chars().count() > max {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", t)
    } else {
        s
    }
}

/// Collapses a string to a single displayable line.
/// Replaces control characters (newlines, tabs, …) with spaces and compresses runs.
pub fn sanitize(s: &str) -> String {
    let single: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    single.split_whitespace().collect::<Vec<_>>().join(" ")
}
