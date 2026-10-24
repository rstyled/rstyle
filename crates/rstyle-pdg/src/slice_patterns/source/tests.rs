//! Original compiler receipts and independently specified pattern expectations.
mod attacks;

use super::*;
use crate::HirFnFacts;

fn originals() -> Vec<HirFnFacts> {
    serde_json::from_str(include_str!("tests/originals.json")).unwrap()
}

fn find<'a>(node: &'a HirNode, kind: &str) -> &'a HirNode {
    fn visit<'a>(node: &'a HirNode, kind: &str) -> Option<&'a HirNode> {
        if node.kind == kind {
            return Some(node);
        }
        node.children.iter().find_map(|child| visit(child, kind))
    }
    visit(node, kind).expect("Required original recorded node")
}

fn find_mut<'a>(node: &'a mut HirNode, kind: &str) -> &'a mut HirNode {
    fn visit<'a>(node: &'a mut HirNode, kind: &str) -> Option<&'a mut HirNode> {
        if node.kind == kind {
            return Some(node);
        }
        node.children
            .iter_mut()
            .find_map(|child| visit(child, kind))
    }
    visit(node, kind).expect("Required original recorded node")
}

fn edit(facts: &mut HirFnFacts, kind: &str, change: impl FnOnce(&mut Observation)) {
    let node = find_mut(&mut facts.body, kind);
    let mut observed = Observation::decode(node).unwrap();
    change(&mut observed);
    node.res = Some(observed.resolution().unwrap());
}

fn rejected(facts: &HirFnFacts, change: impl FnOnce(&mut HirFnFacts)) -> String {
    let mut changed = facts.clone();
    change(&mut changed);
    let root = find(&changed.body, "pat_slice_prefix");
    verify(root, &changed)
        .err()
        .expect("Contradictory evidence must fail")
}

#[test]
fn original_digit_prefixes_retain_exact_ordered_binders_ranges_and_real_rest() {
    for (index, facts) in originals().iter().enumerate() {
        let root = find(&facts.body, "pat_slice_prefix");
        let checked = verify(root, facts).unwrap();
        assert_eq!(checked.pattern(), root);
        assert_eq!(checked.digits().len(), index + 1);
        for (digit, name) in checked.digits().iter().zip(["a", "b", "c"]) {
            assert!(digit.binding().snippet.starts_with(name));
            assert_eq!(digit.range().snippet, "b'0'..=b'9'");
            assert_eq!(digit.lower().snippet, "b'0'");
            assert_eq!(digit.upper().snippet, "b'9'");
        }
        assert_eq!(checked.remainder().binding().snippet, "remaining @ ..");
        assert_eq!(checked.remainder().rest().snippet, "..");
    }
}

#[test]
fn missing_observations_remain_unknown_and_explicitly_unsupported_stays_distinct() {
    let facts = originals().remove(0);
    let missing = rejected(&facts, |facts| {
        edit(facts, "pat_slice_prefix", |observed| {
            observed.status = CaptureStatus::Unknown("Missing compiler observation".into());
        })
    });
    assert!(missing.starts_with("Unknown slice prefix evidence:"));
    let unsupported = rejected(&facts, |facts| {
        edit(facts, "pat_slice_prefix", |observed| {
            observed.status = CaptureStatus::Unsupported("Overloaded dereference".into());
        })
    });
    assert!(unsupported.starts_with("Unsupported slice prefix evidence:"));
}
