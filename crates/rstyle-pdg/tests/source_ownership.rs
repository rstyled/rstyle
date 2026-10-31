//! Source-only ownership expectations, independent of producer graphs.

use rstyle_pdg::source_evidence::functions;

#[test]
fn nested_functions_have_distinct_exact_owners() {
    let source = "#[inline]\npub fn outer() { fn inner() {} inner(); }";
    let definitions = functions(source).unwrap();
    assert_eq!(definitions.len(), 2);
    assert_eq!(
        definitions[0].function_source,
        "pub fn outer() { fn inner() {} inner(); }"
    );
    assert_eq!(definitions[1].function_source, "fn inner() {}");
    assert_eq!(definitions[0].function_span.start, 10);
    assert_eq!(definitions[1].function_span.start, 27);
}

#[test]
fn same_named_conditional_and_method_owners_remain_distinct() {
    let source =
        "#[cfg(a)] fn f() {}\n#[cfg(b)] fn f() {}\nstruct S; impl S { pub fn f(&self) {} }";
    let definitions = functions(source).unwrap();
    assert_eq!(definitions.len(), 3);
    assert!(definitions.iter().all(|definition| definition.name == "f"));
    assert_eq!(definitions[0].function_span.start, 10);
    assert_eq!(definitions[1].function_span.start, 30);
    assert_eq!(definitions[2].function_source, "pub fn f(&self) {}");
}

#[test]
fn bom_and_crlf_offsets_are_original_file_bytes() {
    let source = "\u{feff}#[inline]\r\npub fn f() {\r\n    fn g() {}\r\n}\r\n";
    let definitions = functions(source).unwrap();
    assert_eq!(definitions.len(), 2);
    assert_eq!(definitions[0].function_span.start, 14);
    assert_eq!(
        definitions[0].function_source,
        "pub fn f() {\r\n    fn g() {}\r\n}"
    );
    assert_eq!(definitions[1].function_span.start, 32);
    assert_eq!(definitions[1].function_source, "fn g() {}");
}

fn inactivity(source: &str, cfgs: &[&str]) -> Vec<bool> {
    let cfgs: Vec<_> = cfgs.iter().map(|cfg| (*cfg).to_owned()).collect();
    rstyle_pdg::source_evidence::configured_functions(source, Some(&cfgs))
        .unwrap()
        .iter()
        .map(|definition| definition.inactive_by_cfg)
        .collect()
}

#[test]
fn cfg_definition_identity_and_enclosing_scopes_are_independent() {
    let source = r#"
        #[cfg(feature = "off")] fn f() { fn nested() {} }
        #[cfg(feature = "on")] fn f() {}
        #[cfg(off)] mod m { fn inside() {} }
        struct S;
        #[cfg(off)] impl S { fn method() {} }
        #[cfg(off)] trait T { fn defaulted() {} }
        fn active() {}
    "#;
    assert_eq!(
        inactivity(source, &["feature=\"on\""]),
        [true, true, false, true, true, true, false]
    );
}

#[test]
fn nested_cfg_scopes_restore_siblings_and_preserve_unknown_predicates() {
    let source = r#"
        fn f() {
            #[cfg(off)] { fn hidden() {} }
            #[cfg_attr(on, cfg(off))] fn conditional() {}
            #[cfg(unsupported(1))] fn unknown() {}
            #[cfg(off)] #[cfg(unsupported(1))] fn false_despite_unknown() {}
            fn sibling() {}
        }
        impl S { #[cfg(off)] fn method() { fn nested() {} } fn next() {} }
        trait T { #[cfg(off)] fn defaulted() {} fn next() {} }
    "#;
    assert_eq!(
        inactivity(source, &["on"]),
        [
            false, true, true, false, true, false, true, true, false, true, false
        ]
    );
}

#[test]
fn file_cfg_and_disabled_inventory_keep_exact_source_ownership() {
    let source = "#![cfg(off)]\nfn f() {}";
    assert_eq!(inactivity(source, &[]), [true]);
    let definitions = functions(source).unwrap();
    assert!(!definitions[0].inactive_by_cfg);
    assert_eq!(definitions[0].function_source, "fn f() {}");
    assert_eq!(definitions[0].function_span.start, 13);
}

#[test]
fn unicode_before_nested_crlf_definitions_keeps_exact_byte_ownership() {
    let source = "// π🦀\r\npub fn café() {\r\n    fn 値() {}\r\n}\r\n";
    let definitions = functions(source).unwrap();
    assert_eq!(definitions.len(), 2);
    assert_eq!(definitions[0].function_span.start, 11);
    assert_eq!(
        definitions[0].function_source,
        "pub fn café() {\r\n    fn 値() {}\r\n}"
    );
    assert_eq!(definitions[1].function_span.start, 33);
    assert_eq!(definitions[1].function_source, "fn 値() {}");
}
