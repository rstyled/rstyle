//! Retained original captures; expectations come from the original prefix source.
use super::*;
use crate::HirFnFacts;

fn originals() -> Vec<HirFnFacts> {
    serde_json::from_str(include_str!("../source/tests/originals.json")).unwrap()
}

fn matched(facts: &HirFnFacts) -> &HirNode {
    &facts.body.children[0]
}

#[test]
fn original_prefixes_retain_ordered_safe_selectors_and_distinct_body_reads() {
    for (position, facts) in originals().iter().enumerate() {
        let checked = verify(matched(facts), facts).unwrap();
        let contract = checked.contract();
        let count = position + 1;
        assert_eq!(checked.consumed() as usize, count);
        assert_eq!(contract.selectors.len(), count + 1);
        assert_eq!(
            contract.selectors[0].kind,
            SelectorKind::Length {
                minimum: count as u8
            }
        );
        assert_eq!(contract.selectors[0].requires, None);
        assert_eq!(contract.selectors[0].content, None);
        assert_eq!(contract.selectors[0].on_false, checked.failure().hir);

        ordered_digits(&checked, &contract);
        saved_input_and_suffix(&checked, &contract);
        assert_eq!(
            contract.selectors[count].on_true,
            Destination::Success(checked.success().hir.clone())
        );
        assert_eq!(
            contract.suffix.region,
            Region::Suffix { start: count as u8 }
        );
        assert_eq!(contract.suffix.requires, contract.selectors[count].source);
        assert!(contract.none_references.is_empty());
        contract.validate(&checked).unwrap();
    }
}

fn saved_input_and_suffix(checked: &Checked<'_>, contract: &Contract) {
    assert_eq!(contract.formal, checked.formal().hir);
    assert_eq!(contract.saved_input, checked.scrutinee().hir);
    assert_eq!(contract.pattern, checked.prefix().pattern().hir);
    assert_eq!(contract.selectors[0].source, checked.prefix().pattern().hir);
    for selector in &contract.selectors {
        assert_eq!(selector.saved_input, checked.scrutinee().hir);
    }

    let remainder = checked.prefix().remainder();
    assert_eq!(contract.suffix.binding, remainder.binding().hir);
    assert_eq!(contract.suffix.rest, remainder.rest().hir);
    assert_eq!(contract.suffix.value, checked.remainder().hir);
    assert_eq!(contract.none, checked.failure().hir);
}

fn ordered_digits(checked: &Checked<'_>, contract: &Contract) {
    let count = checked.digits().len();
    for index in 0..count {
        let selector = &contract.selectors[index + 1];
        let digit = &checked.digits()[index];
        assert_eq!(selector.source, digit.range().hir);
        assert_eq!(
            selector.kind,
            SelectorKind::Digit {
                position: index as u8,
                lower: 48,
                upper: 57
            }
        );
        assert_eq!(
            selector.requires,
            Some(contract.selectors[index].source.clone())
        );
        assert_eq!(
            contract.selectors[index].on_true,
            Destination::Selector(selector.source.clone())
        );
        assert_eq!(selector.on_false, checked.failure().hir);
        assert_eq!(
            selector.content,
            Some(Region::Element {
                position: index as u8
            })
        );
        assert_ne!(selector.source, contract.elements[index].read);
        assert_eq!(
            contract.elements[index].requires,
            contract.selectors[count].source
        );
        assert_eq!(contract.elements[index].binding, digit.binding().hir);
        assert_eq!(contract.elements[index].pointer, digit.pointer().hir);
        assert_eq!(contract.elements[index].read, digit.read().hir);
    }
}

#[test]
fn wire_attacks_cannot_erase_guards_reads_offsets_or_empty_none_case() {
    let facts = originals().remove(2);
    let checked = verify(matched(&facts), &facts).unwrap();
    let expected = checked.contract();
    let attacks: [fn(&mut Contract); 6] = [
        |changed| changed.selectors[1].requires = None,
        |changed| changed.selectors[2].content = None,
        |changed| changed.suffix.region = Region::Suffix { start: 2 },
        |changed| changed.elements[0].requires = changed.selectors[0].source.clone(),
        |changed| changed.none_references.push(Region::Suffix { start: 3 }),
        |changed| changed.selectors.swap(1, 2),
    ];

    for attack in attacks {
        let mut changed = expected.clone();
        attack(&mut changed);
        assert!(changed.validate(&checked).is_err());
    }
}

#[test]
fn missing_duplicate_or_retargeted_original_body_loads_remain_unproved() {
    let original = originals().remove(0);
    let mut changed = original.clone();
    let read =
        &mut changed.body.children[0].children[1].children[1].children[0].children[1].children[0];
    assert_eq!(read.kind, "expr_unary");
    read.children[0].res.as_mut().unwrap().target =
        Some(original.signature.params[0].pat.hir.clone());
    assert!(verify(matched(&changed), &changed).is_err());

    let mut changed = original.clone();
    let numeric = &mut changed.body.children[0].children[1].children[1].children[0].children[1];
    let duplicate = numeric.children[0].clone();
    numeric.children.push(duplicate);
    assert!(verify(matched(&changed), &changed).is_err());

    let mut changed = original.clone();
    let numeric = &mut changed.body.children[0].children[1].children[1].children[0].children[1];
    numeric.children.remove(0);
    assert!(verify(matched(&changed), &changed).is_err());
}
