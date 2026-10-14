//! Verified same-compilation-unit helper dependencies. Names never prove effects.

use crate::{HirFnFacts, HirNode, producer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) mod transport;
mod transport_lexical;

pub const KEY: &str = "verified_helper";
const MAX_DEPTH: usize = 8;
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub summary: Summary,
    pub frontend: HirFnFacts,
    pub pdg: String,
    pub cfg: String,
    pub manifest: producer::Manifest,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub identity: String,
    pub argument_types: Vec<String>,
    pub result_type: String,
    pub mutates_arguments: bool,
    pub memory_effects: bool,
    pub may_panic: bool,
    pub always_returns: bool,
    pub argument_evaluation: String,
    pub result_dependence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_outputs: Option<Vec<crate::data_origins::Origin>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_reads: Option<Vec<crate::helper_reads::Read>>,
}

fn summary(facts: &HirFnFacts, pair: &producer::Pair) -> Result<Summary, String> {
    let argument_types = facts
        .signature
        .params
        .iter()
        .map(|parameter| {
            parameter
                .ty
                .normalized
                .clone()
                .ok_or_else(|| "Missing verified helper argument type".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let result_type = facts
        .signature
        .return_ty
        .normalized
        .clone()
        .ok_or("Missing verified helper result type")?;

    let reference_outputs = crate::helper_references::returned(facts, pair)?;
    let references = reference_outputs.is_some();
    let memory_reads = crate::helper_reads::derive(facts, pair)?;
    let reads = memory_reads.is_some();
    Ok(Summary {
        identity: if reads {
            "rstyle.helper.read-only-body/1"
        } else if references {
            "rstyle.helper.shared-return/1"
        } else {
            "rstyle.helper.source-body/1"
        }
        .into(),
        argument_types,
        result_type,
        mutates_arguments: false,
        memory_effects: reads,
        may_panic: false,
        always_returns: true,
        argument_evaluation: "source-order-once/1".into(),
        result_dependence: if references {
            "verified-formal-referent-substitution/1"
        } else {
            "call-input-snapshots-with-verified-callee-return-graph/1"
        }
        .into(),
        reference_outputs,
        memory_reads,
    })
}

fn identity(facts: &HirFnFacts) -> Option<&str> {
    facts
        .build
        .frontend_options
        .get("definition_identity")?
        .as_str()
}

/// Native-width semantics propagate through verified dependencies, even when
/// a caller's own interface and operations have only fixed-width scalar types.
pub fn native_width(facts: &HirFnFacts) -> Result<bool, String> {
    let data = crate::extraction_profile::ExtractionProfile::from_facts(facts)?.is_data();
    let interface = facts
        .signature
        .params
        .iter()
        .try_fold(false, |seen, parameter| {
            native_type(&parameter.ty, data).map(|native| seen | native)
        })?
        | native_type(&facts.signature.return_ty, data)?;

    Ok(interface | node_native_width(&facts.body, data)?)
}

fn native_type(ty: &crate::ResolvedTy, data: bool) -> Result<bool, String> {
    let Some(token) = ty.normalized.as_deref() else {
        return Ok(false);
    };
    if data {
        return crate::data_types::parse(token).map(|token| token.native_width());
    }
    Ok(matches!(token, "isize" | "usize"))
}

fn node_native_width(node: &HirNode, data: bool) -> Result<bool, String> {
    let own = node
        .ty
        .as_ref()
        .map(|ty| native_type(ty, data))
        .transpose()?
        .unwrap_or(false);
    let children = node.children.iter().try_fold(false, |native, child| {
        node_native_width(child, data).map(|seen| native | seen)
    })?;
    let callee = if let Some(resolution) = direct_resolution(node) {
        let dependency: Dependency = transport::decode(&resolution[KEY])
            .map_err(|_| "Missing verified dependency for transitive target-width evidence")?;
        native_width(&dependency.frontend)?
    } else {
        false
    };

    Ok(own | children | callee)
}

pub fn direct_resolution(node: &HirNode) -> Option<Value> {
    let direct = node.kind == "expr_call"
        && node.res.as_ref()?.kind == "resolved_direct_callee"
        && node.dispatch.as_ref()?.kind == "resolved_direct";
    let inherent = node.kind == "expr_method_call"
        && node.res.as_ref()?.kind == "resolved_inherent_callee"
        && node.dispatch.as_ref()?.kind == "resolved_method";
    if (!direct && !inherent) || node.res.as_ref()?.target.is_some() {
        return None;
    }

    serde_json::from_str(&node.res.as_ref()?.description).ok()
}

pub(crate) fn call_nodes(facts: &HirFnFacts) -> Vec<&HirNode> {
    let mut calls = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_call", &mut calls);

    let mut methods = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut methods);
    calls.extend(
        methods
            .into_iter()
            .filter(|node| direct_resolution(node).is_some()),
    );

    calls
}

/// Capture all local bodies first, then build the acyclic dependency handoff.
pub fn capture(facts: &mut [HirFnFacts]) -> Result<(), String> {
    let mut state = CaptureState {
        verified: BTreeMap::new(),
        pending: (0..facts.len()).collect(),
    };
    for _depth in 0..=MAX_DEPTH {
        if !state.advance(facts)? {
            break;
        }
    }
    // Unsupported or recursive callees remain explicit call exclusions.
    attach_all(facts, &state.verified)?;

    for function in facts {
        crate::helper_reads::check_budgets(function)?;
    }
    Ok(())
}

struct CaptureState {
    verified: BTreeMap<String, Dependency>,
    pending: Vec<usize>,
}

struct Prepared {
    index: usize,
    facts: HirFnFacts,
    dependency: Option<Dependency>,
}

impl CaptureState {
    fn prepare(&self, index: usize, facts: &HirFnFacts) -> Result<Prepared, String> {
        let mut facts = facts.clone();
        attach(&mut facts.body, &self.verified)?;
        let dependency = if crate::classify(&facts).status == crate::EligibilityVerdict::Eligible {
            certify(&facts)?
        } else {
            None
        };

        Ok(Prepared {
            index,
            facts,
            dependency,
        })
    }

    fn advance(&mut self, facts: &mut [HirFnFacts]) -> Result<bool, String> {
        let prepared = self
            .pending
            .iter()
            .map(|index| self.prepare(*index, &facts[*index]))
            .collect::<Result<Vec<_>, _>>()?;
        let completed: BTreeMap<_, _> = prepared
            .iter()
            .filter_map(|record| {
                record.dependency.as_ref().map(|dependency| {
                    (
                        identity(&record.facts).unwrap_or_default().to_owned(),
                        dependency.clone(),
                    )
                })
            })
            .collect();
        let count = prepared
            .iter()
            .filter(|record| record.dependency.is_some())
            .count();
        if count != completed.len() || completed.contains_key("") {
            return Err("Missing or duplicate resolved helper definition identity".into());
        }
        if completed.is_empty() {
            return Ok(false);
        }
        self.verified.extend(completed);
        self.pending = prepared
            .iter()
            .filter(|record| record.dependency.is_none())
            .map(|record| record.index)
            .collect();
        apply_prepared(facts, prepared);

        Ok(!self.pending.is_empty())
    }
}

fn apply_prepared(facts: &mut [HirFnFacts], prepared: Vec<Prepared>) {
    for record in prepared
        .into_iter()
        .filter(|record| record.dependency.is_some())
    {
        facts[record.index] = record.facts;
    }
}

fn attach_all(
    facts: &mut [HirFnFacts],
    verified: &BTreeMap<String, Dependency>,
) -> Result<(), String> {
    for function in facts {
        attach(&mut function.body, verified)?;
    }

    Ok(())
}

fn attach(node: &mut HirNode, verified: &BTreeMap<String, Dependency>) -> Result<(), String> {
    attach_children(node, verified)?;
    let Some(mut resolution) = direct_resolution(node) else {
        return Ok(());
    };
    let Some(dependency) = resolution["definition_identity"]
        .as_str()
        .and_then(|key| verified.get(key))
    else {
        return Ok(());
    };
    if resolution["local"] != true {
        return Ok(());
    }
    resolution[KEY] = transport::encode(dependency)?;
    let description = serde_json::to_string(&resolution).map_err(|error| error.to_string())?;
    if description.len() > MAX_BYTES {
        return Err("Helper evidence exceeds explicit 8 MiB per-call budget".into());
    }
    node.res.as_mut().expect("resolved callee").description = description;
    check_depth(node, 0)?;

    Ok(())
}

fn attach_children(
    node: &mut HirNode,
    verified: &BTreeMap<String, Dependency>,
) -> Result<(), String> {
    for child in &mut node.children {
        attach(child, verified)?;
    }

    Ok(())
}

fn certify(facts: &HirFnFacts) -> Result<Option<Dependency>, String> {
    if !crate::helper_references::supported_result(facts)? {
        return Ok(None);
    }
    let pair = producer::produce(facts).map_err(|errors| errors.join("; "))?;
    if check_boundary(&pair, facts).is_err() {
        return Ok(None);
    }
    let serialized = pair.serialize().map_err(|error| error.to_string())?;

    Ok(Some(Dependency {
        summary: summary(facts, &pair)?,
        frontend: facts.clone(),
        pdg: String::from_utf8(serialized.pdg_bytes).map_err(|error| error.to_string())?,
        cfg: String::from_utf8(serialized.cfg_bytes).map_err(|error| error.to_string())?,
        manifest: serialized.manifest,
    }))
}

fn check_boundary(pair: &producer::Pair, facts: &HirFnFacts) -> Result<(), Vec<String>> {
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
        || !pair.pdg.memory_locations.is_empty() && !crate::helper_reads::supported(facts, pair)
        || pair
            .pdg
            .nodes
            .iter()
            .any(|node| matches!(node.attributes.construct.as_deref(), Some("loop" | "while")))
    {
        return Err(vec![
            "Helper summary requires verified memory-free or registered shared read-only effects, nonpanicking acyclic returning flow"
                .into(),
        ]);
    }

    Ok(())
}

pub fn verify(node: &HirNode, caller: &HirFnFacts) -> Result<Dependency, String> {
    let evidence = node
        .res
        .as_ref()
        .map(|resolution| resolution.description.as_bytes());
    crate::diagnostics::Stage::new("helper.verify", &caller.name, evidence)
        .observe(|| verify_dependency(node, caller))
}

fn verify_dependency(node: &HirNode, caller: &HirFnFacts) -> Result<Dependency, String> {
    let description = &node
        .res
        .as_ref()
        .ok_or("Missing helper resolution")?
        .description;
    if description.len() > MAX_BYTES {
        return Err("Helper evidence exceeds explicit 8 MiB per-call budget".into());
    }
    let resolution = direct_resolution(node).ok_or("Unresolved direct helper call")?;
    let dependency: Dependency = transport::decode(&resolution[KEY])
        .map_err(|_| "Missing or invalid verified callee dependency")?;
    check_depth(&dependency.frontend.body, 1)?;
    check_signature(node, caller, &resolution, &dependency.frontend)?;
    crate::inherent_calls::verify(node, caller, &dependency.frontend, &resolution)?;
    let serialized = producer::SerializedPair {
        pdg_bytes: dependency.pdg.as_bytes().to_vec(),
        cfg_bytes: dependency.cfg.as_bytes().to_vec(),
        manifest: dependency.manifest.clone(),
    };
    let pair = producer::verify_serialized(&serialized, &dependency.frontend)
        .map_err(|errors| errors.join("; "))?;
    check_boundary(&pair, &dependency.frontend).map_err(|errors| errors.join("; "))?;
    if dependency.summary != summary(&dependency.frontend, &pair)? {
        return Err("Helper summary signature, effects or returning behavior contradict verified callee evidence".into());
    }
    crate::helper_reads::verify_arguments(&dependency.summary, node, caller)?;

    Ok(dependency)
}

fn check_signature(
    node: &HirNode,
    caller: &HirFnFacts,
    resolution: &Value,
    callee: &HirFnFacts,
) -> Result<(), String> {
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing resolved helper signature")?;
    let arguments: Option<Vec<_>> = callee
        .signature
        .params
        .iter()
        .map(|param| param.ty.normalized.clone())
        .collect();
    let result = callee.signature.return_ty.normalized.as_ref();
    let signature = arguments.as_ref() == Some(&dispatch.operand_tys)
        && !dispatch.operand_tys.iter().any(|ty| ty == "unit")
        && result == Some(&dispatch.result_ty)
        && node.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) == result
        && node.children.len() == dispatch.operand_tys.len()
        && node
            .children
            .iter()
            .zip(&dispatch.operand_tys)
            .all(|(child, expected)| {
                child.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) == Some(expected)
            });
    let build = same_build(caller, callee)?;
    if !signature
        || !build
        || resolution["local"] != true
        || resolution["path"] != callee.name
        || dispatch.op != callee.name
        || resolution["definition_identity"].as_str() != identity(callee)
    {
        return Err(
            "Resolved helper identity, signature or semantic build evidence disagrees".into(),
        );
    }

    Ok(())
}

fn same_build(caller: &HirFnFacts, callee: &HirFnFacts) -> Result<bool, String> {
    let profile = crate::extraction_profile::ExtractionProfile::from_facts(caller)?;
    let compatible = caller.build.overflow_checks == callee.build.overflow_checks
        && caller.build.debug_assertions == callee.build.debug_assertions
        && caller.build.panic_strategy == callee.build.panic_strategy
        && caller.build.pointer_width == callee.build.pointer_width
        && profile == crate::extraction_profile::ExtractionProfile::from_facts(callee)?;
    if !profile.is_data() {
        return Ok(compatible);
    }
    Ok(compatible && crate::helper_references::same_unit(caller, callee)?)
}

fn check_depth(node: &HirNode, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("Helper dependencies exceed explicit depth budget 8".into());
    }
    for child in &node.children {
        check_depth(child, depth)?;
    }
    if let Some(resolution) = direct_resolution(node)
        && !resolution[KEY].is_null()
    {
        let dependency: Dependency = transport::decode(&resolution[KEY])
            .map_err(|_| "Corrupt transitive helper dependency")?;
        check_depth(&dependency.frontend.body, depth + 1)?;
    }

    Ok(())
}

/// Independent serialized inventory validation; never invokes caller lowering.
pub fn verify_documents(pair: &producer::Pair, facts: &HirFnFacts) -> Result<(), String> {
    crate::inherent_calls::definition(facts)?;
    let calls = call_nodes(facts);
    let records = pair
        .pdg
        .extensions
        .get(crate::call_summaries::KEY)
        .and_then(Value::as_array)
        .ok_or("Missing serialized call inventory")?;
    if pair.cfg.extensions.get(crate::call_summaries::KEY)
        != pair.pdg.extensions.get(crate::call_summaries::KEY)
    {
        return Err("Paired serialized call inventories disagree".into());
    }
    for call in &calls {
        verify_document_call(pair, facts, call, records)?;
    }
    let nodes = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == crate::NodeCategory::Call
                && (node.original_kind.known_value() == Some("expr_call")
                    || records.iter().any(|record| {
                        record["pdg_node_id"] == node.id && record.get("dependency").is_some()
                    }))
        })
        .count();
    let dependencies = records
        .iter()
        .filter(|record| record.get("dependency").is_some())
        .count();
    if nodes != calls.len() || dependencies != calls.len() {
        return Err(
            "Serialized helper call/dependency inventories are incomplete or duplicated".into(),
        );
    }

    Ok(())
}

fn verify_document_call(
    pair: &producer::Pair,
    facts: &HirFnFacts,
    call: &HirNode,
    records: &[Value],
) -> Result<(), String> {
    let nodes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == crate::NodeCategory::Call
                && node.original_kind.known_value() == Some(call.kind.as_str())
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(call.span.start) && span.end == u64::from(call.span.end)
                })
        })
        .collect();
    let [node] = nodes.as_slice() else {
        return Err("No unique serialized source-backed helper CALL".into());
    };

    let dependency = verify(call, facts)?;
    let inventory: Vec<_> = records
        .iter()
        .filter(|record| record["pdg_node_id"] == node.id)
        .collect();
    let [record] = inventory.as_slice() else {
        return Err("Missing or duplicate serialized helper summary".into());
    };

    crate::helper_reads::verify_effects(node, call, facts, &dependency.summary)?;
    verify_call_metadata(node, call, record, dependency)?;
    verify_call_operands(pair, node, call, record)
}

fn verify_call_metadata(
    node: &crate::PdgNode,
    call: &HirNode,
    record: &Value,
    dependency: Dependency,
) -> Result<(), String> {
    if record["dependency"] != transport::encode(&dependency)?
        || record["source_span"]
            != serde_json::to_value(call.span).map_err(|error| error.to_string())?
        || node
            .attributes
            .callee
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != call.dispatch.as_ref().map(|dispatch| dispatch.op.as_str())
        || node.attributes.call_form != Some(crate::CallForm::Direct)
    {
        return Err(
            "Serialized helper summary or resolved callee disagrees with captured evidence".into(),
        );
    }

    Ok(())
}

pub(crate) fn verify_call_operands(
    pair: &producer::Pair,
    node: &crate::PdgNode,
    call: &HirNode,
    record: &Value,
) -> Result<(), String> {
    let operands = record["operand_pdg_node_ids"]
        .as_array()
        .ok_or("Missing serialized helper operands")?;
    if operands.len() != call.children.len() {
        return Err("Serialized helper argument arity disagrees".into());
    }
    let data_count = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .count();
    if data_count != operands.len() {
        return Err("Extra serialized helper argument dependence".into());
    }
    for (position, operand) in operands.iter().enumerate() {
        let edges: Vec<_> = pair
            .pdg
            .edges
            .iter()
            .filter(|edge| {
                edge.target == node.id
                    && edge.kind == crate::EdgeFamily::Data
                    && edge.subkind == "value_operand"
                    && edge.operand_position == Some(position as u64)
            })
            .collect();
        if edges.len() != 1 || operand.as_str() != Some(edges[0].source.as_str()) {
            return Err("Serialized helper argument evaluation/dependence disagrees".into());
        }
        verify_operand(pair, &edges[0].source, &call.children[position])?;
    }

    Ok(())
}

fn verify_operand(pair: &producer::Pair, operand: &str, argument: &HirNode) -> Result<(), String> {
    let value = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.id == operand)
        .ok_or("Missing helper argument value node")?;
    let span = value
        .source_span
        .clone()
        .or_else(|| {
            serde_json::from_value(
                value
                    .attributes
                    .extensions
                    .get("compat.rust.hir-r0:source-operation")?
                    .get("construct_span")?
                    .clone(),
            )
            .ok()
        })
        .ok_or("Missing source correspondence for helper argument value")?;
    let ty = value
        .annotations
        .get("compat.rust.hir-r0:type")
        .and_then(|fact| fact.get("value"))
        .and_then(Value::as_str);
    if span.start < u64::from(argument.span.start)
        || span.end > u64::from(argument.span.end)
        || ty != argument.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
    {
        return Err("Helper argument value lies outside its source/type correspondence".into());
    }

    Ok(())
}

#[cfg(test)]
mod width_tests {
    use super::*;
    use crate::data_types::TypeToken;

    fn resolved(token: &TypeToken) -> crate::ResolvedTy {
        crate::ResolvedTy {
            normalized: Some(token.text().unwrap()),
            observed: "forensics only".into(),
            spelling: String::new(),
        }
    }

    #[test]
    fn data_native_width_follows_nested_referents_and_elements() {
        let native = TypeToken::Shared(Box::new(TypeToken::Tuple(vec![
            TypeToken::Scalar("u8".into()),
            TypeToken::Array {
                element: Box::new(TypeToken::Scalar("usize".into())),
                length: 2,
            },
        ])));
        assert!(native_type(&resolved(&native), true).unwrap());
        assert!(!native_type(&resolved(&native), false).unwrap());
        let fixed = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
            "u64".into(),
        )))));
        assert!(!native_type(&resolved(&fixed), true).unwrap());
    }

    #[test]
    fn malformed_data_width_evidence_is_an_error() {
        let ty = crate::ResolvedTy {
            normalized: Some("{broken data token".into()),
            observed: "usize".into(),
            spelling: "usize".into(),
        };
        assert!(native_type(&ty, true).is_err());
        assert!(!native_type(&ty, false).unwrap());
    }
}
