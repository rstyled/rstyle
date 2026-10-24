use super::*;
use crate::{
    nominal_binding::{PACKAGE_IDENTITY, PackageContext},
    nominal_boundary::UnitContext,
};
use std::collections::BTreeSet;

struct Fixture {
    census: Census,
    modules: Vec<census::Module>,
    receipt: AnonymousPrelude,
}

fn definition(path: &str) -> Definition {
    Definition {
        path: path.into(),
        definition_identity: format!("definition:{path}"),
        crate_name: "consumer".into(),
        stable_crate_id: "consumer-id".into(),
        local: true,
    }
}

fn location(bytes: &crate::SourceSnapshot, text: &str) -> FileSpan {
    let start = bytes.find(text).unwrap() as u32;
    FileSpan {
        path: "/consumer/src/lib.rs".into(),
        digest: bytes.checksum().into(),
        span: ByteSpan {
            start,
            end: start + text.len() as u32,
        },
    }
}

fn census(bytes: crate::SourceSnapshot) -> Census {
    let manifest: crate::SourceSnapshot = "manifest".into();
    Census {
        identity: census::IDENTITY.into(),
        context: UnitContext {
            crate_name: "consumer".into(),
            root_source: "/consumer/src/lib.rs".into(),
            package: PackageContext {
                identity: PACKAGE_IDENTITY.into(),
                package_id: "consumer-package".into(),
                name: "consumer".into(),
                version: "1".into(),
                manifest_path: "/consumer/Cargo.toml".into(),
                manifest_digest: manifest.checksum().into(),
                manifest_bytes: manifest,
            },
        },
        files: vec![census::UnitFile {
            path: "/consumer/src/lib.rs".into(),
            digest: bytes.checksum().into(),
            bytes,
        }],
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    }
}

fn fixture(source: &str, declaration: &str, leaf: &str) -> Fixture {
    let bytes: crate::SourceSnapshot = source.into();
    let site = Site {
        definition: definition("{use#0}"),
        owner: definition(""),
        scope: vec![],
        path: vec!["renamed".into(), "prelude".into()],
        declaration: location(&bytes, declaration),
        leaf: location(&bytes, leaf),
        binding: Binding::Glob,
        public: declaration.starts_with("pub "),
        target: Definition {
            local: false,
            ..definition("producer::prelude")
        },
        target_kind: "Mod".into(),
    };
    let module = census::Module {
        path: "/consumer/src/lib.rs".into(),
        namespace: vec![],
        items: syn::parse_file(source).unwrap().items,
        verified_macro_imports: BTreeSet::new(),
        verified_external_imports: BTreeSet::new(),
    };
    let census = census(bytes);
    let receipt = AnonymousPrelude {
        census: Arc::new(census.clone()),
        module: site.target.clone(),
        traits: [definition("Extend"), definition("Truncate")],
        import: site,
    };

    Fixture {
        census,
        modules: vec![module],
        receipt,
    }
}

fn check(fixture: &Fixture) -> Result<(), ProofError> {
    verify_consumer(&fixture.census, &fixture.modules, &fixture.receipt)
}

#[test]
fn direct_and_grouped_globs_retain_exact_original_paths_and_prefix_leaf_spans() {
    for (source, leaf) in [
        ("use renamed::prelude::*;", "renamed::prelude"),
        ("pub use renamed::{prelude::*, Other};", "prelude"),
    ] {
        let fixture = fixture(source, source, leaf);
        assert!(check(&fixture).is_ok());
    }
}

#[test]
fn direct_named_module_import_requires_the_original_unrenamed_path() {
    let mut fixture = fixture(
        "pub use renamed::prelude;",
        "pub use renamed::prelude;",
        "renamed::prelude",
    );
    fixture.receipt.import.binding = Binding::Named("prelude".into());
    assert!(verify_site(&fixture.census, &fixture.modules, fixture.receipt.import()).is_ok());
    fixture.receipt.import.binding = Binding::Named("different".into());
    assert!(verify_site(&fixture.census, &fixture.modules, fixture.receipt.import()).is_err());
}

#[test]
fn named_module_receipts_reject_grouped_renamed_and_conditional_source() {
    for (source, leaf) in [
        (
            "pub use renamed::prelude as other;",
            "renamed::prelude as other",
        ),
        ("pub use renamed::{prelude};", "prelude"),
        ("#[cfg(test)] pub use renamed::prelude;", "renamed::prelude"),
    ] {
        let mut fixture = fixture(source, source, leaf);
        fixture.receipt.import.binding = Binding::Named("prelude".into());
        assert!(verify_site(&fixture.census, &fixture.modules, fixture.receipt.import()).is_err());
    }
}

#[test]
fn unique_nested_module_scope_is_required_without_reparsing_or_omitting_neighbors() {
    let source = "mod nested { use renamed::prelude::*; fn neighbor() {} }";
    let mut fixture = fixture(source, "use renamed::prelude::*;", "renamed::prelude");
    let syn::Item::Mod(module) = &fixture.modules[0].items[0] else {
        unreachable!()
    };
    fixture.modules[0].items = module.content.as_ref().unwrap().1.clone();
    fixture.modules[0].namespace = vec!["nested".into()];
    fixture.receipt.import.scope = vec!["nested".into()];
    fixture.receipt.import.owner = definition("nested");
    fixture.receipt.import.definition = definition("nested::{use#0}");

    assert!(check(&fixture).is_ok());
    assert_eq!(fixture.modules[0].items.len(), 2);
}

#[test]
fn source_path_spans_binding_visibility_and_local_owners_cannot_be_replaced() {
    let mutations: &[fn(&mut Fixture)] = &[
        |f| f.receipt.import.path[0] = "different".into(),
        |f| f.receipt.import.binding = Binding::Anonymous,
        |f| f.receipt.import.public = true,
        |f| f.receipt.import.leaf.span.end += 3,
        |f| f.receipt.import.leaf.span.start += 1,
        |f| f.receipt.import.declaration.span.end -= 1,
        |f| f.receipt.import.leaf.digest.push('x'),
        |f| f.receipt.import.leaf.path = "/consumer/src/other.rs".into(),
        |f| f.receipt.import.owner.path = "consumer".into(),
        |f| f.receipt.import.owner.local = false,
        |f| f.receipt.import.definition.path = "other::{use#0}".into(),
        |f| f.receipt.import.definition.path = "{use#x}".into(),
        |f| f.receipt.import.definition.stable_crate_id.push('x'),
        |f| f.modules[0].path = "/consumer/src/other.rs".into(),
        |f| f.modules[0].namespace.push("inactive".into()),
    ];
    for mutation in mutations {
        let mut fixture = fixture(
            "use renamed::prelude::*;",
            "use renamed::prelude::*;",
            "renamed::prelude",
        );
        mutation(&mut fixture);
        assert!(check(&fixture).is_err());
    }
}

#[test]
fn missing_duplicate_or_inactive_ownership_never_becomes_a_supported_prefix() {
    let mut fixture = fixture(
        "use renamed::prelude::*;",
        "use renamed::prelude::*;",
        "renamed::prelude",
    );
    fixture.modules[0].items.clear();
    assert!(matches!(check(&fixture), Err(ProofError::Missing(_))));
    fixture.modules.push(census::Module {
        path: "/consumer/src/lib.rs".into(),
        namespace: vec![],
        items: vec![],
        verified_macro_imports: BTreeSet::new(),
        verified_external_imports: BTreeSet::new(),
    });
    assert!(check(&fixture).is_err());
}

#[test]
fn attributed_uses_cannot_claim_active_source_from_raw_module_records() {
    for attribute in [
        "#[cfg(any())]",
        "#[cfg_attr(all(), allow(unused_imports))]",
        "#[custom::transform]",
        "#[allow(unused_imports)]",
    ] {
        let source = format!("{attribute}\nuse renamed::prelude::*;");
        let fixture = fixture(&source, &source, "renamed::prelude");
        assert!(
            matches!(check(&fixture), Err(ProofError::Unsupported(_))),
            "{attribute}"
        );
    }
}
