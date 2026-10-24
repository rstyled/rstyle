//! Every node slot has an exact shape, including absent optional evidence.
use super::{Result, Role};
use crate::{BindingFact, HirNode};

#[derive(Clone, Copy)]
pub(super) struct Shape {
    kind: &'static str,
    children: usize,
    pub role: Option<Role>,
    resolution: Option<&'static str>,
    method: bool,
    pattern: bool,
    parameter: Option<u32>,
    literal: Option<&'static str>,
}

impl Shape {
    pub fn value(kind: &'static str, children: usize, role: Role) -> Self {
        Self {
            kind,
            children,
            role: Some(role),
            resolution: None,
            method: false,
            pattern: false,
            parameter: None,
            literal: None,
        }
    }
    pub fn statement() -> Self {
        Self {
            role: None,
            ..Self::value("stmt_let", 2, Role::Bytes)
        }
    }
    pub fn binding(role: Role, parameter: Option<u32>) -> Self {
        Self {
            pattern: true,
            parameter,
            ..Self::value("pat_binding", 0, role)
        }
    }
    pub fn method(role: Role) -> Self {
        Self {
            method: true,
            resolution: Some("resolved_callee"),
            ..Self::value("expr_method_call", 1, role)
        }
    }
    pub fn path(role: Role) -> Self {
        Self {
            resolution: Some("local"),
            ..Self::value("expr_path", 0, role)
        }
    }
    pub fn variant() -> Self {
        Self {
            resolution: Some("resolved_fieldless_variant"),
            ..Self::value("expr_path", 0, Role::Anchored)
        }
    }
    pub fn literal(role: Role, literal: &'static str) -> Self {
        Self {
            literal: Some(literal),
            ..Self::value("expr_lit", 0, role)
        }
    }
}

pub(super) fn verify(node: &HirNode, shape: Shape) -> Result<()> {
    if node.kind != shape.kind
        || node.children.len() != shape.children
        || node.ty.is_some() != shape.role.is_some()
        || node.is_unsafe_block
        || node.expansion.is_some()
        || node.param_index != shape.parameter
        || node.lit.as_deref() != shape.literal
        || node.binding.is_some() != shape.pattern
        || node.binding_mut != shape.pattern.then_some(false)
        || node.dispatch.is_some() != shape.method
        || node.res.as_ref().map(|resolution| resolution.kind.as_str()) != shape.resolution
        || node.hir.krate != 0
    {
        return Err(
            "Constructor complete HIR node has missing, extra or contradictory evidence".into(),
        );
    }
    if shape.resolution != Some("local")
        && node.res.as_ref().is_some_and(|res| res.target.is_some())
    {
        return Err("Constructor nonlocal HIR operation invents a local target".into());
    }
    Ok(())
}

pub(super) fn binding(
    binding: &BindingFact,
    node: &HirNode,
    parameter: bool,
    scope: String,
) -> Result<()> {
    if binding.hir != node.hir
        || binding.decl_span != node.span
        || binding.is_param != parameter
        || binding.is_mut
        || binding.scope != scope
        || binding.name != "haystack"
        || node.binding.as_ref() != Some(&binding.symbol)
        || node.ty.as_ref() != Some(&binding.ty)
        || binding.symbol.is_empty()
    {
        return Err(
            "Constructor complete binding inventory differs from original declarations".into(),
        );
    }
    Ok(())
}
