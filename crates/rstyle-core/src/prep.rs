//! Pre-processing of facts before mining: suppression, devirtualization, trait hiding.

use crate::facts::{EdgeKind, FnFacts, NodeKind, Pdg, PdgNode, TyShape};
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

/// Rewrites calls to a trait method into calls to the impl for the receiver's static type,
/// so `cow.speak()` and `hen.speak()` become distinct, comparable callees.
fn devirtualized(pdg: &Pdg, table: &Impls<'_>) -> Pdg {
    let nodes = pdg.nodes.iter().enumerate().map(|(index, node)| {
        let target = (node.kind == NodeKind::Call)
            .then(|| table.get(&(node.callee_id.as_deref()?, receiver_adt(pdg, index)?)))
            .flatten();
        match target {
            Some(t) => PdgNode {
                callee_id: Some(t.id.clone()),
                callee_sig: t.sig.clone(),
                ..node.clone()
            },
            None => node.clone(),
        }
    });
    Pdg {
        nodes: nodes.collect(),
        edges: pdg.edges.clone(),
    }
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
    let active: Vec<FnFacts> = facts
        .iter()
        .filter(|f| !suppress::allows(&f.preceding_line, "abstraction") && !generated(f))
        .cloned()
        .collect();
    let table = impls(&active);
    let rewritten: Vec<Option<Pdg>> = active
        .iter()
        .map(|f| f.pdg.as_ref().map(|p| devirtualized(p, &table)))
        .collect();
    active
        .iter()
        .zip(rewritten)
        .map(|(f, pdg)| FnFacts { pdg, ..f.clone() })
        .collect()
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
