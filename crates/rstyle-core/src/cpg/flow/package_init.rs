//! Go package readiness includes variables without an initializer. These steps
//! order declarations, without introducing extra runtime stores.
use super::Derived;
use crate::cpg::model::{FileSpan, Function, NodeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

#[derive(Deserialize)]
struct Group {
    names: Vec<FileSpan>,
    assignment: Option<String>,
    requires: Vec<usize>,
    #[serde(default)]
    interface_selection: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Readiness {
    pub earliest_ready: Vec<usize>,
    pub prerequisites: Vec<Vec<usize>>,
    /// Source groups whose transitive method references include interface
    /// dispatch. The concrete method body cannot add a lexical prerequisite.
    pub interface_selections: Vec<usize>,
    pub zero_only: Vec<usize>,
}

/// Kahn's algorithm with declaration order as the priority of a ready group.
fn selection(groups: &[Group]) -> Option<Vec<usize>> {
    let mut children = vec![Vec::new(); groups.len()];
    let mut pending = Vec::with_capacity(groups.len());
    let mut ready = BTreeSet::new();
    for (i, group) in groups.iter().enumerate() {
        if group.names.is_empty() || (group.assignment.is_none() && !group.requires.is_empty()) {
            return None;
        }
        let mut seen = HashSet::new();
        for &dep in &group.requires {
            if !seen.insert(dep) {
                return None;
            }
            children.get_mut(dep)?.push(i);
        }
        pending.push(seen.len());
        if seen.is_empty() {
            ready.insert(i);
        }
    }
    let mut order = Vec::with_capacity(groups.len());
    while let Some(next) = ready.pop_first() {
        order.push(next);
        for &child in &children[next] {
            pending[child] -= 1;
            if pending[child] == 0 {
                ready.insert(child);
            }
        }
    }
    (order.len() == groups.len()).then_some(order)
}

fn assignments_agree(body: &Body<'_>, root: &str, groups: &[Group], order: &[usize]) -> Option<()> {
    let mut observed = Vec::new();
    for child in &body.node(root).ok()?.children {
        let statement = body.node(child).ok()?;
        if let [action] = statement.children.as_slice() {
            let n = body.node(action).ok()?;
            if n.original_kind == "ValueSpec.initialize" && n.kind == NodeKind::Assignment {
                observed.push(n.id.as_str());
            }
        }
    }
    let expected = order
        .iter()
        .filter_map(|&i| groups[i].assignment.as_deref());
    observed.into_iter().eq(expected).then_some(())
}

pub(crate) fn derive(tables: &Tables<'_>, function: &Function) -> Option<Readiness> {
    if tables.language() != "go"
        || !function
            .signature
            .modifiers
            .iter()
            .any(|m| m == "go:package-initialization")
    {
        return None;
    }
    let raw = tables
        .extension("go:initializer-groups")?
        .get(&function.id)?;
    let groups: Vec<Group> = serde_json::from_value(raw.clone()).ok()?;
    let order = selection(&groups)?;
    let body = Body::new(function).ok()?;
    assignments_agree(&body, &function.root, &groups, &order)?;
    Some(Readiness {
        earliest_ready: order,
        zero_only: groups
            .iter()
            .enumerate()
            .filter_map(|(i, g)| g.assignment.is_none().then_some(i))
            .collect(),
        interface_selections: groups
            .iter()
            .enumerate()
            .filter_map(|(i, g)| g.interface_selection.then_some(i))
            .collect(),
        prerequisites: groups.into_iter().map(|g| g.requires).collect(),
    })
}

pub(crate) fn attach(tables: &Tables<'_>, function: &Function, derived: &mut Derived) {
    derived.package_readiness = derive(tables, function);
}
