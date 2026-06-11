#[cfg(feature = "rdf-star")]
use oxrdf::Triple;
use oxrdf::{Literal, NamedNode, Subject, Term};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FocusTerm {
    NamedNode(NamedNode),
    #[cfg(feature = "rdf-star")]
    Triple(Box<Triple>),
}

impl FocusTerm {
    pub fn as_named_node(&self) -> Option<&NamedNode> {
        match self {
            Self::NamedNode(node) => Some(node),
            #[cfg(feature = "rdf-star")]
            Self::Triple(_) => None,
        }
    }
}

impl From<NamedNode> for FocusTerm {
    fn from(node: NamedNode) -> Self {
        Self::NamedNode(node)
    }
}

#[cfg(feature = "rdf-star")]
impl From<Box<Triple>> for FocusTerm {
    fn from(triple: Box<Triple>) -> Self {
        Self::Triple(triple)
    }
}

impl From<FocusTerm> for Term {
    fn from(resource: FocusTerm) -> Self {
        match resource {
            FocusTerm::NamedNode(node) => Term::NamedNode(node),
            #[cfg(feature = "rdf-star")]
            FocusTerm::Triple(triple) => Term::Triple(triple),
        }
    }
}

impl TryFrom<Term> for FocusTerm {
    type Error = ();

    fn try_from(term: Term) -> Result<Self, Self::Error> {
        match term {
            Term::NamedNode(node) => Ok(Self::NamedNode(node)),
            Term::BlankNode(_) | Term::Literal(_) => Err(()),
            #[cfg(feature = "rdf-star")]
            Term::Triple(triple) => Ok(Self::Triple(triple)),
        }
    }
}

impl From<FocusTerm> for Subject {
    fn from(resource: FocusTerm) -> Self {
        match resource {
            FocusTerm::NamedNode(node) => Subject::NamedNode(node),
            #[cfg(feature = "rdf-star")]
            FocusTerm::Triple(triple) => Subject::Triple(triple),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BrowserData {
    pub focus: FocusTerm,
}

// Flat list of navigable items in the browser view
#[derive(Debug, Clone)]
pub enum BrowserItem {
    LiteralProp { prop: NamedNode, value: Literal },
    OutgoingLink { prop: NamedNode, target: FocusTerm },
    IncomingLink { prop: NamedNode, source: FocusTerm },
    AsPredicateRow { subject: FocusTerm, object: Term },
}

impl BrowserItem {
    pub fn navigable_focus(&self) -> Option<&FocusTerm> {
        match self {
            Self::OutgoingLink { target, .. } => Some(target),
            Self::IncomingLink { source, .. } => Some(source),
            Self::AsPredicateRow { subject, .. } => Some(subject),
            BrowserItem::LiteralProp { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SparqlResult {
    pub variables: Vec<String>,
    pub rows: Vec<Vec<Option<Term>>>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub resource: FocusTerm,
    pub property: NamedNode,
    pub matched_value: String,
}
