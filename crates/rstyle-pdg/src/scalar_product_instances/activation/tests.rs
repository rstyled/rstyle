//! Independent original captures retain the complete body and reject neighboring work.
use super::*;
use crate::HirFnFacts;

fn originals() -> Vec<HirFnFacts> {
    serde_json::from_str(include_str!(
        "../../slice_patterns/source/tests/originals.json"
    ))
    .unwrap()
}

#[test]
fn deferred_pattern_coverage_requires_the_exact_original_finding() {
    for facts in originals() {
        let pattern = &facts.body.children[0].children[1].children[0];
        let finding = crate::HirDiagnostic {
            capability: "rust.scalar-interface".into(),
            construct: "binding_pattern".into(),
            span: pattern.span,
            snippet: pattern.snippet.clone(),
            severity: "error".into(),
            reason: "Source construct is deferred to rust.scalar-interface; remove it or assess it under a later policy.".into(),
        };
        assert!(integration::covers_pattern(&finding, pattern));
        for field in [
            "capability",
            "construct",
            "span",
            "snippet",
            "severity",
            "reason",
        ] {
            let mut changed = finding.clone();
            match field {
                "capability" => changed.capability = "rust.macros".into(),
                "construct" => changed.construct = "expr_call".into(),
                "span" => changed.span.end += 1,
                "snippet" => changed.snippet.push(' '),
                "severity" => changed.severity = "warning".into(),
                "reason" => changed.reason.clear(),
                _ => unreachable!(),
            }
            assert!(!integration::covers_pattern(&changed, pattern), "{field}");
        }
    }
}

#[test]
fn complete_original_bodies_preserve_the_exact_match_extent() {
    for facts in originals() {
        let matched = body::verify(&facts).unwrap();
        assert_eq!(matched, &facts.body.children[0]);
        assert!(matched.span.start > facts.body.span.start);
        assert!(matched.span.end < facts.body.span.end);
    }
}

#[test]
fn extra_recorded_work_cannot_receive_supported_prefix_permission() {
    for mut facts in originals() {
        facts.body.children.push(facts.body.children[0].clone());
        assert!(body::verify(&facts).is_err());
    }
}

#[test]
fn source_statement_omission_and_source_extent_changes_reject() {
    for original in originals() {
        let mut omitted = original.clone();
        omitted.source.bytes = omitted
            .source
            .bytes
            .replacen("{\n", "{ let omitted = 1;\n", 1)
            .into();
        assert!(body::verify(&omitted).is_err());

        let mut truncated = original.clone();
        truncated.body.span = truncated.body.children[0].span;
        truncated.body.snippet = truncated.body.children[0].snippet.clone();
        assert!(body::verify(&truncated).is_err());
    }
}

#[test]
fn nested_source_items_and_receiver_permissions_reject() {
    for original in originals() {
        let mut nested = original.clone();
        nested.source.bytes = nested
            .source
            .bytes
            .replacen("{\n", "{ struct Some;\n", 1)
            .into();
        assert!(body::verify(&nested).is_err());

        let mut receiver = original;
        receiver.signature.has_self = true;
        assert!(body::verify(&receiver).is_err());
    }
}

fn success(facts: &HirFnFacts) -> Result<&HirNode, String> {
    let matched = body::verify(facts)?;
    let prefix = semantics::verify(matched, facts)?;
    variants::success_body(&prefix, facts)
}

#[test]
fn original_success_blocks_retain_every_local_before_the_some_tail() {
    for (index, facts) in originals().iter().enumerate() {
        let value = success(facts).unwrap();
        assert_eq!(value.kind, "expr_call");
        assert!(value.snippet.starts_with("Some(ParsedItem("));
        let body = &facts.body.children[0].children[1].children[1];
        if index == 0 {
            assert_eq!(value, body);
        } else {
            assert_eq!(body.kind, "expr_block");
            assert_eq!(body.children.len(), index + 2);
            assert!(
                body.children[..index + 1]
                    .iter()
                    .all(|node| node.kind == "stmt_let")
            );
            assert_eq!(value, body.children.last().unwrap());
        }
    }
}

#[test]
fn omitted_or_surplus_prelude_statements_cannot_certify_the_tail() {
    for original in originals().into_iter().skip(1) {
        let mut omitted = original.clone();
        omitted.body.children[0].children[1].children[1]
            .children
            .remove(0);
        assert!(success(&omitted).is_err());
        let mut surplus = original;
        let block = &mut surplus.body.children[0].children[1].children[1];
        let mut hidden = block.children[0].clone();
        hidden.kind = "stmt_semi".into();
        block.children.insert(0, hidden);
        assert!(success(&surplus).is_err());
    }
}

#[test]
fn mutated_or_missing_local_binding_records_cannot_certify_success() {
    for original in originals().into_iter().skip(1) {
        let binding = original.body.children[0].children[1].children[1].children[0].children[0]
            .hir
            .clone();
        let mut missing = original.clone();
        missing.bindings.retain(|record| record.hir != binding);
        assert!(success(&missing).is_err());
        let mut changed = original;
        changed
            .bindings
            .iter_mut()
            .find(|record| record.hir == binding)
            .unwrap()
            .is_mut = true;
        assert!(success(&changed).is_err());
    }
}

#[test]
fn changed_zero_and_overloaded_arithmetic_do_not_gain_prelude_permission() {
    for original in originals().into_iter().skip(1) {
        let mut zero = original.clone();
        let value = &mut zero.body.children[0].children[1].children[1].children[0].children[1];
        let difference = if value.kind == "expr_cast" {
            &mut value.children[0]
        } else {
            value
        };
        difference.children[1].lit = Some("b'1'".into());
        assert!(success(&zero).is_err());
        let mut overloaded = original;
        let value =
            &mut overloaded.body.children[0].children[1].children[1].children[0].children[1];
        let difference = if value.kind == "expr_cast" {
            &mut value.children[0]
        } else {
            value
        };
        difference.dispatch.as_mut().unwrap().kind = "overloaded".into();
        assert!(success(&overloaded).is_err());
    }
}
