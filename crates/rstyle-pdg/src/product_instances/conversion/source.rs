//! Original receiver evaluation and the independent canonical Span identity anchor.
use super::model::{Argument, Definition, Observation, Structural};
use crate::{HirFnFacts, HirNode, concrete_types::TypeTree};
use serde_json::Value;

mod local_from;
mod range;
pub(super) use local_from::verify as join_local_from;
pub(super) use range::verify as range_identity;

pub(super) fn site(canonical: &HirFnFacts) -> Result<(&HirNode, &HirNode), String> {
    let operations =
        crate::field_locations::operations(&canonical.body, &["expr_method_call", "expr_call"]);
    let [call] = operations.as_slice() else {
        return Err("Product conversion requires its complete one-call body inventory".into());
    };
    let [receiver] = call.children.as_slice() else {
        return Err("Product conversion requires exactly one receiver operand".into());
    };
    let parameter = canonical
        .signature
        .params
        .get(1)
        .ok_or("Missing product input parameter")?;
    if call.kind != "expr_method_call"
        || call.expansion.is_some()
        || call.is_unsafe_block
        || receiver.kind != "expr_path"
        || !receiver.children.is_empty()
        || receiver.res.as_ref().is_none_or(|resolution| {
            resolution.kind != "local" || resolution.target.as_ref() != Some(&parameter.pat.hir)
        })
    {
        return Err(
            "Product conversion receiver is not its unchanged original value formal".into(),
        );
    }

    syntax(canonical, call, parameter)?;
    Ok((call, receiver))
}

fn syntax(
    canonical: &HirFnFacts,
    call: &HirNode,
    parameter: &crate::ParamFact,
) -> Result<(), String> {
    let source = canonical
        .source
        .bytes
        .get(call.span.start as usize..call.span.end as usize)
        .ok_or("Missing original product conversion source")?;
    let expression: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Expr::MethodCall(method) = expression else {
        return Err("Product conversion source is not an original method call".into());
    };
    let syn::Expr::Path(receiver) = &*method.receiver else {
        return Err("Product conversion source receiver introduces evaluation".into());
    };

    if call.snippet != source
        || !method.attrs.is_empty()
        || !method.args.is_empty()
        || method.turbofish.is_some()
        || method.method != "into"
        || receiver.qself.is_some()
        || !receiver.attrs.is_empty()
        || !receiver.path.is_ident(parameter.pat.snippet.as_str())
    {
        return Err("Product conversion source loses its single unchanged receiver".into());
    }
    Ok(())
}

pub(super) fn span_identity(
    canonical: &HirFnFacts,
    observed: &Observation,
    span: &TypeTree,
) -> Result<Structural, String> {
    let inventory = canonical
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing independent original generic-call inventory")?;
    let calls = inventory["calls"]
        .as_array()
        .ok_or("Missing original call roster")?;
    let [call] = calls.as_slice() else {
        return Err("Original product call inventory is not complete and singular".into());
    };
    original_call(inventory, call, observed)?;
    let arguments = call["requested_arguments"]
        .as_array()
        .ok_or("Missing original substitutions")?;
    let [generic, concrete] = arguments.as_slice() else {
        return Err("Original Into arguments must retain generic self and concrete Span".into());
    };
    if generic["kind"] != "type"
        || generic["display_only"] != true
        || generic["identity_status"] != "unsupported"
        || generic.get("identity").is_some()
        || generic["reason"].as_str().is_none_or(str::is_empty)
    {
        return Err("Original Into self is not its unresolved generic observation".into());
    }

    let concrete: Argument =
        serde_json::from_value(concrete.clone()).map_err(|error| error.to_string())?;
    concrete.verify(&concrete.identity)?;
    nominal(canonical, &concrete.identity, span)?;
    Ok(concrete.identity)
}

fn original_call(inventory: &Value, call: &Value, observed: &Observation) -> Result<(), String> {
    let requested: Definition =
        serde_json::from_value(call["requested"].clone()).map_err(|error| error.to_string())?;
    let mut expected = observed.into.requested.clone();
    expected.crate_hash = None;

    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
        || call["status"] != "unresolved"
        || call["expanded"] != false
        || requested != expected
        || serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            != Some(&observed.hir)
        || serde_json::from_value::<crate::ByteSpan>(call["span"].clone()).ok()
            != Some(observed.span)
    {
        return Err("Product selection is not bound to its independent original call".into());
    }
    Ok(())
}

fn nominal(canonical: &HirFnFacts, identity: &Structural, span: &TypeTree) -> Result<(), String> {
    local_nominal(canonical, identity, span)?;
    let TypeTree::Struct { fields, .. } = span else {
        return Err("Product conversion Span lacks its actual captured structure".into());
    };
    if fields.len() != 2
        || fields
            .iter()
            .zip(["start", "end"])
            .enumerate()
            .any(|(position, (field, name))| {
                field.position != position
                    || field.name != name
                    || field.ty != TypeTree::Scalar("usize".into())
            })
    {
        return Err("Concrete Span differs from its original nominal/compiler structure".into());
    }
    Ok(())
}

pub(super) fn local_nominal(
    canonical: &HirFnFacts,
    identity: &Structural,
    tree: &TypeTree,
) -> Result<(), String> {
    let (nominal, _) = tree
        .nominal_parts()
        .ok_or("Missing complete product nominal")?;
    let Structural::Adt {
        definition,
        arguments,
    } = identity
    else {
        return Err("Product local nominal requires its complete ADT identity".into());
    };
    if !super::authority::local_compiled(
        canonical,
        definition,
        "Missing original product compiled-crate identity",
    )? || !arguments.is_empty()
        || definition.path != nominal.definition_path
    {
        return Err("Product observation differs from its source-resolved local nominal".into());
    }
    Ok(())
}
