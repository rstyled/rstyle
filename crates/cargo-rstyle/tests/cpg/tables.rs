//! A unit's tables name each definition and type once, with one record, whichever function
//! of the crate reaches it first: a store packing a crate's documents together merges them.

use super::*;

/// The documents `cpg emit` writes for a crate named `tables` whose library is `source`.
fn emitted(source: &str) -> Vec<Value> {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, text: &str| {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "Cargo.toml",
        "[package]\nname = \"tables\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
    );
    write("src/lib.rs", source);
    let out = dir.path().join("cpg");
    let emit = rstyle(
        dir.path(),
        &["rstyle", "cpg", "emit", "--out", out.to_str().unwrap()],
    );
    assert_eq!(emit.status.code(), Some(0), "{emit:?}");
    parsed(&documents_in(&out))
}

/// Every record of one table in the documents for `source`.
fn table(source: &str, name: &str) -> Vec<Value> {
    emitted(source)
        .iter()
        .flat_map(|document| document["unit"][name].as_array().unwrap().clone())
        .collect()
}

#[test]
fn a_module_and_a_function_of_one_path_are_two_definitions() {
    // `use analyze::{self}` names the module alone, as an import of the path from another
    // module of rstyle named both before the module got its own ID.
    let source = "pub mod analyze {\n    pub fn inner() -> u8 {\n        1\n    }\n}\n\npub fn analyze() -> u8 {\n    analyze::inner()\n}\n\npub mod user {\n    use super::analyze::{self as module};\n\n    pub fn call() -> u8 {\n        module::inner() + super::analyze()\n    }\n}\n";
    let definitions = table(source, "definitions");
    let kinds = |id: &str| -> Vec<String> {
        definitions
            .iter()
            .filter(|record| record["id"] == id)
            .map(|record| record["kind"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(
        kinds("def:tables::analyze"),
        ["function"],
        "{definitions:#?}"
    );
    assert_eq!(
        kinds("def:tables::analyze#type"),
        ["module"],
        "{definitions:#?}"
    );
}

#[test]
fn closures_of_one_macro_call_have_their_own_types() {
    // Each call of `one!` writes its closure where the macro does, so both are spelled alike.
    let source = "macro_rules! one {\n    () => {\n        || 1u8\n    };\n}\n\npub fn first() -> u8 {\n    one!()()\n}\n\npub fn second() -> u8 {\n    one!()()\n}\n";
    let closures: Vec<Value> = table(source, "types")
        .into_iter()
        .filter(|record| record["kind"] == "closure")
        .collect();
    let ids: std::collections::BTreeSet<&str> = closures
        .iter()
        .map(|record| record["id"].as_str().unwrap())
        .collect();
    let definitions: std::collections::BTreeSet<&str> = closures
        .iter()
        .map(|record| record["definition"].as_str().unwrap())
        .collect();
    assert_eq!(definitions.len(), 2, "{closures:#?}");
    assert_eq!(ids.len(), 2, "{closures:#?}");
}

#[test]
fn types_declared_in_blocks_of_one_function_have_their_own_ids() {
    let source = "pub fn both() -> u16 {\n    let a = {\n        struct Field(u8);\n        Field(1).0 as u16\n    };\n    let b = {\n        struct Field(u16);\n        Field(2).0\n    };\n    a + b\n}\n";
    let fields: Vec<Value> = table(source, "types")
        .into_iter()
        .filter(|record| {
            record["spelling"]
                .as_str()
                .is_some_and(|s| s.ends_with("Field"))
        })
        .collect();
    let ids: std::collections::BTreeSet<&str> = fields
        .iter()
        .map(|record| record["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 2, "{fields:#?}");
}

#[test]
fn a_type_first_met_under_a_late_bound_lifetime_is_judged_as_any_other() {
    // The signature binds the elided lifetime its result's tuple holds, which the spelling
    // `Option<&str>` doesn't show; `Copy` and drop don't depend on lifetimes, so the record
    // says what it would anywhere else.
    let source =
        "pub fn split(text: &str) -> (usize, &str, Option<&str>) {\n    (0, text, None)\n}\n";
    let option: Vec<Value> = table(source, "types")
        .into_iter()
        .filter(|record| record["id"] == "type:std::option::Option<&str>")
        .collect();
    assert!(!option.is_empty(), "no Option<&str> record");
    for record in &option {
        let modifiers = record["modifiers"].as_array().cloned().unwrap_or_default();
        assert!(modifiers.contains(&json!("rust:copy")), "{record:#}");
    }
}
