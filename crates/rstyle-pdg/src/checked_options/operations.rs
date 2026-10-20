//! Executable checked-reference vocabulary; no ABI representation is inferred.
use super::*;
use crate::{call_summaries::Summary, data_origins::Origin, field_locations::Projection};

pub(crate) const KEY: &str = "rstyle.data:checked-option";

pub(crate) fn enabled(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, "rust.checked-slice-option")
}

pub(crate) fn get_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    node.kind == "expr_method_call" && candidate(node, facts)
}

pub(crate) fn match_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    node.kind == "expr_match" && candidate(node, facts)
}

fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    if !enabled(facts) {
        return false;
    }

    match node.kind.as_str() {
        "expr_method_call" => node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.op == GET),
        "expr_match" => node
            .children
            .first()
            .is_some_and(|scrutinee| types::payload(scrutinee).is_ok()),
        _ => false,
    }
}

pub(crate) fn resolve(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Summary, String> {
    verify_get(node, facts)?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing checked-get dispatch")?;
    Ok(Summary {
        id: "rstyle.data.slice.get-scalar".into(),
        version: "1".into(),
        callee_path: GET.into(),
        callee_crate: "core".into(),
        argument_types: dispatch.operand_tys.clone(),
        result_type: dispatch.result_ty.clone(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "not_applicable".into(),
    })
}

pub(crate) fn project(mut roots: Vec<Origin>) -> Vec<Origin> {
    for root in &mut roots {
        root.path.push(Projection::CheckedElement);
    }
    roots
}

pub(crate) fn semantics(construct: &str) -> serde_json::Value {
    match construct {
        "data.option-some" => interpretation([
            ("rule", "scalar-option-some".into()),
            ("payload_operand", 0.into()),
            ("variant", "some".into()),
            ("content_read", false.into()),
        ]),
        "data.option-none" => interpretation([
            ("rule", "scalar-option-none".into()),
            ("variant", "none".into()),
            ("has_payload", false.into()),
            ("content_read", false.into()),
        ]),
        "data.option-value-payload" => interpretation([
            ("rule", "project-scalar-option-payload".into()),
            ("option_operand", 0.into()),
            ("requires", "some".into()),
            ("content_read", false.into()),
        ]),
        "data.option-get" => interpretation([
            ("rule", "index-less-than-length-option".into()),
            ("receiver_operand", 0.into()),
            ("index_operand", 1.into()),
            ("success", "some-shared-element".into()),
            ("failure", "none".into()),
            ("content_read", false.into()),
        ]),
        "data.option-payload" => interpretation([
            ("rule", "copy-shared-scalar-payload".into()),
            ("option_operand", 0.into()),
            ("requires", "some".into()),
        ]),
        "match.option-some" => interpretation([
            ("rule", "option-is-some".into()),
            ("option_operand", 0.into()),
        ]),
        "match.option-none" => interpretation([
            ("rule", "option-is-none".into()),
            ("option_operand", 0.into()),
        ]),
        _ => unreachable!("registered checked Option operation"),
    }
}

fn interpretation<const N: usize>(fields: [(&str, serde_json::Value); N]) -> serde_json::Value {
    serde_json::Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}

pub(crate) fn matches(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_match"], match_candidate)
}

pub(crate) fn owner<'a>(
    binding: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Match<'a>, String> {
    for source in matches(facts) {
        let checked = verify_match(source, facts)?;
        if checked.binding.hir == binding.hir {
            return Ok(checked);
        }
    }
    Err("Payload load lacks its verified Option match owner".into())
}

pub(crate) fn locations(
    binding: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<crate::field_locations::derived::Located>, String> {
    let checked = owner(binding, facts)?;
    let scalar = crate::data_types::parse(crate::data_aggregates::token(binding)?)?;
    let roots = crate::data_origins::origins(checked.scrutinee, facts)?;
    if roots.is_empty() {
        return Err("Checked payload has no retained conditional origins".into());
    }

    crate::field_locations::derived::from_origins(scalar, facts, roots)
}
