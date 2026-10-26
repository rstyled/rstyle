//! Independently verify saved input, disjoint payload transfers and error return.
// Shared topology checks confer no scalar or reference payload authority.
use super::borrowed_result_propagation::Operations;
use super::*;
use crate::{HirNode, named_error_result_propagation as propagation, unit_store::EvidenceFacts};

const KEY: &str = "rstyle.data:named-error-reader-propagation";
const WITNESS: &str = "rstyle.data:named-error-reader-propagation-types";
const CONSTRUCTS: [&str; 3] = [
    "try.reader-result-is-err",
    "try.reader-result-error",
    "try.reader-result-ok-unit",
];

const CONTRACT: super::propagation::Contract<3> = super::propagation::Contract {
    key: KEY,
    constructs: CONSTRUCTS,
    capability: propagation::CAPABILITY,
    policy_error: "Named error reader Result propagation requires its registered document policy",
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
        identity: "rstyle.rust.named-error-reader-flow-types/1",
        input: &checked.input_token,
        output: &checked.output_token,
        payload: &checked.payload_token,
        error: &checked.error_token,
        source_span: &source.span,
        operand_span: &checked.operand.span,
        early_return_span: &checked.early_return.span,
    })
    .expect("Named error reader Result metadata contains only primitive fields")
}

fn semantics(construct: &str) -> serde_json::Value {
    static RULES: std::sync::OnceLock<[serde_json::Value; 3]> = std::sync::OnceLock::new();
    super::propagation::semantic_rule(construct, &CONSTRUCTS, &RULES, semantic_rules)
}

fn semantic_rules() -> [serde_json::Value; 3] {
    [
        r#"{"rule":"inspect-saved-core-result-tag/1","result_operand":0,"true":"err","false":"ok","moves_payload":false,"content_read":false}"#,
        r#"{"rule":"move-named-error-rewrap-reader-result/1","result_operand":0,"requires":"err","payload":"same-error-once","output":"Err(error)","conversion":"pinned-reflexive-from/1","reference_routes":"preserve-saved-err-components","forwards_input_result":false,"content_read":false}"#,
        r#"{"rule":"saved-core-result-unit-payload/1","result_operand":0,"requires":"ok","payload":"unit","reference_origins":[],"content_read":false}"#,
    ].map(|text| serde_json::from_str(text).expect("Named error reader Result rules are static JSON"))
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    CONTRACT.verify_annotations(
        pair,
        facts,
        super::propagation::AnnotationInventory {
            witness: WITNESS,
            error: "Named error reader Result semantic/type inventory disagrees with source",
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
    super::scalar_result_propagation::verify_transfer(
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

fn operation_effects(
    node: &PdgNode,
    category: NodeCategory,
    construct: &str,
    metadata: &serde_json::Value,
) -> Result<(), String> {
    super::verify_value_effects(
        node,
        false,
        "Named error reader Result propagation invents value effects",
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
        return Err(
            "Named error reader Result propagation loses its exact payload interpretation".into(),
        );
    }
    Ok(())
}
