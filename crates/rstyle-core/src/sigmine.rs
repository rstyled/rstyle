//! Layer 1: groups methods of different types by normalized signature.
//! This is a blocking key for later layers, not a finding by itself.

use crate::facts::FnFacts;
use crate::shape::{normalize, short};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// One method in a signature group.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Member {
    pub path: String,
    pub line: usize,
    pub id: String,
    pub name: String,
    pub self_ty: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SigGroup {
    /// Normalized signature (see `shape::NormSig::key`).
    pub key: String,
    /// Inherent methods, one per site; at least two distinct `self_ty`.
    pub members: Vec<Member>,
    /// Trait-impl methods with the same signature: counted, never proposed.
    pub traited: Vec<Member>,
    pub self_types: usize,
    /// Two or more distinct types define a method with the same short name.
    pub same_name: bool,
    /// `1000 * ln(total / group size)`: rare signatures score high, getters near zero.
    pub specificity_milli: u32,
    /// Specificity, plus up to 50% scaled by the share of types that share a name.
    pub score_milli: u32,
}

fn member(f: &FnFacts, self_ty: &str) -> Member {
    Member {
        path: f.path.clone(),
        line: f.line,
        id: f.id.clone(),
        name: f.name.clone(),
        self_ty: self_ty.into(),
    }
}

/// Largest number of distinct types that share one short method name (0 or 1: none).
fn shared_name_types(members: &[Member]) -> usize {
    let mut owners: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for m in members {
        owners.entry(short(&m.name)).or_default().insert(&m.self_ty);
    }
    owners.values().map(BTreeSet::len).max().unwrap_or(0)
}

fn specificity_milli(total: usize, size: usize) -> u32 {
    let ratio = total as f64 / size.max(1) as f64;
    (ratio.ln() * 1000.0).round().max(0.0) as u32
}

/// Sorted and deduplicated by function id, so input order never matters.
fn sorted(mut members: Vec<Member>) -> Vec<Member> {
    members.sort();
    members.dedup_by(|a, b| a.id == b.id);
    members
}

fn build(key: String, all: Vec<(Member, bool)>, total: usize) -> Option<SigGroup> {
    let (traited, inherent): (Vec<_>, Vec<_>) = all.into_iter().partition(|(_, traited)| *traited);
    let members = sorted(inherent.into_iter().map(|(m, _)| m).collect());
    let traited = sorted(traited.into_iter().map(|(m, _)| m).collect());
    let types: BTreeSet<&str> = members.iter().map(|m| m.self_ty.as_str()).collect();

    if types.len() < 2 {
        return None;
    }
    let specificity = specificity_milli(total, members.len() + traited.len());
    let shared = shared_name_types(&members);
    let same_name = shared >= 2;
    // The boost scales with the share of types that actually share the name.
    let boost = if same_name {
        (u64::from(specificity) / 2 * shared as u64 / types.len() as u64) as u32
    } else {
        0
    };
    Some(SigGroup {
        key,
        self_types: types.len(),
        same_name,
        specificity_milli: specificity,
        score_milli: specificity + boost,
        members,
        traited,
    })
}

type Buckets = BTreeMap<String, Vec<(Member, bool)>>;

/// Methods of ADTs by normalized signature; each member is flagged when it implements a trait.
fn buckets(facts: &[FnFacts]) -> Buckets {
    let sigs = facts.iter().filter_map(|f| {
        let sig = f.sig.as_ref()?;
        let self_ty = sig.owner()?;
        Some((
            normalize(sig).key,
            (member(f, &self_ty), f.implements.is_some()),
        ))
    });
    sigs.fold(Buckets::new(), |mut acc, (key, entry)| {
        acc.entry(key).or_default().push(entry);
        acc
    })
}

/// Signature groups with two or more distinct self types, best first
/// (score, then key) so output is independent of input order.
pub fn mine(facts: &[FnFacts]) -> Vec<SigGroup> {
    let buckets = buckets(facts);
    let total = buckets.values().map(Vec::len).sum();
    let mut groups: Vec<SigGroup> = buckets
        .into_iter()
        .filter_map(|(key, all)| build(key, all, total))
        .collect();

    groups.sort_by(|a, b| b.score_milli.cmp(&a.score_milli).then(a.key.cmp(&b.key)));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Receiver, TyShape, sample};
    use crate::shape::fixtures::*;

    fn getter(self_ty: &str, name: &str, ret: TyShape) -> FnFacts {
        FnFacts {
            id: format!("k::{self_ty}::{name}"),
            name: format!("{self_ty}::{name}"),
            sig: Some(method(&format!("k::{self_ty}"), Receiver::Ref, vec![], ret)),
            ..sample(name)
        }
    }

    fn string_method(self_ty: &str, name: &str) -> FnFacts {
        getter(self_ty, name, string())
    }

    fn traited(mut f: FnFacts) -> FnFacts {
        f.implements = Some("k::Speak::speak".into());
        f
    }

    #[test]
    fn group_needs_two_distinct_self_types() {
        let same_type = [string_method("Dog", "bark"), string_method("Dog", "howl")];
        assert!(mine(&same_type).is_empty());
        let two = [string_method("Dog", "bark"), string_method("Cat", "meow")];
        let groups = mine(&two);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].self_types, 2);
        assert_eq!(groups[0].key, "fn(&Self) -> String");
    }

    #[test]
    fn trait_impl_members_are_excluded_but_counted() {
        let facts = [
            string_method("Dog", "bark"),
            string_method("Cat", "meow"),
            traited(string_method("Cow", "speak")),
            traited(string_method("Hen", "speak")),
        ];
        let group = &mine(&facts)[0];
        assert_eq!(group.members.len(), 2);
        assert_eq!(group.traited.len(), 2);
        // Counted in the base rate: 4 of 4 functions share the signature.
        assert_eq!(group.specificity_milli, 0);
        let only_traited = [
            traited(string_method("Cow", "speak")),
            traited(string_method("Hen", "speak")),
        ];
        assert!(mine(&only_traited).is_empty());
    }

    #[test]
    fn rare_signature_outranks_getter_signature() {
        let mut facts: Vec<FnFacts> = (0..10)
            .map(|i| string_method(&format!("T{i}"), &format!("label{i}")))
            .collect();
        facts.push(getter("Dog", "chase", adt("k::Dog")));
        facts.push(getter("Cat", "chase", adt("k::Cat")));
        facts.extend(
            (0..20).map(|i| getter("Solo", &format!("m{i}"), TyShape::Prim(format!("p{i}")))),
        );
        let groups = mine(&facts);
        assert_eq!(groups[0].key, "fn(&Self) -> Self");
        assert!(groups[0].score_milli > groups[1].score_milli);
        assert!(groups[0].same_name);
        assert!(!groups[1].same_name);
    }

    #[test]
    fn same_name_boost_applies() {
        let facts = [string_method("Dog", "name"), string_method("Cat", "name")];
        let g = &mine(&[
            facts[0].clone(),
            facts[1].clone(),
            getter("Rock", "x", TyShape::Other),
        ])[0];
        assert!(g.same_name);
        assert_eq!(g.score_milli, g.specificity_milli + g.specificity_milli / 2);
    }

    #[test]
    fn boost_scales_with_share_of_types_that_share_the_name() {
        let facts = [
            string_method("Dog", "name"),
            string_method("Cat", "name"),
            string_method("Cow", "moo"),
            getter("Rock", "x", TyShape::Other),
            getter("Rock", "y", TyShape::Other),
            getter("Rock", "z", TyShape::Other),
        ];
        let g = &mine(&facts)[0];
        assert!(g.same_name);
        assert!(g.specificity_milli > 0);
        assert_eq!(
            g.score_milli,
            g.specificity_milli + g.specificity_milli / 2 * 2 / 3
        );
    }

    fn wrapper_name(args: Vec<TyShape>) -> FnFacts {
        let mut f = string_method("Wrapper", "name");
        f.id = format!(
            "k::Wrapper::name{}",
            args.len() + usize::from(args.contains(&u16_ty()))
        );
        f.sig.as_mut().expect("sig").self_args = args;
        f
    }

    fn u16_ty() -> TyShape {
        TyShape::Prim("u16".into())
    }

    #[test]
    fn same_self_ty_with_different_concrete_args_forms_a_group() {
        let facts = [
            wrapper_name(vec![TyShape::Prim("u8".into())]),
            wrapper_name(vec![u16_ty()]),
        ];
        let groups = mine(&facts);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].self_types, 2);
        assert!(groups[0].same_name);
    }

    #[test]
    fn same_self_ty_with_param_args_only_is_one_owner() {
        let facts = [
            wrapper_name(vec![TyShape::Param(0)]),
            wrapper_name(vec![TyShape::Param(0), TyShape::Param(1)]),
        ];
        assert!(mine(&facts).is_empty());
    }

    #[test]
    fn output_independent_of_input_order() {
        let mut facts = vec![
            string_method("Dog", "bark"),
            string_method("Cat", "meow"),
            getter("Dog", "chase", adt("k::Dog")),
            getter("Cat", "chase", adt("k::Cat")),
            traited(string_method("Cow", "speak")),
        ];
        let forward = mine(&facts);
        facts.reverse();
        assert_eq!(forward, mine(&facts));
    }
}
