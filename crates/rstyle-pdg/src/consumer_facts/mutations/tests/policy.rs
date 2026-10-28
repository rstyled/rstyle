use super::*;
use crate::{consumer_facts, nominal_boundary::census};
use std::sync::Arc;

fn owned(source: &str, writes: &[&str], prefix: &str) -> HirFnFacts {
    let mut facts = fixture(source, writes);
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    facts.signature.sig_span = consumer_facts::header_span(source, &function);
    consumer_facts::tests::source_owner(&mut facts, source);
    if !prefix.is_empty() {
        let file = format!("{prefix}{source}");
        let previous = facts
            .build
            .frontend_options
            .data::<Arc<census::Census>>()
            .unwrap();
        let mut census = previous.as_ref().as_ref().clone();
        census.files[0].bytes = file.clone().into();
        census.files[0].digest = census.files[0].bytes.checksum().into();
        facts.fn_span = ByteSpan {
            start: prefix.len() as u32,
            end: file.len() as u32,
        };
        facts.build.frontend_options.insert_data(Arc::new(census));
    }
    facts
}

fn assert_same_direct(facts: &HirFnFacts) {
    let original = inspect(facts).unwrap();
    let policy = inspect_with_policy(facts).unwrap();
    assert_eq!(
        serde_json::to_value(&original).unwrap(),
        serde_json::to_value(&policy.direct).unwrap()
    );
}

#[test]
fn policy_inputs_share_exact_direct_evidence_and_borrow_original_bytes() {
    let facts = owned(
        SIMPLE,
        &["*target=1"],
        "// rstyle-allow(mutation_per_target): reviewed\n",
    );
    let policy = inspect_with_policy(&facts).unwrap();
    assert_eq!(policy.direct.coverage, Coverage::Complete);
    assert_eq!(policy.direct.sites.len(), 1);
    let Availability::Known(source) = policy.source else {
        panic!("owned source unavailable")
    };
    assert_eq!(source.as_ptr(), facts.source.bytes.as_ptr());
    assert_eq!(
        policy.preceding_comment,
        Availability::Known("// rstyle-allow(mutation_per_target): reviewed".into())
    );
    assert_same_direct(&facts);
}

#[test]
fn incomplete_body_does_not_discard_checked_source_or_comment() {
    for source in [
        "pub fn f(target:&mut u32){opaque();*target=1;}",
        "pub fn f(target:&mut u32){loop {*target=1;}}",
    ] {
        let facts = owned(source, &["*target=1"], "// policy candidate\n");
        let policy = inspect_with_policy(&facts).unwrap();
        assert_eq!(policy.direct.coverage, Coverage::Incomplete);
        assert!(matches!(policy.source, Availability::Known(_)));
        assert_eq!(
            policy.preceding_comment,
            Availability::Known("// policy candidate".into())
        );
        assert_same_direct(&facts);
    }
}

#[test]
fn later_native_budget_preserves_checked_policy_comment() {
    let mut facts = owned(SIMPLE, &["*target=1"], "// policy candidate\n");
    facts.bindings = vec![facts.bindings[0].clone(); 16385];
    let policy = inspect_with_policy(&facts).unwrap();
    assert_eq!(policy.direct.execution, Execution::BudgetExhausted);
    assert!(matches!(policy.source, Availability::Known(_)));
    assert_eq!(
        policy.preceding_comment,
        Availability::Known("// policy candidate".into())
    );
    assert_same_direct(&facts);
}

#[test]
fn syntax_budget_retains_owned_source_without_granting_comment_authority() {
    let source = format!(
        "pub fn f(target:&mut u32){{/*{}*/}}",
        " ".repeat(256 * 1024)
    );
    let facts = owned(&source, &[], "// policy candidate\n");
    let policy = inspect_with_policy(&facts).unwrap();
    assert_eq!(policy.direct.execution, Execution::BudgetExhausted);
    assert!(matches!(policy.source, Availability::Known(_)));
    assert!(matches!(
        policy.preceding_comment,
        Availability::Unknown { .. }
    ));
    assert_same_direct(&facts);
}

#[test]
fn attributed_syntax_has_no_checked_comment_even_with_owned_source() {
    let source = format!("#[opaque] {SIMPLE}");
    let facts = owned(&source, &["*target=1"], "// policy candidate\n");
    let policy = inspect_with_policy(&facts).unwrap();
    assert_eq!(policy.direct.coverage, Coverage::Incomplete);
    assert!(matches!(policy.source, Availability::Known(_)));
    assert!(matches!(
        policy.preceding_comment,
        Availability::Unknown { .. }
    ));
    assert_same_direct(&facts);
}

#[test]
fn unavailable_source_owner_cannot_supply_policy_inputs() {
    let facts = fixture(SIMPLE, &["*target=1"]);
    let policy = inspect_with_policy(&facts).unwrap();
    assert!(matches!(policy.source, Availability::Unknown { .. }));
    assert!(matches!(
        policy.preceding_comment,
        Availability::Unknown { .. }
    ));
    assert_eq!(policy.direct.coverage, Coverage::Incomplete);
    assert_same_direct(&facts);
}

#[test]
fn changed_source_checksum_and_signature_fail_before_policy_comment() {
    let mut facts = owned(SIMPLE, &["*target=1"], "// policy candidate\n");
    facts.source.digest = "changed".into();
    assert!(inspect_with_policy(&facts).is_err());
    let mut facts = owned(SIMPLE, &["*target=1"], "// policy candidate\n");
    facts.signature.params[0].position = 99;
    assert!(inspect_with_policy(&facts).is_err());
}

#[test]
fn known_empty_comment_is_distinct_from_missing_comment_authority() {
    let facts = owned(SIMPLE, &["*target=1"], "");
    assert_eq!(
        inspect_with_policy(&facts).unwrap().preceding_comment,
        Availability::Known("".into())
    );
}

#[test]
fn expanded_function_never_exposes_original_policy_authority() {
    let mut facts = owned(SIMPLE, &["*target=1"], "// policy candidate\n");
    facts.source_diagnostics.push(crate::HirDiagnostic {
        capability: "rust.macros".into(),
        construct: "macro_generated_function".into(),
        span: facts.signature.sig_span,
        snippet: String::new(),
        severity: "error".into(),
        reason: "expanded".into(),
    });
    let policy = inspect_with_policy(&facts).unwrap();
    assert!(matches!(policy.source, Availability::Unsupported { .. }));
    assert!(matches!(
        policy.preceding_comment,
        Availability::Unsupported { .. }
    ));
    assert_same_direct(&facts);
}
