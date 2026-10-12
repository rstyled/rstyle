use crate::{NodeCategory, frontend::ByteSpan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Atom {
    pub controller_pdg_node_id: String,
    pub outcome: String,
}

pub type Guard = Vec<Vec<Atom>>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub category: NodeCategory,
    pub span: ByteSpan,
    pub construct_span: ByteSpan,
    pub original_kind: String,
    pub role: String,
    pub ty: String,
    pub symbol: Option<String>,
    pub definition: Option<String>,
    pub operator: Option<String>,
    pub literal: Option<String>,
    pub construct: Option<String>,
    pub write_form: Option<String>,
    pub return_form: Option<String>,
    pub position: Option<u64>,
    pub operands: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Transfer {
    pub source: String,
    pub target: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alternative {
    pub value_pdg_node_id: String,
    pub outcome: String,
    pub completion_cfg_node_ids: Vec<String>,
    pub guards: Guard,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueSite {
    pub pdg_node_id: String,
    pub construct_span: ByteSpan,
    pub controller_pdg_node_id: String,
    pub cfg_successor_node_id: Option<String>,
    pub guards: Guard,
    pub alternatives: Vec<Alternative>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<Operation>,
    pub transfers: Vec<Transfer>,
    pub value_sites: Vec<ValueSite>,
}

pub fn normalize(terms: Guard) -> Guard {
    let mut terms: std::collections::BTreeSet<Vec<Atom>> = terms
        .into_iter()
        .map(|mut term| {
            term.sort();
            term.dedup();
            term
        })
        .collect();

    loop {
        let next = simplify(&terms);
        if next == terms {
            return next.into_iter().collect();
        }
        terms = next;
    }
}

fn simplify(
    terms: &std::collections::BTreeSet<Vec<Atom>>,
) -> std::collections::BTreeSet<Vec<Atom>> {
    let smaller = terms.iter().filter(|term| {
        !terms
            .iter()
            .any(|other| other.len() < term.len() && other.iter().all(|atom| term.contains(atom)))
    });
    let combined = terms
        .iter()
        .flat_map(|left| terms.iter().filter_map(move |right| combine(left, right)));

    smaller.cloned().chain(combined).collect()
}

fn combine(left: &[Atom], right: &[Atom]) -> Option<Vec<Atom>> {
    if left.len() != right.len() {
        return None;
    }
    let difference: Vec<_> = left.iter().zip(right).filter(|(a, b)| a != b).collect();
    let [(a, b)] = difference.as_slice() else {
        return None;
    };
    if a.controller_pdg_node_id != b.controller_pdg_node_id {
        return None;
    }

    Some(left.iter().filter(|atom| *atom != *a).cloned().collect())
}
