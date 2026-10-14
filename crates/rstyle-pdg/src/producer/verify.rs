//! Producer handoff checks using captured source/build evidence and the real
//! interchange boundary. Producer conformance flags are never an input proof.

use super::{Pair, analyze_facts, emit, lower};
use crate::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn verify(pair: &Pair, facts: &HirFnFacts) -> Result<(), Vec<String>> {
    let digest = format!("sha256:{:x}", Sha256::digest(facts.source.bytes.as_bytes()));
    if digest != facts.source.digest {
        return Err(vec!["Captured source identity is inconsistent".to_string()]);
    }
    let graph = lower(facts).map_err(|e| {
        e.diagnostics
            .iter()
            .map(|d| d.reason.clone())
            .collect::<Vec<_>>()
    })?;
    let analysis = analyze_facts(&graph, facts).map_err(|e| vec![e])?;
    let revision = facts
        .build
        .producer_revision
        .known_value()
        .ok_or_else(|| vec!["Missing captured producer revision".to_string()])?;
    let expected = emit(facts, &graph, &analysis, revision).map_err(|e| vec![e])?;
    let renaming = correspondence(pair, &expected)?;
    compare_documents(pair, &expected, &renaming)?;

    if crate::extraction_profile::ExtractionProfile::from_facts(facts)
        .is_ok_and(|profile| profile.supports("rust.verified-helper-calls"))
    {
        crate::helper_calls::verify_documents(pair, facts).map_err(|error| vec![error])?;
    }

    if crate::extraction_profile::ExtractionProfile::from_facts(facts)
        .is_ok_and(|profile| profile.supports("rust.concrete-read-only-data"))
    {
        crate::data_documents::verify(pair, facts).map_err(|error| vec![error])?;
    }
    validate_boundary(pair, facts)
}

fn validate_boundary(pair: &Pair, facts: &HirFnFacts) -> Result<(), Vec<String>> {
    let bytes = pair
        .pdg
        .sources
        .iter()
        .map(|s| (s.id.clone(), facts.source.bytes.as_bytes().to_vec()))
        .collect();
    let width = facts
        .build
        .pointer_width
        .value
        .and_then(|width| u8::try_from(width).ok())
        .ok_or_else(|| vec!["Missing actual target width".to_string()])?;
    let context = ValidationContext::CompilerEvidence {
        pointer_width_bits: width,
    };
    let report = validate(&pair.pdg, &context, &bytes);
    let structural = validate_cfg(&pair.cfg)
        .into_iter()
        .chain(validate_paired_agreement(
            &pair.pdg.extensions,
            &pair.cfg.extensions,
        ))
        .map(|i| i.message);
    let map = read_operation_map(&pair.pdg.extensions).map_err(|e| vec![e])?;
    let node_ids = pair.pdg.nodes.iter().map(|n| n.id.clone()).collect();
    let correspondence = validate_correspondence(&map, &pair.cfg, &node_ids)
        .into_iter()
        .map(|i| i.message);
    let values = validate_value_sites(&pair.pdg, &pair.cfg)
        .err()
        .unwrap_or_default();
    let errors: Vec<_> = report
        .issues
        .into_iter()
        .map(|i| i.message)
        .chain(structural)
        .chain(correspondence)
        .chain(values)
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn correspondence(actual: &Pair, expected: &Pair) -> Result<BTreeMap<String, String>, Vec<String>> {
    let mut names = node_names(actual, expected)?;
    add_binding_names(actual, expected, &mut names)?;
    add_cfg_names(actual, expected, &mut names)?;
    if actual.pdg.sources.len() != expected.pdg.sources.len() {
        return Err(vec!["Source inventory differs".to_string()]);
    }
    for source in &actual.pdg.sources {
        let expected = expected
            .pdg
            .sources
            .iter()
            .find(|s| s.content_identity == source.content_identity)
            .ok_or_else(|| vec!["Source identity mismatch".to_string()])?;
        names.insert(source.id.clone(), expected.id.clone());
    }

    Ok(names)
}

fn add_binding_names(
    actual: &Pair,
    expected: &Pair,
    names: &mut BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    add_symbols(actual, expected, names)?;
    add_definitions(actual, expected, names)
}

fn node_names(actual: &Pair, expected: &Pair) -> Result<BTreeMap<String, String>, Vec<String>> {
    if actual.pdg.nodes.len() != expected.pdg.nodes.len() {
        return Err(vec!["Source/value operation inventory differs".to_string()]);
    }
    let mut names = BTreeMap::new();
    for node in &actual.pdg.nodes {
        let candidates: Vec<_> = expected
            .pdg
            .nodes
            .iter()
            .filter(|e| {
                node.category == e.category
                    && node.source_span.as_ref().map(|s| (s.start, s.end))
                        == e.source_span.as_ref().map(|s| (s.start, s.end))
                    && node.original_kind == e.original_kind
                    && node.attributes.position == e.attributes.position
                    && node
                        .attributes
                        .extensions
                        .get("compat.rust.hir-r0:source-operation")
                        == e.attributes
                            .extensions
                            .get("compat.rust.hir-r0:source-operation")
            })
            .collect();
        let [candidate] = candidates.as_slice() else {
            return Err(vec![format!(
                "No unique source-backed correspondence for node {}",
                node.id
            )]);
        };
        if names.values().any(|id| id == &candidate.id) {
            return Err(vec![
                "Two nodes claim the same source operation".to_string(),
            ]);
        }
        names.insert(node.id.clone(), candidate.id.clone());
    }

    Ok(names)
}

fn add_symbols(
    actual: &Pair,
    expected: &Pair,
    names: &mut BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    for symbol in &actual.pdg.symbols {
        let candidates: Vec<_> = expected
            .pdg
            .symbols
            .iter()
            .filter(|e| {
                e.scope_id == symbol.scope_id
                    && e.extensions.get("compat.rust.hir-r0:binding-key")
                        == symbol.extensions.get("compat.rust.hir-r0:binding-key")
            })
            .collect();
        let [candidate] = candidates.as_slice() else {
            return Err(vec!["Binding token/scope evidence mismatch".to_string()]);
        };
        names.insert(symbol.id.clone(), candidate.id.clone());
    }

    Ok(())
}

fn add_definitions(
    actual: &Pair,
    expected: &Pair,
    names: &mut BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    for definition in &actual.pdg.definitions {
        let candidate = expected
            .pdg
            .definitions
            .iter()
            .find(|e| names.get(&definition.node_id) == Some(&e.node_id))
            .ok_or_else(|| vec!["Definition source operation mismatch".to_string()])?;
        names.insert(definition.id.clone(), candidate.id.clone());
    }

    Ok(())
}

fn add_cfg_names(
    actual: &Pair,
    expected: &Pair,
    names: &mut BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    let actual_map = read_operation_map(&actual.pdg.extensions).map_err(|e| vec![e])?;
    let expected_map = read_operation_map(&expected.pdg.extensions).map_err(|e| vec![e])?;
    for entry in actual_map {
        let expected = expected_map
            .iter()
            .find(|e| names.get(&entry.pdg_node_id) == Some(&e.pdg_node_id))
            .ok_or_else(|| vec!["CFG operation correspondence mismatch".to_string()])?;
        names.extend([
            (entry.cfg_node_id, expected.cfg_node_id.clone()),
            (entry.cfg_operation_id, expected.cfg_operation_id.clone()),
        ]);
    }
    names.insert(
        actual.cfg.entry_node_id.clone(),
        expected.cfg.entry_node_id.clone(),
    );
    if actual.cfg.exit_node_ids.len() != 1 {
        return Err(vec!["R0 has exactly one normal exit".to_string()]);
    }
    names.insert(
        actual.cfg.exit_node_ids[0].clone(),
        expected.cfg.exit_node_ids[0].clone(),
    );

    Ok(())
}

fn compare_documents(
    actual: &Pair,
    expected: &Pair,
    renaming: &BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    let actual_docs = [
        serde_json::to_value(&actual.pdg),
        serde_json::to_value(&actual.cfg),
    ];
    let expected_docs = [
        serde_json::to_value(&expected.pdg),
        serde_json::to_value(&expected.cfg),
    ];
    let empty = BTreeMap::new();
    for (actual, expected) in actual_docs.into_iter().zip(expected_docs) {
        let actual = normalize(actual.map_err(|e| vec![e.to_string()])?, "", renaming);
        let expected = normalize(expected.map_err(|e| vec![e.to_string()])?, "", &empty);
        if actual != expected {
            return Err(vec![format!(
                "Serialized extraction facts disagree with captured source/build/normal-flow evidence: {}",
                first_difference(&actual, &expected, "")
            )]);
        }
    }

    Ok(())
}

fn normalize(value: Value, field: &str, renaming: &BTreeMap<String, String>) -> Value {
    match value {
        Value::Object(object) => {
            let entries = object
                .into_iter()
                .filter(|(key, _)| {
                    !(field == "conformance" && matches!(key.as_str(), "status" | "reasons"))
                })
                .filter(|(key, _)| !(matches!(field, "edges" | "diagnostics") && key == "id"))
                // Historical selection records carried an informational document
                // path. The registered profile identity remains authoritative.
                .filter(|(key, _)| {
                    !(field == crate::extraction_profile::EVIDENCE_KEY && key == "contract")
                })
                .map(|(key, value)| {
                    let value = normalize(value, &key, renaming);
                    (key, value)
                })
                .collect();
            Value::Object(entries)
        }
        Value::Array(values) => {
            let mut values: Vec<_> = values
                .into_iter()
                .map(|v| normalize(v, field, renaming))
                .collect();
            if unordered(field) {
                values.sort_by_key(Value::to_string);
            }
            Value::Array(values)
        }
        Value::String(value) if reference_field(field) => {
            Value::String(renaming.get(&value).cloned().unwrap_or(value))
        }
        value => value,
    }
}

fn unordered(field: &str) -> bool {
    matches!(
        field,
        "nodes"
            | "edges"
            | "symbols"
            | "definitions"
            | "sources"
            | "diagnostics"
            | "alternatives"
            | "memory_locations"
            | "entry_reachable_cfg_node_ids"
            | "completion_cfg_node_ids"
            | "compat.rust.hir-r0:operation-map"
            | "compat.rust.hir-r0:operation-provenance"
            | "compat.rust.hir-r0:value-sites"
            | "compat.rust.hir-r0:guards"
    )
}

fn reference_field(field: &str) -> bool {
    matches!(
        field,
        "id" | "source"
            | "target"
            | "node_id"
            | "symbol_id"
            | "symbol_ids"
            | "defines"
            | "definition_id"
            | "source_id"
            | "cfg_node_id"
            | "cfg_operation_id"
            | "pdg_node_id"
            | "controller_pdg_node_id"
            | "value_pdg_node_id"
            | "cfg_successor_node_id"
            | "completion_cfg_node_ids"
            | "entry_reachable_cfg_node_ids"
            | "entry_node_id"
            | "exit_node_ids"
            | "origin_node_ids"
    )
}

fn first_difference(actual: &Value, expected: &Value, path: &str) -> String {
    if let (Some(actual), Some(expected)) = (actual.as_object(), expected.as_object()) {
        for key in actual.keys().chain(expected.keys()) {
            if actual.get(key) != expected.get(key) {
                return match actual.get(key).zip(expected.get(key)) {
                    Some((a, e)) => first_difference(a, e, &format!("{path}/{key}")),
                    None => format!("{path}/{key}"),
                };
            }
        }
    }

    path.to_string()
}
