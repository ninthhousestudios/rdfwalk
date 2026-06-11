mod app;
mod config;
mod rdf;
mod ui;
mod util;

use anyhow::Result;
use app::{App, View};
use arboard;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use oxrdf::NamedNode;
use ratatui::{Terminal, backend::CrosstermBackend};
use rdf::sparql::SparqlClient;
use std::io;

#[derive(Parser)]
#[command(name = "rdfwalk", about = "TUI browser for RDF data via SPARQL")]
struct Args {
    /// SPARQL endpoint URL
    endpoint: Option<String>,
    /// Optional starting URI
    start_uri: Option<String>,
    /// Local RDF file (requires --features local)
    #[cfg(feature = "local")]
    #[arg(long, conflicts_with = "endpoint")]
    local: Option<String>,
    /// Maximum number of rows returned per query (default: 1000)
    #[arg(long, default_value = "1000")]
    limit: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let start_uri = args.start_uri.clone();
    let limit = args.limit;

    // Build the client before entering the TUI so that:
    // - a loading notice can be printed on the normal terminal for local files
    // - errors are reported without the TUI getting in the way
    #[cfg(feature = "local")]
    let client = {
        if let (None, Some(path)) = (&args.endpoint, &args.local) {
            eprintln!("Loading {}…", path);
        }
        match (args.endpoint, args.local) {
            (Some(ep), _) => SparqlClient::remote(ep),
            (None, Some(path)) => SparqlClient::local(&path)?,
            _ => anyhow::bail!("provide a SPARQL endpoint URL or --local <file>"),
        }
    };
    #[cfg(not(feature = "local"))]
    let client = match args.endpoint {
        Some(ep) => SparqlClient::remote(ep),
        None => anyhow::bail!("a SPARQL endpoint URL is required"),
    };

    let client = client.with_limit(limit);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, client, start_uri);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    client: SparqlClient,
    start_uri: Option<String>,
) -> Result<()> {
    let mut app = App::new(client);

    if let Some(uri_str) = start_uri {
        match NamedNode::new(uri_str) {
            Ok(uri) => app.navigate_to_node(uri),
            Err(e) => app.status = format!("Invalid URI: {}", e),
        }
    } else {
        app.load_types()?;
    }

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                let in_text_input = matches!(
                    (&app.view, app.sparql_mode_input, app.search_mode_input),
                    (View::Sparql, true, _) | (View::Search, _, true)
                );

                match (&app.view, key.code, key.modifiers) {
                    // Text input: handle char keys first so they don't leak to global bindings
                    (View::Sparql, KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT)
                        if app.sparql_mode_input =>
                    {
                        app.sparql_push_char(c)
                    }
                    (View::Search, KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT)
                        if app.search_mode_input =>
                    {
                        app.search_push_char(c)
                    }

                    // Global quit (only when not typing)
                    (_, KeyCode::Char('q') | KeyCode::Char('Q'), _) if !in_text_input => break,

                    // Switch views (only when not typing)
                    (_, KeyCode::Char('t'), KeyModifiers::NONE) if !in_text_input => {
                        app.view = View::Types;
                        if app.types_list.is_empty() {
                            app.load_types()?;
                        }
                    }
                    (_, KeyCode::Char('s'), KeyModifiers::NONE) if !in_text_input => {
                        app.view = View::Sparql;
                        app.sparql_mode_input = true;
                    }
                    (_, KeyCode::Char('f'), KeyModifiers::NONE) if !in_text_input => {
                        app.view = View::Search;
                        app.search_mode_input = true;
                    }
                    (_, KeyCode::Char('m'), KeyModifiers::NONE) if !in_text_input => {
                        app.view = View::Bookmarks;
                    }
                    (
                        View::Sparql | View::Search | View::Bookmarks,
                        KeyCode::Char('b'),
                        KeyModifiers::NONE,
                    ) if !in_text_input => {
                        if app.browser_data.is_some() {
                            app.view = View::Browser;
                        }
                    }
                    (View::Sparql | View::Search | View::Bookmarks, KeyCode::Esc, _) => {
                        if app.browser_data.is_some() {
                            app.view = View::Browser;
                        }
                    }

                    // Bookmark toggle in browser
                    (View::Browser, KeyCode::Char('b'), KeyModifiers::NONE) => {
                        app.toggle_bookmark()
                    }

                    // Copy current triple to clipboard
                    (View::Browser, KeyCode::Char('c'), KeyModifiers::NONE) => {
                        if let Some(text) = app.current_triple_sparql() {
                            match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&text)) {
                                Ok(_) => app.status = "Copied to clipboard".into(),
                                Err(e) => app.status = format!("Clipboard error: {}", e),
                            }
                        }
                    }

                    // Browser navigation
                    (View::Browser, KeyCode::Up, _) => app.browser_select_up(),
                    (View::Browser, KeyCode::Down, _) => app.browser_select_down(),
                    (View::Browser, KeyCode::Tab, KeyModifiers::NONE) => app.browser_next_section(),
                    (View::Browser, KeyCode::BackTab, _) => app.browser_prev_section(),
                    (View::Browser, KeyCode::Enter, _) => app.browser_activate(),
                    (View::Browser, KeyCode::Left, _) => app.history_back(),
                    (View::Browser, KeyCode::Right, _) => app.history_forward(),

                    // Types navigation
                    (View::Types, KeyCode::Up, _) => app.types_select_up(),
                    (View::Types, KeyCode::Down, _) => app.types_select_down(),
                    (View::Types, KeyCode::Enter, _) => app.types_activate(),

                    // SPARQL
                    (View::Sparql, KeyCode::Tab, _) => {
                        if app.sparql_result.is_some() {
                            app.sparql_mode_input = !app.sparql_mode_input;
                        }
                    }
                    (View::Sparql, KeyCode::Enter, _) if app.sparql_mode_input => app.sparql_run(),
                    (View::Sparql, KeyCode::Backspace, _) if app.sparql_mode_input => {
                        app.sparql_backspace()
                    }
                    (View::Sparql, KeyCode::Left, _) if app.sparql_mode_input => {
                        app.sparql_cursor_left()
                    }
                    (View::Sparql, KeyCode::Right, _) if app.sparql_mode_input => {
                        app.sparql_cursor_right()
                    }
                    (View::Sparql, KeyCode::Char('u'), KeyModifiers::CONTROL)
                        if app.sparql_mode_input =>
                    {
                        app.sparql_clear()
                    }
                    (View::Sparql, KeyCode::Char('c'), KeyModifiers::CONTROL)
                        if app.sparql_mode_input =>
                    {
                        match arboard::Clipboard::new()
                            .and_then(|mut cb| cb.set_text(&app.sparql_input))
                        {
                            Ok(_) => app.status = "Copied".into(),
                            Err(e) => app.status = format!("Clipboard error: {}", e),
                        }
                    }
                    (View::Sparql, KeyCode::Char('v'), KeyModifiers::CONTROL)
                        if app.sparql_mode_input =>
                    {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => {
                                app.sparql_input.insert_str(app.sparql_cursor, &text);
                                app.sparql_cursor += text.len();
                            }
                            Err(e) => app.status = format!("Clipboard error: {}", e),
                        }
                    }
                    (View::Sparql, KeyCode::Up, _) if !app.sparql_mode_input => {
                        app.sparql_result_up()
                    }
                    (View::Sparql, KeyCode::Down, _) if !app.sparql_mode_input => {
                        app.sparql_result_down()
                    }
                    (View::Sparql, KeyCode::Enter, _) if !app.sparql_mode_input => {
                        app.sparql_activate()
                    }

                    // Bookmarks
                    (View::Bookmarks, KeyCode::Up, _) => app.bookmarks_select_up(),
                    (View::Bookmarks, KeyCode::Down, _) => app.bookmarks_select_down(),
                    (View::Bookmarks, KeyCode::Enter, _) => app.bookmarks_activate(),
                    (View::Bookmarks, KeyCode::Delete, _) => {
                        if app.bookmarks_selection < app.bookmarks.len() {
                            app.bookmarks.remove(app.bookmarks_selection);
                            if app.bookmarks_selection > 0
                                && app.bookmarks_selection >= app.bookmarks.len()
                            {
                                app.bookmarks_selection -= 1;
                            }
                            app.save_bookmarks();
                        }
                    }

                    // Search
                    (View::Search, KeyCode::Tab, _) => {
                        if !app.search_results.is_empty() {
                            app.search_mode_input = !app.search_mode_input;
                        }
                    }
                    (View::Search, KeyCode::Enter, _) if app.search_mode_input => app.search_run(),
                    (View::Search, KeyCode::Backspace, _) if app.search_mode_input => {
                        app.search_backspace()
                    }
                    (View::Search, KeyCode::Left, _) if app.search_mode_input => {
                        app.search_cursor_left()
                    }
                    (View::Search, KeyCode::Right, _) if app.search_mode_input => {
                        app.search_cursor_right()
                    }
                    (View::Search, KeyCode::Up, _) if !app.search_mode_input => {
                        app.search_result_up()
                    }
                    (View::Search, KeyCode::Down, _) if !app.search_mode_input => {
                        app.search_result_down()
                    }
                    (View::Search, KeyCode::Enter, _) if !app.search_mode_input => {
                        app.search_activate()
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}
