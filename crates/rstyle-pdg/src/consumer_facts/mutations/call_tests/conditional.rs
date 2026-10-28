use super::*;
use syn::visit::Visit;

struct CapturedCalls<'a, 'b> {
    fixture: &'a Fixture<'b>,
    nodes: Vec<HirNode>,
}

impl<'ast> Visit<'ast> for CapturedCalls<'_, '_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        self.nodes.push(self.fixture.call(self.nodes.len(), call));
        syn::visit::visit_expr_call(self, call);
    }
}

fn conditional_fixture(source: &str) -> HirFnFacts {
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    let mut fixture = Fixture::new(source);
    fixture.signature(&function);
    fixture.body(&function);
    let mut calls = CapturedCalls {
        fixture: &fixture,
        nodes: vec![],
    };
    calls.visit_block(&function.block);
    let nodes = calls.nodes;
    fixture.facts.body.children = nodes;
    ownership(&mut fixture.facts, source);
    fixture.facts
}

#[test]
fn nested_branch_calls_keep_original_witnesses_without_certifying_control() {
    let source = "pub fn caller(value:u32) { if true { leaf(value); if false { leaf(1); } } else { leaf(2); } }";
    let facts = conditional_fixture(source);
    let observed = inspect_calls_with_uses(&facts).unwrap();
    assert_eq!(observed.calls.len(), 3);
    assert_eq!(observed.direct.coverage, Coverage::Incomplete);
    assert_eq!(observed.uses.coverage, Coverage::Incomplete);
    assert!(
        observed
            .direct
            .boundaries
            .iter()
            .any(|boundary| boundary.reason.contains("Conditional control"))
    );
    for (call, text) in observed
        .calls
        .iter()
        .zip(["leaf(value)", "leaf(1)", "leaf(2)"])
    {
        assert_eq!(
            crate::source_evidence::byte_range(source, call.span).unwrap(),
            text
        );
        assert_eq!(
            observed.direct.boundaries[call.boundary_index].span,
            call.span
        );
        assert_eq!(call.target, Availability::Known("leaf-id".into()));
    }
    assert!(
        inspect(&facts)
            .unwrap()
            .boundaries
            .iter()
            .all(|boundary| boundary.reason != "Unknown call boundary")
    );
}

#[test]
fn branch_calls_never_mint_outer_formal_claims_in_sibling_or_following_scopes() {
    let facts = conditional_fixture(
        "pub fn caller(mut value:u32) { leaf(value); if true { value=1; leaf(value); } else { leaf(value); } leaf(value); }",
    );
    let observed = inspect_calls_with_uses(&facts).unwrap();
    assert_eq!(observed.calls.len(), 4);
    assert_eq!(
        observed.calls[0].arguments[0].formal,
        Availability::Known(Some(0))
    );
    for call in &observed.calls[1..] {
        assert!(matches!(
            call.arguments[0].formal,
            Availability::Unknown { .. }
        ));
        assert!(matches!(
            call.arguments[0].unchanged_formal,
            Availability::Unknown { .. }
        ));
    }
    assert_eq!(observed.uses.coverage, Coverage::Incomplete);
}

#[test]
fn conditional_native_call_correspondence_remains_mandatory() {
    let mut facts = conditional_fixture("pub fn caller(value:u32) { if true { leaf(value); } }");
    facts.body.children[0]
        .dispatch
        .as_mut()
        .unwrap()
        .operand_tys
        .clear();
    assert!(inspect_calls(&facts).is_err());
    let mut facts = conditional_fixture("pub fn caller(value:u32) { if true { leaf(value); } }");
    facts.body.children.clear();
    let observed = inspect_calls_with_uses(&facts).unwrap();
    assert!(observed.calls.is_empty());
    assert_eq!(observed.uses.coverage, Coverage::Incomplete);
}

#[test]
fn opaque_attributed_and_let_conditions_do_not_expose_branch_calls() {
    for condition in [
        "#[cfg(any())] true",
        "let value = 1",
        "true && let value = 1",
        "(|| true)()",
        "opaque!()",
    ] {
        let source = format!("pub fn caller(value:u32) {{ if {condition} {{ leaf(value); }} }}");
        let observed = inspect_calls_with_uses(&conditional_fixture(&source)).unwrap();
        assert!(observed.calls.is_empty(), "{condition}");
        assert_eq!(observed.direct.coverage, Coverage::Incomplete);
        assert_eq!(observed.uses.coverage, Coverage::Incomplete);
    }
}

#[test]
fn condition_calls_are_observed_before_lexical_branch_calls() {
    let facts = conditional_fixture(
        "pub fn caller(value:u32) { if leaf(value) { leaf(1); } else { leaf(2); } }",
    );
    let observed = inspect_calls(&facts).unwrap();
    assert_eq!(observed.calls.len(), 3);
    assert_eq!(
        observed.calls[0].span,
        text_span(&facts.source.bytes, "leaf(value)")
    );
    assert!(matches!(
        observed.calls[0].arguments[0].formal,
        Availability::Unknown { .. }
    ));
}
