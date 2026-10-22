use super::*;

fn check(
    source: &str,
    name: &str,
    cfgs: &[String],
    change: impl FnOnce(&mut HirFnFacts),
) -> Result<()> {
    let mut original: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let syntax = syn::parse_file(source).unwrap();
    let function = syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) => Some(function),
            _ => None,
        })
        .unwrap();
    let start = function_start(source, function);
    let end = crate::source_evidence::original_span(source, function.block.span()).end;
    original.fn_span = crate::ByteSpan { start, end };
    original.source.bytes = source[start as usize..end as usize].to_owned().into();
    change(&mut original);
    let path = Path::new("/sample/src/lib.rs");
    let files = BTreeMap::from([(path, source)]);
    Owner {
        files: &files,
        configuration: crate::conditional_source::Configuration::new(cfgs),
        original: &original,
        source: path,
    }
    .file(&Site {
        path,
        directory: Path::new("/sample/src"),
        names: &[name],
    })
}

#[test]
fn original_function_is_bound_without_certifying_neighbor_effects() {
    let source = "trait Neighbor { fn effect(&self); } pub fn leaf<T: PartialOrd>(a:T)->T {a}";
    assert!(check(source, "leaf", &[], |_| {}).is_ok());
    assert!(check(source, "other", &[], |_| {}).is_err());
    assert!(check(source, "leaf", &[], |facts| facts.fn_span.start += 1).is_err());
}

#[test]
fn cfg_and_expanding_owner_attributes_fail_closed() {
    assert!(check("#[cfg(feature=\"on\")] fn leaf() {}", "leaf", &[], |_| {}).is_err());
    assert!(
        check(
            "#[cfg(feature=\"on\")] fn leaf() {}",
            "leaf",
            &["feature=\"on\"".into()],
            |_| {}
        )
        .is_ok()
    );
    assert!(check("#![custom] fn leaf() {}", "leaf", &[], |_| {}).is_err());
    assert!(
        check(
            "#![cfg_attr(feature=\"on\", custom)] fn leaf() {}",
            "leaf",
            &["feature=\"on\"".into()],
            |_| {}
        )
        .is_err()
    );
}

#[test]
fn duplicate_and_corrupted_inputs_are_rejected() {
    let context = crate::nominal_boundary::tests::context();
    let file = census::UnitFile {
        path: context.root_source.clone(),
        bytes: "fn leaf() {}".into(),
        digest: "wrong".into(),
    };
    let mut census = census::Census {
        identity: census::IDENTITY.into(),
        context,
        files: vec![file],
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    };
    assert!(files(&census).is_err());
    census.files[0].digest = census.files[0].bytes.identity().into();
    assert!(files(&census).is_ok());
    census.files.push(census.files[0].clone());
    assert!(files(&census).is_err());
}

#[test]
fn original_num_traits_root_metadata_is_inert_and_exact() {
    let source = "//! Numeric traits\n#![doc(html_root_url = \"https://docs.rs/num-traits/0.2\")]\n#![deny(unconditional_recursion)]\n#![no_std]\n#[inline]\n#[allow(clippy::eq_op)]\npub fn leaf<T: PartialOrd>(a:T)->T {a}";
    assert!(check(source, "leaf", &[], |_| {}).is_ok());
    for attribute in [
        "doc(html_root_url = include_str!(\"url\"))",
        "doc(alias = \"other\")",
        "doc(html_root_url = \"url\", hidden)",
    ] {
        let source = format!("#![{attribute}] fn leaf() {{}}");
        assert!(check(&source, "leaf", &[], |_| {}).is_err());
    }
}

#[test]
fn lexical_context_matches_compiler_crate_and_package_root() {
    let mut original: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    original.build.frontend_options.insert(
        "rstyle.rust.compiled-crate".into(),
        serde_json::json!({"identity":"rstyle.rust.compiled-crate/1", "crate_name":"sample"}),
    );
    let mut context = crate::nominal_boundary::tests::context();
    assert!(context_binding(&original, &context).is_ok());
    context.crate_name = "transplanted".into();
    assert!(context_binding(&original, &context).is_err());
    context.crate_name = "sample".into();
    context.root_source = "/elsewhere/lib.rs".into();
    assert!(context_binding(&original, &context).is_err());
    context.root_source = "/sample/src/lib.rs".into();
    context.package.manifest_path = "/sample/../sample/Cargo.toml".into();
    assert!(context_binding(&original, &context).is_err());
}
