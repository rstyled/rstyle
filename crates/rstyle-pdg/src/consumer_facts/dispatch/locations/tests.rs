use super::*;
use crate::{ByteSpan, SourceText, nominal_boundary};

fn census(files: &[(&str, &str)]) -> census::Census {
    let mut context = nominal_boundary::tests::context();
    context.package.manifest_digest = context.package.manifest_bytes.checksum().into();
    census::Census {
        identity: census::IDENTITY.into(),
        context,
        files: files
            .iter()
            .map(|(path, source)| {
                let bytes: SourceSnapshot = (*source).into();
                census::UnitFile {
                    path: (*path).into(),
                    digest: bytes.checksum().into(),
                    bytes,
                }
            })
            .collect(),
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    }
}

fn facts(path: &str, source: &str, name: &str) -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let parsed = syn::parse_file(source.strip_prefix('\u{feff}').unwrap_or(source)).unwrap();
    let function = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .unwrap();
    let range = shapes::function_range(source, function);
    facts.source.bytes =
        SourceText::from(crate::source_evidence::byte_range(source, range).unwrap());
    facts.source.digest = facts.source.bytes.checksum().into();
    facts.fn_span = range;
    facts.signature.sig_span = ByteSpan {
        start: 0,
        end: shapes::function_header(source, function).end - range.start,
    };
    facts.signature.params.clear();
    facts.signature.has_self = false;
    facts.signature.receiver = None;
    facts.source_diagnostics.clear();
    facts.build.cfgs.clear();
    facts
        .build
        .frontend_options
        .insert("source_file".into(), serde_json::json!(path));
    facts
        .build
        .frontend_options
        .insert("source_identifier".into(), serde_json::json!(name));
    facts
}

#[test]
fn external_files_with_identical_offsets_and_names_keep_distinct_namespaces() {
    let source = "pub fn same() {}";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod first; mod second;"),
        ("/sample/src/first.rs", source),
        ("/sample/src/second/mod.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    for (path, qualified) in [
        ("/sample/src/first.rs", "crate::first::same"),
        ("/sample/src/second/mod.rs", "crate::second::same"),
    ] {
        let facts = facts(path, source, "same");
        assert_eq!(
            locations.location(&facts, source).unwrap(),
            (qualified.into(), "free-function")
        );
    }
}

#[test]
fn external_under_inline_and_external_modules_has_complete_qualification() {
    let source = "fn value() {}";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod outer { mod middle; }"),
        ("/sample/src/outer/middle.rs", "mod leaf;"),
        ("/sample/src/outer/middle/leaf.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    let facts = facts("/sample/src/outer/middle/leaf.rs", source, "value");
    assert_eq!(
        locations.location(&facts, source).unwrap().0,
        "crate::outer::middle::leaf::value"
    );
}

#[test]
fn bom_crlf_and_unicode_preserve_original_external_function_offsets() {
    let source = "\u{feff}// 🦀\r\npub fn café() {}\r\n";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod worker;"),
        ("/sample/src/worker.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    let facts = facts("/sample/src/worker.rs", source, "café");
    assert_eq!(facts.fn_span.start, 12);
    assert_eq!(
        locations.location(&facts, source).unwrap().0,
        "crate::worker::café"
    );
}

#[test]
fn selected_module_and_function_cfgs_keep_exact_lookup_context() {
    let source = "#[cfg(feature=\"enabled\")] fn selected() {} #[cfg(not(feature=\"enabled\"))] fn selected() {}";
    let proof = census(&[
        (
            "/sample/src/lib.rs",
            "#[cfg(feature=\"enabled\")] mod worker; #[cfg(not(feature=\"enabled\"))] mod absent;",
        ),
        ("/sample/src/worker.rs", source),
    ]);
    let cfgs = vec!["feature=\"enabled\"".into()];
    let locations = Locations::new(&proof, &cfgs).unwrap();
    let mut facts = facts("/sample/src/worker.rs", source, "selected");
    facts.build.cfgs = cfgs;
    assert_eq!(
        locations.location(&facts, source).unwrap().0,
        "crate::worker::selected"
    );
    facts.build.cfgs.clear();
    assert!(
        locations
            .location(&facts, source)
            .unwrap_err()
            .contains("cfg context")
    );
    assert!(Locations::new(&proof, &[]).is_err());
}

#[test]
fn missing_ambiguous_orphan_and_duplicate_files_prevent_publication() {
    let cases: &[&[(&str, &str)]] = &[
        &[("/sample/src/lib.rs", "mod worker;")],
        &[
            ("/sample/src/lib.rs", "mod worker;"),
            ("/sample/src/worker.rs", ""),
            ("/sample/src/worker/mod.rs", ""),
        ],
        &[
            ("/sample/src/lib.rs", "mod worker;"),
            ("/sample/src/worker.rs", ""),
            ("/sample/src/orphan.rs", ""),
        ],
        &[
            ("/sample/src/lib.rs", "mod worker;"),
            ("/sample/src/worker.rs", ""),
            ("/sample/src/worker.rs", ""),
        ],
    ];
    for files in cases {
        assert!(Locations::new(&census(files), &[]).is_err(), "{files:?}");
    }
}

#[test]
fn duplicate_qualified_declarations_and_path_remaps_remain_unavailable() {
    for (root, worker) in [
        ("mod worker;", "fn same() {} fn same() {}"),
        ("mod worker;", "mod nested {} mod nested {}"),
        ("#[path=\"worker.rs\"] mod worker;", "fn same() {}"),
    ] {
        let proof = census(&[
            ("/sample/src/lib.rs", root),
            ("/sample/src/worker.rs", worker),
        ]);
        assert!(Locations::new(&proof, &[]).is_err(), "{root} {worker}");
    }
}

#[test]
fn lookup_checks_immutable_file_and_owned_function_bytes_header_and_identifier() {
    let source = "pub fn value() {}";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod worker;"),
        ("/sample/src/worker.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    for change in 0..5 {
        let mut facts = facts("/sample/src/worker.rs", source, "value");
        match change {
            0 => facts.signature.sig_span.end += 1,
            1 => facts.fn_span.start += 1,
            2 => facts.source.bytes = "different".into(),
            3 => {
                facts.build.frontend_options.insert(
                    "source_file".into(),
                    serde_json::json!("/sample/src/absent.rs"),
                );
            }
            _ => {
                facts
                    .build
                    .frontend_options
                    .insert("source_identifier".into(), serde_json::json!("another"));
            }
        }
        assert!(
            locations.location(&facts, source).is_err(),
            "change {change}"
        );
    }
    let facts = facts("/sample/src/worker.rs", source, "value");
    assert!(locations.location(&facts, "pub fn value() { }").is_err());
}

#[test]
fn item_and_byte_budget_exhaustion_never_publishes_a_partial_population() {
    let many = "fn repeated() {}".repeat(4097);
    let oversized = " ".repeat(256 * 1024 + 1);
    for source in [many, oversized] {
        let proof = census(&[
            ("/sample/src/lib.rs", "mod worker;"),
            ("/sample/src/worker.rs", &source),
        ]);
        let error = Locations::new(&proof, &[]).err().expect("budget exceeded");
        assert!(error.contains("budget"), "{error}");
    }
}

#[test]
fn root_and_generated_functions_do_not_acquire_external_free_identity() {
    let source = "fn value() {} struct Item; impl Item { fn method() {} }";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod worker; fn root() {}"),
        ("/sample/src/worker.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    let root = facts("/sample/src/lib.rs", "mod worker; fn root() {}", "root");
    assert!(
        locations
            .location(&root, "mod worker; fn root() {}")
            .is_err()
    );
    let mut generated = facts("/sample/src/worker.rs", source, "value");
    generated.source_diagnostics.push(crate::HirDiagnostic {
        capability: "rust.macros".into(),
        construct: "macro_generated_function".into(),
        span: generated.signature.sig_span,
        snippet: String::new(),
        severity: "error".into(),
        reason: "expanded".into(),
    });
    assert!(
        locations
            .location(&generated, source)
            .unwrap_err()
            .contains("Generated")
    );
}

#[test]
fn mismatched_method_headers_do_not_acquire_external_free_identity() {
    let source = "fn value() {} struct Item; impl Item { fn method() {} }";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod worker;"),
        ("/sample/src/worker.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    let function = &syn::parse_file(source).unwrap().items[2];
    let syn::Item::Impl(implementation) = function else {
        panic!("impl fixture")
    };
    let syn::ImplItem::Fn(method) = &implementation.items[0] else {
        panic!("method fixture")
    };
    let mut original_method = facts("/sample/src/worker.rs", source, "value");
    original_method.fn_span =
        crate::source_evidence::original_span(source, syn::spanned::Spanned::span(method));
    original_method.source.bytes =
        crate::source_evidence::byte_range(source, original_method.fn_span)
            .unwrap()
            .into();
    original_method.source.digest = original_method.source.bytes.checksum().into();
    assert!(locations.location(&original_method, source).is_err());
}

fn associated_facts(source: &str, name: &str) -> HirFnFacts {
    let declarations = methods::declarations(syn::parse_file(source).unwrap().items, &[]).unwrap();
    let (_, function, _) = declarations
        .iter()
        .find(|(candidate, _, _)| candidate == name)
        .unwrap();
    let mut facts = facts("/sample/src/worker.rs", source, "seed");
    facts.fn_span = shapes::function_range(source, function);
    facts.source.bytes = crate::source_evidence::byte_range(source, facts.fn_span)
        .unwrap()
        .into();
    facts.source.digest = facts.source.bytes.checksum().into();
    facts.signature.sig_span.end =
        shapes::function_header(source, function).end - facts.fn_span.start;
    facts.build.frontend_options.insert(
        "source_identifier".into(),
        serde_json::json!(function.sig.ident.to_string()),
    );
    facts
}

#[test]
fn inherent_and_distinct_trait_methods_keep_exact_external_qualified_identities() {
    let source = "fn seed() {} struct Item; trait One { fn read(); } trait Two { fn read(); } impl Item { fn read() {} } impl One for Item { fn read() {} } impl Two for Item { fn read() {} }";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod worker;"),
        ("/sample/src/worker.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    for name in ["Item::read", "<Item as One>::read", "<Item as Two>::read"] {
        let facts = associated_facts(source, name);
        assert_eq!(
            locations.location(&facts, source).unwrap(),
            (format!("crate::worker::{name}"), "method")
        );
    }
}

#[test]
fn duplicate_external_method_identities_refuse_the_whole_location_index() {
    for source in [
        "struct Item; impl Item { fn read() {} fn read() {} }",
        "struct Item; impl Item { fn read() {} } impl Item { fn read() {} }",
        "struct Item; trait One { fn read(); } impl One for Item { fn read() {} } impl One for Item { fn read() {} }",
    ] {
        let proof = census(&[
            ("/sample/src/lib.rs", "mod worker;"),
            ("/sample/src/worker.rs", source),
        ]);
        let error = Locations::new(&proof, &[]).err().unwrap();
        assert!(error.contains("ambiguous qualified"), "{error}");
    }
}

#[test]
fn impl_and_trait_members_count_toward_the_complete_declaration_budget() {
    let methods: String = (0..4097)
        .map(|index| format!("fn member_{index}() {{}}"))
        .collect();
    for declaration in [
        format!("struct Item; impl Item {{{methods}}}"),
        format!("trait Capability {{{methods}}}"),
    ] {
        let source = format!("fn observed() {{}} {declaration}");
        let proof = census(&[
            ("/sample/src/lib.rs", "mod worker;"),
            ("/sample/src/worker.rs", &source),
        ]);
        let error = Locations::new(&proof, &[])
            .err()
            .expect("member budget exceeded");
        assert!(error.contains("population budget"), "{error}");
    }
}

#[test]
fn lifetime_owner_location_checks_original_header_and_owned_function_bytes() {
    let source = "fn seed() {} struct Item<'a>(&'a str); impl Item<'_> { fn read() {} }";
    let proof = census(&[
        ("/sample/src/lib.rs", "mod worker;"),
        ("/sample/src/worker.rs", source),
    ]);
    let locations = Locations::new(&proof, &[]).unwrap();
    let original = associated_facts(source, "Item::read");
    assert_eq!(
        locations.location(&original, source).unwrap(),
        ("crate::worker::Item::read".into(), "method")
    );
    let mut changed = original.clone();
    changed.signature.sig_span.end += 1;
    assert!(locations.location(&changed, source).is_err());
    changed = original.clone();
    changed.source.bytes = "fn read() { }".into();
    changed.source.digest = changed.source.bytes.checksum().into();
    assert!(locations.location(&changed, source).is_err());
    assert!(
        locations
            .location(&original, &source.replace("'_", "'static"))
            .is_err()
    );
}

#[test]
fn canonical_lifetime_method_collisions_refuse_the_entire_location_index() {
    for implementation in [
        "impl Item<'static> { fn read() {} } impl Item<'_> { fn read() {} }",
        "impl<'a> Item<'a> { fn read() {} } impl<'b> Item<'b> { fn read() {} }",
        "impl Read for Item<'static> { fn read() {} } impl Read for Item<'_> { fn read() {} }",
    ] {
        let source = format!("struct Item<'a>(&'a str); trait Read {{}} {implementation}");
        let proof = census(&[
            ("/sample/src/lib.rs", "mod worker;"),
            ("/sample/src/worker.rs", &source),
        ]);
        let error = Locations::new(&proof, &[]).err().unwrap();
        assert!(error.contains("ambiguous qualified"), "{error}");
    }
}
