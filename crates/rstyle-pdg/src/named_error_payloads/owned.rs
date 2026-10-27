//! A whole incoming generated error moves once into its exact declared field.
use super::*;
use crate::copy_tagged_values::origins::Component;
use serde::{Deserialize, Serialize};

pub(crate) const CAPABILITY: &str = "rust.named-error-owned-transfer";
pub(crate) const KEY: &str = "rstyle.data:named-error-owned-transfer";

/// Rebuilt from original complete declarations and cleanup receipts, never trusted
/// merely because an identical-looking record appeared on a serialized graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Transfer {
    pub identity: String,
    pub parameter: u32,
    pub binding: crate::HirRef,
    pub input: TypeTree,
    pub output: Vec<Component>,
}

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && facts.signature.params.len() == 2
        && facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .is_some_and(|text| text.contains("\"kind\":\"mixed_enum\""))
        && facts.body.kind == "expr_block"
        && matches!(facts.body.children.as_slice(), [tail] if tail.kind == "expr_struct"
            && matches!(tail.children.as_slice(), [payload] if payload.kind == "expr_struct"))
        && facts.signature.params[0]
            .ty
            .normalized
            .as_deref()
            .is_some_and(|text| text.contains("\"kind\":\"struct\""))
}

pub(crate) fn checked(facts: &dyn EvidenceFacts) -> Result<Option<Transfer>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let [
        Observation::Captured { ty: input },
        Observation::Captured { ty: message },
    ] = interface.parameters.as_slice()
    else {
        return Err("Owned error constructor requires both complete parameter observations".into());
    };
    if *message != TypeTree::Shared(Box::new(TypeTree::Str)) || !shape(input) {
        return Err(
            "Owned error constructor requires generated scalar error and static message".into(),
        );
    }

    let Observation::Captured { ty: output } = &interface.result else {
        return Err("Owned error constructor requires its complete output".into());
    };
    let output = output_route(facts, input, message, output)?;
    cleanup(facts, input)?;

    Ok(Some(Transfer {
        identity: "rstyle.rust.named-error-whole-owned-transfer/1".into(),
        parameter: 0,
        binding: facts.signature.params[0].pat.hir.clone(),
        input: input.clone(),
        output,
    }))
}

fn output_route(
    facts: &dyn EvidenceFacts,
    input: &TypeTree,
    message: &TypeTree,
    output: &TypeTree,
) -> Result<Vec<Component>, String> {
    let (wrapper, enumeration, variants) = output_parts(output)?;
    let selected = selected_variant(facts)?;
    let selected = variants
        .iter()
        .find(|variant| variant.name == selected)
        .ok_or("Owned error constructor selects another nominal variant")?;
    let VariantShape::Named { fields } = &selected.shape else {
        return Err("Owned error constructor requires declared named payloads".into());
    };

    let [(_, error), (_, what)] = fields.as_slice() else {
        return Err("Owned error constructor requires exactly error and message fields".into());
    };
    if error.position != 0 || what.position != 1 || error.ty != *input || what.ty != *message {
        return Err(
            "Owned error constructor changes complete generated error or message field".into(),
        );
    }
    let resolution = facts.nominal_resolution()?;
    if !resolution.checked_no_drop(wrapper) || !resolution.checked_no_drop(enumeration) {
        return Err("Owned error constructor output lacks whole independent cleanup proof".into());
    }

    Ok(vec![
        Component::ProductField {
            nominal: wrapper.clone(),
            position: 0,
            name: "0".into(),
        },
        Component::EnumPayload {
            nominal: enumeration.clone(),
            variant: selected.name.clone(),
            position: 0,
        },
    ])
}

fn output_parts(
    tree: &TypeTree,
) -> Result<(&Nominal, &Nominal, &[crate::concrete_types::MixedVariant]), String> {
    let TypeTree::Struct {
        nominal, fields, ..
    } = tree
    else {
        return Err("Owned error output is not its original wrapper".into());
    };

    let [field] = fields.as_slice() else {
        return Err("Owned error wrapper must retain its sole field".into());
    };
    let TypeTree::MixedEnum {
        nominal: enumeration,
        variants,
        ..
    } = &field.ty
    else {
        return Err("Owned error output lacks its complete mixed enum".into());
    };
    if field.name != "0" {
        return Err("Owned error wrapper changes its sole declared slot".into());
    }
    Ok((nominal, enumeration, variants))
}

fn selected_variant(facts: &dyn EvidenceFacts) -> Result<String, String> {
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    source::signature(&syntax)?;
    let [syn::Stmt::Expr(syn::Expr::Call(wrapper), None)] = syntax.block.stmts.as_slice() else {
        return Err("Owned error constructor requires its single whole wrapper tail".into());
    };

    let Some(syn::Expr::Struct(variant)) = wrapper.args.first().filter(|_| wrapper.args.len() == 1)
    else {
        return Err("Owned error wrapper requires one fresh named variant".into());
    };
    source::variant(variant).map(|(_, name)| name)
}

pub(crate) fn shape(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::GeneratedStruct { fields, .. }
        if matches!(fields.as_slice(), [field] if field.name == "0" && field.position == 0
            && matches!(&field.ty, TypeTree::Struct { fields, .. }
                if matches!(fields.as_slice(), [leaf]
                    if leaf.position == 0 && matches!(&leaf.ty, TypeTree::Scalar(value) if value == "u64")))))
}

fn cleanup(facts: &dyn EvidenceFacts, tree: &TypeTree) -> Result<(), String> {
    let TypeTree::GeneratedStruct {
        nominal, fields, ..
    } = tree
    else {
        return Err("Owned error requires generated-declaration provenance".into());
    };

    let child = fields[0]
        .ty
        .nominal_identity()
        .ok_or("Owned error loses scalar child identity")?;
    let resolution = facts.nominal_resolution()?;
    if !resolution.checked_no_drop(nominal) || !resolution.checked_no_drop(child) {
        return Err(
            "Owned error requires generated and original child source/cleanup receipts".into(),
        );
    }
    Ok(())
}

pub(super) fn spelling(
    ty: &syn::Type,
    token: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let transfer = checked(facts)?.ok_or("Owned error type lacks its whole constructor context")?;
    let syn::Type::Path(path) = ty else {
        return Err("Owned error requires an original nominal path".into());
    };

    let resolved = facts.nominal_resolution()?.resolve(path)?;
    if resolved != transfer.input || TypeToken::from_tree(&resolved) != *token {
        return Err("Owned error source path differs from exact generated declaration".into());
    }
    Ok(())
}

pub(crate) fn permits_type(text: &str, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !candidate(facts) {
        return Ok(false);
    }
    let Some(transfer) = checked(facts)? else {
        return Ok(false);
    };
    Ok(crate::data_types::parse(text)? == TypeToken::from_tree(&transfer.input))
}
