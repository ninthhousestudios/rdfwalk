use crate::app::model::FocusTerm;
#[cfg(feature = "rdf-star")]
use oxrdf::Subject;
use oxrdf::{Literal, NamedNode, Term};
use std::collections::HashMap;

pub struct DisplayContext {
    prefixes: HashMap<String, String>, // prefix -> namespace IRI
    label_cache: HashMap<String, Option<String>>, // IRI -> label
}

impl DisplayContext {
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();
        prefixes.insert(
            "rdf".into(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
        );
        prefixes.insert(
            "rdfs".into(),
            "http://www.w3.org/2000/01/rdf-schema#".into(),
        );
        prefixes.insert("owl".into(), "http://www.w3.org/2002/07/owl#".into());
        prefixes.insert("xsd".into(), "http://www.w3.org/2001/XMLSchema#".into());
        prefixes.insert("skos".into(), "http://www.w3.org/2004/02/skos/core#".into());
        prefixes.insert("dc".into(), "http://purl.org/dc/elements/1.1/".into());
        prefixes.insert("dct".into(), "http://purl.org/dc/terms/".into());
        prefixes.insert("foaf".into(), "http://xmlns.com/foaf/0.1/".into());
        prefixes.insert("schema".into(), "https://schema.org/".into());
        Self {
            prefixes,
            label_cache: HashMap::new(),
        }
    }

    pub fn cache_label(&mut self, iri: &str, label: Option<String>) {
        self.label_cache.insert(iri.to_string(), label);
    }

    pub fn display_node(&self, node: &NamedNode) -> String {
        let iri = node.as_str();
        if let Some(Some(label)) = self.label_cache.get(iri) {
            return label.clone();
        }
        self.shorten_iri(iri)
    }

    pub fn display_term(&self, term: &Term) -> String {
        match term {
            Term::NamedNode(n) => self.display_node(n),
            Term::BlankNode(b) => format!("_:{}", b.as_str()),
            Term::Literal(l) => self.display_literal(l),
            #[cfg(feature = "rdf-star")]
            Term::Triple(t) => self.display_triple(t),
        }
    }

    pub fn display_term_plain(&self, term: &oxrdf::Term) -> String {
        match term {
            oxrdf::Term::NamedNode(n) => self.display_node(n),
            oxrdf::Term::BlankNode(b) => format!("_:{}", b.as_str()),
            oxrdf::Term::Literal(l) => {
                let (v, suffix) = self.display_literal_parts(l, 60);
                match suffix {
                    Some(s) => format!("{} {}", v, s),
                    None => v,
                }
            }
            #[cfg(feature = "rdf-star")]
            oxrdf::Term::Triple(t) => self.display_triple(t),
        }
    }

    pub fn display_focus(&self, focus: &FocusTerm) -> String {
        match focus {
            FocusTerm::NamedNode(n) => self.display_node(n),
            #[cfg(feature = "rdf-star")]
            FocusTerm::Triple(t) => self.display_triple(t),
        }
    }

    pub fn display_literal(&self, lit: &Literal) -> String {
        let (value, suffix) = self.literal_parts(lit);
        match suffix {
            Some(s) => format!("{} {}", value, s),
            None => value,
        }
    }

    /// Returns `(value, Option<type_or_lang_suffix>)` with the value sanitized and
    /// truncated to fit `avail_chars` (accounting for the suffix width).
    pub fn display_literal_parts(
        &self,
        lit: &Literal,
        avail_chars: usize,
    ) -> (String, Option<String>) {
        let (value, suffix) = self.literal_parts(lit);
        let suffix_len = suffix.as_ref().map(|s| s.chars().count() + 1).unwrap_or(0);
        let value_max = avail_chars.saturating_sub(suffix_len);
        (crate::util::string::truncate(value, value_max), suffix)
    }

    pub fn literal_parts(&self, lit: &Literal) -> (String, Option<String>) {
        let value = crate::util::string::sanitize(lit.value());
        if let Some(lang) = lit.language() {
            (value, Some(format!("@{}", lang)))
        } else {
            let dt = lit.datatype();
            if dt.as_str() == "http://www.w3.org/2001/XMLSchema#string" {
                (value, None)
            } else {
                (value, Some(format!("^^{}", self.shorten_iri(dt.as_str()))))
            }
        }
    }

    /// Format a URI for SPARQL: prefix:local or <full-uri>. Never uses rdfs:label.
    pub fn sparql_node(&self, node: &NamedNode) -> String {
        self.shorten_iri(node.as_str())
    }

    /// Format a literal for SPARQL: "value"^^type or "value"@lang, with proper escaping.
    pub fn sparql_literal(&self, lit: &Literal) -> String {
        let escaped = lit
            .value()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        if let Some(lang) = lit.language() {
            format!("\"{}\"@{}", escaped, lang)
        } else {
            let dt = lit.datatype();
            if dt.as_str() == "http://www.w3.org/2001/XMLSchema#string" {
                format!("\"{}\"", escaped)
            } else {
                format!("\"{}\"^^{}", escaped, self.shorten_iri(dt.as_str()))
            }
        }
    }

    pub fn sparql_term(&self, term: &Term) -> String {
        match term {
            Term::NamedNode(n) => self.sparql_node(n),
            Term::BlankNode(b) => format!("_:{}", b.as_str()),
            Term::Literal(l) => self.sparql_literal(l),
            #[cfg(feature = "rdf-star")]
            Term::Triple(t) => self.sparql_triple(t),
        }
    }

    #[cfg(feature = "rdf-star")]
    pub fn sparql_subject(&self, subject: &Subject) -> String {
        match subject {
            Subject::NamedNode(n) => self.sparql_node(n),
            Subject::BlankNode(b) => format!("_:{}", b.as_str()),
            #[cfg(feature = "rdf-star")]
            Subject::Triple(t) => self.sparql_triple(t),
        }
    }

    pub fn sparql_focus(&self, focus: &FocusTerm) -> String {
        match focus {
            FocusTerm::NamedNode(n) => self.sparql_node(n),
            #[cfg(feature = "rdf-star")]
            FocusTerm::Triple(t) => self.sparql_triple(t),
        }
    }

    #[cfg(feature = "rdf-star")]
    fn display_triple(&self, triple: &oxrdf::Triple) -> String {
        format!(
            "<< {} {} {} >>",
            self.display_subject(&triple.subject),
            self.display_node(&triple.predicate),
            self.display_term(&triple.object)
        )
    }

    #[cfg(feature = "rdf-star")]
    fn display_subject(&self, subject: &Subject) -> String {
        match subject {
            Subject::NamedNode(n) => self.display_node(n),
            Subject::BlankNode(b) => format!("_:{}", b.as_str()),
            Subject::Triple(t) => self.display_triple(t),
        }
    }

    #[cfg(feature = "rdf-star")]
    fn sparql_triple(&self, triple: &oxrdf::Triple) -> String {
        format!(
            "<< {} {} {} >>",
            self.sparql_subject(&triple.subject),
            self.sparql_node(&triple.predicate),
            self.sparql_term(&triple.object)
        )
    }

    fn shorten_iri(&self, iri: &str) -> String {
        for (prefix, ns) in &self.prefixes {
            if let Some(local) = iri.strip_prefix(ns.as_str()) {
                if !local.is_empty() {
                    return format!("{}:{}", prefix, local);
                }
            }
        }
        format!("<{}>", iri)
    }
}
