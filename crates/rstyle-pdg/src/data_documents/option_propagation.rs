//! Independent saved tag, fresh None, guarded reference and early-exit checks.
use super::result_propagation::{only_selected_consumers, return_effects, return_to_exit};
use super::*;
use crate::{CfgEdgeKind, HirNode, option_propagation, unit_store::EvidenceFacts};

const KEY: &str = "rstyle.data:concrete-option-propagation";
const CONSTRUCTS: [&str; 3] = [
    "try.option-is-none",
    "try.option-none",
    "try.option-some-reference",
];

const CONTRACT: super::propagation::Contract<3> = super::propagation::Contract {
    key: KEY,
    constructs: CONSTRUCTS,
    capability: option_propagation::CAPABILITY,
    policy_error: "Concrete Option propagation requires its registered document policy",
    candidate: option_propagation::candidate,
    semantics,
    invocation,
};

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    CONTRACT.verify_document_policy(profile, facts)
}

pub(super) fn install(pair: &mut Pair) {
    CONTRACT.install(pair);
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum Semantics {
    Selector {
        rule: &'static str,
        option_operand: u8,
        #[serde(rename = "true")]
        on_true: &'static str,
        #[serde(rename = "false")]
        on_false: &'static str,
        content_read: bool,
    },
    None {
        rule: &'static str,
        requires: &'static str,
        has_payload: bool,
        content_read: bool,
    },
    Some {
        rule: &'static str,
        option_operand: u8,
        requires: &'static str,
        content_read: bool,
        failure: &'static str,
    },
}

fn semantics(construct: &str) -> serde_json::Value {
    let value = match construct {
        "try.option-is-none" => Semantics::Selector {
            rule: "inspect-saved-core-option-tag/1",
            option_operand: 0,
            on_true: "none",
            on_false: "some",
            content_read: false,
        },
        "try.option-none" => Semantics::None {
            rule: "fresh-output-option-none/1",
            requires: "input-none",
            has_payload: false,
            content_read: false,
        },
        "try.option-some-reference" => Semantics::Some {
            rule: "saved-core-option-shared-payload/1",
            option_operand: 0,
            requires: "some",
            content_read: false,
            failure: "return-fresh-output-none/1",
        },
        _ => return serde_json::Value::Null,
    };

    serde_json::to_value(value).expect("Option propagation semantics contain only primitive fields")
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    CONTRACT.verify(pair, facts)
}

fn invocation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
    guards: &super::conditional_payloads::Guards<'_>,
) -> Result<(), String> {
    let checked = option_propagation::verify(source, facts)?;
    let selector = super::typed_operation(pair, source, CONSTRUCTS[0], "bool")?;
    let none = super::typed_operation(pair, source, CONSTRUCTS[1], &checked.output_token)?;
    let success = super::typed_operation(pair, source, CONSTRUCTS[2], &checked.payload_token)?;
    let returned = super::serialized_operation(pair, &checked.early_return.span, |node| {
        node.category == NodeCategory::Return
    })?;
    let saved = super::value_operation(pair, checked.operand)?;
    for operation in [selector, success] {
        super::verify_saved_operands(pair, operation, &[saved])?;
    }

    super::verify_saved_operands(pair, none, &[])?;
    super::verify_saved_operands(pair, returned, &[none])?;
    operation_effects(selector, NodeCategory::Control, CONSTRUCTS[0])?;
    operation_effects(none, NodeCategory::Computation, CONSTRUCTS[1])?;
    operation_effects(success, NodeCategory::Computation, CONSTRUCTS[2])?;
    return_effects(returned, facts)?;
    guards.guarded(selector, none, CfgEdgeKind::True)?;
    guards.guarded(selector, returned, CfgEdgeKind::True)?;
    guards.guarded(selector, success, CfgEdgeKind::False)?;
    only_selected_consumers(pair, saved, [selector, success])?;
    return_to_exit(pair, returned)
}

fn operation_effects(
    node: &PdgNode,
    category: NodeCategory,
    construct: &str,
) -> Result<(), String> {
    super::verify_value_effects(node, false, "Option propagation invents value effects")?;
    if node.category != category
        || node.attributes.literal.is_some()
        || node.annotations.get(KEY) != Some(&semantics(construct))
        || node
            .attributes
            .extensions
            .contains_key("compat.rust.hir-r0:return-form")
        || node.attributes.write_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err(
            "Option propagation loses its exact tag, None or shared-payload interpretation".into(),
        );
    }
    Ok(())
}
