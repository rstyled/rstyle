use super::*;
use crate::{
    ByteSpan,
    concrete_types::digest,
    nominal_boundary::census::{UnitFile, verify_imported},
};

fn fixture() -> Census {
    let source = "#[macro_use] extern crate alloc;\npub fn f(n: usize) -> usize { n }\n";
    Census {
        identity: super::super::IDENTITY.into(),
        context: crate::nominal_boundary::tests::context(),
        files: vec![UnitFile {
            path: "/sample/src/lib.rs".into(),
            bytes: source.into(),
            digest: digest(source.as_bytes()),
        }],
        expansions: Vec::new(),
        documentation: Vec::new(),
        external_imports: vec![Import {
            declaration: FileSpan {
                path: "/sample/src/lib.rs".into(),
                digest: digest(source.as_bytes()),
                span: ByteSpan { start: 13, end: 32 },
            },
            exports: crate::data_library::registry()
                .alloc_macro_exports
                .clone()
                .unwrap(),
        }],
    }
}

#[test]
fn exact_imports_account_for_both_names_without_authorizing_macro_invocation() {
    let census = fixture();
    assert!(super::super::verify(&census).is_err());
    assert!(super::super::verify_selected(&census, &[]).is_err());
    let modules = verify_imported(&census, &[], false).unwrap();
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].items.len(), 2);
    assert!(modules[0].verified_external_imports.contains("alloc"));
    assert_eq!(
        census.external_imports[0]
            .exports
            .macros
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        ["format", "vec"]
    );
}

#[test]
fn modified_import_source_identity_or_incomplete_namespace_rejects() {
    for mutation in [
        "owner",
        "span",
        "digest",
        "library",
        "macro",
        "kind",
        "visibility",
        "missing",
        "extra",
        "duplicate",
        "missing_import",
        "duplicate_import",
    ] {
        let mut census = fixture();
        let import = &mut census.external_imports[0];
        match mutation {
            "owner" => import.declaration.path = "/sample/src/other.rs".into(),
            "span" => import.declaration.span.start += 1,
            "digest" => import.declaration.digest.push('x'),
            "library" => import.exports.library.crate_hash.push('x'),
            "macro" => import.exports.macros[0]
                .definition
                .definition_hash
                .push('x'),
            "kind" => import.exports.macros[0].kind = "derive".into(),
            "visibility" => import.exports.macros[0].public = false,
            "missing" => {
                import.exports.macros.pop();
            }
            "extra" => {
                let mut item = import.exports.macros[0].clone();
                item.name = "cfg_if".into();
                import.exports.macros.push(item);
            }
            "duplicate" => import.exports.macros.push(import.exports.macros[0].clone()),
            "missing_import" => census.external_imports.clear(),
            "duplicate_import" => census
                .external_imports
                .push(census.external_imports[0].clone()),
            _ => unreachable!(),
        }
        let result = verify_imported(&census, &[], false);
        assert!(result.is_err(), "{mutation}");
    }
}

#[test]
fn pinned_export_contract_binds_reviewed_source_and_distinct_target_identities() {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let source = std::fs::read(
        std::path::Path::new(sysroot.trim())
            .join("lib/rustlib/src/rust/library/alloc/src/macros.rs"),
    )
    .unwrap();
    let selection: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../eval/coverage-development/base64ct-std-import-selection.json"
    ))
    .unwrap();
    assert_eq!(
        digest(&source),
        format!(
            "sha256:{}",
            selection["pinned_source_audit"]["sha256"].as_str().unwrap()
        )
    );
    let first = crate::data_library::registry()
        .alloc_macro_exports
        .as_ref()
        .unwrap();
    for registry in crate::data_library::registries() {
        let current = registry.alloc_macro_exports.as_ref().unwrap();
        assert_eq!(
            current
                .macros
                .iter()
                .map(|m| (&m.name, &m.kind, m.public))
                .collect::<Vec<_>>(),
            first
                .macros
                .iter()
                .map(|m| (&m.name, &m.kind, m.public))
                .collect::<Vec<_>>()
        );
        if registry.target != crate::data_library::registry().target {
            assert_ne!(current.library, first.library);
        }
    }
}
