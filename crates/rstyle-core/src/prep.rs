//! Pre-processing of facts before mining: suppression, devirtualization, trait hiding.

use crate::facts::{EdgeKind, FnFacts, NodeKind, Pdg, SigFacts, TyShape};
use crate::suppress;
use std::collections::BTreeMap;

/// Hides trait links for the evaluation oracle: facts whose `implements` contains any
/// pattern lose it (`*` hides all), so their methods look like ordinary inherent ones.
pub fn hide_implements(mut facts: Vec<FnFacts>, patterns: &[String]) -> Vec<FnFacts> {
    let hidden = |trait_method: &str| {
        patterns
            .iter()
            .any(|p| p == "*" || trait_method.contains(p.as_str()))
    };
    facts.iter_mut().for_each(|f| {
        if f.implements.as_deref().is_some_and(hidden) {
            f.implements = None;
        }
    });
    facts
}

/// Impl method per (trait method id, self ADT id).
type Impls<'a> = BTreeMap<(&'a str, &'a str), &'a FnFacts>;

fn impls(facts: &[FnFacts]) -> Impls<'_> {
    facts
        .iter()
        .filter_map(|f| {
            let adt = f.sig.as_ref()?.self_ty.as_deref()?;
            Some(((f.implements.as_deref()?, adt), f))
        })
        .collect()
}

fn peel(ty: &TyShape) -> &TyShape {
    match ty {
        TyShape::Ref { inner, .. } => peel(inner),
        other => other,
    }
}

/// ADT id of the receiver (data operand 0) of call node `call`.
fn receiver_adt(pdg: &Pdg, call: usize) -> Option<&str> {
    let edge = pdg
        .edges
        .iter()
        .find(|e| e.to == call && e.kind == EdgeKind::Data && e.arg_pos == 0)?;
    match peel(pdg.nodes.get(edge.from)?.ty.as_ref()?) {
        TyShape::Adt { id, .. } => Some(id),
        _ => None,
    }
}

/// Only resolved call metadata changes; graph nodes, edges and source stay owned.
struct Rewrite {
    function: usize,
    call: usize,
    callee: String,
    signature: Option<SigFacts>,
}

impl Rewrite {
    fn apply(self, facts: &mut [FnFacts]) {
        let pdg = facts[self.function].pdg.as_mut().expect("selected graph");
        let node = &mut pdg.nodes[self.call];
        node.callee_id = Some(self.callee);
        node.callee_sig = self.signature;
    }
}

fn rewrites(facts: &[FnFacts]) -> Vec<Rewrite> {
    let table = impls(facts);
    let mut changes = Vec::new();
    for (function, facts) in facts.iter().enumerate() {
        let Some(pdg) = &facts.pdg else { continue };
        for (call, node) in pdg.nodes.iter().enumerate() {
            let target = (node.kind == NodeKind::Call)
                .then(|| table.get(&(node.callee_id.as_deref()?, receiver_adt(pdg, call)?)))
                .flatten();
            if let Some(target) = target {
                changes.push(Rewrite {
                    function,
                    call,
                    callee: target.id.clone(),
                    signature: target.sig.clone(),
                });
            }
        }
    }
    changes
}

/// Derives and proc macros wrap their output in `const _: () = { .. }`: not hand-written code.
fn generated(f: &FnFacts) -> bool {
    // rustc disambiguates repeated anonymous consts as `_#1`, `_#2`, ..
    f.id.split("::")
        .any(|segment| segment == "_" || segment.starts_with("_#"))
}

/// Facts ready for mining: `rstyle-allow(abstraction)` functions removed, trait calls
/// resolved to impls. Order is preserved.
pub fn prepare(facts: &[FnFacts]) -> Vec<FnFacts> {
    prepare_owned(facts.to_vec())
}

/// Consume facts without copying source, nodes or edges. Resolve all calls
/// against the original input before changing any graph, like the borrowed API.
pub fn prepare_owned(mut facts: Vec<FnFacts>) -> Vec<FnFacts> {
    facts.retain(|f| !suppress::allows(&f.preceding_line, "abstraction") && !generated(f));
    for change in rewrites(&facts) {
        change.apply(&mut facts);
    }
    facts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Receiver, sample};
    use crate::pdgtest::*;
    use crate::shape::fixtures::{method, string};

    fn speaker(adt: &str) -> FnFacts {
        FnFacts {
            id: format!("k::{adt}::speak"),
            implements: Some("k::Speak::speak".into()),
            sig: Some(method(
                &format!("k::{adt}"),
                Receiver::Ref,
                vec![],
                string(),
            )),
            ..sample("speak")
        }
    }

    fn caller_of_trait_method(adt: &str) -> FnFacts {
        let mut pdg = emit_loop(adt, "x");
        pdg.nodes[4].callee_id = Some("k::Speak::speak".into());
        FnFacts {
            pdg: Some(pdg),
            ..sample(&format!("emit_{adt}"))
        }
    }

    #[test]
    fn trait_calls_resolve_to_the_receiver_impl() {
        let facts = [
            speaker("Cow"),
            speaker("Hen"),
            caller_of_trait_method("Cow"),
        ];
        let out = prepare(&facts);
        let pdg = out[2].pdg.as_ref().expect("pdg");
        assert_eq!(pdg.nodes[4].callee_id.as_deref(), Some("k::Cow::speak"));
    }

    #[test]
    fn owned_preparation_preserves_body_buffers_and_unrelated_metadata() {
        let caller = caller_of_trait_method("Cow");
        let original = caller.pdg.as_ref().unwrap();
        let buffers = (
            caller.source.as_ptr(),
            original.nodes.as_ptr(),
            original.edges.as_ptr(),
        );
        let mut expected = caller.clone();
        let target = speaker("Cow");
        let call = &mut expected.pdg.as_mut().unwrap().nodes[4];
        call.callee_id = Some(target.id.clone());
        call.callee_sig = target.sig.clone();

        let result = prepare_owned(vec![caller, target]);
        let graph = result[0].pdg.as_ref().unwrap();
        assert_eq!(result[0], expected);
        assert_eq!(
            buffers,
            (
                result[0].source.as_ptr(),
                graph.nodes.as_ptr(),
                graph.edges.as_ptr()
            )
        );
    }

    #[test]
    fn unresolved_calls_and_absent_graphs_remain_unchanged() {
        let caller = caller_of_trait_method("Cow");
        let mut other = speaker("Hen");
        other.pdg = Some(Pdg::default());
        let facts = vec![caller, other, sample("without_graph")];
        assert_eq!(prepare_owned(facts.clone()), facts);
    }

    #[test]
    fn functions_inside_anonymous_const_are_dropped() {
        let mut generated = speaker("Cow");
        generated.id = "k::m::_::{impl#0}::project".into();
        let mut kept = speaker("Cow");
        kept.id = "k::m::{impl#0}::project".into();
        let out = prepare(&[generated, kept]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "k::m::{impl#0}::project");
    }

    #[test]
    fn disambiguated_anonymous_consts_are_dropped_too() {
        let mut f = speaker("Cow");
        f.id = "k::m::_#1::{impl#0}::project".into();
        assert!(prepare(&[f]).is_empty());
        let mut g = speaker("Cow");
        g.id = "k::m::_x::{impl#0}::project".into();
        assert_eq!(prepare(&[g]).len(), 1);
    }

    #[test]
    fn anonymous_const_filter_only_matches_whole_segments() {
        let mut f = speaker("Cow");
        f.id = "k::my_mod::x".into();
        assert_eq!(prepare(&[f]).len(), 1);
    }

    #[test]
    fn allow_comment_removes_function() {
        let mut f = caller_of_trait_method("Cow");
        f.preceding_line = "// rstyle-allow(abstraction): intentional".into();
        assert!(prepare(&[f]).is_empty());
    }

    #[test]
    fn hide_drops_matching_trait_links_only() {
        let mut other = speaker("Hen");
        other.implements = Some("k::Other::run".into());
        let out = hide_implements(vec![speaker("Cow"), other], &["Speak".into()]);
        assert_eq!(out[0].implements, None);
        assert!(out[1].implements.is_some());
        assert_eq!(hide_implements(out, &["*".into()])[1].implements, None);
    }
}
