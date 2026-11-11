//! The Rust frontend's node and symbol IDs. They begin with a prefix taken from the function's
//! stable identity, so a function's IDs stay the same as items are added and removed around it.

use super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const KEPT: &str = "pub fn kept(items: &[u32]) -> u32 {\n    let mut total = 0;\n    for item in items {\n        total += *item;\n    }\n    total\n}\n";

/// Two paths whose sha256 digests share their first 8 hex digits, `4102858f`.
const CLASHING: [&str; 2] = ["clash_5072", "clash_52422"];

/// The functions `cpg emit` writes for a crate named `ids` whose library is `source`, by
/// stable identity.
fn functions(source: &str) -> BTreeMap<String, Value> {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, text: &str| {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "Cargo.toml",
        "[package]\nname = \"ids\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
    );
    write("src/lib.rs", source);
    let out = dir.path().join("cpg");
    let emit = rstyle(
        dir.path(),
        &["rstyle", "cpg", "emit", "--out", out.to_str().unwrap()],
    );
    assert_eq!(emit.status.code(), Some(0), "{emit:?}");

    let mut functions = BTreeMap::new();
    for document in parsed(&documents_in(&out)) {
        let unit = &document["unit"];
        let identity = |definition: &Value| {
            unit["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|record| record["id"] == *definition)
                .map(|record| record["stable_identity"].as_str().unwrap().to_owned())
                .unwrap()
        };
        for function in unit["functions"].as_array().unwrap() {
            functions.insert(identity(&function["definition"]), function.clone());
        }
    }
    functions
}

/// A function's root and the IDs of its nodes and symbols, in order.
fn ids(function: &Value) -> Vec<String> {
    let listed = |key: &str| {
        function[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    let mut ids = vec![function["root"].as_str().unwrap().to_owned()];
    ids.extend(listed("nodes"));
    ids.extend(listed("symbols"));
    ids
}

fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn a_function_keeps_its_ids_when_items_are_added_before_it() {
    let before = functions(KEPT);
    let after = functions(&format!(
        "pub struct Added;\n\npub fn first() -> u32 {{\n    1\n}}\n\n{KEPT}"
    ));
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(ids(&after["ids::kept"]), ids(&before["ids::kept"]));
}

#[test]
fn ids_begin_with_the_sha256_of_the_function_identity() {
    let source: String = CLASHING
        .iter()
        .map(|name| format!("pub fn {name}(value: u32) -> u32 {{\n    value\n}}\n\n"))
        .chain([KEPT.to_owned()])
        .collect();
    let functions = functions(&source);
    let clashing: Vec<String> = CLASHING.iter().map(|name| format!("ids::{name}")).collect();
    assert_eq!(
        sha256(&clashing[0])[..8],
        sha256(&clashing[1])[..8],
        "the fixture's paths share their first 8 digits"
    );

    for (identity, function) in &functions {
        let digest = sha256(identity);
        // Functions sharing the first 8 digits take all 64.
        let digits = match clashing.contains(identity) {
            true => &digest[..],
            false => &digest[..8],
        };
        let prefix = format!("f{digits}.");
        let ids = ids(function);
        let (symbols, nodes): (Vec<&String>, Vec<&String>) =
            ids.iter().partition(|id| id.starts_with('s'));
        assert!(!nodes.is_empty() && !symbols.is_empty(), "{identity}");
        for id in nodes {
            assert!(id.starts_with(&prefix), "{identity}: node {id}");
        }
        for id in symbols {
            assert!(
                id.starts_with(&format!("s{prefix}")),
                "{identity}: symbol {id}"
            );
        }
    }
}
