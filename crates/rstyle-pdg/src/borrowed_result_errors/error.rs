//! A fresh unit variant of a complete source-proved no-drop scalar enum.
use super::*;
use crate::concrete_types::{TypeTree, VariantShape, source_proof::PathResolution};

pub(crate) struct Error {
    pub token: TypeToken,
    pub variant: String,
    pub position: usize,
}

pub(crate) fn error_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    !crate::reference_instances::products::unit_candidate(node, facts)
        && crate::extraction_profile::gated_node(node, facts, CAPABILITY, |node| {
            node.kind == "expr_path"
                && node
                    .res
                    .as_ref()
                    .is_some_and(|resolution| resolution.kind == "resolved_fieldless_variant")
                && crate::data_aggregates::parsed_type(node)
                    .is_ok_and(|token| matches!(token, TypeToken::MixedEnum { .. }))
        })
}

pub(crate) fn verify_error(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Error, String> {
    if !error_candidate(node, facts)
        || node.expansion.is_some()
        || node.dispatch.is_some()
        || !node.children.is_empty()
    {
        return Err(
            "Fresh scalar error requires an original resolved unit-variant expression".into(),
        );
    }
    let token = crate::data_aggregates::parsed_type(node)?;
    let nominal = crate::borrowed_result_types::error(&token)
        .ok_or("Fresh error requires its complete bounded scalar enum")?;
    let resolution = facts.nominal_resolution()?;
    if !resolution.checked_no_drop(nominal) {
        return Err(
            "Fresh error requires an independently checked complete no-drop declaration".into(),
        );
    }
    let (owner, name) = source::variant(node, facts)?;
    let tree = resolution.resolve(&owner)?;
    if TypeToken::from_tree(&tree) != token {
        return Err("Fresh error source owner disagrees with its complete observed enum".into());
    }

    identity(node, facts, nominal, &name)?;
    let position = unit_variant(&tree, &name)?;

    Ok(Error {
        token,
        variant: name,
        position,
    })
}

pub(super) fn unit_variant(tree: &TypeTree, name: &str) -> Result<usize, String> {
    let TypeTree::MixedEnum { variants, .. } = tree else {
        return Err("Fresh error requires its complete original mixed enum".into());
    };
    let matches: Vec<_> = variants
        .iter()
        .enumerate()
        .filter(|(_, variant)| variant.name == name)
        .collect();
    let [(position, variant)] = matches.as_slice() else {
        return Err("Fresh error variant is missing or ambiguous".into());
    };
    if variant.position != *position || !matches!(variant.shape, VariantShape::Unit {}) {
        return Err("Fresh error supports only an original unit variant".into());
    }

    Ok(*position)
}

fn identity(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    nominal: &crate::concrete_types::Nominal,
    name: &str,
) -> Result<(), String> {
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing fresh error variant identity")?;
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let crate_name = facts
        .build
        .frontend_options
        .get("rstyle.nominal_selected_unit")
        .and_then(|unit| unit.get("crate_name"))
        .and_then(serde_json::Value::as_str);
    if resolution.target.is_some()
        || callee.crate_hash != "local-unencoded"
        || callee.implementation.is_some()
        || callee.definition_hash.is_empty()
        || Some(callee.crate_name.as_str()) != crate_name
        || callee.path != format!("{}::{name}", nominal.definition_path)
    {
        return Err(
            "Fresh error resolved variant disagrees with exact local source ownership".into(),
        );
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct Semantics<'a> {
    rule: &'static str,
    variant: &'a str,
    position: usize,
    payload: bool,
    copy_permission: bool,
    cleanup: &'static str,
}

pub(crate) fn error_semantics(checked: &Error) -> serde_json::Value {
    let semantics = Semantics {
        rule: "fresh-no-drop-unit-error",
        variant: &checked.variant,
        position: checked.position,
        payload: false,
        copy_permission: false,
        cleanup: "complete-original-source-proof",
    };

    serde_json::to_value(semantics).expect("registered fresh error semantics")
}
