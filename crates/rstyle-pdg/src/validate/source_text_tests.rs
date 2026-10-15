use super::*;

#[test]
fn owned_identity_validation_matches_public_boundary_checks() {
    let mut example: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vendor-compat/pdg.example.json")).unwrap();
    example["function"]["reference_count"]["policy"] = example["profile"].clone();
    let policy: ProfileId = serde_json::from_value(example["profile"].clone()).unwrap();
    for edge in example["edges"].as_array_mut().unwrap() {
        edge["policy"] = serde_json::to_value(&policy).unwrap();
    }
    example["extraction_evidence"]["conformance"] =
        serde_json::to_value(vec![Conformance::not_evaluated(policy, "parity fixture")]).unwrap();
    let mut document: PdgDocument = serde_json::from_value(example).unwrap();
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
