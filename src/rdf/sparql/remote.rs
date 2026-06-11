use super::{QueryResult, SparqlBackend};
use anyhow::{Context, Result};
use oxrdf::Term;
use sparesults::{QueryResultsFormat, QueryResultsParser, ReaderQueryResultsParserOutput};

pub(super) struct RemoteBackend {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl RemoteBackend {
    pub(super) fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn query_raw(&self, sparql: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(&self.endpoint)
            .header("Accept", "application/sparql-results+xml")
            .query(&[("query", sparql)])
            .send()
            .context("HTTP request failed")?;
        let status = response.status();
        let bytes = response.bytes().context("reading response body")?;
        if !status.is_success() {
            let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(300)]);
            anyhow::bail!("HTTP {} — {}", status, preview.trim());
        }
        Ok(bytes.to_vec())
    }
}

impl SparqlBackend for RemoteBackend {
    fn run_query(&self, sparql: &str) -> Result<QueryResult> {
        let bytes = self.query_raw(sparql)?;
        let parser = QueryResultsParser::from_format(QueryResultsFormat::Xml);
        let output = parser.for_reader(bytes.as_slice()).with_context(|| {
            let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]);
            format!("parsing SPARQL XML results (got: {})", preview.trim())
        })?;
        match output {
            ReaderQueryResultsParserOutput::Solutions(solutions) => {
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
            ReaderQueryResultsParserOutput::Boolean(_) => Ok(QueryResult {
                variables: vec![],
                rows: vec![],
            }),
        }
    }
}
