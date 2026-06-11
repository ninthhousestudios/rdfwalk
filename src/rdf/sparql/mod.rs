use crate::app::model::FocusTerm;
use anyhow::Result;
#[cfg(feature = "rdf-star")]
use oxrdf::Subject;
use oxrdf::{Literal, NamedNode, Term};

#[cfg(feature = "local")]
mod local;
mod remote;

pub struct QueryResult {
    pub variables: Vec<String>,
    pub rows: Vec<Vec<Option<Term>>>,
}

impl QueryResult {
    fn get_var<'a>(&self, row: &'a [Option<Term>], var: &str) -> Option<&'a Term> {
        let idx = self.variables.iter().position(|v| v == var)?;
        row.get(idx)?.as_ref()
    }
}

// Backend Trait
pub trait SparqlBackend: Send + Sync {
    fn run_query(&self, sparql: &str) -> Result<QueryResult>;
}

// Public facade
pub struct SparqlClient {
    backend: Box<dyn SparqlBackend>,
    limit: usize,
}

impl SparqlClient {
    pub fn remote(endpoint: String) -> Self {
        Self {
            backend: Box::new(remote::RemoteBackend::new(endpoint)),
            limit: 1000,
        }
    }

    #[cfg(feature = "local")]
    pub fn local(path: &str) -> Result<Self> {
        Ok(Self {
            backend: Box::new(local::LocalBackend::from_file(path)?),
            limit: 1000,
        })
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn literal_properties(&self, focus: &FocusTerm) -> Result<Vec<(NamedNode, Literal)>> {
        let limit = self.limit;
        let current = sparql_focus(focus);
        let q = format!(
            "SELECT ?p ?o WHERE {{ {current} ?p ?o . FILTER(isLiteral(?o)) }} LIMIT {limit}"
        );
        let result = self.backend.run_query(&q)?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                match (
                    result.get_var(row, "p").cloned(),
                    result.get_var(row, "o").cloned(),
                ) {
                    (Some(Term::NamedNode(p)), Some(Term::Literal(o))) => Some((p, o)),
                    _ => None,
                }
            })
            .collect())
    }

    pub fn outgoing_links(&self, focus: &FocusTerm) -> Result<Vec<(NamedNode, FocusTerm)>> {
        let limit = self.limit;
        let current = sparql_focus(focus);
        let q = format!(
            "SELECT ?p ?o WHERE {{ {current} ?p ?o . FILTER(!isLiteral(?o)) }} LIMIT {limit}"
        );
        let result = self.backend.run_query(&q)?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                match (
                    result.get_var(row, "p").cloned(),
                    result.get_var(row, "o").cloned(),
                ) {
                    (Some(Term::NamedNode(p)), Some(o)) => {
                        FocusTerm::try_from(o).ok().map(|o| (p, o))
                    }
                    _ => None,
                }
            })
            .collect())
    }

    pub fn incoming_links(&self, focus: &FocusTerm) -> Result<Vec<(NamedNode, FocusTerm)>> {
        let limit = self.limit;
        let current = sparql_focus(focus);
        let q = format!("SELECT ?s ?p WHERE {{ ?s ?p {current} . }} LIMIT {limit}");
        let result = self.backend.run_query(&q)?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                match (
                    result.get_var(row, "s").cloned(),
                    result.get_var(row, "p").cloned(),
                ) {
                    (Some(s), Some(Term::NamedNode(p))) => {
                        FocusTerm::try_from(s).ok().map(|s| (p, s))
                    }
                    _ => None,
                }
            })
            .collect())
    }

    pub fn as_predicate(&self, focus: &FocusTerm) -> Result<Vec<(FocusTerm, Term)>> {
        let Some(predicate) = focus.as_named_node() else {
            return Ok(Vec::new());
        };
        let limit = self.limit;
        let q = format!(
            "SELECT ?s ?o WHERE {{ ?s <{}> ?o . }} LIMIT {limit}",
            predicate.as_str()
        );
        let result = self.backend.run_query(&q)?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                match (
                    result.get_var(row, "s").cloned(),
                    result.get_var(row, "o").cloned(),
                ) {
                    (Some(s), Some(o)) => FocusTerm::try_from(s).ok().map(|s| (s, o)),
                    _ => None,
                }
            })
            .collect())
    }

    pub fn all_types(&self) -> Result<Vec<NamedNode>> {
        let limit = self.limit;
        let q = format!(
            "SELECT DISTINCT ?x WHERE {{ \
             ?s <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?x . \
             FILTER(isIRI(?x)) \
             }} ORDER BY ?x LIMIT {limit}"
        );
        let result = self.backend.run_query(&q)?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| match result.get_var(row, "x").cloned() {
                Some(Term::NamedNode(n)) => Some(n),
                _ => None,
            })
            .collect())
    }

    pub fn label_for(&self, uri: &NamedNode) -> Result<Option<String>> {
        let q = format!(
            "SELECT ?l WHERE {{ \
             <{}> <http://www.w3.org/2000/01/rdf-schema#label> ?l \
             }} LIMIT 1",
            uri.as_str()
        );
        let result = self.backend.run_query(&q)?;
        Ok(result
            .rows
            .iter()
            .find_map(|row| match result.get_var(row, "l").cloned() {
                Some(Term::Literal(l)) => Some(l.value().to_string()),
                _ => None,
            }))
    }

    pub fn run_query(&self, sparql: &str) -> Result<QueryResult> {
        self.backend.run_query(sparql)
    }

    pub fn search_resources(&self, term: &str) -> Result<Vec<(FocusTerm, NamedNode, String)>> {
        let limit = self.limit;
        let escaped = term.replace('\\', "\\\\").replace('"', "\\\"");
        let q = format!(
            "SELECT DISTINCT ?s ?p ?o WHERE {{ \
             ?s ?p ?o . \
             FILTER(isLiteral(?o) && CONTAINS(LCASE(STR(?o)), LCASE(\"{escaped}\"))) \
             }} LIMIT {limit}"
        );
        let result = self.backend.run_query(&q)?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                match (
                    result.get_var(row, "s").cloned(),
                    result.get_var(row, "p").cloned(),
                    result.get_var(row, "o").cloned(),
                ) {
                    (Some(s), Some(Term::NamedNode(p)), Some(Term::Literal(o))) => {
                        FocusTerm::try_from(s)
                            .ok()
                            .map(|s| (s, p, o.value().to_string()))
                    }
                    _ => None,
                }
            })
            .collect())
    }
}

fn sparql_focus(focus: &FocusTerm) -> String {
    match focus {
        FocusTerm::NamedNode(n) => sparql_node(n),
        #[cfg(feature = "rdf-star")]
        FocusTerm::Triple(t) => sparql_triple(t),
    }
}

#[cfg(feature = "rdf-star")]
fn sparql_subject(subject: &Subject) -> String {
    match subject {
        Subject::NamedNode(n) => sparql_node(n),
        Subject::BlankNode(b) => format!("_:{}", b.as_str()),
        #[cfg(feature = "rdf-star")]
        Subject::Triple(t) => sparql_triple(t),
    }
}

fn sparql_node(node: &NamedNode) -> String {
    format!("<{}>", node.as_str())
}

#[cfg(feature = "rdf-star")]
fn sparql_literal(lit: &Literal) -> String {
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
            format!("\"{}\"^^<{}>", escaped, dt.as_str())
        }
    }
}

#[cfg(feature = "rdf-star")]
fn sparql_term(term: &Term) -> String {
    match term {
        Term::NamedNode(n) => sparql_node(n),
        Term::BlankNode(b) => format!("_:{}", b.as_str()),
        Term::Literal(l) => sparql_literal(l),
        #[cfg(feature = "rdf-star")]
        Term::Triple(t) => sparql_triple(t),
    }
}

#[cfg(feature = "rdf-star")]
fn sparql_triple(triple: &oxrdf::Triple) -> String {
    format!(
        "<< {} {} {} >>",
        sparql_subject(&triple.subject),
        sparql_node(&triple.predicate),
        sparql_term(&triple.object)
    )
}
