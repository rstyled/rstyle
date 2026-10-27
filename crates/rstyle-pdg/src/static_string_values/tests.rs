//! Source spelling and decoded static identity are independently retained.
use super::*;
use crate::{HirFnFacts, ResolvedTy};

fn fixture(spelling: &str) -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.build.frontend_options.insert(
        "extraction_profile".into(),
        serde_json::json!("hir-data-v3"),
    );
    facts.source.bytes = spelling.into();
    let token = crate::data_types::token(&crate::concrete_types::TypeTree::Shared(Box::new(
        crate::concrete_types::TypeTree::Str,
    )))
    .unwrap();
    facts.body.kind = "expr_lit".into();
    facts.body.span = ByteSpan {
        start: 0,
        end: spelling.len() as u32,
    };
    facts.body.snippet = spelling.into();
    facts.body.ty = Some(ResolvedTy {
        normalized: Some(token),
        observed: "&str".into(),
        spelling: String::new(),
    });
    facts.body.lit = Some(spelling.into());
    facts.body.children.clear();
    facts.body.res = None;
    facts.body.dispatch = None;
    facts.body.expansion = None;
    facts.body.binding = None;
    facts.body.binding_mut = None;
    facts.body.param_index = None;
    facts.body.is_unsafe_block = false;
    facts
}

#[test]
fn static_literal_descriptor_preserves_spelling_and_decoded_value() {
    for (spelling, expected) in [
        ("\"version\"", "version"),
        ("r#\"endianness check\"#", "endianness check"),
        ("\"\\u{76}ersion\"", "version"),
    ] {
        let facts = fixture(spelling);
        let descriptor = verify(&facts.body, &facts).unwrap();
        assert_eq!(descriptor.value, expected);
        assert_eq!(descriptor.spelling, spelling);
        assert_eq!(descriptor.hir, facts.body.hir);
        assert_eq!(descriptor.source_digest, facts.source.digest);
    }
}

#[test]
fn static_literal_identity_rejects_forged_metadata_and_hidden_work() {
    for mutation in ["spelling", "literal", "type", "child", "owner", "binding"] {
        let mut facts = fixture("\"version\"");
        match mutation {
            "spelling" => facts.body.snippet = "\"endianness check\"".into(),
            "literal" => facts.body.lit = Some("\"different\"".into()),
            "type" => facts.body.ty.as_mut().unwrap().normalized = Some("u32".into()),
            "child" => facts.body.children.push(facts.body.clone()),
            "owner" => facts.body.hir.krate = 1,
            "binding" => facts.body.binding = Some("invented".into()),
            _ => unreachable!(),
        }
        assert!(verify(&facts.body, &facts).is_err(), "{mutation}");
    }
}

#[test]
fn static_literal_requires_original_owner_and_no_expansion() {
    let facts = fixture("\"version\"");
    let mut changed = facts.body.clone();
    changed.hir.owner += 1;
    assert!(verify(&changed, &facts).is_err());

    changed = facts.body.clone();
    changed.expansion = Some(crate::ExpansionSite {
        callsite: changed.span,
        snippet: changed.snippet.clone(),
    });
    assert!(verify(&changed, &facts).is_err());
}
