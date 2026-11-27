//! Recover each possible communication and the destinations of selected
//! receives. Intrinsic readiness is known for nil and directly made channels;
//! other channels may change concurrently, so receive or send readiness can vary.

use super::Derived;
use crate::cpg::model::{Function, Node, NodeKind};
use crate::cpg::tree::Body;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub when_none: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub cases: BTreeMap<String, Communication>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Communication {
    pub node: String,
    pub channel: String,
    pub readiness: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_value: Option<String>,
    /// Destination of each receive result, in component order (value, then ok).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receive_targets: Vec<String>,
}

fn communication<'a>(body: &Body<'a>, arm: &Node) -> Option<(&'a Node, Vec<String>)> {
    let header = body.child(arm, 1).ok()?;
    if header.original_kind != "select.header" {
        return None;
    }
    let mut stmt = body.child(header, 0).ok()?;
    if stmt.kind == NodeKind::ExpressionStatement {
        stmt = body.child(stmt, 0).ok()?;
    }
    if matches!(stmt.kind, NodeKind::Assignment | NodeKind::LocalDeclaration) {
        let lhs = body.child(stmt, 0).ok()?;
        let receive = body.child(stmt, 1).ok()?;
        if receive.operator.as_deref() != Some("go:receive") {
            return None;
        }
        let targets = if matches!(lhs.kind, NodeKind::Tuple | NodeKind::PatternTuple) {
            lhs.children.clone()
        } else {
            vec![lhs.id.clone()]
        };
        return Some((receive, targets));
    }
    Some((stmt, Vec::new()))
}

fn fallback(cases: &BTreeMap<String, Communication>, has_default: bool) -> &'static str {
    if cases.values().any(|c| c.readiness == "ready") {
        "impossible"
    } else if has_default {
        "default"
    } else if cases.values().all(|c| c.readiness == "disabled") {
        "permanent_block"
    } else {
        "suspend"
    }
}

fn decision(
    body: &Body<'_>,
    node: &Node,
    availability: Option<&serde_json::Value>,
    reached: &HashSet<&str>,
) -> Option<Selection> {
    let mut default = None;
    let mut cases = BTreeMap::new();
    for id in node.children.iter().skip(1) {
        let arm = body.node(id).ok()?;
        let pattern = body.child(arm, 0).ok()?;
        if pattern.original_kind == "select.implicit-pattern" {
            continue;
        }
        if pattern.original_kind == "Clause.default" {
            default = Some(arm.id.clone());
            continue;
        }
        let (comm, receive_targets) = communication(body, arm)?;
        let (channel, sent_value) = match (comm.operator.as_deref(), &comm.children[..]) {
            (Some("go:receive"), [ch]) => (ch.clone(), None),
            (Some("go:send"), [ch, val]) => (ch.clone(), Some(val.clone())),
            _ => return None,
        };
        let fact = availability
            .and_then(|v| v.get(&comm.id))
            .and_then(|v| v.as_str());
        let reachable = reached.contains(comm.id.as_str());
        let readiness = match (fact, reachable) {
            (Some("ready_buffered_send"), true) => "ready",
            (Some("nil_channel" | "isolated_channel"), false) => "disabled",
            (None, true) => "depends_on_channel",
            _ => return None,
        };
        cases.insert(
            arm.id.clone(),
            Communication {
                node: comm.id.clone(),
                channel,
                readiness: readiness.into(),
                sent_value,
                receive_targets,
            },
        );
    }
    let when_none = fallback(&cases, default.is_some());
    Some(Selection {
        when_none: when_none.into(),
        default,
        cases,
    })
}

pub(crate) fn attach(function: &Function, derived: &mut Derived) {
    let heads: Vec<_> = function
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Match && n.original_kind == "SelectStmt")
        .collect();
    if heads.is_empty() {
        return;
    }
    let Ok(body) = Body::new(function) else {
        return;
    };
    let available = function
        .extensions
        .as_ref()
        .and_then(|e| e.get("go:channel-availability"));
    super::each_reached(function, derived, |_, reached, own| {
        for head in &heads {
            let Some(header) = head.children.first() else {
                continue;
            };
            if reached.contains(header.as_str())
                && let Some(row) = decision(&body, head, available, &reached)
            {
                own.select_cases.insert(header.clone(), row);
            }
        }
    });
}
