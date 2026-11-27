//! Conditional destinations for concrete dynamic types at a Go type switch, at
//! the calling instantiation. A missing row makes no assertion about that type.

use super::{Derived, Flow};
use crate::cpg::model::{Dispatch, Function, Node, NodeKind, Ref, RefStatus, TypeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeSwitch {
    pub call: String,
    pub switch_node: String,
    pub dynamic_type: String,
    pub arm: String,
}

// Materialize concrete identities, excluding interfaces, whose implementation
// sets require a different query. A depth bound also handles malformed cycles.
fn concrete(tables: &Tables<'_>, id: &str, args: &[String], depth: u8) -> Option<String> {
    let next = depth.checked_sub(1)?;
    let ty = tables.ty(id).ok()?;
    if ty.kind == TypeKind::Interface {
        return None;
    }
    if let Some(def) = &ty.definition {
        if ty.spelling.contains('[') {
            return None;
        }
        return ty
            .arguments
            .as_deref()
            .unwrap_or_default()
            .is_empty()
            .then(|| def.clone());
    }
    match ty.kind {
        TypeKind::Parameter => concrete(tables, args.get(ty.index? as usize)?, &[], next),
        TypeKind::Sequence => {
            let element = concrete(tables, ty.element.as_deref()?, args, next)?;
            let length = if ty.fixed? {
                ty.length?.to_string()
            } else {
                String::new()
            };
            Some(format!("[{length}]{element}"))
        }
        TypeKind::Reference | TypeKind::Pointer => {
            concrete(tables, ty.target.as_deref()?, args, next).map(|t| format!("*{t}"))
        }
        TypeKind::Integer | TypeKind::Float | TypeKind::String | TypeKind::Bool => Some(
            match ty.spelling.as_str() {
                "byte" => "uint8",
                "rune" => "int32",
                other => other,
            }
            .to_owned(),
        ),
        _ => None,
    }
}

fn alternatives<'a>(body: &'a Body<'a>, arm: &Node) -> Option<Vec<&'a Node>> {
    let mut pattern = body.node(arm.children.first()?).ok()?;
    if pattern.kind == NodeKind::PatternAlias {
        pattern = body.node(pattern.children.get(1)?).ok()?;
    }
    match pattern.kind {
        NodeKind::PatternWildcard => Some(Vec::new()),
        NodeKind::PatternOr => pattern.children.iter().map(|n| body.node(n).ok()).collect(),
        _ => Some(vec![pattern]),
    }
}

fn selected<'a>(
    tables: &Tables<'_>,
    body: &'a Body<'a>,
    switch: &Node,
    args: &[String],
) -> BTreeMap<String, &'a str> {
    let mut selected = BTreeMap::new();
    // A previous unknown or interface case could already match. Retain only
    // destinations whose earlier alternatives were understood.
    for id in switch.children.iter().skip(1) {
        let Ok(arm) = body.node(id) else { break };
        let Some(patterns) = alternatives(body, arm) else {
            break;
        };
        for pattern in patterns {
            let ty = match (pattern.kind, pattern.literal.as_deref()) {
                (NodeKind::PatternLiteral, Some("nil")) => Some("nil".to_string()),
                (NodeKind::PatternConstructor, _) => pattern
                    .ty
                    .as_deref()
                    .and_then(|id| concrete(tables, id, args, 48)),
                _ => None,
            };
            let Some(ty) = ty else { return selected };
            selected.entry(ty).or_insert(&arm.id);
        }
    }
    selected
}

pub(crate) fn attach(tables: &Tables<'_>, caller: &Function, flow: &mut Derived) {
    if tables.language() != "go" {
        return;
    }
    if !caller
        .refs
        .iter()
        .any(|r| r.dispatch == Some(Dispatch::Static))
    {
        return;
    }
    super::each_reached(caller, flow, |_, reached, own| {
        for reference in &caller.refs {
            if !reached.contains(reference.node.as_str()) {
                continue;
            }
            if reference.dispatch != Some(Dispatch::Static)
                || reference.status != RefStatus::Resolved
            {
                continue;
            }
            let Some(callee) = reference
                .target
                .as_ref()
                .and_then(|r| r.id.as_deref())
                .and_then(|id| tables.function(id))
            else {
                continue;
            };
            attach_callee(tables, reference, callee, own);
        }
    });
}

fn attach_callee(tables: &Tables<'_>, reference: &Ref, callee: &Function, own: &mut Derived) {
    let switches = callee
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Match && n.original_kind == "TypeSwitchStmt");
    let mut switches = switches.peekable();
    if switches.peek().is_none() {
        return;
    };
    let Ok(body) = Body::new(callee) else {
        return;
    };
    let args = reference.instance_arguments.as_deref().unwrap_or_default();
    let Some(callee_flow) = Flow::of(callee) else {
        return;
    };
    let reached = callee_flow.reached();
    for switch in switches.filter(|n| {
        n.children
            .first()
            .is_some_and(|guard| reached.contains(guard.as_str()))
    }) {
        own.type_switches
            .extend(selected(tables, &body, switch, args).into_iter().map(
                |(dynamic_type, arm)| TypeSwitch {
                    call: reference.node.clone(),
                    switch_node: switch.id.clone(),
                    dynamic_type,
                    arm: arm.to_owned(),
                },
            ));
    }
}
