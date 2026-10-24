//! Real original source plus independently stated footprint and rejection cases.
use super::*;
use crate::{
    nominal_binding::{PACKAGE_IDENTITY, PackageContext},
    nominal_boundary::UnitContext,
};
use std::collections::BTreeSet;

const SOURCE: &str = include_str!("source.txt");
const ROOT: &str = "#![no_std]\npub mod convert;\n";

pub(in crate::dependency_namespaces::proof) fn projection() -> Projection {
    checked(SOURCE, ROOT).unwrap()
}

fn checked(source: &str, root: &str) -> Result<Projection, ProofError> {
    let manifest: SourceSnapshot = "manifest".into();
    let census = Census {
        identity: census::IDENTITY.into(),
        context: UnitContext {
            crate_name: "time_core".into(),
            root_source: "/producer/src/lib.rs".into(),
            package: PackageContext {
                identity: PACKAGE_IDENTITY.into(),
                package_id: "time-core@0.1.8".into(),
                name: "time-core".into(),
                version: "0.1.8".into(),
                manifest_path: "/producer/Cargo.toml".into(),
                manifest_digest: manifest.checksum().into(),
                manifest_bytes: manifest,
            },
        },
        files: vec![file("lib.rs", root), file("convert.rs", source)],
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    };
    let modules = vec![
        module("lib.rs", root, &[]),
        module("convert.rs", source, &["convert"]),
    ];
    verify(&census, &modules, &["convert".into()])
}

fn file(name: &str, source: &str) -> UnitFile {
    let bytes: SourceSnapshot = source.into();
    UnitFile {
        path: format!("/producer/src/{name}").into(),
        digest: bytes.checksum().into(),
        bytes,
    }
}

fn module(name: &str, source: &str, namespace: &[&str]) -> Module {
    Module {
        path: format!("/producer/src/{name}").into(),
        namespace: namespace.iter().map(|name| (*name).into()).collect(),
        items: syn::parse_file(source).unwrap().items,
        verified_macro_imports: BTreeSet::new(),
        verified_external_imports: BTreeSet::new(),
    }
}

#[test]
fn original_eight_units_and_all_thirty_six_ratio_groups_are_retained() {
    let projection = projection();
    let names: Vec<_> = projection.units().iter().map(Unit::name).collect();
    assert_eq!(
        names,
        [
            "Nanosecond",
            "Microsecond",
            "Millisecond",
            "Second",
            "Minute",
            "Hour",
            "Day",
            "Week"
        ]
    );
    let counts: Vec<_> = projection
        .units()
        .iter()
        .map(|unit| unit.ratios().len())
        .collect();
    assert_eq!(counts, [8, 7, 6, 5, 4, 3, 2, 1]);
    assert_eq!(projection.units()[0].ratios()[1].default_output(), "u16");
    assert_eq!(projection.units()[0].ratios()[1].larger(), "Microsecond");
    assert_eq!(projection.units()[0].ratios()[1].floats(), ["f32", "f64"]);
    assert_eq!(projection.units()[7].ratios()[0].integers().len(), 12);
    assert_eq!(
        slice(&projection, projection.template(TemplateRole::Struct)),
        "pub struct $t;"
    );
    assert_eq!(
        slice(&projection, projection.definition()),
        "macro_rules! impl_per"
    );
    assert!(slice(&projection, projection.invocation()).starts_with("impl_per! {"));
    for unit in projection.units() {
        assert_eq!(slice(&projection, unit.source()), unit.name());
    }
}

fn slice<'a>(projection: &'a Projection, site: &FileSpan) -> &'a str {
    &projection.source()[site.span.start as usize..site.span.end as usize]
}

#[test]
fn macro_name_is_not_authority_and_token_coordinates_preserve_original_whitespace() {
    let source = SOURCE
        .replace("impl_per", "unit_inventory")
        .replace("pub struct $t;", "pub  struct $t ;");
    let projection = checked(&source, ROOT).unwrap();
    assert_eq!(projection.macro_name(), "unit_inventory");
    assert_eq!(
        slice(&projection, projection.template(TemplateRole::Struct)),
        "pub  struct $t ;"
    );
}

#[test]
fn every_matcher_template_derive_and_neighbor_change_remains_checked() {
    for (before, after) in [
        ("$t:ident", "$t:ty"),
        ("Debug, Clone, Copy", "Debug, Clone"),
        ("Debug, Clone, Copy", "Debug, Clone, Unknown"),
        ("pub struct $t;", "pub struct $t; pub struct Hidden;"),
        (
            "pub struct $t;",
            "pub struct $t; impl Drop for $t { fn drop(&mut self) {} }",
        ),
        ("impl $t {", "impl $t { hidden!();"),
        (
            "const VALUE: $int_output = $int_value;",
            "const VALUE: $int_output = hidden!();",
        ),
        ("pub trait DefaultOutput<T>", "pub trait Drop<T>"),
        ("T::VALUE\n}", "hidden!()\n}"),
        ("concat!(\"A unit", "unknown!(\"A unit"),
        ("    )*};", "    )*}; () => {};"),
        (
            "use self::sealed::{DefaultOutput, MultipleOf};",
            "use foreign::{DefaultOutput, MultipleOf};",
        ),
    ] {
        assert!(SOURCE.contains(before), "missing mutation anchor: {before}");
        assert!(
            checked(&SOURCE.replacen(before, after, 1), ROOT).is_err(),
            "{before}"
        );
    }
    assert!(checked(&format!("{SOURCE}\npub struct Hidden;"), ROOT).is_err());
}

#[test]
fn invocation_consumption_primitive_ownership_and_ratio_rosters_fail_closed() {
    for (before, after) in [
        ("Nanosecond (\"nanosecond\")", "Drop (\"nanosecond\")"),
        (
            "Nanosecond (\"nanosecond\")",
            "Nanosecond (\"nanosecond\", hidden!())",
        ),
        ("[u16] u16|u32", "[u8] u16|u32"),
        ("u8|u16|u32", "u8|u8|u32"),
        ("usize|i8|i16", "Foreign|i8|i16"),
        (" = 1_000;", " = hidden!();"),
        (" = 1_000;", " = 1_000u16;"),
        (" = 1_000.;", " = 1e999;"),
        ("Microsecond: [u16]", "Foreign: [u16]"),
        ("Week (\"week\")", "Week (\"week\") extra"),
    ] {
        assert!(SOURCE.contains(before), "missing mutation anchor: {before}");
        assert!(
            checked(&SOURCE.replacen(before, after, 1), ROOT).is_err(),
            "{before}"
        );
    }
}

#[test]
fn inherited_root_macros_imports_attributes_and_ambiguous_modules_are_not_omitted() {
    for root in [
        "macro_rules! concat { () => {} } pub mod convert;",
        "use foreign::*; pub mod convert;",
        "#[macro_use] mod foreign; pub mod convert;",
        "#[cfg(feature = \"on\")] pub mod convert;",
        "pub mod convert; pub mod convert;",
        "mod convert;",
    ] {
        assert!(checked(SOURCE, root).is_err(), "{root}");
    }
}
