use super::*;
use crate::{
    ByteSpan,
    concrete_types::digest,
    nominal_boundary::census::{UnitFile, verify_documented},
};

fn fixture() -> Census {
    let source = "#![doc = include_str!(\"../README.md\")]\npub fn f() {}\n";
    let bytes = "# Documentation\nπ\n";
    let syntax = syn::parse_file(source).unwrap();
    let syn::Meta::NameValue(value) = &syntax.attrs[0].meta else {
        unreachable!()
    };
    let registry = crate::data_library::registry();
    Census {
        identity: super::super::IDENTITY.into(),
        context: crate::nominal_boundary::tests::context(),
        files: vec![
            UnitFile {
                path: "/sample/src/lib.rs".into(),
                bytes: source.into(),
                digest: digest(source.as_bytes()),
            },
            UnitFile {
                path: "/sample/src/../README.md".into(),
                bytes: bytes.into(),
                digest: digest(bytes.as_bytes()),
            },
        ],
        expansions: Vec::new(),
        external_imports: Vec::new(),
        documentation: vec![Input {
            invocation: FileSpan {
                path: "/sample/src/lib.rs".into(),
                digest: digest(source.as_bytes()),
                span: crate::source_evidence::original_span(source, value.value.span()),
            },
            input: FileSpan {
                path: "/sample/src/../README.md".into(),
                digest: digest(bytes.as_bytes()),
                span: ByteSpan {
                    start: 0,
                    end: bytes.len() as u32,
                },
            },
            physical_path: "/sample/README.md".into(),
            builtin: Callee {
                path: "std::include_str".into(),
                crate_name: "core".into(),
                crate_hash: registry.core_hash.clone(),
                definition_hash: registry.definitions[DEFINITION].clone(),
                implementation: None,
            },
            expanded_digest: digest(bytes.as_bytes()),
        }],
    }
}

#[test]
fn documentation_accounts_for_full_input_without_certifying_it_as_rust() {
    let census = fixture();
    assert!(super::super::verify(&census).is_err());
    let modules = verify_documented(&census, &[], false).unwrap();
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].path, census.context.root_source);
    assert_eq!(modules[0].items.len(), 1);
}

#[test]
fn changed_documentation_observations_and_unaccounted_inputs_reject() {
    for mutation in [
        "owner",
        "span",
        "digest",
        "expanded",
        "builtin",
        "redirect",
        "duplicate",
        "orphan",
        "bytes",
        "partial",
        "module_alias",
    ] {
        let mut census = fixture();
        match mutation {
            "owner" => census.documentation[0].invocation.path = "/sample/src/other.rs".into(),
            "span" => census.documentation[0].invocation.span.start += 1,
            "digest" => census.documentation[0].input.digest.push('x'),
            "expanded" => census.documentation[0].expanded_digest.push('x'),
            "builtin" => census.documentation[0].builtin.definition_hash.push('x'),
            "redirect" => census.documentation[0].physical_path = "/outside/README.md".into(),
            "duplicate" => census.documentation.push(census.documentation[0].clone()),
            "orphan" => census.files.push(UnitFile {
                path: "/sample/src/orphan.rs".into(),
                bytes: "pub struct Hidden;".into(),
                digest: digest(b"pub struct Hidden;"),
            }),
            "bytes" => {
                census.files[1].bytes = "changed".into();
                census.files[1].digest = digest(b"changed");
                census.documentation[0].input.digest = census.files[1].digest.clone();
            }
            "partial" => census.documentation[0].input.span.end -= 1,
            "module_alias" => {
                let mut file = census.files[1].clone();
                file.path = "/sample/README.md".into();
                census.files.push(file);
            }
            _ => unreachable!(),
        }
        assert!(
            verify_documented(&census, &[], false).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn documentation_paths_cannot_escape_or_substitute_different_inputs() {
    let root = Path::new("/sample");
    assert_eq!(
        in_package(root, Path::new("/sample/src/../README.md")).unwrap(),
        root.join("README.md")
    );
    for path in [
        "/sample/../outside.md",
        "/outside/doc.md",
        "/sample/src/../../sample/doc.md",
    ] {
        assert!(in_package(root, Path::new(path)).is_err(), "{path}");
    }
    let mut census = fixture();
    census.documentation[0].input.path = "/sample/README.md".into();
    assert!(verify_documented(&census, &[], false).is_err());
}
