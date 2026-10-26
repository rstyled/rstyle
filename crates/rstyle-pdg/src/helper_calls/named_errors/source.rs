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
    if function.formals.len() == 2 {
        return Ok(empty_contract());
    }
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
                    parameter: function.formals[0].position as usize,
                    binding: function.formals[0].pattern.hir.clone(),
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
    let [field] = fields.as_slice() else {
        return Err("Fresh error helper output requires its sole static formal payload".into());
    };
    if field.formal.position != function.formals[0].position
        || field.formal.pattern.hir != function.formals[0].pattern.hir
    {
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
