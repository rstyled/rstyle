use super::*;
use crate::{ParamFact, SourceText};

fn facts(source: &str) -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.source.bytes = SourceText::from(source);
    facts.source.digest = facts.source.bytes.checksum().into();
    facts.fn_span = ByteSpan {
        start: 0,
        end: source.len() as u32,
    };
    facts.source_scan_complete = false;
    source_owner(&mut facts, source);
    signature(&mut facts, source);
    facts
}

fn source_owner(facts: &mut HirFnFacts, source: &str) {
    facts.build.frontend_options = Default::default();
    let mut context = nominal_boundary::tests::context();
    context.package.manifest_digest = context.package.manifest_bytes.checksum().into();
    let path = context.root_source.clone();
    facts.build.frontend_options.insert(
        nominal_boundary::CONTEXT_KEY.into(),
        serde_json::to_value(&context).unwrap(),
    );
    facts
        .build
        .frontend_options
        .insert("source_file".into(), serde_json::json!(path));
    facts
        .build
        .frontend_options
        .insert_data(Arc::new(census::Census {
            identity: census::IDENTITY.into(),
            context,
            files: vec![census::UnitFile {
                path,
                bytes: source.into(),
                digest: facts.source.digest.clone(),
            }],
            expansions: vec![],
            documentation: vec![],
            external_imports: vec![],
        }));
}

fn signature(facts: &mut HirFnFacts, source: &str) {
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    // Captured function bytes begin at the full header, including visibility.
    facts.signature.sig_span = ByteSpan {
        start: 0,
        end: relative_span(source, function.sig.span()).end,
    };
    facts.signature.has_self = function.sig.receiver().is_some();
    facts.signature.is_variadic = function.sig.variadic.is_some();
    facts.signature.params = function
        .sig
        .inputs
        .iter()
        .enumerate()
        .map(|(position, input)| {
            let (pattern, ty) = match input {
                syn::FnArg::Typed(argument) => (argument.pat.span(), argument.ty.span()),
                syn::FnArg::Receiver(receiver) => (receiver.self_token.span(), receiver.span()),
            };
            let span = relative_span(source, pattern);
            let mut pat = facts.body.clone();
            pat.span = span;
            pat.snippet = crate::source_evidence::byte_range(source, span)
                .unwrap()
                .into();
            pat.param_index = Some(position as u32);
            ParamFact {
                symbol: String::new(),
                position: position as u32,
                name_span: span,
                ty_span: relative_span(source, ty),
                ty: facts.signature.return_ty.clone(),
                pat,
            }
        })
        .collect();
    facts.signature.receiver = function.sig.receiver().map(|_| crate::ReceiverFact {
        span: facts.signature.params[0].name_span,
        snippet: facts.signature.params[0].pat.snippet.clone(),
    });
}

#[test]
fn known_empty_signature_differs_from_missing_ownership() {
    let mut facts = facts("fn empty() {}");
    assert_eq!(
        inspect(&facts).unwrap().signature,
        Availability::Known(Signature {
            parameters: 0,
            has_self: false
        })
    );
    facts.build.frontend_options = Default::default();
    assert!(matches!(
        inspect(&facts).unwrap().signature,
        Availability::Unknown { .. }
    ));
}

#[test]
fn unsupported_bodies_and_types_do_not_gate_exact_signature_or_source() {
    for (source, expected, has_self) in [
        (
            "fn opaque<T>((a, b): (T, T), _: T) { let x = || unknown!(); }",
            2,
            false,
        ),
        (
            "fn method(&self, value: impl Iterator<Item = u32>) { unknown!(value); }",
            1,
            true,
        ),
        ("fn café(π: u32) {\r\n    unknown!();\r\n}", 1, false),
    ] {
        let mut facts = facts(source);
        facts.body.kind = "unimplemented_body".into();
        facts.body.children.clear();
        let inspected = inspect(&facts).unwrap();
        assert_eq!(inspected.source, Availability::Known(source));
        assert_eq!(
            inspected.signature,
            Availability::Known(Signature {
                parameters: expected,
                has_self
            })
        );
    }
}

#[test]
fn variadic_signature_is_unsupported_but_complete_source_remains_known() {
    let facts = facts("unsafe extern \"C\" fn variadic(first: u32, ...) {}");
    let inspected = inspect(&facts).unwrap();
    assert!(matches!(inspected.source, Availability::Known(_)));
    assert!(matches!(
        inspected.signature,
        Availability::Unsupported { .. }
    ));
}

#[test]
fn contradictory_checksum_span_context_and_roster_fail_without_facts() {
    let original = facts("fn inputs(a: u32, b: u32) {}");
    let mut edited = original.clone();
    edited.source.digest = crate::source_identity::checksum(b"other");
    assert!(inspect(&edited).is_err());
    edited = original.clone();
    edited.fn_span.end -= 1;
    assert!(inspect(&edited).is_err());
    edited = original.clone();
    edited.signature.params.pop();
    assert!(inspect(&edited).is_err());
    edited = original.clone();
    edited.signature.params[1].position = 0;
    assert!(inspect(&edited).is_err());
    edited = original;
    let mut context = nominal_boundary::tests::context();
    context.package.manifest_digest = context.package.manifest_bytes.checksum().into();
    context.crate_name = "another_unit".into();
    edited.build.frontend_options.insert(
        nominal_boundary::CONTEXT_KEY.into(),
        serde_json::to_value(context).unwrap(),
    );
    assert!(inspect(&edited).is_err());
}

#[test]
fn receiver_contradiction_and_changed_file_bytes_are_errors() {
    let mut edited = facts("fn method(&self, input: u32) {}");
    edited.signature.has_self = false;
    assert!(inspect(&edited).is_err());
    let mut edited = facts("fn original() {}");
    let mut census = (**edited
        .build
        .frontend_options
        .data::<Arc<census::Census>>()
        .unwrap())
    .clone();
    census.files[0].bytes = "fn changed_() {}".into();
    census.files[0].digest = census.files[0].bytes.checksum().into();
    edited.build.frontend_options.insert_data(Arc::new(census));
    assert!(inspect(&edited).is_err());
}

#[test]
fn availability_keeps_empty_values_and_unknown_unsupported_separate() {
    let value = serde_json::to_value(LocalFacts {
        source: Availability::Known(""),
        signature: Availability::Unsupported {
            reason: "variadic".into(),
        },
    })
    .unwrap();
    assert_eq!(value["source"]["status"], "known");
    assert_eq!(value["source"]["value"], "");
    assert_eq!(value["signature"]["status"], "unsupported");
}

#[test]
fn expanded_function_is_unavailable_without_hiding_body_macro_sources() {
    let mut facts = facts("fn generated(a: u32) { opaque!(); }");
    assert!(matches!(
        inspect(&facts).unwrap().signature,
        Availability::Known(_)
    ));
    facts.source_diagnostics.push(crate::HirDiagnostic {
        capability: "rust.macros".into(),
        construct: "macro_generated_function".into(),
        span: facts.signature.sig_span,
        snippet: String::new(),
        severity: "error".into(),
        reason: "expanded".into(),
    });
    let inspected = inspect(&facts).unwrap();
    assert!(matches!(inspected.source, Availability::Unsupported { .. }));
    assert!(matches!(
        inspected.signature,
        Availability::Unsupported { .. }
    ));
}

#[test]
fn oversized_ownership_inventory_cannot_yield_local_facts() {
    let mut facts = facts("fn small() {}");
    let mut census = (**facts
        .build
        .frontend_options
        .data::<Arc<census::Census>>()
        .unwrap())
    .clone();
    census
        .files
        .resize(census::MAX_FILES + 1, census.files[0].clone());
    facts.build.frontend_options.insert_data(Arc::new(census));
    assert!(inspect(&facts).is_err());
}

#[test]
fn full_header_spans_include_public_restricted_and_private_visibility() {
    for header in [
        "fn local(input: u32)",
        "pub fn public(input: u32)",
        "pub(crate) fn restricted(input: u32)",
        "pub(in crate::module) async fn scoped<T>(input: T) -> T where T: Copy",
        "pub const unsafe extern \"C\" fn qualified(input: u32) -> u32",
    ] {
        let source = format!("{header} {{ input }}");
        let mut facts = facts(&source);
        // Independent byte expectations, not a projection of the parser helper.
        facts.signature.sig_span = ByteSpan {
            start: 0,
            end: header.len() as u32,
        };
        assert!(
            matches!(inspect(&facts).unwrap().signature, Availability::Known(_)),
            "{header}"
        );
        if header.starts_with("pub") {
            facts.signature.sig_span.start = header.find("fn ").unwrap() as u32;
            assert!(
                inspect(&facts).is_err(),
                "Visibility cannot be omitted: {header}"
            );
        }
        for changed in [
            ByteSpan {
                start: 1,
                end: header.len() as u32,
            },
            ByteSpan {
                start: 0,
                end: header.len() as u32 - 1,
            },
            ByteSpan {
                start: 0,
                end: header.len() as u32 + 1,
            },
        ] {
            facts.signature.sig_span = changed;
            assert!(inspect(&facts).is_err(), "{header}: {changed:?}");
        }
    }
}
