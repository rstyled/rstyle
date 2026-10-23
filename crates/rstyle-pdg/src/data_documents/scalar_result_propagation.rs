//! Independently verify saved input, disjoint payload transfers and error return.
// Shared topology checks confer no scalar or reference payload authority.
use super::borrowed_result_propagation::{Operations, consumers, flow};

use super::result_propagation::{return_effects, return_to_exit};
use super::*;
use crate::{
    CfgEdgeKind, HirNode, scalar_result_propagation as propagation, unit_store::EvidenceFacts,
};

const KEY: &str = "rstyle.data:scalar-result-propagation";
const WITNESS: &str = "rstyle.data:scalar-result-propagation-types";
const CONSTRUCTS: [&str; 3] = [
    "try.scalar-result-is-err",
    "try.scalar-result-error",
    "try.scalar-result-ok-scalar",
];

const CONTRACT: super::propagation::Contract<3> = super::propagation::Contract {
    key: KEY,
    constructs: CONSTRUCTS,
    capability: propagation::CAPABILITY,
    policy_error: "Scalar Result propagation requires its registered document policy",
    candidate: propagation::candidate,
    semantics,
    invocation,
};

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    CONTRACT.verify_document_policy(profile, facts)
}

fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(
        facts,
        &["expr_try_desugar"],
        propagation::candidate,
    )
}

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<(), String> {
    CONTRACT.install(pair);

    for source in sources(facts) {
        let checked = propagation::verify(source, facts)?;

        let metadata = metadata(source, &checked);

        for node in &mut pair.pdg.nodes {
            if operation_for(node, source) {
                node.annotations.insert(WITNESS.into(), metadata.clone());
            }
        }
    }
    Ok(())
}

fn operation_for(node: &PdgNode, source: &HirNode) -> bool {
    node.attributes
        .construct
        .as_deref()
        .is_some_and(|construct| CONSTRUCTS.contains(&construct))
        && node.source_span.as_ref().is_some_and(|span| {
            span.start == u64::from(source.span.start) && span.end == u64::from(source.span.end)
        })
}

#[derive(serde::Serialize)]
struct Metadata<'a> {
    identity: &'static str,
    input: &'a str,
    output: &'a str,
    payload: &'a str,
    error: &'a str,
    source_span: &'a crate::ByteSpan,
    operand_span: &'a crate::ByteSpan,
    early_return_span: &'a crate::ByteSpan,
}

fn metadata(source: &HirNode, checked: &propagation::Checked<'_>) -> serde_json::Value {
    serde_json::to_value(Metadata {
        identity: "rstyle.rust.scalar-result-flow-types/1",
        input: &checked.input_token,
        output: &checked.output_token,
        payload: &checked.payload_token,
        error: &checked.error_token,
        source_span: &source.span,
        operand_span: &checked.operand.span,
        early_return_span: &checked.early_return.span,
    })
    .expect("Scalar Result metadata contains only primitive fields")
}

fn semantics(construct: &str) -> serde_json::Value {
    static RULES: std::sync::OnceLock<[serde_json::Value; 3]> = std::sync::OnceLock::new();
    super::propagation::semantic_rule(construct, &CONSTRUCTS, &RULES, semantic_rules)
}

fn semantic_rules() -> [serde_json::Value; 3] {
    [
        r#"{"rule":"inspect-saved-core-result-tag/1","result_operand":0,"true":"err","false":"ok","moves_payload":false,"content_read":false}"#,
        r#"{"rule":"move-error-rewrap-output-result/1","result_operand":0,"requires":"err","payload":"same-error-once","output":"Err(error)","conversion":"pinned-reflexive-from/1","forwards_input_result":false,"content_read":false}"#,
        r#"{"rule":"saved-core-result-scalar-payload/1","result_operand":0,"requires":"ok","payload":"same-usize-value","content_read":false}"#,
    ].map(|text| serde_json::from_str(text).expect("Scalar Result rules are static JSON"))
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    CONTRACT.verify_annotations(
        pair,
        facts,
        super::propagation::AnnotationInventory {
            witness: WITNESS,
            error: "Scalar Result semantic/type inventory disagrees with source",
        },
    )
}

fn invocation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
    guards: &super::conditional_payloads::Guards<'_>,
) -> Result<(), String> {
    let checked = propagation::verify(source, facts)?;

    let selector = super::typed_operation(pair, source, CONSTRUCTS[0], "bool")?;

    let error = super::typed_operation(pair, source, CONSTRUCTS[1], &checked.output_token)?;

    let success = super::typed_operation(pair, source, CONSTRUCTS[2], &checked.payload_token)?;

    let returned = super::serialized_operation(pair, &checked.early_return.span, |node| {
        node.category == NodeCategory::Return
    })?;

    let saved = super::value_operation(pair, checked.operand)?;

    let metadata = metadata(source, &checked);

    for (operation, category, construct) in [
        (selector, NodeCategory::Control, CONSTRUCTS[0]),
        (error, NodeCategory::Computation, CONSTRUCTS[1]),
        (success, NodeCategory::Computation, CONSTRUCTS[2]),
    ] {
        super::verify_saved_operands(pair, operation, &[saved])?;

        operation_effects(operation, category, construct, &metadata)?;
    }
    verify_transfer(
        pair,
        facts,
        guards,
        Operations {
            saved,
            selector,
            error,
            success,
            returned,
        },
    )
}

/// Shared branch topology grants no payload, ownership or origin permission.
pub(super) fn verify_transfer(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    guards: &super::conditional_payloads::Guards<'_>,
    operations: Operations<'_>,
) -> Result<(), String> {
    let Operations {
        saved,
        selector,
        error,
        success,
        returned,
    } = operations;
    super::verify_saved_operands(pair, returned, &[error])?;

    return_effects(returned, facts)?;

    guards.guarded(selector, error, CfgEdgeKind::True)?;

    guards.guarded(selector, returned, CfgEdgeKind::True)?;

    guards.guarded(selector, success, CfgEdgeKind::False)?;

    consumers::verify(pair, saved, [selector, error, success])?;

    consumers::verify(pair, error, [returned])?;

    operations.order(pair)?;

    flow::verify(pair, &operations)?;

    return_to_exit(pair, returned)
}

fn operation_effects(
    node: &PdgNode,
    category: NodeCategory,
    construct: &str,
    metadata: &serde_json::Value,
) -> Result<(), String> {
    super::verify_value_effects(
        node,
        false,
        "Scalar Result propagation invents value effects",
    )?;

    if node.category != category
        || node.annotations.get(KEY) != Some(&semantics(construct))
        || node.annotations.get(WITNESS) != Some(metadata)
        || node
            .attributes
            .extensions
            .contains_key("compat.rust.hir-r0:return-form")
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Scalar Result propagation loses its exact payload interpretation".into());
    }
    Ok(())
}
