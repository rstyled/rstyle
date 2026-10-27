//! Materialize only the complete independently checked substitution roster.
use super::{
    Result, Site,
    body::{Role, Roles},
    site,
    transport::*,
    types::{self, Types},
};
use crate::{
    HirFnFacts, HirNode, ResolvedTy,
    concrete_types::{InterfaceObservations, Observation},
};
use std::collections::BTreeMap;

pub(super) fn verify(
    original: &HirFnFacts,
    projection: &Projection,
    roles: &Roles,
    types: &Types,
) -> Result<()> {
    types::arguments_match(&projection.arguments, &[types::builtin("u8")])?;
    types.endpoint(&projection.result, Role::Result)?;
    let parameters = original
        .signature
        .params
        .iter()
        .map(|p| &p.pat)
        .collect::<Vec<_>>();

    roster(&projection.parameters, &parameters, roles, types)?;
    if projection.bindings.len() != original.bindings.len() {
        return Err("Projected binding roster differs".into());
    }

    for (record, binding) in projection.bindings.iter().zip(&original.bindings) {
        let role = roles
            .values
            .get(&site(&binding.hir))
            .ok_or("Missing original binding role")?;
        typed(record, &binding.hir, *role, types)?;
        if !record.adjustments.is_empty() || !record.dispatch_operands.is_empty() {
            return Err("Projected binding invents operations".into());
        }
        original_type(&binding.ty, *role, types)?;
    }

    let mut nodes = Vec::new();
    collect(&original.body, &mut nodes);
    roster(&projection.nodes, &nodes, roles, types)?;

    for (index, parameter) in original.signature.params.iter().enumerate() {
        original_type(
            &parameter.ty,
            [Role::Bytes, Role::Usize, Role::Text][index],
            types,
        )?;
    }
    original_type(&original.signature.return_ty, Role::Result, types)
}

fn collect<'a>(node: &'a HirNode, nodes: &mut Vec<&'a HirNode>) {
    if node.ty.is_some() {
        nodes.push(node);
    }
    for child in &node.children {
        collect(child, nodes);
    }
}

fn roster(records: &[Typed], nodes: &[&HirNode], roles: &Roles, types: &Types) -> Result<()> {
    if records.len() != nodes.len() {
        return Err("Projected type roster omits or invents original nodes".into());
    }
    for (record, node) in records.iter().zip(nodes) {
        let role = *roles
            .values
            .get(&site(&node.hir))
            .ok_or("Projected type lacks an independent original role")?;
        typed(record, &node.hir, role, types)?;
        original_type(
            node.ty.as_ref().ok_or("Missing original typed node")?,
            role,
            types,
        )?;
        adjustments(record, node, role, types)?;
        operands(record, node, roles, types)?;
    }
    Ok(())
}

fn typed(record: &Typed, hir: &crate::HirRef, role: Role, types: &Types) -> Result<()> {
    if record.hir != *hir {
        return Err("Projected type record changes original HIR identity/order".into());
    }
    types.endpoint(&record.concrete, role)
}

fn original_type(ty: &ResolvedTy, role: Role, types: &Types) -> Result<()> {
    if role == Role::Bytes {
        return if ty.normalized.is_none() {
            Ok(())
        } else {
            Err("Original generic slice was already concretized".into())
        };
    }
    let expected = types.resolved(ty, role)?;
    if expected.normalized != ty.normalized || role == Role::Never && ty.observed != "!" {
        return Err("Original concrete type disagrees with independent source role".into());
    }
    Ok(())
}

fn adjustments(record: &Typed, node: &HirNode, role: Role, types: &Types) -> Result<()> {
    let expected = if role == Role::Bytes && node.kind == "expr_path" {
        vec![
            Type::Slice {
                element: Box::new(types::builtin("u8")),
            },
            types.expected(Role::Bytes),
        ]
    } else if role == Role::Never {
        vec![types::unit()]
    } else {
        Vec::new()
    };
    if record.adjustments.len() != expected.len() {
        return Err("Projected node adjustment roster differs".into());
    }
    for (actual, expected) in record.adjustments.iter().zip(&expected) {
        if types::identity(actual)? != expected {
            return Err("Projected node adjustment target differs".into());
        }
    }
    Ok(())
}

fn operands(record: &Typed, node: &HirNode, roles: &Roles, types: &Types) -> Result<()> {
    let children = if node.dispatch.is_some() {
        node.children.as_slice()
    } else {
        &[]
    };
    if record.dispatch_operands.len() != children.len() {
        return Err("Projected dispatch operand roster differs".into());
    }
    for (operand, child) in record.dispatch_operands.iter().zip(children) {
        let role = *roles
            .values
            .get(&site(&child.hir))
            .ok_or("Dispatch child lacks source role")?;
        types.endpoint(operand, role)?;
    }
    Ok(())
}

pub(super) fn facts(
    original: &HirFnFacts,
    roles: &Roles,
    types: &Types,
    methods: crate::data_receivers::Capture,
) -> Result<HirFnFacts> {
    Ok(HirFnFacts {
        signature: signature(&original.signature, roles, types)?,
        bindings: original
            .bindings
            .iter()
            .map(|binding| {
                let role = *roles
                    .values
                    .get(&site(&binding.hir))
                    .ok_or("Missing checked binding role")?;
                Ok(crate::BindingFact {
                    ty: types.resolved(&binding.ty, role)?,
                    ..binding.clone()
                })
            })
            .collect::<Result<_>>()?,
        body: node(&original.body, &roles.values, types)?,
        build: build(&original.build, types, methods)?,
        ..original.clone()
    })
}

fn signature(
    original: &crate::SignatureFacts,
    roles: &Roles,
    types: &Types,
) -> Result<crate::SignatureFacts> {
    let params = original
        .params
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            Ok(crate::ParamFact {
                ty: types.resolved(&parameter.ty, [Role::Bytes, Role::Usize, Role::Text][index])?,
                pat: node(&parameter.pat, &roles.values, types)?,
                ..parameter.clone()
            })
        })
        .collect::<Result<_>>()?;

    Ok(crate::SignatureFacts {
        params,
        return_ty: types.resolved(&original.return_ty, Role::Result)?,
        ..original.clone()
    })
}

fn build(
    original: &crate::BuildFacts,
    types: &Types,
    methods: crate::data_receivers::Capture,
) -> Result<crate::BuildFacts> {
    let interface = InterfaceObservations {
        identity: crate::concrete_types::IDENTITY.into(),
        parameters: [Role::Bytes, Role::Usize, Role::Text]
            .into_iter()
            .map(|role| {
                types
                    .tree(role)
                    .map(|ty| Observation::Captured { ty })
                    .ok_or("Missing checked formal type".into())
            })
            .collect::<Result<_>>()?,
        result: Observation::Captured {
            ty: types.result.clone(),
        },
    };

    let mut build = original.clone();
    build.frontend_options.insert_data(interface);
    build.frontend_options.insert_data(methods);
    Ok(build)
}

fn node(original: &HirNode, roles: &BTreeMap<Site, Role>, types: &Types) -> Result<HirNode> {
    let ty = original
        .ty
        .as_ref()
        .map(|ty| {
            let role = *roles
                .get(&site(&original.hir))
                .ok_or("Missing checked materialization role")?;
            types.resolved(ty, role)
        })
        .transpose()?;

    let dispatch = original
        .dispatch
        .as_ref()
        .map(|_| dispatch_overlay(original, ty.as_ref(), roles, types))
        .transpose()?;

    let children = original
        .children
        .iter()
        .map(|child| node(child, roles, types))
        .collect::<Result<_>>()?;

    Ok(HirNode {
        ty,
        dispatch,
        children,
        ..original.clone()
    })
}

fn dispatch_overlay(
    original: &HirNode,
    ty: Option<&ResolvedTy>,
    roles: &BTreeMap<Site, Role>,
    types: &Types,
) -> Result<crate::Dispatch> {
    let dispatch = original
        .dispatch
        .as_ref()
        .ok_or("Missing checked dispatch")?;
    let operand_tys = original
        .children
        .iter()
        .map(|child| {
            let role = *roles
                .get(&site(&child.hir))
                .ok_or("Missing checked dispatch role")?;
            types
                .resolved(
                    child.ty.as_ref().ok_or("Missing dispatch child type")?,
                    role,
                )?
                .normalized
                .ok_or("Unsupported dispatch child".into())
        })
        .collect::<Result<_>>()?;
    let result_ty = ty
        .and_then(|ty| ty.normalized.clone())
        .ok_or("Missing checked dispatch result")?;

    Ok(crate::Dispatch {
        operand_tys,
        result_ty,
        ..dispatch.clone()
    })
}
