use super::*;
use crate::nominal_boundary::census::{IDENTITY, UnitFile};

const DEFINITION: &str = "macro_rules! index_type_impls { ($name:ident, $err:ident, $iter:ident, $withiter:ident) => { impl $name { pub fn new(value: usize) -> Result<$name, $err> { SmallIndex::new(value).map($name).map_err($err) } } pub struct $err(SmallIndexError); pub struct $iter; pub struct $withiter; }; }";
const PATTERN: &str =
    "index_type_impls!(PatternID, PatternIDError, PatternIDIter, WithPatternIDIter);";
const STATE: &str = "index_type_impls!(StateID, StateIDError, StateIDIter, WithStateIDIter);";

fn census(source: &str) -> Census {
    Census {
        identity: IDENTITY.into(),
        context: crate::nominal_boundary::tests::context(),
        files: vec![UnitFile {
            path: "/sample/src/lib.rs".into(),
            bytes: source.into(),
            digest: crate::source_identity::checksum(source.as_bytes()),
        }],
        expansions: Vec::new(),
        documentation: Vec::new(),
        external_imports: Vec::new(),
    }
}

fn anchor(census: &Census, text: &str) -> FileSpan {
    let file = &census.files[0];
    let start = file.bytes.find(text).unwrap();
    FileSpan {
        path: file.path.clone(),
        digest: file.digest.clone(),
        span: crate::ByteSpan {
            start: start.try_into().unwrap(),
            end: (start + text.len()).try_into().unwrap(),
        },
    }
}

fn check(
    census: &Census,
    definition: &str,
    invocation: &str,
    namespace: &[String],
) -> Result<CheckedExpansion, String> {
    verify(Input {
        census,
        cfgs: &[],
        namespace,
        definition: &anchor(census, definition),
        invocation: &anchor(census, invocation),
    })
}

fn owner(expansion: &CheckedExpansion) -> String {
    let syn::Item::Impl(item) = &expansion.items()[0] else {
        panic!("Expected emitted impl")
    };
    let syn::Type::Path(path) = item.self_ty.as_ref() else {
        panic!("Expected nominal owner")
    };
    path.path.get_ident().unwrap().to_string()
}

#[test]
fn exact_original_ident_substitution_retains_ordered_bindings_and_anchors() {
    let census = census(&format!("\u{feff}// π\r\n{DEFINITION}\r\n{PATTERN}"));
    let expansion = check(&census, DEFINITION, PATTERN, &[]).unwrap();
    assert_eq!(expansion.definition(), &anchor(&census, DEFINITION));
    assert_eq!(expansion.invocation(), &anchor(&census, PATTERN));
    assert!(expansion.namespace().is_empty());
    assert_eq!(expansion.items().len(), 4);
    assert_eq!(owner(&expansion), "PatternID");
    assert_eq!(
        expansion.bindings(),
        [
            IdentBinding {
                parameter: "name".into(),
                value: "PatternID".into()
            },
            IdentBinding {
                parameter: "err".into(),
                value: "PatternIDError".into()
            },
            IdentBinding {
                parameter: "iter".into(),
                value: "PatternIDIter".into()
            },
            IdentBinding {
                parameter: "withiter".into(),
                value: "WithPatternIDIter".into()
            },
        ]
    );
}

#[test]
fn state_arguments_are_independent_and_stale_pattern_anchor_cannot_transfer() {
    let pattern = census(&format!("{DEFINITION}\n{PATTERN}"));
    let state = census(&format!("{DEFINITION}\n{STATE}"));
    let expansion = check(&state, DEFINITION, STATE, &[]).unwrap();
    assert_eq!(owner(&expansion), "StateID");
    assert_eq!(expansion.bindings()[1].value, "StateIDError");
    assert!(
        verify(Input {
            census: &state,
            cfgs: &[],
            namespace: &[],
            definition: &anchor(&state, DEFINITION),
            invocation: &anchor(&pattern, PATTERN),
        })
        .is_err()
    );
}

#[test]
fn exact_temporal_binding_rejects_later_and_shadowed_definitions() {
    let later = DEFINITION.replace("pub struct $iter;", "pub struct $iter(u8);");
    let proof = census(&format!("{DEFINITION}\n{later}\n{PATTERN}"));
    assert!(check(&proof, DEFINITION, PATTERN, &[]).is_err());
    assert!(check(&proof, &later, PATTERN, &[]).is_ok());
    let proof = census(&format!("{PATTERN}\n{DEFINITION}"));
    assert!(check(&proof, DEFINITION, PATTERN, &[]).is_err());
}

#[test]
fn child_inherits_at_declaration_and_macro_use_exports_local_definition() {
    let later = DEFINITION.replace("pub struct $iter;", "pub struct $iter(u8);");
    let proof = census(&format!("{DEFINITION}\nmod child {{ {PATTERN} }}\n{later}"));
    let namespace = ["child".into()];
    assert!(check(&proof, DEFINITION, PATTERN, &namespace).is_ok());
    assert!(check(&proof, &later, PATTERN, &namespace).is_err());
    assert!(check(&proof, DEFINITION, PATTERN, &[]).is_err());
    let proof = census(&format!(
        "#[macro_use] mod support {{ {DEFINITION} }}\n{PATTERN}"
    ));
    assert!(check(&proof, DEFINITION, PATTERN, &[]).is_ok());
    let proof = census(&format!("mod support {{ {DEFINITION} }}\n{PATTERN}"));
    assert!(check(&proof, DEFINITION, PATTERN, &[]).is_err());
}

#[test]
fn prior_unknown_expansion_import_or_attribute_prevents_textual_claim() {
    for prior in [
        "unknown!();",
        "use other::*;",
        "use other::Copy;",
        "#[macro_use] extern crate other;",
        "#[unknown] struct Hidden;",
    ] {
        let proof = census(&format!("{DEFINITION}\n{prior}\n{PATTERN}"));
        assert!(check(&proof, DEFINITION, PATTERN, &[]).is_err(), "{prior}");
    }
    let proof = census(&format!(
        "{DEFINITION}\n#[cfg(missing)] unknown!();\n{PATTERN}"
    ));
    assert!(check(&proof, DEFINITION, PATTERN, &[]).is_ok());
}

#[test]
fn malformed_matchers_rules_arguments_and_nonitems_fail_closed() {
    for (definition, invocation) in [
        (
            "macro_rules! m { ($x:ident) => { struct $x; }; ($x:ident) => { enum $x {} }; }",
            "m!(Name);",
        ),
        (
            "macro_rules! m { ($x:ident, $x:ident) => { struct $x; }; }",
            "m!(Name, Other);",
        ),
        (
            "macro_rules! m { ($x:ident,) => { struct $x; }; }",
            "m!(Name);",
        ),
        (
            "macro_rules! m { ($x:ident) => { struct $x; }; }",
            "m!(Name,);",
        ),
        (
            "macro_rules! m { ($x:expr) => { struct Name; }; }",
            "m!(Name);",
        ),
        (
            "macro_rules! m { ($x:ident) => { struct $x; }; }",
            "m!(Name, Other);",
        ),
        (
            "macro_rules! m { ($x:ident) => { struct $x; }; }",
            "m!(other::Name);",
        ),
        (
            "macro_rules! m { ($x:ident) => { $missing }; }",
            "m!(Name);",
        ),
        ("macro_rules! m { ($x:ident) => { 1 + 2 }; }", "m!(Name);"),
        ("macro_rules! m { ($x:ident) => {}; }", "m!(Name);"),
    ] {
        let proof = census(&format!("{definition}\n{invocation}"));
        assert!(
            check(&proof, definition, invocation, &[]).is_err(),
            "{definition}"
        );
    }
}

#[test]
fn stale_definition_and_invocation_coordinates_are_rejected() {
    let proof = census(&format!("{DEFINITION}\n{PATTERN}"));
    for definition_changed in [false, true] {
        for mutation in 0..3 {
            let mut definition = anchor(&proof, DEFINITION);
            let mut invocation = anchor(&proof, PATTERN);
            let target = if definition_changed {
                &mut definition
            } else {
                &mut invocation
            };
            match mutation {
                0 => target.digest.push('x'),
                1 => target.path = "/sample/src/other.rs".into(),
                _ => target.span.end -= 1,
            }
            assert!(
                verify(Input {
                    census: &proof,
                    cfgs: &[],
                    namespace: &[],
                    definition: &definition,
                    invocation: &invocation
                })
                .is_err()
            );
        }
    }
    let mut stale = proof;
    stale.files[0].bytes = stale.files[0]
        .bytes
        .replace("SmallIndex::new", "OtherIndex::new")
        .into();
    assert!(
        check(
            &stale,
            DEFINITION
                .replace("SmallIndex::new", "OtherIndex::new")
                .as_str(),
            PATTERN,
            &[]
        )
        .is_err()
    );
}

#[test]
fn refreshed_original_body_changes_the_parsed_expansion() {
    let changed = DEFINITION.replace("pub struct $iter;", "pub struct $iter(u8);");
    let proof = census(&format!("{changed}\n{PATTERN}"));
    let expansion = check(&proof, &changed, PATTERN, &[]).unwrap();
    let syn::Item::Struct(item) = &expansion.items()[2] else {
        panic!("Expected emitted iterator struct")
    };
    assert!(matches!(&item.fields, syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1));
}

#[test]
fn original_crate_rooted_glob_with_macro_free_surface_preserves_lookup() {
    let proof = census(&format!(
        "pub use crate::util::search::*; #[macro_use] mod macros {{ macro_rules! helper {{ ($x:ident) => {{ struct $x; }}; }} }} pub mod util {{ pub mod search {{ use core::ops::Range; #[derive(Clone)] pub struct Input; }} pub mod primitives {{ {DEFINITION} {PATTERN} }} }}"
    ));
    let namespace = ["util".into(), "primitives".into()];
    let expansion = check(&proof, DEFINITION, PATTERN, &namespace).unwrap();
    assert_eq!(owner(&expansion), "PatternID");
}

#[test]
fn crate_rooted_glob_rejects_uncertain_exports_and_import_alias_paths() {
    let namespace = ["primitives".into()];
    for exports in [
        "pub use other::index_type_impls;",
        "pub use other::*;",
        "macro_rules! hidden { ($x:ident) => { struct $x; }; }",
        "unknown!();",
        "#[unknown] pub struct Input;",
        "#[macro_use] mod support { macro_rules! hidden { ($x:ident) => { struct $x; }; } }",
    ] {
        let proof = census(&format!(
            "pub use crate::search::*; mod search {{ {exports} }} mod primitives {{ {DEFINITION} {PATTERN} }}"
        ));
        assert!(
            check(&proof, DEFINITION, PATTERN, &namespace).is_err(),
            "{exports}"
        );
    }
    let proof = census(&format!(
        "use crate::other as search; pub use crate::search::*; mod other {{}} mod search {{ pub struct Input; }} mod primitives {{ {DEFINITION} {PATTERN} }}"
    ));
    assert!(check(&proof, DEFINITION, PATTERN, &namespace).is_err());
}
