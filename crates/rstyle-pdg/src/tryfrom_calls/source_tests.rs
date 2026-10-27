//! Source anchors and complete member roster are independent from body evidence.
use super::*;
use crate::tryfrom_calls::tests::{definition, facts};
use serde_json::{Value, json};

const SOURCE: &str = "// adjacent source remains part of the owned file\nimpl TryFrom<usize> for Index { type Error = IndexError; fn try_from(i: usize) -> Result<Index, IndexError> { todo!() } }";

fn fixture() -> (HirFnFacts, syn::ItemImpl, Value) {
    let item: syn::ItemImpl = syn::parse_str(SOURCE).unwrap();
    let syn::ImplItem::Fn(method) = &item.items[1] else {
        panic!("method")
    };
    let mut facts = facts();
    facts.fn_span = method_range(SOURCE, method);
    facts.source.bytes = crate::source_evidence::byte_range(SOURCE, facts.fn_span)
        .unwrap()
        .into();
    facts
        .build
        .frontend_options
        .insert("source_file".into(), json!("original.rs"));
    let anchor = |span| {
        json!({"path":"original.rs", "span":span,
        "digest":crate::source_identity::checksum(SOURCE.as_bytes())})
    };
    let implementation = ByteSpan {
        start: crate::source_evidence::original_span(SOURCE, item.impl_token.span).start,
        end: crate::source_evidence::original_span(SOURCE, item.brace_token.span.close()).end,
    };
    let associated = anchor(crate::source_evidence::original_span(
        SOURCE,
        item.items[0].span(),
    ));
    let signature = json!({"inputs":[], "output":[], "bound_variables":0,
        "abi":"\"Rust\"", "safety":"Safe", "variadic":false});
    let method = definition("Index::try_from", true);
    let trait_method = definition("core::convert::TryFrom::try_from", false);
    let error = definition("Index::Error", true);
    let trait_error = definition("core::convert::TryFrom::Error", false);
    let observation = json!({
        "identity":"rstyle.rust.concrete-conversion-member-observation/1", "admission_authority":false,
        "method":method, "implementation":definition("Index-impl", true),
        "trait_definition":definition("core::convert::TryFrom", false), "trait_method":trait_method,
        "implementation_source":anchor(implementation), "method_source":anchor(facts.fn_span),
        "method_header":{"is_const":false,"is_async":false},
        "members":[{"definition":error,"trait_member":trait_error,"source":associated},
            {"definition":method,"trait_member":trait_method,"source":anchor(facts.fn_span)}],
        "trait_arguments":[], "selected":method, "selected_arguments":[],
        "requested_signature":signature, "selected_signature":signature,
        "associated_error":{"trait_member":trait_error,"implementation_member":error,"source":associated,
            "declared_type":[],"normalized_projection":[]}
    });
    (facts, item, observation)
}

fn checked(facts: &HirFnFacts, item: &syn::ItemImpl, raw: Value) -> Result<(), String> {
    let observation: model::Observation =
        serde_json::from_value(raw).map_err(|error| error.to_string())?;
    let checker = Checker {
        facts,
        bytes: SOURCE,
        observation: &observation,
        anchors: crate::source_evidence::trait_members::SourceAnchors::new(
            facts,
            SOURCE,
            "TryFrom source anchor differs from complete original file",
        ),
    };
    checker.source(
        item,
        ByteSpan {
            start: crate::source_evidence::original_span(SOURCE, item.impl_token.span).start,
            end: crate::source_evidence::original_span(SOURCE, item.brace_token.span.close()).end,
        },
    )
}

#[test]
fn source_requires_whole_file_checksum_exact_ranges_and_complete_original_member_order() {
    let (facts, item, original) = fixture();
    assert!(checked(&facts, &item, original.clone()).is_ok());
    let mut attacks = Vec::new();
    for field in ["method_source", "implementation_source"] {
        for (key, value) in [
            ("path", json!("other.rs")),
            ("digest", json!("xxh3-64:0000000000000000")),
            ("span", json!({"start":0,"end":1})),
        ] {
            let mut attack = original.clone();
            attack[field][key] = value;
            attacks.push(attack);
        }
    }
    let mut reordered = original.clone();
    reordered["members"].as_array_mut().unwrap().reverse();
    attacks.push(reordered);
    let mut missing = original.clone();
    missing["members"].as_array_mut().unwrap().pop();
    attacks.push(missing);
    let mut extra = original.clone();
    extra["members"]
        .as_array_mut()
        .unwrap()
        .push(original["members"][0].clone());
    attacks.push(extra);
    let mut wrong_error = original.clone();
    wrong_error["associated_error"]["source"]["path"] = json!("elsewhere.rs");
    attacks.push(wrong_error);
    let mut snippet_hash = original.clone();
    snippet_hash["method_source"]["digest"] = json!(crate::source_identity::checksum(
        facts.source.bytes.as_bytes()
    ));
    attacks.push(snippet_hash);
    for attack in attacks {
        assert!(checked(&facts, &item, attack).is_err());
    }
}

#[test]
fn source_requires_exact_captured_function_bytes_and_member_identities() {
    let (mut facts, item, original) = fixture();
    facts.source.bytes = "fn unrelated() {}".into();
    assert!(checked(&facts, &item, original.clone()).is_err());
    let (facts, item, _) = fixture();
    for position in [0, 1] {
        for field in ["definition", "trait_member"] {
            let mut attack = original.clone();
            attack["members"][position][field]["definition_identity"] = json!("another-definition");
            assert!(checked(&facts, &item, attack).is_err());
        }
    }
}
