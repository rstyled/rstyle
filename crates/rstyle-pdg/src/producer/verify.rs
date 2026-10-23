//! Producer handoff checks using captured source/build evidence and the real
//! interchange boundary. Producer conformance flags are never an input proof.

mod node_names;
mod normalization;
use node_names::node_names;
#[cfg(test)]
mod tests;
use normalization::normalize;

use super::{Pair, PairView, analyze_facts, emit, lower};
use crate::*;
use serde_json::Value;
use std::collections::BTreeMap;

pub fn verify(
    pair: &Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), Vec<String>> {
    verify_view(pair.as_view(), facts)
}

/// Apply the exact same verification to borrowed typed documents.
pub fn verify_view(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), Vec<String>> {
    crate::verification_session::within(|| verify_pair(pair, facts))
}

fn verify_pair(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), Vec<String>> {
    let observation = crate::diagnostics::Stage::new("producer.verify", &facts.name, None).enter();
    if crate::verification_policy::is_generated() {
        return observation.finish(validate_boundary(pair, facts));
    }
    if facts.source.bytes.identity() != facts.source.digest {
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
    let mut expected = emit(facts, &graph, &analysis, revision).map_err(|e| vec![e])?;
    let renaming = correspondence(pair, expected.as_view())?;
    compare_documents(pair, &mut expected, &renaming)?;

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
    observation.finish(validate_boundary(pair, facts))
}

fn validate_boundary(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), Vec<String>> {
    validate_frontend_reference(pair, facts)?;
    let width = facts
        .build
        .pointer_width
        .value
        .and_then(|width| u8::try_from(width).ok())
        .ok_or_else(|| vec!["Missing actual target width".to_string()])?;
    let context = ValidationContext::CompilerEvidence {
        pointer_width_bits: width,
    };
    let report = crate::validate::validate_source_text(pair.pdg, &context, &facts.source.bytes);
    let structural = validate_cfg(pair.cfg)
        .into_iter()
        .chain(validate_paired_agreement(
            &pair.pdg.extensions,
            &pair.cfg.extensions,
        ))
        .map(|i| i.message);
    let map = read_operation_map(&pair.pdg.extensions).map_err(|e| vec![e])?;
    let node_ids = pair.pdg.nodes.iter().map(|n| n.id.clone()).collect();
    let correspondence = validate_correspondence(&map, pair.cfg, &node_ids)
        .into_iter()
        .map(|i| i.message);
    let values = validate_value_sites(pair.pdg, pair.cfg)
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

fn validate_frontend_reference(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), Vec<String>> {
    let key = crate::extraction_profile::EVIDENCE_KEY;
    let actual = pair
        .pdg
        .extensions
        .get(key)
        .and_then(|evidence| evidence.get("frontend_ref"));
    let expected = facts.frontend_reference();

    if actual.is_some() && pair.pdg.extensions[key].get("frontend").is_some() {
        return Err(vec![
            "Contradictory inline and referenced frontend evidence".into(),
        ]);
    }
    if pair.pdg.extensions.contains_key(key) && actual != expected.as_ref() {
        return Err(vec![
            "Frontend reference requires its exact owning unit/function".into(),
        ]);
    }
    if pair.cfg.extensions.get(key) != pair.pdg.extensions.get(key) {
        return Err(vec!["Paired frontend references disagree".into()]);
    }
    Ok(())
}

fn correspondence(
    actual: PairView<'_>,
    expected: PairView<'_>,
) -> Result<BTreeMap<String, String>, Vec<String>> {
    let mut names = node_names(actual, expected)?;
    add_binding_names(actual, expected, &mut names)?;
    add_context_names(actual, expected, &mut names)?;
    Ok(names)
}

fn add_context_names(
    actual: PairView<'_>,
    expected: PairView<'_>,
    names: &mut BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    add_cfg_names(actual, expected, names)?;
    for (actual, expected) in crate::data_documents::location_correspondence(actual, expected)
        .map_err(|error| vec![error])?
    {
        if names
            .get(&actual)
            .is_some_and(|previous| previous != &expected)
        {
            return Err(vec!["Ambiguous local reference namespaces".into()]);
        }
        names.insert(actual, expected);
    }

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

    Ok(())
}

fn add_binding_names(
    actual: PairView<'_>,
    expected: PairView<'_>,
    names: &mut BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    add_symbols(actual, expected, names)?;
    add_definitions(actual, expected, names)
}

fn add_symbols(
    actual: PairView<'_>,
    expected: PairView<'_>,
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
    actual: PairView<'_>,
    expected: PairView<'_>,
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
    actual: PairView<'_>,
    expected: PairView<'_>,
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
    actual: PairView<'_>,
    expected: &mut Pair,
    renaming: &BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    // Current deterministic emission usually preserves IDs and record order.
    // Align only the historical informational conformance exception; profiles,
    // inventories and all semantic fields still participate in typed equality.
    if renaming.iter().all(|(actual, expected)| actual == expected) {
        for (actual, expected) in actual
            .pdg
            .extraction_evidence
            .conformance
            .iter()
            .zip(&mut expected.pdg.extraction_evidence.conformance)
        {
            expected.status = actual.status;
            expected.reasons.clone_from(&actual.reasons);
        }
    }
    compare_document(actual.pdg, &expected.pdg, renaming)?;
    compare_document(actual.cfg, &expected.cfg, renaming)
}

fn compare_document<T: serde::Serialize + PartialEq>(
    actual: &T,
    expected: &T,
    renaming: &BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    if renaming.iter().all(|(actual, expected)| actual == expected) && actual == expected {
        return Ok(());
    }
    let actual = normalize(
        serde_json::to_value(actual).map_err(|e| vec![e.to_string()])?,
        "",
        renaming,
    );
    let expected = normalize(
        serde_json::to_value(expected).map_err(|e| vec![e.to_string()])?,
        "",
        &BTreeMap::new(),
    );
    if actual != expected {
        return Err(vec![format!(
            "Serialized extraction facts disagree with captured source/build/normal-flow evidence: {}",
            first_difference(&actual, &expected, "")
        )]);
    }

    Ok(())
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
