//! Output events retain operand order at the call site. The representation does not
//! assume a destination stream or an implementation's rendering of the arguments.
use super::{Derived, each_reached};
use crate::cpg::model::{
    DefinitionKind, Dispatch, Function, Node, NodeKind, RefStatus, TargetKind,
};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Output {
    pub arguments: Vec<String>,
    /// A guaranteed delimiter, when the operation specifies one.
    pub separator: Option<String>,
    pub terminator: Option<String>,
}

fn output(tables: &Tables<'_>, body: &Body<'_>, call: &Node) -> Option<Output> {
    let (callee, args) = call.children.split_first()?;
    let reference = body.reference(&call.id)?;
    let name = body.reference(callee)?;
    let target = reference.target.as_ref()?;
    if call.kind != NodeKind::Call
        || reference.dispatch != Some(Dispatch::Builtin)
        || reference.status != RefStatus::Resolved
        || name.status != RefStatus::Resolved
        || target.kind != TargetKind::Definition
        || name.target.as_ref()?.kind != TargetKind::Definition
        || target.id != name.target.as_ref()?.id
    {
        return None;
    }
    let def = tables.definition(target.id.as_deref()?).ok()?;
    if !def.standard || def.kind != DefinitionKind::Function {
        return None;
    }
    let delimited = match def.operation.as_deref()? {
        "go:print" => false,
        "go:println" => true,
        _ => return None,
    };
    Some(Output {
        arguments: args.to_vec(),
        separator: delimited.then(|| " ".to_owned()),
        terminator: delimited.then(|| "\n".to_owned()),
    })
}

pub(crate) fn attach(tables: &Tables<'_>, function: &Function, derived: &mut Derived) {
    if tables.language() != "go"
        || !function
            .refs
            .iter()
            .any(|r| r.dispatch == Some(Dispatch::Builtin))
    {
        return;
    }
    let Ok(body) = Body::new(function) else {
        return;
    };
    let events: Vec<_> = function
        .nodes
        .iter()
        .filter_map(|n| Some((&n.id, output(tables, &body, n)?)))
        .collect();
    if !events.is_empty() {
        each_reached(function, derived, |_, reached, own| {
            for (id, event) in &events {
                if reached.contains(id.as_str())
                    && event.arguments.iter().all(|a| reached.contains(a.as_str()))
                {
                    own.output.insert((*id).clone(), event.clone());
                }
            }
        });
    }
}
