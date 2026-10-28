use super::*;

fn replace_source(facts: &mut HirFnFacts, source: String, body_start: usize) {
    facts.source.bytes = source.into();
    facts.source.digest = facts.source.bytes.checksum().into();
    facts.fn_span.end = facts.source.bytes.len() as u32;
    facts.body.span = ByteSpan {
        start: body_start as u32,
        end: facts.fn_span.end,
    };
    facts.body.snippet = facts.source.bytes[body_start..].into();
    let snapshot = facts.source.bytes.to_string();
    tests::source_owner(facts, &snapshot);
}

fn with_body(body: &str) -> HirFnFacts {
    let mut facts = tests::facts("pub fn original(value: u32) -> u32 { value }");
    let start = facts.body.span.start as usize;
    let source = format!("{}{body}", &facts.source.bytes[..start]);
    replace_source(&mut facts, source, start);
    facts
}

fn assert_retained_header(facts: &HirFnFacts) {
    let inspected = inspect(facts).unwrap();
    assert_eq!(inspected.execution, InspectionExecution::Completed);
    assert_eq!(
        inspected.source,
        Availability::Known(facts.source.bytes.as_str())
    );
    assert_eq!(
        inspected.signature,
        Availability::Known(Signature {
            parameters: 1,
            has_self: false,
        })
    );
    assert!(matches!(
        inspected.preceding_comment,
        Availability::Known(_)
    ));
    assert!(matches!(
        inspected.top_statements,
        Availability::NotApplicable { .. }
    ));
}

fn assert_statement_exhaustion(facts: &HirFnFacts) {
    let inspected = inspect_with_statements(facts).unwrap();
    assert_eq!(inspected.execution, InspectionExecution::BudgetExhausted);
    assert_eq!(
        inspected.signature,
        Availability::Known(Signature {
            parameters: 1,
            has_self: false,
        })
    );
    assert!(matches!(inspected.source, Availability::Known(_)));
    assert!(matches!(
        inspected.preceding_comment,
        Availability::Known(_)
    ));
    assert!(
        matches!(inspected.top_statements, Availability::Unknown { reason }
        if reason.contains("budget"))
    );
}

#[test]
fn ordinary_large_body_does_not_consume_signature_or_suppression_authority() {
    for body in [
        format!("{{ {} value }}", "let unused = 1; ".repeat(200)),
        format!("{{ /*{}*/ value }}", " ".repeat(256 * 1024)),
    ] {
        let mut facts = with_body(&body);
        facts.body.kind = "unsupported_original_body".into();
        facts.body.children.clear();
        assert_retained_header(&facts);
        assert_statement_exhaustion(&facts);
    }
}

#[test]
fn recursive_body_shapes_are_never_parsed_for_header_only_inspection() {
    for body in [
        format!("{{ {}value{} }}", "(".repeat(1000), ")".repeat(1000)),
        format!("{{ {}true }}", "!".repeat(1000)),
        format!("{{ {}0 }}", "return ".repeat(1000)),
        format!("{{ {}value }}", "value + ".repeat(1000)),
    ] {
        let facts = with_body(&body);
        assert_retained_header(&facts);
        assert_statement_exhaustion(&facts);
    }
}

#[test]
fn checked_header_rosters_and_boundaries_still_reject_contradictions_with_large_bodies() {
    let original = with_body(&format!("{{ {}value }}", "value + ".repeat(1000)));
    let mut missing = original.clone();
    missing.signature.params.clear();
    assert!(inspect(&missing).is_err());
    let mut truncated = original.clone();
    truncated.signature.sig_span.end = "pub fn original(value: u32)".len() as u32;
    assert!(inspect(&truncated).is_err());
    let mut widened = original;
    widened.signature.sig_span.end = widened.body.span.start;
    assert!(inspect(&widened).is_err());
}

#[test]
fn root_body_boundaries_require_original_braces_utf8_and_complete_item_extent() {
    let original = tests::facts("fn café(value: u32) -> u32 { value }");
    for span in [
        ByteSpan {
            start: original.body.span.start + 1,
            end: original.body.span.end,
        },
        ByteSpan {
            start: original.body.span.start,
            end: original.body.span.end - 1,
        },
        ByteSpan {
            start: original.body.span.start,
            end: original.body.span.end + 1,
        },
        ByteSpan {
            start: "fn caf".len() as u32 + 1,
            end: original.body.span.end,
        },
    ] {
        let mut changed = original.clone();
        changed.body.span = span;
        assert!(inspect(&changed).is_err(), "{span:?}");
    }
}

#[test]
fn oversized_header_is_unavailable_without_losing_owned_source_or_parsing_body() {
    let mut facts = tests::facts("fn f() {}");
    let header = format!("fn f(value: {}u32)", "&".repeat(1000));
    let start = header.len() + 1;
    replace_source(&mut facts, format!("{header} {{}}"), start);
    facts.signature.sig_span.end = header.len() as u32;
    for inspected in [
        inspect(&facts).unwrap(),
        inspect_with_statements(&facts).unwrap(),
    ] {
        assert_eq!(inspected.execution, InspectionExecution::BudgetExhausted);
        assert!(matches!(inspected.source, Availability::Known(_)));
        assert!(
            matches!(inspected.signature, Availability::Unknown { reason }
            if reason.contains("budget"))
        );
        assert!(matches!(
            inspected.preceding_comment,
            Availability::Unknown { .. }
        ));
    }
    facts.source.digest = crate::source_identity::checksum(b"different owner");
    assert!(inspect(&facts).is_err());
    assert!(inspect_with_statements(&facts).is_err());
}

#[test]
fn borrowed_header_parse_preserves_qualifiers_receivers_where_clauses_and_original_offsets() {
    for source in [
        "pub(crate) const unsafe extern \"C\" fn café(value: u32) -> u32\r\n/* π */ { value }",
        "pub fn generic<T>(value: T) -> T where T: Copy /* original */ { value }",
        "async fn method(&self, value: u32) -> u32 { value }",
    ] {
        let facts = tests::facts(source);
        let inspected = inspect(&facts).unwrap();
        assert_eq!(inspected.execution, InspectionExecution::Completed);
        assert_eq!(
            inspected.signature,
            Availability::Known(Signature {
                parameters: 1,
                has_self: source.contains("&self"),
            })
        );
        assert!(matches!(
            inspected.preceding_comment,
            Availability::Known(_)
        ));
    }
    let mut facts = tests::facts("fn generic<T>(value: T) -> T where T: Copy { value }");
    facts.signature.sig_span.end = "fn generic<T>(value: T) -> T".len() as u32;
    assert!(inspect(&facts).is_err());
}
