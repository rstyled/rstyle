//! Stable names for functions, used by the ignore ledger.
//!
//! The driver's id for an impl method is `<crate>::<module>::{impl#N}::<name>`, where `N` is the
//! position of the impl block in its module: adding an impl block above renumbers the method.
//! A locator names the method by its owner type (and trait) instead, so unrelated edits keep it.

use crate::facts::FnFacts;
use std::collections::{BTreeMap, BTreeSet};

const IMPL_SEGMENT: &str = "{impl#";

/// The trait a trait-impl method belongs to: its method id without the last segment.
fn trait_of(method_id: &str) -> &str {
    method_id.rsplit_once("::").map_or(method_id, |(t, _)| t)
}

/// Locator before duplicate numbering: the fn id for free fns, `Owner::name` for inherent
/// methods and `<Owner as Trait>::name` for trait-impl methods.
pub fn base(f: &FnFacts) -> String {
    let segments: Vec<&str> = f.id.split("::").collect();
    let at = segments.iter().position(|s| s.starts_with(IMPL_SEGMENT));
    let owner = f.sig.as_ref().and_then(|s| s.owner());
    let (Some(at), Some(owner)) = (at, owner) else {
        return f.id.clone();
    };
    let name = segments[at + 1..].join("::");

    match f.implements.as_deref() {
        Some(method) => format!("<{owner} as {}>::{name}", trait_of(method)),
        None => format!("{owner}::{name}"),
    }
}

/// Locator of every function, keyed by fn id. Functions that still share one (for example
/// `impl From<A> for X` and `impl From<B> for X`) get `#2`, `#3`, ... in source order.
pub fn assign(facts: &[FnFacts]) -> BTreeMap<String, String> {
    let mut order: Vec<&FnFacts> = facts.iter().collect();
    order.sort_by(|a, b| (&a.path, a.line, &a.id).cmp(&(&b.path, b.line, &b.id)));
    let mut seen = BTreeSet::new();
    let unique = order.into_iter().filter(|f| seen.insert(f.id.as_str()));
    let mut taken: BTreeMap<String, usize> = BTreeMap::new();

    unique
        .map(|f| (f.id.clone(), numbered(base(f), &mut taken)))
        .collect()
}

/// `base` for its first user, `base#2`, `base#3`, ... after that.
fn numbered(base: String, taken: &mut BTreeMap<String, usize>) -> String {
    let n = taken.entry(base.clone()).or_insert(0);
    *n += 1;

    if *n == 1 { base } else { format!("{base}#{n}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Receiver, sample};
    use crate::shape::fixtures::{method, string};

    fn method_fn(id: &str, owner: &str, implements: Option<&str>) -> FnFacts {
        FnFacts {
            id: id.into(),
            implements: implements.map(Into::into),
            sig: Some(method(owner, Receiver::Ref, vec![], string())),
            ..sample("m")
        }
    }

    fn locator_of(f: &FnFacts) -> String {
        assign(std::slice::from_ref(f))[&f.id].clone()
    }

    #[test]
    fn free_fn_keeps_its_id() {
        assert_eq!(locator_of(&sample("run")), "krate::run");
    }

    #[test]
    fn inherent_method_survives_impl_renumbering() {
        let before = method_fn("k::m::{impl#2}::get", "k::m::Dog", None);
        let after = method_fn("k::m::{impl#3}::get", "k::m::Dog", None);

        assert_eq!(locator_of(&before), "k::m::Dog::get");
        assert_eq!(locator_of(&before), locator_of(&after));
    }

    #[test]
    fn trait_impls_of_one_method_name_do_not_collide() {
        let display = method_fn(
            "k::{impl#0}::fmt",
            "k::Dog",
            Some("core::fmt::Display::fmt"),
        );
        let debug = method_fn("k::{impl#1}::fmt", "k::Dog", Some("core::fmt::Debug::fmt"));

        assert_eq!(locator_of(&display), "<k::Dog as core::fmt::Display>::fmt");
        assert_ne!(locator_of(&display), locator_of(&debug));
    }

    #[test]
    fn owners_with_different_type_arguments_differ() {
        let mut tcp = method_fn("k::{impl#0}::send", "k::Transport", None);
        let mut unix = method_fn("k::{impl#1}::send", "k::Transport", None);
        tcp.sig.as_mut().unwrap().self_args = vec![crate::shape::fixtures::adt("k::Tcp")];
        unix.sig.as_mut().unwrap().self_args = vec![crate::shape::fixtures::adt("k::Unix")];

        assert_ne!(locator_of(&tcp), locator_of(&unix));
    }

    #[test]
    fn remaining_duplicates_are_numbered_in_source_order() {
        let mut first = method_fn(
            "k::{impl#0}::from",
            "k::X",
            Some("core::convert::From::from"),
        );
        let mut second = method_fn(
            "k::{impl#1}::from",
            "k::X",
            Some("core::convert::From::from"),
        );
        first.line = 10;
        second.line = 20;
        let named = assign(&[second.clone(), first.clone()]);

        assert_eq!(named[&first.id], "<k::X as core::convert::From>::from");
        assert_eq!(named[&second.id], "<k::X as core::convert::From>::from#2");
    }

    #[test]
    fn the_same_fn_seen_twice_is_not_a_duplicate() {
        let f = sample("run");
        let named = assign(&[f.clone(), f.clone()]);

        assert_eq!(named.len(), 1);
        assert_eq!(named[&f.id], "krate::run");
    }
}
