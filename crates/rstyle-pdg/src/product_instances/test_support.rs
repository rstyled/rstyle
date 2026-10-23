//! One reduced original capture loader; no source closure is inferred from it.
use crate::{HirFnFacts, concrete_types::TypeTree};
use serde_json::Value;

pub(super) fn conversion() -> (HirFnFacts, Value, TypeTree) {
    let fixture: Value = serde_json::from_str(include_str!("conversion/fixture.json")).unwrap();
    let raw = serde_json::to_vec(&fixture["snapshot"]).unwrap();
    let mut facts = crate::unit_store::read_diagnostic_snapshot(&raw).unwrap();
    assert_eq!(facts.len(), 1);
    (
        facts.pop().unwrap(),
        fixture["instance"].clone(),
        serde_json::from_value(fixture["span"].clone()).unwrap(),
    )
}
