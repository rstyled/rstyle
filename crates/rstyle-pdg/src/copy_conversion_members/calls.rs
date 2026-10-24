//! Exact explicit same-unit conversion calls; helper effects and saved graphs stay separate.
use super::{Direction, Observation, authority};
use crate::{HirNode, concrete_types::TypeTree, tryfrom_calls::model, unit_store::EvidenceFacts};
use serde_json::Value;

mod link;
#[cfg(test)]
mod tests;
pub(crate) use link::selected_resolution;

pub(crate) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    if !crate::helper_calls::same_build(caller, callee)? {
        return Err("Conversion call changes its exact semantic build or selected unit".into());
    }
    let resolution = callee.nominal_resolution()?;
    let checked = resolution
        .copy_conversion_source(callee.facts())?
        .ok_or("Conversion call lacks its retained checked original callee")?;

    let observation = owning_observation(callee)?;
    let call = original_call(node, caller)?;
    selection(&call, &observation)?;
    selected_owner(&call.selected, caller, callee)?;
    let direction = match checked.output() {
        TypeTree::MixedEnum { .. } => Direction::Wrap,
        TypeTree::Result { .. } => Direction::Unwrap,
        _ => return Err("Conversion call has another complete output shape".into()),
    };
    let pins = authority::reviewed(caller)?;
    pins.conversion(direction, &observation)?;
    let expected = Interface {
        input: checked.input(),
        output: checked.output(),
        direction,
    };
    requested_node(node, &call, caller)?;
    expected.dispatch(node, &call.signature)?;
    expected.source(node, caller)
}

fn owning_observation(callee: &dyn EvidenceFacts) -> Result<Observation, String> {
    let value = callee
        .build
        .frontend_options
        .get(crate::tryfrom_calls::KEY)
        .ok_or("Missing original conversion owner observation")?;
    if value["status"] != "observed" {
        return Err("Conversion callee has no observed original member".into());
    }
    serde_json::from_value(value["observation"].clone()).map_err(|error| error.to_string())
}

fn original_call(node: &HirNode, caller: &dyn EvidenceFacts) -> Result<model::Call, String> {
    let inventory = caller
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Conversion caller lacks its original call inventory")?;
    inventory_call(node, inventory)
}

fn inventory_call(node: &HirNode, inventory: &Value) -> Result<model::Call, String> {
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Conversion caller requires a complete original call inventory".into());
    }

    let rows = inventory["calls"]
        .as_array()
        .ok_or("Missing conversion call roster")?;
    let mut matching = rows.iter().filter(|row| {
        serde_json::from_value::<crate::HirRef>(row["hir"].clone())
            .ok()
            .as_ref()
            == Some(&node.hir)
            || serde_json::from_value::<crate::ByteSpan>(row["span"].clone()).ok()
                == Some(node.span)
    });
    let row = matching
        .next()
        .ok_or("Missing original conversion callsite")?;
    if matching.next().is_some() {
        return Err("Duplicated or ambiguous original conversion callsite".into());
    }
    let call: model::Call =
        serde_json::from_value(row.clone()).map_err(|error| error.to_string())?;
    if call.hir != node.hir
        || call.span != node.span
        || call.expanded
        || call.status != "resolved"
        || call.compiled_dependency.is_some()
        || node.kind != "expr_call"
        || node.expansion.is_some()
    {
        return Err("Conversion call is not an exact original local static selection".into());
    }
    Ok(call)
}

fn selection(call: &model::Call, owner: &Observation) -> Result<(), String> {
    if call.requested != owner.trait_method
        || call.selected != owner.method
        || call.selected != owner.selected
        || !call.selected.local
        || !call.selected_arguments.is_empty()
        || !owner.selected_arguments.is_empty()
        || call.requested_arguments.len() != owner.trait_arguments.len()
    {
        return Err("Conversion call differs from its exact selected owning member".into());
    }
    for (actual, expected) in call.requested_arguments.iter().zip(&owner.trait_arguments) {
        if actual.known()? != expected.known()? {
            return Err(
                "Conversion call changes concrete structural Self/input substitutions".into(),
            );
        }
    }
    Ok(())
}

fn selected_owner(
    selected: &model::Definition,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    let caller_owner = super::verify::Owner::from_facts(caller.facts())?;
    let callee_owner = super::verify::Owner::from_facts(callee.facts())?;
    selected_context(selected, &caller_owner, &callee_owner)
}

fn selected_context(
    selected: &model::Definition,
    caller_owner: &super::verify::Owner<'_>,
    callee_owner: &super::verify::Owner<'_>,
) -> Result<(), String> {
    if !selected.local
        || selected.path != callee_owner.name
        || selected.definition_identity != callee_owner.identity
        || selected.crate_name != callee_owner.crate_name
        || selected.stable_crate_id != callee_owner.stable_crate_id
        || selected.crate_name != caller_owner.crate_name
        || selected.stable_crate_id != caller_owner.stable_crate_id
    {
        return Err(
            "Conversion selected body differs from original same-unit compiler owner".into(),
        );
    }
    Ok(())
}

fn requested_node(
    node: &HirNode,
    call: &model::Call,
    caller: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry = crate::data_library::registry_for(caller)
        .ok_or("Unpinned conversion call compiler/target")?;
    requested_resolution(node, call, &registry.core_hash)
}

fn requested_resolution(node: &HirNode, call: &model::Call, core_hash: &str) -> Result<(), String> {
    let resolution = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "resolved_callee" && resolution.target.is_none())
        .ok_or("Conversion call lacks its original requested core resolution")?;
    let observed: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    if observed.path != call.requested.path
        || observed.crate_name != call.requested.crate_name
        || observed.definition_hash != call.requested.definition_identity
        || observed.crate_hash != core_hash
        || observed
            .implementation
            .as_ref()
            .is_some_and(|implementation| {
                implementation.path != call.selected.path
                    || implementation.crate_name != call.selected.crate_name
                    || implementation.definition_hash != call.selected.definition_identity
                    || implementation.crate_hash != "local-unencoded"
                    || implementation.implementation.is_some()
            })
    {
        return Err("Conversion call node contradicts its exact requested core member".into());
    }
    Ok(())
}

struct Interface<'a> {
    input: &'a TypeTree,
    output: &'a TypeTree,
    direction: Direction,
}

impl Interface<'_> {
    fn dispatch(&self, node: &HirNode, display: &Value) -> Result<(), String> {
        let [operand] = node.children.as_slice() else {
            return Err("Conversion call requires one evaluated whole-value input".into());
        };
        let input = crate::data_types::TypeToken::from_tree(self.input).text()?;
        let output = crate::data_types::TypeToken::from_tree(self.output).text()?;
        let dispatch = node
            .dispatch
            .as_ref()
            .ok_or("Missing conversion call dispatch")?;

        let resolution: crate::data_library::Callee = serde_json::from_str(
            &node
                .res
                .as_ref()
                .ok_or("Missing requested conversion member")?
                .description,
        )
        .map_err(|error| error.to_string())?;
        if dispatch.kind != "resolved_direct"
            || dispatch.op != resolution.path
            || dispatch.operand_tys != [input.clone()]
            || dispatch.result_ty != output
            || operand.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(input.as_str())
            || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(output.as_str())
        {
            return Err("Conversion call differs from exact callee input/output types".into());
        }
        display_signature(display, operand, node)
    }

    fn source(&self, node: &HirNode, caller: &dyn EvidenceFacts) -> Result<(), String> {
        let expression = crate::source_evidence::original_expression(node, caller)?;
        let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&expression) else {
            return Err("Conversion requires an explicit original static call".into());
        };
        let method = match self.direction {
            Direction::Wrap => "from",
            Direction::Unwrap => "try_from",
        };
        let path = static_path(call, method)?;
        let [operand] = node.children.as_slice() else {
            return Err("Missing conversion operand".into());
        };
        let arguments: Vec<_> = call.args.iter().collect();
        let [argument] = arguments.as_slice() else {
            return Err("Conversion source changes argument count".into());
        };
        crate::source_evidence::verify_child_source(argument, operand, node, caller)?;

        let mut owner_path = path.clone();
        owner_path.path.segments.pop();
        owner_path.path.segments.pop_punct();
        let owner = match (self.direction, self.output) {
            (Direction::Wrap, output) => output,
            (Direction::Unwrap, TypeTree::Result { ok, .. }) => ok.as_ref(),
            _ => return Err("Missing concrete conversion source owner".into()),
        };
        let nominal = owner
            .nominal_parts()
            .ok_or("Conversion source owner is not nominal")?
            .0;
        caller.nominal_resolution()?.matches_nominal(
            &syn::TypePath {
                qself: None,
                path: owner_path.path,
            },
            nominal,
        )
    }
}

fn display_signature(display: &Value, operand: &HirNode, call: &HirNode) -> Result<(), String> {
    let input = operand
        .ty
        .as_ref()
        .ok_or("Missing conversion argument display")?
        .observed
        .as_str();
    let output = call
        .ty
        .as_ref()
        .ok_or("Missing conversion output display")?
        .observed
        .as_str();
    let Some(arguments) = display["inputs"].as_array() else {
        return Err("Missing conversion call signature inputs".into());
    };
    let [argument] = arguments.as_slice() else {
        return Err("Conversion call signature arity differs".into());
    };
    let same = |actual: &Value, expected: &str| {
        actual.as_str().is_some_and(|actual| {
            actual.replace("std::", "core::") == expected.replace("std::", "core::")
        })
    };

    if display["display_only"] != true
        || !same(argument, input)
        || !same(&display["output"], output)
    {
        return Err("Conversion display signature contradicts its independently typed call".into());
    }
    Ok(())
}

fn static_path<'a>(call: &'a syn::ExprCall, method: &str) -> Result<&'a syn::ExprPath, String> {
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return Err("Conversion call lacks its original concrete owner path".into());
    };
    if !call.attrs.is_empty()
        || !path.attrs.is_empty()
        || path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() < 2
        || path
            .path
            .segments
            .iter()
            .any(|segment| !segment.arguments.is_empty())
        || path
            .path
            .segments
            .last()
            .is_none_or(|segment| segment.ident != method)
    {
        return Err("Conversion source changes original static owner/member/substitutions".into());
    }
    Ok(path)
}
