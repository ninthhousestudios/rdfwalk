use super::{QueryResult, SparqlBackend};
use anyhow::{Context, Result};
use oxrdf::Term;
use std::sync::Arc;

// ArcDataset wraps Arc<Dataset> and implements QueryableDataset so that
// execute() receives an owned value without cloning the actual quad data.
// Each run_query call clones the Arc (O(1) refcount bump), then delegates
// every dataset method through the Arc to Dataset's own implementation.
struct ArcDataset(Arc<oxrdf::Dataset>);

impl spareval::QueryableDataset for ArcDataset {
    type InternalTerm = Term;
    type Error = std::convert::Infallible;

    fn internal_quads_for_pattern(
        &self,
        subject: Option<&Term>,
        predicate: Option<&Term>,
        object: Option<&Term>,
        graph_name: Option<Option<&Term>>,
    ) -> Box<dyn Iterator<Item = Result<spareval::InternalQuad<Self>, std::convert::Infallible>>>
    {
        use spareval::InternalQuad;
        let quads: Vec<_> =
            <oxrdf::Dataset as spareval::QueryableDataset>::internal_quads_for_pattern(
                &self.0, subject, predicate, object, graph_name,
            )
            .map(|r| {
                r.map(|q| InternalQuad {
                    subject: q.subject,
                    predicate: q.predicate,
                    object: q.object,
                    graph_name: q.graph_name,
                })
            })
            .collect();
        Box::new(quads.into_iter())
    }

    fn internalize_term(&self, term: Term) -> Result<Term, std::convert::Infallible> {
        Ok(term)
    }

    fn externalize_term(&self, term: Term) -> Result<Term, std::convert::Infallible> {
        Ok(term)
    }
}

pub(super) struct LocalBackend {
    dataset: Arc<oxrdf::Dataset>,
}

impl LocalBackend {
    pub(super) fn from_file(path: &str) -> Result<Self> {
        use oxrdfio::{RdfFormat, RdfParser};
        let fmt = match std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
        {
            Some("nt" | "ntriples") => RdfFormat::NTriples,
            Some("ttl" | "turtle") => RdfFormat::Turtle,
            Some("n3") => RdfFormat::N3,
            Some("rdf" | "xml") => RdfFormat::RdfXml,
            ext => anyhow::bail!("unsupported RDF file extension: {:?}", ext),
        };
        let file = std::fs::File::open(path).with_context(|| format!("opening {path}"))?;
        let mut dataset = oxrdf::Dataset::new();
        for quad in RdfParser::from_format(fmt).for_reader(file) {
            dataset.insert(&quad.context("parsing RDF file")?);
        }
        Ok(Self {
            dataset: Arc::new(dataset),
        })
    }
}

impl SparqlBackend for LocalBackend {
    fn run_query(&self, sparql: &str) -> Result<QueryResult> {
        use spareval::{QueryEvaluator, QueryResults};
        use spargebra::Query;
        let query = Query::parse(sparql, None).context("parsing SPARQL query")?;
        let results = QueryEvaluator::new()
            .execute(ArcDataset(Arc::clone(&self.dataset)), &query)
            .context("query evaluation")?;
        match results {
            QueryResults::Solutions(solutions) => {
                let variables: Vec<String> = solutions
                    .variables()
                    .iter()
                    .map(|v| v.as_str().to_string())
                    .collect();
                let mut rows: Vec<Vec<Option<Term>>> = Vec::new();
                for sol in solutions {
                    let sol = sol.context("reading solution")?;
                    rows.push(sol.values().iter().cloned().collect());
                }
                Ok(QueryResult { variables, rows })
            }
            QueryResults::Boolean(_) | QueryResults::Graph(_) => Ok(QueryResult {
                variables: vec![],
                rows: vec![],
            }),
        }
    }
}

#[cfg(all(test, feature = "rdf-star"))]
mod tests {
    use super::*;
    use crate::app::model::FocusTerm;
    use crate::rdf::sparql::SparqlClient;
    use oxrdf::Term;

    #[test]
    fn turtle_star_quoted_triples_are_queryable_and_browsable() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "rdfwalk-rdf-star-{}-{}.ttl",
            std::process::id(),
            "quoted-triples"
        ));
        std::fs::write(
            &path,
            r#"@prefix ex: <http://ex/> .
ex:s ex:p ex:o .
<< ex:s ex:p ex:o >> ex:note "meta" .
ex:a ex:said << ex:s ex:p ex:o >> .
"#,
        )
        .context("writing Turtle-star fixture")?;

        let client = SparqlClient::local(path.to_str().unwrap())?;
        let result =
            client.run_query(r#"SELECT ?s WHERE { ?s <http://ex/note> "meta" . } LIMIT 1"#)?;
        let focus = result
            .rows
            .first()
            .and_then(|row| row.first())
            .and_then(|term| term.clone())
            .and_then(|term| FocusTerm::try_from(term).ok())
            .context("expected quoted triple subject")?;

        assert!(matches!(Term::from(focus.clone()), Term::Triple(_)));

        let literals = client.literal_properties(&focus)?;
        assert_eq!(literals.len(), 1);
        assert_eq!(literals[0].0.as_str(), "http://ex/note");
        assert_eq!(literals[0].1.value(), "meta");

        let incoming = client.incoming_links(&focus)?;
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].0.as_str(), "http://ex/said");
        assert!(matches!(&incoming[0].1, FocusTerm::NamedNode(n) if n.as_str() == "http://ex/a"));

        let _ = std::fs::remove_file(path);
        Ok(())
    }
}
