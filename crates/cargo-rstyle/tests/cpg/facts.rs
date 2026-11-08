//! Rule facts derived from `compat.cpg` documents, over the lowering fixture written
//! to exercise each part of the derivation.

use super::*;

/// The facts core derives, frozen from the driver's own extraction when the two were
/// proven equal over this fixture and rstyle's workspace: one record per function.
#[test]
fn the_derivation_keeps_the_frozen_rule_facts() {
    let (_dir, out) = emitted();
    let facts = derived(&out);
    assert_eq!(facts.len(), 21);
    insta::assert_json_snapshot!(facts);
}
