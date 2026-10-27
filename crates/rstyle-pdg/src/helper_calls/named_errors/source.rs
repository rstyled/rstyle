//! Source-established nominal route from one static string formal into its error.
use super::*;
use crate::{
    copy_tagged_values::origins::{
        Alternative, Case, CaseKind, Component, Condition, FormalRoot, Leaf, Route,
    },
    data_types::TypeToken,
    named_error_payloads::{Constructor, Function},
};

pub(super) fn derive(function: &Function<'_>) -> Result<WireContract, String> {
    if function
        .formals
        .iter()
        .all(|formal| formal.token == TypeToken::Scalar("u32".into()))
    {
        return Ok(empty_contract());
    }

    let formal = function
        .formals
        .iter()
        .find(|formal| formal.token == TypeToken::Shared(Box::new(TypeToken::Str)))
        .ok_or("Fresh error helper lacks its exact static formal")?;
    let route = output_route(function)?;
    let input = vec![Component::SharedReferent];
    Ok(WireContract {
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![input.clone()],
        }],
        output_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![route.clone()],
        }],
        alternatives: vec![Alternative {
            input: CaseKind::Whole,
            output: CaseKind::Whole,
            condition: Condition::Always,
            leaves: vec![Leaf {
                root: FormalRoot {
                    parameter: formal.position as usize,
                    binding: formal.pattern.hir.clone(),
                },
                input,
                output: route,
                guard: None,
            }],
        }],
    })
}

fn output_route(function: &Function<'_>) -> Result<Route, String> {
    let Constructor::Wrapper { token, .. } = &function.wrapper else {
        return Err("Fresh error helper output lacks its checked unary wrapper".into());
    };
    let TypeToken::Struct { nominal, fields } = token else {
        return Err("Fresh error helper output must retain its exact nominal product".into());
    };
    let [(name, _)] = fields.as_slice() else {
        return Err("Fresh error helper output changed its complete wrapper fields".into());
    };

    let mut route = vec![Component::ProductField {
        nominal: nominal.clone(),
        position: 0,
        name: name.clone(),
    }];
    route.extend(variant_route(function)?);
    Ok(route)
}

fn variant_route(function: &Function<'_>) -> Result<Route, String> {
    let Constructor::Named {
        nominal,
        variant,
        fields,
        ..
    } = &function.variant
    else {
        return Err("Fresh error helper output lacks its checked named variant".into());
    };

    let static_fields: Vec<_> = fields
        .iter()
        .filter(|field| field.formal.token == TypeToken::Shared(Box::new(TypeToken::Str)))
        .collect();
    let [field] = static_fields.as_slice() else {
        return Err("Fresh error helper output requires its sole static formal payload".into());
    };
    if !function.formals.iter().any(|formal| {
        formal.position == field.formal.position && formal.pattern.hir == field.formal.pattern.hir
    }) {
        return Err("Fresh error helper output changes its exact static formal".into());
    }
    Ok(vec![
        Component::EnumPayload {
            nominal: nominal.clone(),
            variant: variant.clone(),
            position: field.position,
        },
        Component::SharedReferent,
    ])
}

fn empty_contract() -> WireContract {
    WireContract {
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![],
        }],
        output_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![],
        }],
        alternatives: vec![Alternative {
            input: CaseKind::Whole,
            output: CaseKind::Whole,
            condition: Condition::Always,
            leaves: vec![],
        }],
    }
}

pub(crate) fn empty_scalar_contract(contract: &WireContract) -> bool {
    contract == &empty_contract()
}

/// Shape-only validation never establishes generated source or move authority.
pub(super) fn owned_shape(
    summary: &super::super::Summary,
    contract: &WireContract,
) -> Result<(), String> {
    let input = crate::data_types::parse(&summary.argument_types[0])?;
    let message = TypeToken::Shared(Box::new(TypeToken::Str));
    if !owned_token(&input) || crate::data_types::parse(&summary.argument_types[1])? != message {
        return Err("Owned error helper changes its whole input/message types".into());
    }

    let TypeToken::Struct {
        nominal: wrapper,
        fields,
    } = crate::data_types::parse(&summary.result_type)?
    else {
        return Err("Owned error summary lacks its complete output wrapper".into());
    };
    let [(name, TypeToken::MixedEnum { nominal, variants })] = fields.as_slice() else {
        return Err("Owned error summary changes its unary mixed output".into());
    };
    let [alternative] = contract.alternatives.as_slice() else {
        return Err("Owned error summary requires one unconditional route".into());
    };
    let [leaf] = alternative.leaves.as_slice() else {
        return Err("Owned error summary requires its sole static reference leaf".into());
    };
    let [
        Component::ProductField {
            nominal: root,
            position: 0,
            name: slot,
        },
        Component::EnumPayload {
            nominal: enumeration,
            variant,
            position: 1,
        },
        Component::SharedReferent,
    ] = leaf.output.as_slice()
    else {
        return Err("Owned error summary changes its static message output route".into());
    };
    if name != "0" || slot != name || root != &wrapper || enumeration != nominal
        || leaf.root.parameter != 1 || leaf.input != [Component::SharedReferent] || leaf.guard.is_some()
        || !variants.iter().any(|(name, shape)| name == variant && matches!(shape,
            crate::concrete_types::VariantShape::Named { fields } if matches!(fields.as_slice(), [(_, err), (_, what)] if err == &input && what == &message))) {
        return Err("Owned error summary changes exact nominal field routes or types".into());
    }
    owned_cases(contract, leaf)
}

fn owned_cases(contract: &WireContract, leaf: &Leaf) -> Result<(), String> {
    let expected = WireContract {
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![leaf.input.clone()],
        }],
        output_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![leaf.output.clone()],
        }],
        alternatives: vec![Alternative {
            input: CaseKind::Whole,
            output: CaseKind::Whole,
            condition: Condition::Always,
            leaves: vec![leaf.clone()],
        }],
    };
    if *contract != expected {
        return Err("Owned error summary changes its exhaustive reference cases".into());
    }
    Ok(())
}

fn owned_token(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Struct { fields, .. }
        if matches!(fields.as_slice(), [(name, TypeToken::Struct { fields: child, .. })]
            if name == "0" && matches!(child.as_slice(), [(_, TypeToken::Scalar(value))] if value == "u64")))
}
