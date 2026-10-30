//! Temporary compiler namespaces for discovery, restored before reporting or review.
use crate::clones::identity::UnitId;
use rstyle_core::facts::{FnFacts, Pdg, PdgNode, SigFacts, TyShape};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn qualify(
    facts: &mut [FnFacts],
    unit: UnitId,
    scopes: &BTreeMap<String, UnitId>,
    workspace_roots: &BTreeSet<String>,
) {
    let local = facts
        .iter()
        .map(|facts| root(&facts.id).to_owned())
        .collect();
    let names = Names::Qualify {
        unit,
        local,
        scopes,
        workspace_roots,
    };

    for facts in facts {
        facts.id = format!("@u{}::{}", unit.ordinal(), facts.id);
        visit_metadata(facts, &names);
    }
}

pub(super) fn root(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

pub(super) fn restore_id(id: &mut String) {
    let Some((scope, original)) = id.split_once("::") else {
        return;
    };
    let ordinal = scope
        .strip_prefix("@u")
        .or_else(|| scope.strip_prefix("@missing"));
    if ordinal.is_some_and(|number| number.parse::<u32>().is_ok()) {
        *id = original.to_owned();
    }
}

pub(super) fn restore_facts(facts: &mut FnFacts) {
    restore_id(&mut facts.id);
    visit_metadata(facts, &Names::Restore);
}

/// A known mapping keeps traversal independent of indirect mutable callbacks.
enum Names<'a> {
    Qualify {
        unit: UnitId,
        local: BTreeSet<String>,
        scopes: &'a BTreeMap<String, UnitId>,
        workspace_roots: &'a BTreeSet<String>,
    },
    Restore,
}

impl Names<'_> {
    fn rename(&self, id: &mut String) {
        let Self::Qualify {
            unit,
            local,
            scopes,
            workspace_roots,
        } = self
        else {
            restore_id(id);
            return;
        };
        let namespace = root(id);
        if matches!(namespace, "core" | "alloc" | "std") {
            return;
        }
        let target = if local.contains(namespace) {
            Some(*unit)
        } else {
            scopes.get(namespace).copied()
        };

        if let Some(target) = target {
            *id = format!("@u{}::{id}", target.ordinal());
        } else if workspace_roots.contains(namespace) {
            *id = format!("@missing{}::{id}", unit.ordinal());
        }
    }

    fn map_type(&self, ty: TyShape) -> TyShape {
        match ty {
            TyShape::Adt { id, args } => TyShape::Adt {
                id: self.map_id(id),
                args: self.map_types(args),
            },
            TyShape::Dyn(id) => TyShape::Dyn(self.map_id(id)),
            TyShape::Ref { mutable, inner } => TyShape::Ref {
                mutable,
                inner: self.map_inner(inner),
            },
            TyShape::Slice(inner) => TyShape::Slice(self.map_inner(inner)),
            TyShape::Array(inner) => TyShape::Array(self.map_inner(inner)),
            TyShape::Tuple(types) => TyShape::Tuple(self.map_types(types)),
            other => other,
        }
    }

    fn map_id(&self, mut id: String) -> String {
        self.rename(&mut id);
        id
    }

    fn map_inner(&self, mut inner: Box<TyShape>) -> Box<TyShape> {
        replace_type(&mut inner, self);
        inner
    }

    fn map_types(&self, mut types: Vec<TyShape>) -> Vec<TyShape> {
        for ty in &mut types {
            replace_type(ty, self);
        }
        types
    }
}

fn visit_metadata(facts: &mut FnFacts, names: &Names<'_>) {
    visit_links(facts, names);
    optional_signature(&mut facts.sig, names);
    visit_graph(&mut facts.pdg, names);
}

fn visit_links(facts: &mut FnFacts, names: &Names<'_>) {
    optional_id(&mut facts.implements, names);
    visit_edges(&mut facts.edges, names);
    visit_ids(&mut facts.mutated_types, names);
}

fn visit_edges(edges: &mut [rstyle_core::facts::Edge], names: &Names<'_>) {
    for edge in edges {
        names.rename(&mut edge.callee);
    }
}

fn visit_ids(ids: &mut [String], names: &Names<'_>) {
    for id in ids {
        names.rename(id);
    }
}

fn visit_graph(graph: &mut Option<Pdg>, names: &Names<'_>) {
    for node in graph.iter_mut().flat_map(|graph| &mut graph.nodes) {
        visit_node(node, names);
    }
}

fn visit_node(node: &mut PdgNode, names: &Names<'_>) {
    optional_id(&mut node.callee_id, names);
    optional_type(&mut node.ty, names);
    optional_signature(&mut node.callee_sig, names);
}

fn optional_id(id: &mut Option<String>, names: &Names<'_>) {
    if let Some(id) = id {
        names.rename(id);
    }
}

fn optional_signature(signature: &mut Option<SigFacts>, names: &Names<'_>) {
    if let Some(signature) = signature {
        visit_signature(signature, names);
    }
}

fn visit_signature(signature: &mut SigFacts, names: &Names<'_>) {
    optional_id(&mut signature.self_ty, names);
    visit_parameter_types(signature, names);
    replace_type(&mut signature.ret, names);
}

fn visit_parameter_types(signature: &mut SigFacts, names: &Names<'_>) {
    for ty in signature.self_args.iter_mut().chain(&mut signature.params) {
        replace_type(ty, names);
    }
}

fn optional_type(ty: &mut Option<TyShape>, names: &Names<'_>) {
    if let Some(ty) = ty {
        replace_type(ty, names);
    }
}

fn replace_type(ty: &mut TyShape, names: &Names<'_>) {
    let original = std::mem::replace(ty, TyShape::Other);
    *ty = names.map_type(original);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clones::{capture, identity};
    use rstyle_core::facts::{EdgeKind, PdgEdge};
    use serde_json::json;

    fn units() -> Vec<UnitId> {
        let units: Vec<capture::Unit> = ["caller", "shared"]
            .into_iter()
            .map(|name| {
                serde_json::from_value(json!({
                    "extraction_profile": "r0",
                    "target": {
                        "package": name, "manifest_dir": format!("/{name}"),
                        "source": format!("/{name}/src/lib.rs"), "name": name,
                        "kind": ["lib"], "test": false
                    },
                    "test": false, "invocation": [], "directory": format!("/run/{name}"),
                    "artifacts": []
                }))
                .unwrap()
            })
            .collect();
        identity::assign(&units)
            .unwrap()
            .into_iter()
            .map(|assignment| assignment.context.id)
            .collect()
    }

    fn fact(id: &str) -> FnFacts {
        serde_json::from_value(json!({
            "id": id, "path": "src/lib.rs", "line": 1, "column": 1, "name": id,
            "params": 0, "has_self": false, "statements": 1,
            "source": "fn original() {}", "preceding_line": "",
            "mutations": [], "calls": [], "effects": [], "edges": [], "mutated_types": [],
            "sig": {
                "self_ty": "shared::Value", "receiver": "ref", "params": [],
                "ret": {"prim": "u32"}
            },
            "pdg": {"nodes": [
                {"kind": "param", "ty": {"adt": {"id": "shared::Value", "args": []}}},
                {"kind": "call", "callee_id": "external::Trait::read"}
            ], "edges": []}
        }))
        .unwrap()
    }

    fn pair() -> [FnFacts; 2] {
        let mut caller = fact("caller::read");
        caller.sig = None;
        caller.pdg.as_mut().unwrap().edges.push(PdgEdge {
            from: 0,
            to: 1,
            kind: EdgeKind::Data,
            arg_pos: 0,
        });
        let mut implementation = fact("shared::{impl#0}::read");
        implementation.implements = Some("external::Trait::read".into());
        caller.edges.push(rstyle_core::facts::Edge {
            callee: implementation.id.clone(),
            line: 1,
            dynamic: false,
            args: Vec::new(),
        });
        [caller, implementation]
    }

    #[test]
    fn unresolved_nominal_owner_cannot_devirtualize_to_an_unrelated_workspace_type() {
        let units = units();
        let [mut caller, mut implementation] = pair();
        let roots = BTreeSet::from(["caller".into(), "shared".into()]);
        qualify(
            std::slice::from_mut(&mut caller),
            units[0],
            &BTreeMap::new(),
            &roots,
        );
        qualify(
            std::slice::from_mut(&mut implementation),
            units[1],
            &BTreeMap::new(),
            &roots,
        );
        let prepared = rstyle_core::prep::prepare_owned(vec![caller, implementation]);
        let call = &prepared[0].pdg.as_ref().unwrap().nodes[1];
        assert_eq!(call.callee_id.as_deref(), Some("external::Trait::read"));
        assert_ne!(
            prepared[0].pdg.as_ref().unwrap().nodes[0].ty,
            prepared[1].pdg.as_ref().unwrap().nodes[0].ty
        );
    }

    #[test]
    fn routed_nominal_owner_and_function_links_join_the_captured_dependency() {
        let units = units();
        let [mut caller, mut implementation] = pair();
        let roots = BTreeSet::from(["caller".into(), "shared".into()]);
        let routes = BTreeMap::from([("shared".into(), units[1])]);
        qualify(std::slice::from_mut(&mut caller), units[0], &routes, &roots);
        qualify(
            std::slice::from_mut(&mut implementation),
            units[1],
            &BTreeMap::new(),
            &roots,
        );
        let expected = implementation.id.clone();
        assert_eq!(caller.edges[0].callee, expected);
        let prepared = rstyle_core::prep::prepare_owned(vec![caller, implementation]);
        assert_eq!(
            prepared[0].pdg.as_ref().unwrap().nodes[1]
                .callee_id
                .as_deref(),
            Some(expected.as_str())
        );
    }

    fn nested_facts() -> FnFacts {
        let mut facts = fact("caller::read");
        let signature = facts.sig.as_mut().unwrap();
        signature.self_args = vec![TyShape::Tuple(vec![TyShape::Ref {
            mutable: false,
            inner: Box::new(TyShape::Slice(Box::new(TyShape::Array(Box::new(
                TyShape::Adt {
                    id: "shared::Value".into(),
                    args: vec![TyShape::Dyn("shared::Trait".into()), TyShape::Param(0)],
                },
            ))))),
        }])];
        signature.params = vec![TyShape::Adt {
            id: "std::option::Option".into(),
            args: vec![
                TyShape::Prim("u32".into()),
                TyShape::Dyn("core::fmt::Debug".into()),
                TyShape::Adt {
                    id: "alloc::string::String".into(),
                    args: Vec::new(),
                },
            ],
        }];
        facts.pdg.as_mut().unwrap().nodes[1].callee_sig = facts.sig.clone();
        facts.mutated_types.push("shared::Value".into());
        facts.implements = Some("shared::Trait::read".into());
        facts.edges.push(rstyle_core::facts::Edge {
            callee: "shared::read".into(),
            line: 1,
            dynamic: false,
            args: Vec::new(),
        });
        facts
    }

    #[test]
    fn nested_metadata_restores_exactly_without_replacing_the_source_buffer() {
        let units = units();
        let mut facts = nested_facts();
        let original = facts.clone();
        let source = facts.source.as_ptr();
        let roots = BTreeSet::from([
            "caller".into(),
            "shared".into(),
            "std".into(),
            "core".into(),
            "alloc".into(),
        ]);
        qualify(
            std::slice::from_mut(&mut facts),
            units[0],
            &BTreeMap::new(),
            &roots,
        );
        assert_ne!(facts, original);
        assert_eq!(
            facts.sig.as_ref().unwrap().params,
            original.sig.as_ref().unwrap().params
        );
        restore_facts(&mut facts);
        assert_eq!(facts, original);
        assert_eq!(facts.source.as_ptr(), source);
    }
}
