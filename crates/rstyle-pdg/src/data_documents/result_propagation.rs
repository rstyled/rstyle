//! Independent tag, linear whole-error forwarding and early-exit checks.
use super::*;
use crate::{CfgEdgeKind, HirNode, result_propagation, unit_store::EvidenceFacts};

const KEY: &str = "rstyle.data:same-error-result-propagation";
const CONSTRUCTS: [&str; 2] = ["try.result-is-err", "try.result-ok-unit"];

fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::operations(&facts.body, &["expr_try_desugar"])
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        result_propagation::CAPABILITY,
        || !sources(facts).is_empty(),
        "Same-error Result propagation requires its registered document policy",
    )
}

pub(super) fn install(pair: &mut Pair) {
    for node in &mut pair.pdg.nodes {
        if let Some(construct) = node.attributes.construct.as_deref()
            && CONSTRUCTS.contains(&construct)
        {
            node.annotations.insert(KEY.into(), semantics(construct));
        }
    }
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum Semantics {
    Selector {
        rule: &'static str,
        result_operand: u8,
        #[serde(rename = "true")]
        on_true: &'static str,
        #[serde(rename = "false")]
        on_false: &'static str,
        moves_payload: bool,
    },
    Success {
        rule: &'static str,
        result_operand: u8,
        requires: &'static str,
        failure: &'static str,
        conversion: &'static str,
    },
}

fn semantics(construct: &str) -> serde_json::Value {
    let semantics = match construct {
        "try.result-is-err" => Semantics::Selector {
            rule: "borrow-core-result-discriminant/1",
            result_operand: 0,
            on_true: "err",
            on_false: "ok",
            moves_payload: false,
        },
        "try.result-ok-unit" => Semantics::Success {
            rule: "consume-core-result-unit-success/1",
            result_operand: 0,
            requires: "ok",
            failure: "return-same-saved-result/1",
            conversion: "pinned-reflexive-from/1",
        },
        _ => return serde_json::Value::Null,
    };

    serde_json::to_value(semantics)
        .expect("Result propagation semantics contain only primitive fields")
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = sources(facts);
    super::verify_construct_inventory(pair, CONSTRUCTS, [sources.len(); 2])?;
    verify_document_policy(&pair.pdg.profile, facts)?;
    if sources.is_empty() {
        return Ok(());
    }

    let guards = super::conditional_payloads::Guards::new(pair)?;
    for source in sources {
        invocation(pair, source, facts, &guards)?;
    }
    Ok(())
}

fn invocation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
    guards: &super::conditional_payloads::Guards<'_>,
) -> Result<(), String> {
    let checked = result_propagation::verify(source, facts)?;
    let selector = super::typed_operation(pair, source, CONSTRUCTS[0], "bool")?;
    let success = super::typed_operation(pair, source, CONSTRUCTS[1], "unit")?;
    let returned = super::serialized_operation(pair, &checked.early_return.span, |node| {
        node.category == NodeCategory::Return
    })?;
    let saved = super::value_operation(pair, checked.operand)?;
    for operation in [selector, success, returned] {
        super::verify_saved_operands(pair, operation, &[saved])?;
    }

    operation_effects(selector, NodeCategory::Control, CONSTRUCTS[0])?;
    operation_effects(success, NodeCategory::Computation, CONSTRUCTS[1])?;
    return_effects(returned, facts)?;
    guards.guarded(selector, returned, CfgEdgeKind::True)?;
    guards.guarded(selector, success, CfgEdgeKind::False)?;
    only_selected_consumers(pair, saved, [selector, success, returned])?;
    return_to_exit(pair, returned)
}

fn operation_effects(
    node: &PdgNode,
    category: NodeCategory,
    construct: &str,
) -> Result<(), String> {
    let success = construct == CONSTRUCTS[1];
    super::verify_value_effects(node, success, "Result propagation invents value effects")?;
    if node.category != category
        || node.annotations.get(KEY) != Some(&semantics(construct))
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(|literal| literal.known_value())
            != success.then_some("()")
        || node
            .attributes
            .extensions
            .contains_key("compat.rust.hir-r0:return-form")
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Result propagation loses its exact tag or unit-success interpretation".into());
    }
    Ok(())
}

fn return_effects(node: &PdgNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    super::verify_value_effects(
        node,
        false,
        "Result propagation return invents value effects",
    )?;
    let token = node
        .annotations
        .get("compat.rust.hir-r0:type")
        .and_then(|value| value.get("value"))
        .and_then(serde_json::Value::as_str);
    if node.original_kind.known_value() != Some("expr_ret")
        || token != facts.signature.return_ty.normalized.as_deref()
        || node
            .attributes
            .extensions
            .get("compat.rust.hir-r0:return-form")
            .and_then(serde_json::Value::as_str)
            != Some("explicit")
        || node.attributes.construct.is_some()
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Result propagation loses its exact whole-value return".into());
    }
    Ok(())
}

fn only_selected_consumers(
    pair: PairView<'_>,
    saved: &PdgNode,
    operations: [&PdgNode; 3],
) -> Result<(), String> {
    let actual: BTreeSet<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.source == saved.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .map(|edge| edge.target.as_str())
        .collect();
    let expected: BTreeSet<_> = operations.iter().map(|node| node.id.as_str()).collect();
    if actual != expected {
        return Err(
            "Saved Result has consumers outside its tag and mutually exclusive ownership paths"
                .into(),
        );
    }
    Ok(())
}

fn return_to_exit(pair: PairView<'_>, returned: &PdgNode) -> Result<(), String> {
    let mapping = crate::read_operation_map(&pair.pdg.extensions)?;
    let candidates: Vec<_> = mapping
        .iter()
        .filter(|entry| entry.pdg_node_id == returned.id)
        .collect();
    let [operation] = candidates.as_slice() else {
        return Err("Result propagation return lacks a unique CFG operation".into());
    };

    let outgoing: Vec<_> = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| edge.source == operation.cfg_node_id)
        .collect();
    if !matches!(outgoing.as_slice(), [edge] if edge.kind == CfgEdgeKind::Return && pair.cfg.exit_node_ids.contains(&edge.target))
    {
        return Err("Result error path must return to an exit without falling through".into());
    }
    Ok(())
}
