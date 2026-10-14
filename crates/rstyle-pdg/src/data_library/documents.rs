//! Independent inspection of serialized metadata CALLs and their value operands.

use super::*;
use crate::{HirFnFacts, HirNode, NodeCategory, producer::Pair};
use serde::Serialize;
use serde_json::Value;

pub fn evidence(node: &HirNode, facts: &HirFnFacts) -> Result<Value, String> {
    let registry = registry_for(facts).ok_or("Unregistered library build")?;
    let capture = crate::data_receivers::verify(facts)?;
    let receiver = capture
        .methods
        .iter()
        .find(|record| record.call == node.hir)
        .ok_or("Missing library receiver evidence")?;
    serde_json::to_value(LibraryEvidence {
        registry_identity: &registry.identity,
        semantic_source_digests: &registry.source_digests,
        core_artifacts: verify_evidence(facts)?,
        callee: callee(node)?,
        receiver,
        origins: crate::data_origins::origins(&node.children[0], facts)?,
        rustc_commit: &facts.build.rustc_commit,
        target: &facts.build.target_triple,
        pointer_width: Width {
            status: facts.build.pointer_width.status,
            bits: facts.build.pointer_width.value,
        },
    })
    .map_err(|error| error.to_string())
}

#[derive(Serialize)]
struct LibraryEvidence<'a> {
    registry_identity: &'a str,
    semantic_source_digests: &'a BTreeMap<String, String>,
    core_artifacts: Artifacts,
    callee: Callee,
    receiver: &'a crate::data_receivers::Method,
    origins: Vec<crate::data_origins::Origin>,
    rustc_commit: &'a crate::StringFact,
    target: &'a crate::StringFact,
    pointer_width: Width,
}
#[derive(Serialize)]
struct Width {
    status: crate::FactStatus,
    bits: Option<u64>,
}

pub fn verify_documents(pair: &Pair, facts: &HirFnFacts) -> Result<(), String> {
    let mut methods = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut methods);
    let records = pair
        .pdg
        .extensions
        .get(crate::call_summaries::KEY)
        .and_then(Value::as_array)
        .ok_or("Missing serialized call inventory")?;
    if pair.cfg.extensions.get(crate::call_summaries::KEY)
        != pair.pdg.extensions.get(crate::call_summaries::KEY)
    {
        return Err("Paired library call inventories disagree".into());
    }
    if pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == NodeCategory::Call
                && node.original_kind.known_value() == Some("expr_method_call")
        })
        .count()
        != methods.len()
    {
        return Err("Serialized method CALL inventory is incomplete or excessive".into());
    }

    let context = Documents {
        pair,
        facts,
        records,
    };
    let mut verified = 0;
    for method in methods.into_iter().filter(|node| operation(node).is_some()) {
        let summary = resolve(method, facts)?.ok_or("Unverified serialized library CALL")?;
        context.verify_call(method, &summary)?;
        verified += 1;
    }
    if records
        .iter()
        .filter(|record| record.get("library").is_some())
        .count()
        != verified
    {
        return Err("Extra or missing serialized library summary evidence".into());
    }
    Ok(())
}

struct Documents<'a> {
    pair: &'a Pair,
    facts: &'a HirFnFacts,
    records: &'a [Value],
}

impl Documents<'_> {
    fn verify_call(
        &self,
        call: &HirNode,
        summary: &crate::call_summaries::Summary,
    ) -> Result<(), String> {
        let Self {
            pair,
            facts,
            records,
        } = self;
        let nodes: Vec<_> = pair
            .pdg
            .nodes
            .iter()
            .filter(|node| {
                node.category == NodeCategory::Call
                    && node.original_kind.known_value() == Some("expr_method_call")
                    && node.source_span.as_ref().is_some_and(|span| {
                        span.start == u64::from(call.span.start)
                            && span.end == u64::from(call.span.end)
                    })
            })
            .collect();
        let [node] = nodes.as_slice() else {
            return Err("No unique source-backed library CALL".into());
        };
        let inventory: Vec<_> = records
            .iter()
            .filter(|record| record["pdg_node_id"] == node.id)
            .collect();
        let [record] = inventory.as_slice() else {
            return Err("Missing or duplicate library summary".into());
        };

        verify_metadata(node, call, record, summary)?;
        if record["library"] != evidence(call, facts)?
            || record["overflow_checks"]
                != serde_json::to_value(&facts.build.overflow_checks).map_err(|e| e.to_string())?
            || record["debug_assertions"]
                != serde_json::to_value(&facts.build.debug_assertions).map_err(|e| e.to_string())?
            || record["panic_strategy"]
                != serde_json::to_value(&facts.build.panic_strategy).map_err(|e| e.to_string())?
        {
            return Err("Serialized library/core/receiver/build evidence disagrees".into());
        }
        crate::helper_calls::verify_call_operands(pair, node, call, record)
    }
}

fn verify_metadata(
    node: &crate::PdgNode,
    call: &HirNode,
    record: &Value,
    summary: &crate::call_summaries::Summary,
) -> Result<(), String> {
    if node.synthetic
        || node.provenance.origin != crate::ProvenanceOrigin::Source
        || node.attributes.call_form != Some(crate::CallForm::Direct)
        || node
            .attributes
            .callee
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(summary.callee_path.as_str())
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(summary.callee_path.as_str())
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.exit_kind.is_some()
        || node.annotations["compat.rust.hir-r0:type"]["value"] != summary.result_type
        || record["source_span"] != serde_json::to_value(call.span).map_err(|e| e.to_string())?
        || record["summary"] != serde_json::to_value(summary).map_err(|e| e.to_string())?
    {
        return Err("Serialized metadata CALL type, effects or source identity disagrees".into());
    }
    Ok(())
}
