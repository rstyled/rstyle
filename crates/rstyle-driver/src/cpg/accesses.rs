//! Who reads and writes each binding of a function: one record per node and symbol,
//! from what the tree resolves and what rustc reports of its places.

use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

/// The strongest thing a node does to a symbol. A read with a whole write at one node is a
/// write: naming a place to overwrite it does not read it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Mode {
    Read,
    Write,
    ReadWrite,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Read => "read",
            Mode::Write => "write",
            Mode::ReadWrite => "read_write",
        }
    }
}

/// What a function records that says who touches a binding.
pub struct Records<'a> {
    pub nodes: &'a [Value],
    pub symbols: &'a [Value],
    pub refs: &'a [Value],
    pub places: &'a [Value],
}

#[derive(Default)]
struct Accesses<'a>(BTreeMap<(&'a str, &'a str), Mode>);

impl<'a> Accesses<'a> {
    fn note(&mut self, node: Option<&'a str>, symbol: Option<&'a str>, mode: Mode) {
        if let (Some(node), Some(symbol)) = (node, symbol) {
            let strongest = self.0.entry((node, symbol)).or_insert(mode);
            *strongest = mode.max(*strongest);
        }
    }

    /// A binding is written where it is declared; a parameter's is written at entry.
    fn declarations(&mut self, symbols: &'a [Value]) {
        for symbol in symbols {
            self.note(symbol["node"].as_str(), symbol["id"].as_str(), Mode::Write);
        }
    }

    /// A name reads its symbol, and an alternative of an or-pattern writes the symbol its
    /// first alternative declares.
    fn references(&mut self, records: &Records<'a>, patterns: &HashSet<&'a str>) {
        for reference in records.refs {
            if reference["target"]["kind"] != "symbol" {
                continue;
            }
            let node = reference["node"].as_str();
            let mode = match node.is_some_and(|node| patterns.contains(node)) {
                true => Mode::Write,
                false => Mode::Read,
            };
            self.note(node, reference["target"]["id"].as_str(), mode);
        }
    }

    /// A closure holds what it captures: mutably, or to read it.
    fn captures(&mut self, nodes: &'a [Value]) {
        for node in nodes {
            for capture in node["captures"].as_array().into_iter().flatten() {
                let mode = match capture["mode"].as_str() {
                    Some("exclusive" | "unique") => Mode::ReadWrite,
                    _ => Mode::Read,
                };
                self.note(node["id"].as_str(), capture["symbol"].as_str(), mode);
            }
        }
    }

    /// A place written whole is a write. Part of one, an exclusive borrow, or a compound
    /// assignment reads what it keeps.
    fn places(&mut self, places: &'a [Value], compound_targets: &HashSet<&'a str>) {
        for place in places {
            let node = place["node"].as_str();
            let whole = place["path"].as_array().is_some_and(Vec::is_empty);
            let written = place["access"] == "write";
            let keeps = node.is_some_and(|node| compound_targets.contains(node));
            let mode = match written && whole && !keeps {
                true => Mode::Write,
                false => Mode::ReadWrite,
            };
            self.note(node, place["root"]["symbol"].as_str(), mode);
        }
    }
}

/// The nodes that are patterns, and the targets of compound assignments.
fn roles(nodes: &[Value]) -> (HashSet<&str>, HashSet<&str>) {
    let (mut patterns, mut compound_targets) = (HashSet::new(), HashSet::new());
    for node in nodes {
        let kind = node["kind"].as_str().unwrap_or_default();
        if kind.starts_with("pattern_") {
            patterns.extend(node["id"].as_str());
        } else if kind == "compound_assignment" {
            compound_targets.extend(node["children"][0].as_str());
        }
    }
    (patterns, compound_targets)
}

// rstyle-allow(mutation_per_target): collects each kind of access in turn
/// The function's access records, each node's and symbol's once.
pub fn accesses(records: &Records<'_>) -> Vec<Value> {
    let (patterns, compound_targets) = roles(records.nodes);
    let mut found = Accesses::default();
    found.declarations(records.symbols);
    found.references(records, &patterns);
    found.captures(records.nodes);
    found.places(records.places, &compound_targets);

    found
        .0
        .into_iter()
        .map(|((node, symbol), mode)| json!({"node": node, "symbol": symbol, "mode": mode.name()}))
        .collect()
}
