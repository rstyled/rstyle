//! Specialize direct parameter equality using the static instance and the
//! actual arguments' dynamic types at a resolved local call.
use super::Derived;
use crate::cpg::model::{Dispatch, Function, Node, NodeKind, Ref, Type, TypeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub comparison: String,
    pub instance: String,
    pub operands: Vec<String>,
    pub dynamic_types: Vec<String>,
    pub outcome: String,
}

fn operand<'a>(body: &Body<'a>, tables: &Tables<'a>, node: &Node) -> Option<&'a Type> {
    let ty = tables.ty(node.ty.as_deref()?).ok()?;
    if ty.kind != TypeKind::Interface {
        return Some(ty);
    }
    if node.kind != NodeKind::Cast || node.original_kind != "CallExpr.conversion" {
        return None;
    }
    let dynamic = tables.ty(body.child(node, 0).ok()?.ty.as_deref()?).ok()?;
    (dynamic.kind != TypeKind::Interface && dynamic.kind != TypeKind::Parameter).then_some(dynamic)
}

fn parameter<'a>(callee: &Body<'a>, node: &Node) -> Option<&'a crate::cpg::model::Parameter> {
    let sym = callee.reference(&node.id)?.target.as_ref()?.id.as_deref()?;
    callee
        .function
        .signature
        .parameters
        .iter()
        .find(|p| p.symbol == sym)
}

fn dynamic_result(t: &Type) -> Option<&'static str> {
    match t.kind {
        TypeKind::Sequence if t.fixed == Some(false) => Some("runtime_panic"),
        TypeKind::Map | TypeKind::Function => Some("runtime_panic"),
        TypeKind::Integer
        | TypeKind::Bool
        | TypeKind::String
        | TypeKind::Float
        | TypeKind::Pointer => Some("comparison_completes"),
        _ => None,
    }
}

fn compare<'a>(
    tables: &Tables<'a>,
    caller: &Body<'a>,
    reference: &Ref,
    callee: &Body<'a>,
) -> Option<Comparison> {
    let [stmt] = callee.root().ok()?.children.as_slice() else {
        return None;
    };
    let stmt = callee.node(stmt).ok()?;
    let ret = if stmt.kind == NodeKind::ExpressionStatement && stmt.original_kind == "Stmt" {
        callee.child(stmt, 0).ok()?
    } else {
        stmt
    };
    if ret.kind != NodeKind::Return {
        return None;
    }
    let op = callee.child(ret, 0).ok()?;
    if !matches!(op.operator.as_deref(), Some("eq" | "ne")) {
        return None;
    }
    let call = caller.node(&reference.node).ok()?;
    let mut values = vec![];
    let mut types = vec![];
    for id in &op.children {
        let p = parameter(callee, callee.node(id).ok()?)?;
        let arg = caller.child(call, p.position + 1).ok()?;
        types.push(operand(caller, tables, arg)?);
        values.push(arg.id.clone());
    }
    let [x, y] = types.as_slice() else {
        return None;
    };
    let formal = tables.ty(callee.child(op, 0).ok()?.ty.as_deref()?).ok()?;
    if formal.kind != TypeKind::Parameter
        || callee.child(op, 1).ok()?.ty.as_ref() != Some(&formal.id)
    {
        return None;
    }
    let inst = reference
        .instance_arguments
        .as_ref()?
        .get(formal.index? as usize)?;
    // For interface instances, equal dynamic types determine whether equality can panic.
    if tables.ty(inst).ok()?.kind != TypeKind::Interface || x.id != y.id {
        return None;
    }
    Some(Comparison {
        comparison: op.id.clone(),
        instance: inst.clone(),
        operands: values,
        dynamic_types: vec![x.id.clone(), y.id.clone()],
        outcome: dynamic_result(x)?.into(),
    })
}

pub(crate) fn attach(tables: &Tables<'_>, f: &Function, derived: &mut Derived) {
    if tables.language() != "go" || !f.refs.iter().any(|r| r.instance_arguments.is_some()) {
        return;
    }
    let Ok(caller) = Body::new(f) else { return };
    super::each_reached(f, derived, |_, reached, own| {
        for r in f.refs.iter().filter(|r| {
            r.dispatch == Some(Dispatch::Static)
                && r.instance_arguments.is_some()
                && reached.contains(r.node.as_str())
        }) {
            let Some(target) = r
                .target
                .as_ref()
                .and_then(|t| t.id.as_deref())
                .and_then(|id| tables.function(id))
            else {
                continue;
            };
            if let Ok(callee) = Body::new(target)
                && let Some(row) = compare(tables, &caller, r, &callee)
            {
                own.generic_comparisons.insert(r.node.clone(), row);
            }
        }
    });
}
