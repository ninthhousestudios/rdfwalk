pub mod model;

use crate::config;
use crate::rdf::display::DisplayContext;
use crate::rdf::sparql::SparqlClient;
use anyhow::Result;
use model::*;
use oxrdf::{NamedNode, Term};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Browser,
    Types,
    Sparql,
    Search,
    Bookmarks,
}

pub struct App {
    pub client: Arc<SparqlClient>,
    pub display: DisplayContext,
    pub view: View,

    // Browser state
    pub browser_data: Option<BrowserData>,
    pub browser_items: Vec<BrowserItem>,
    pub browser_selection: usize,
    pub browser_section_offsets: [usize; 4], // start index of each section
    pub history: Vec<FocusTerm>,
    pub history_pos: usize,

    // Types state
    pub types_list: Vec<NamedNode>,
    pub types_error: Option<String>,
    pub types_selection: usize,

    // SPARQL state
    pub sparql_input: String,
    pub sparql_cursor: usize,
    pub sparql_result: Option<SparqlResult>,
    pub sparql_error: Option<String>,
    pub sparql_selection: usize,
    pub sparql_mode_input: bool,

    // Search state
    pub search_input: String,
    pub search_cursor: usize,
    pub search_results: Vec<SearchResult>,
    pub search_selection: usize,
    pub search_mode_input: bool,

    // Bookmarks state
    pub bookmarks: Vec<NamedNode>,
    pub bookmarks_selection: usize,

    // Status / error
    pub status: String,
}

impl App {
    pub fn new(client: SparqlClient) -> Self {
        Self {
            client: Arc::new(client),
            display: DisplayContext::new(),
            view: View::Types,
            browser_data: None,
            browser_items: Vec::new(),
            browser_selection: 0,
            browser_section_offsets: [0; 4],
            history: Vec::new(),
            history_pos: 0,
            types_list: Vec::new(),
            types_error: None,
            types_selection: 0,
            sparql_input: String::new(),
            sparql_cursor: 0,
            sparql_result: None,
            sparql_error: None,
            sparql_selection: 0,
            sparql_mode_input: true,
            search_input: String::new(),
            search_cursor: 0,
            search_results: Vec::new(),
            search_selection: 0,
            search_mode_input: true,
            bookmarks: Self::load_bookmarks(),
            bookmarks_selection: 0,
            status: String::new(),
        }
    }

    pub fn load_types(&mut self) -> Result<()> {
        self.status = "Loading types…".into();
        match self.client.all_types() {
            Ok(types) => {
                self.types_list = types.clone();
                self.types_error = None;
                self.fetch_labels_for_nodes(&types);
                self.status = format!("{} types found", self.types_list.len());
            }
            Err(e) => {
                self.types_list.clear();
                self.types_error = Some(format!("{:#}", e));
                self.status = "Error loading types".into();
            }
        }
        Ok(())
    }

    fn fetch_labels_for_nodes(&mut self, nodes: &[NamedNode]) {
        for node in nodes {
            if self.display.display_node(node).starts_with('<') {
                // No label cached, try to fetch
                let label = self.client.label_for(node).ok().flatten();
                self.display.cache_label(node.as_str(), label);
            }
        }
    }

    pub fn navigate_to(&mut self, focus: FocusTerm) {
        // Truncate forward history
        if self.history_pos < self.history.len() {
            self.history.truncate(self.history_pos);
        }
        self.history.push(focus.clone());
        self.history_pos = self.history.len();
        self.load_browser(focus);
        self.view = View::Browser;
    }

    pub fn navigate_to_node(&mut self, uri: NamedNode) {
        self.navigate_to(FocusTerm::NamedNode(uri));
    }

    pub fn history_back(&mut self) {
        if self.history_pos > 1 {
            self.history_pos -= 1;
            let focus = self.history[self.history_pos - 1].clone();
            self.load_browser(focus);
            self.view = View::Browser;
        }
    }

    pub fn history_forward(&mut self) {
        if self.history_pos < self.history.len() {
            self.history_pos += 1;
            let focus = self.history[self.history_pos - 1].clone();
            self.load_browser(focus);
            self.view = View::Browser;
        }
    }

    fn load_browser(&mut self, focus: FocusTerm) {
        self.status = format!("Loading {}…", self.display.display_focus(&focus));
        self.browser_selection = 0;

        let lit = self.client.literal_properties(&focus).unwrap_or_default();
        let out = self.client.outgoing_links(&focus).unwrap_or_default();
        let inc = self.client.incoming_links(&focus).unwrap_or_default();
        let pred = self.client.as_predicate(&focus).unwrap_or_default();

        let mut all_nodes: Vec<NamedNode> = Vec::new();
        for (p, _) in &lit {
            all_nodes.push(p.clone());
        }
        for (p, o) in &out {
            all_nodes.push(p.clone());
            if let Some(n) = o.as_named_node() {
                all_nodes.push(n.clone());
            }
        }
        for (p, s) in &inc {
            all_nodes.push(p.clone());
            if let Some(n) = s.as_named_node() {
                all_nodes.push(n.clone());
            }
        }
        for (s, o) in &pred {
            if let Some(n) = s.as_named_node() {
                all_nodes.push(n.clone());
            }
            if let Term::NamedNode(n) = o {
                all_nodes.push(n.clone());
            }
        }
        self.fetch_labels_for_nodes(&all_nodes);

        let mut items: Vec<BrowserItem> = Vec::new();
        let s0 = 0;
        for (p, v) in &lit {
            items.push(BrowserItem::LiteralProp {
                prop: p.clone(),
                value: v.clone(),
            });
        }
        let s1 = items.len();
        for (p, o) in &out {
            items.push(BrowserItem::OutgoingLink {
                prop: p.clone(),
                target: o.clone(),
            });
        }
        let s2 = items.len();
        for (p, s) in &inc {
            items.push(BrowserItem::IncomingLink {
                prop: p.clone(),
                source: s.clone(),
            });
        }
        let s3 = items.len();
        for (s, o) in &pred {
            items.push(BrowserItem::AsPredicateRow {
                subject: s.clone(),
                object: o.clone(),
            });
        }

        self.browser_data = Some(BrowserData { focus });
        self.browser_items = items;
        self.browser_section_offsets = [s0, s1, s2, s3];
        self.status = String::new();
    }

    pub fn browser_select_up(&mut self) {
        if self.browser_selection > 0 {
            self.browser_selection -= 1;
        }
    }

    pub fn browser_select_down(&mut self) {
        if self.browser_selection + 1 < self.browser_items.len() {
            self.browser_selection += 1;
        }
    }

    pub fn browser_next_section(&mut self) {
        let total = self.browser_items.len();
        // find the start of the next non-empty section after the current selection
        let next = self
            .browser_section_offsets
            .iter()
            .find(|&&o| o > self.browser_selection && o < total);
        if let Some(&o) = next {
            self.browser_selection = o;
        }
    }

    pub fn browser_prev_section(&mut self) {
        // find the start of the last non-empty section that begins before the current selection
        let prev = self
            .browser_section_offsets
            .iter()
            .filter(|&&o| o < self.browser_selection)
            .last();
        if let Some(&o) = prev {
            self.browser_selection = o;
        }
    }

    pub fn browser_activate(&mut self) {
        if let Some(item) = self.browser_items.get(self.browser_selection) {
            if let Some(focus) = item.navigable_focus().cloned() {
                self.navigate_to(focus);
            }
        }
    }

    pub fn types_select_up(&mut self) {
        if self.types_selection > 0 {
            self.types_selection -= 1;
        }
    }

    pub fn types_select_down(&mut self) {
        if self.types_selection + 1 < self.types_list.len() {
            self.types_selection += 1;
        }
    }

    pub fn types_activate(&mut self) {
        if let Some(uri) = self.types_list.get(self.types_selection).cloned() {
            self.navigate_to_node(uri);
        }
    }

    pub fn sparql_push_char(&mut self, c: char) {
        let idx = self.sparql_cursor;
        self.sparql_input.insert(idx, c);
        self.sparql_cursor += c.len_utf8();
    }

    pub fn sparql_backspace(&mut self) {
        if self.sparql_cursor > 0 {
            let idx = self.sparql_input[..self.sparql_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.sparql_input.remove(idx);
            self.sparql_cursor = idx;
        }
    }

    pub fn sparql_cursor_left(&mut self) {
        if self.sparql_cursor > 0 {
            let idx = self.sparql_input[..self.sparql_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.sparql_cursor = idx;
        }
    }

    pub fn sparql_cursor_right(&mut self) {
        if self.sparql_cursor < self.sparql_input.len() {
            let next = self.sparql_input[self.sparql_cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.sparql_cursor + i)
                .unwrap_or(self.sparql_input.len());
            self.sparql_cursor = next;
        }
    }

    pub fn sparql_clear(&mut self) {
        self.sparql_input.clear();
        self.sparql_cursor = 0;
        self.sparql_result = None;
        self.sparql_error = None;
        self.sparql_mode_input = true;
        self.status = String::new();
    }

    pub fn sparql_run(&mut self) {
        let q = self.sparql_input.trim().to_string();
        if q.is_empty() {
            return;
        }
        self.status = "Running query…".into();
        self.sparql_mode_input = false;
        match self.client.run_query(&q) {
            Ok(result) => {
                let count = result.rows.len();
                self.sparql_result = Some(SparqlResult {
                    variables: result.variables,
                    rows: result.rows,
                });
                self.sparql_error = None;
                self.sparql_selection = 0;
                self.status = format!("{} rows", count);
            }
            Err(e) => {
                self.sparql_result = None;
                self.sparql_error = Some(format!("{:#}", e));
                self.status = "Query error".into();
                self.sparql_mode_input = true;
            }
        }
    }

    pub fn sparql_result_up(&mut self) {
        if self.sparql_selection > 0 {
            self.sparql_selection -= 1;
        }
    }

    pub fn sparql_result_down(&mut self) {
        if let Some(r) = &self.sparql_result {
            if self.sparql_selection + 1 < r.rows.len() {
                self.sparql_selection += 1;
            }
        }
    }

    pub fn sparql_activate(&mut self) {
        if let Some(r) = &self.sparql_result {
            if let Some(row) = r.rows.get(self.sparql_selection) {
                let first_focus = row
                    .iter()
                    .find_map(|cell| cell.clone().and_then(|term| FocusTerm::try_from(term).ok()));
                if let Some(focus) = first_focus {
                    self.navigate_to(focus);
                }
            }
        }
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_input.insert(self.search_cursor, c);
        self.search_cursor += c.len_utf8();
    }

    pub fn search_backspace(&mut self) {
        if self.search_cursor > 0 {
            let idx = self.search_input[..self.search_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.search_input.remove(idx);
            self.search_cursor = idx;
        }
    }

    pub fn search_cursor_left(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor = self.search_input[..self.search_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn search_cursor_right(&mut self) {
        if self.search_cursor < self.search_input.len() {
            self.search_cursor = self.search_input[self.search_cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.search_cursor + i)
                .unwrap_or(self.search_input.len());
        }
    }

    pub fn search_run(&mut self) {
        let term = self.search_input.trim().to_string();
        if term.is_empty() {
            return;
        }
        self.status = "Searching…".into();
        self.search_mode_input = false;
        match self.client.search_resources(&term) {
            Ok(results) => {
                let count = results.len();
                let mut all_nodes: Vec<NamedNode> = Vec::new();
                for (s, p, _) in &results {
                    if let Some(n) = s.as_named_node() {
                        all_nodes.push(n.clone());
                    }
                    all_nodes.push(p.clone());
                }
                self.fetch_labels_for_nodes(&all_nodes);
                self.search_results = results
                    .into_iter()
                    .map(|(s, p, v)| SearchResult {
                        resource: s,
                        property: p,
                        matched_value: v,
                    })
                    .collect();
                self.search_selection = 0;
                self.status = format!("{} results", count);
            }
            Err(e) => {
                self.status = format!("Search error: {}", e);
                self.search_mode_input = true;
            }
        }
    }

    pub fn search_result_up(&mut self) {
        if self.search_selection > 0 {
            self.search_selection -= 1;
        }
    }

    pub fn search_result_down(&mut self) {
        if self.search_selection + 1 < self.search_results.len() {
            self.search_selection += 1;
        }
    }

    pub fn current_triple_sparql(&self) -> Option<String> {
        let d = self.browser_data.as_ref()?;
        let item = self.browser_items.get(self.browser_selection)?;
        let current = self.display.sparql_focus(&d.focus);
        let text = match item {
            BrowserItem::LiteralProp { prop, value } => format!(
                "{} {} {} .",
                current,
                self.display.sparql_node(prop),
                self.display.sparql_literal(value)
            ),
            BrowserItem::OutgoingLink { prop, target } => format!(
                "{} {} {} .",
                current,
                self.display.sparql_node(prop),
                self.display.sparql_focus(target)
            ),
            BrowserItem::IncomingLink { prop, source } => format!(
                "{} {} {} .",
                self.display.sparql_focus(source),
                self.display.sparql_node(prop),
                current
            ),
            BrowserItem::AsPredicateRow { subject, object } => format!(
                "{} {} {} .",
                self.display.sparql_focus(subject),
                current,
                self.display.sparql_term(object)
            ),
        };
        Some(text)
    }

    pub fn search_activate(&mut self) {
        if let Some(r) = self.search_results.get(self.search_selection).cloned() {
            self.navigate_to(r.resource);
        }
    }

    fn load_bookmarks() -> Vec<NamedNode> {
        config::load()
            .bookmarks
            .iter()
            .filter_map(|s| NamedNode::new(s).ok())
            .collect()
    }

    pub fn save_bookmarks(&self) {
        let cfg = config::Config {
            bookmarks: self
                .bookmarks
                .iter()
                .map(|n| n.as_str().to_string())
                .collect(),
        };
        config::save(&cfg);
    }

    pub fn is_bookmarked(&self, uri: &NamedNode) -> bool {
        self.bookmarks.iter().any(|b| b == uri)
    }

    pub fn toggle_bookmark(&mut self) {
        if let Some(d) = &self.browser_data {
            let Some(uri) = d.focus.as_named_node().cloned() else {
                self.status = "Only IRIs can be bookmarked".into();
                return;
            };
            if let Some(pos) = self.bookmarks.iter().position(|b| b == &uri) {
                self.bookmarks.remove(pos);
                self.status = "Bookmark removed".into();
            } else {
                self.bookmarks.push(uri);
                self.status = "Bookmarked".into();
            }
            self.save_bookmarks();
        }
    }

    pub fn bookmarks_select_up(&mut self) {
        if self.bookmarks_selection > 0 {
            self.bookmarks_selection -= 1;
        }
    }

    pub fn bookmarks_select_down(&mut self) {
        if self.bookmarks_selection + 1 < self.bookmarks.len() {
            self.bookmarks_selection += 1;
        }
    }

    pub fn bookmarks_activate(&mut self) {
        if let Some(uri) = self.bookmarks.get(self.bookmarks_selection).cloned() {
            self.navigate_to_node(uri);
        }
    }
}
