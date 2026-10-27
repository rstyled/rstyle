use super::*;

fn document() -> PdgDocument {
    let mut example: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vendor-compat/pdg.example.json")).unwrap();
    example["function"]["reference_count"]["policy"] = example["profile"].clone();
    let policy: ProfileId = serde_json::from_value(example["profile"].clone()).unwrap();
    for edge in example["edges"].as_array_mut().unwrap() {
        edge["policy"] = serde_json::to_value(&policy).unwrap();
    }
    example["extraction_evidence"]["conformance"] =
        serde_json::to_value(vec![Conformance::not_evaluated(policy, "parity fixture")]).unwrap();
    serde_json::from_value(example).unwrap()
}

#[test]
fn owned_identity_validation_matches_public_boundary_checks() {
    let mut document = document();
    document.sources[0].content_identity = StringFact::known(&crate::concrete_types::digest(b"a"));
    let mut alias = document.sources[0].clone();
    alias.id = "alias".into();
    document.sources.push(alias);
    for text in [
        crate::SourceText::from("a"),
        crate::SourceText::from("edited é"),
    ] {
        for (end, id) in [(1, "alias"), (99, "alias"), (1, "missing")] {
            document.function.source_span.end = end;
            document.function.source_span.source_id = id.into();
            let bytes = CapturedSources::new(&document.sources, text.as_bytes());
            for context in [
                ValidationContext::FixtureOnly {
                    note: "parity".into(),
                },
                ValidationContext::CompilerEvidence {
                    pointer_width_bits: 64,
                },
            ] {
                assert_eq!(
                    validate_sources(&document, &context, &bytes),
                    validate_source_text(&document, &context, &text)
                );
            }
        }
    }
    document.sources[1].content_identity =
        StringFact::known(&crate::concrete_types::digest(b"different"));
    let text = crate::SourceText::from("a");
    let context = ValidationContext::CompilerEvidence {
        pointer_width_bits: 64,
    };
    let report = validate_source_text(&document, &context, &text);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "pdg.source-digest")
    );
}

#[test]
fn source_checksum_formats_fail_closed_at_owned_and_borrowed_boundaries() {
    let context = ValidationContext::CompilerEvidence {
        pointer_width_bits: 64,
    };
    let text = crate::SourceText::from("a");
    let mut doc = document();
    doc.profile.id = crate::extraction_profile::DATA_ID.into();
    for claim in [
        text.checksum(),
        text.identity(),
        "xxh3-64:0000000000000000",
        "xxh3-64:invalid",
        "sha256:invalid",
        "other:0000000000000000",
    ] {
        doc.sources[0].content_identity = StringFact::known(claim);
        let bytes = CapturedSources::new(&doc.sources, text.as_bytes());
        let owned = validate_source_text(&doc, &context, &text);
        assert_eq!(owned, validate_sources(&doc, &context, &bytes));
        let source_errors: Vec<_> = owned
            .issues
            .iter()
            .filter(|issue| issue.code.starts_with("pdg.source-"))
            .map(|issue| issue.code)
            .collect();
        if claim == text.checksum() || claim == text.identity() {
            assert!(source_errors.is_empty(), "{claim}: {source_errors:?}");
        } else {
            assert!(!source_errors.is_empty(), "{claim}");
        }
    }
    doc.sources[0].content_identity = StringFact::known(text.checksum());
    let missing = validate(&doc, &context, &BTreeMap::new());
    assert!(
        missing
            .issues
            .iter()
            .any(|issue| issue.code == "pdg.source-bytes")
    );
    doc.profile.id = "compat.rust.hir-r0".into();
    let r0 = validate_source_text(&doc, &context, &text);
    assert!(
        r0.issues
            .iter()
            .any(|issue| issue.code == "pdg.source-identity")
    );
}
