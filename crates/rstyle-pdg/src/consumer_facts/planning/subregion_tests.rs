use super::*;

fn captured(source: &str) -> HirFnFacts {
    let mut facts = super::tests::facts(source, "// π\r\n");
    facts.body.span = ByteSpan {
        start: source.find('{').unwrap() as u32,
        end: source.len() as u32,
    };
    facts.body.snippet = source[facts.body.span.start as usize..].into();
    facts
}

fn selected(facts: &HirFnFacts, span: ByteSpan) -> BodyRegion {
    match inspect_subregion(facts, span).unwrap() {
        Availability::Known(region) => region,
        other => panic!("Expected original subregion: {other:?}"),
    }
}

#[test]
fn subregion_preserves_independent_utf8_crlf_offsets_and_shared_owner() {
    let source = "fn café() {\r\n    let π = 1;\r\n    π\r\n}";
    let facts = captured(source);
    let start = source.find("let").unwrap() as u32;
    let end = start + "let π = 1;".len() as u32;
    let region = selected(&facts, ByteSpan { start, end });
    assert_eq!(region.span.start, "// π\r\n".len() as u32 + start);
    assert_eq!(region.span.end, "// π\r\n".len() as u32 + end);
    assert_eq!(
        crate::source_evidence::byte_range(region.snapshot(), region.span).unwrap(),
        "let π = 1;"
    );
    let owner = facts
        .build
        .frontend_options
        .data::<Arc<census::Census>>()
        .unwrap();
    assert!(Arc::ptr_eq(&region.owner, &owner));
    assert!(std::ptr::eq(
        region.snapshot().as_ptr(),
        owner.files[0].bytes.as_ptr()
    ));
}

#[test]
fn subregion_checks_body_containment_nonempty_ranges_and_utf8_boundaries() {
    let facts = captured("fn café() { π }");
    selected(&facts, facts.body.span);
    let inside_character = facts.source.bytes.find('π').unwrap() as u32 + 1;
    for span in [
        ByteSpan {
            start: 0,
            end: facts.body.span.start,
        },
        ByteSpan {
            start: facts.body.span.end,
            end: facts.body.span.end,
        },
        ByteSpan {
            start: facts.body.span.start,
            end: facts.body.span.end + 1,
        },
        ByteSpan {
            start: inside_character,
            end: inside_character + 1,
        },
        ByteSpan {
            start: facts.body.span.end,
            end: facts.body.span.start,
        },
    ] {
        assert!(inspect_subregion(&facts, span).is_err(), "{span:?}");
    }
}

#[test]
fn subregion_requires_owned_unexpanded_source_before_native_ranges() {
    let mut facts = captured("fn f() {}");
    let span = facts.body.span;
    facts.build.frontend_options = Default::default();
    facts.body.span = ByteSpan {
        start: 999,
        end: 1000,
    };
    assert!(matches!(
        inspect_subregion(&facts, span).unwrap(),
        Availability::Unknown { .. }
    ));
    let mut facts = captured("fn f() {}");
    facts.source_diagnostics.push(crate::HirDiagnostic {
        capability: "rust.macros".into(),
        construct: "macro_generated_function".into(),
        severity: "error".into(),
        reason: "expanded".into(),
        span,
        snippet: String::new(),
    });
    assert!(matches!(
        inspect_subregion(&facts, span).unwrap(),
        Availability::Unsupported { .. }
    ));
    facts.source.digest = "corrupt".into();
    assert!(inspect_subregion(&facts, span).is_err());
}

#[test]
fn subregion_rejects_native_body_changes_without_parsing_opaque_body() {
    let mut facts = captured("fn f() { opaque!(); }");
    let span = facts.body.span;
    facts.body.kind = "unsupported".into();
    facts.body.children.clear();
    selected(&facts, span);
    facts.body.snippet = "{}".into();
    assert!(inspect_subregion(&facts, span).is_err());
    let mut facts = captured("fn f() { opaque!(); }");
    facts.body.span.start += 1;
    assert!(inspect_subregion(&facts, span).is_err());
}

#[test]
fn subregion_does_not_apply_syntax_budget_or_parse_body() {
    let source = format!("fn f() {{ {} }}", "(".repeat(256 * 1024));
    let facts = captured(&source);
    selected(&facts, facts.body.span);
}
