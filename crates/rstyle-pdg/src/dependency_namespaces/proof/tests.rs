use super::*;
use crate::nominal_binding::{PACKAGE_IDENTITY, PackageContext};
use crate::nominal_boundary::{
    UnitContext,
    census::{UnitFile, expansions::FileSpan},
};
use capture::{Declaration, Export, Route, Unsupported};

const SOURCE: &str = "pub mod prelude { pub use crate::{Extend as _, Truncate as _}; }\npub trait Extend<T> {}\npub trait Truncate<T> {}\n";

struct Fixture {
    census: Arc<Census>,
    capture: UnitCapture,
    import: Import,
}

fn definition(path: &str, local: bool) -> Definition {
    Definition {
        path: if local {
            path.into()
        } else {
            format!("num_conv::{path}")
        },
        definition_identity: format!("definition:{path}"),
        crate_name: "num_conv".into(),
        stable_crate_id: "compiled-producer".into(),
        local,
    }
}

fn span(source: &str, text: &str) -> FileSpan {
    let start = source.find(text).unwrap() as u32;
    let bytes: crate::SourceSnapshot = source.into();
    FileSpan {
        path: "/producer/src/lib.rs".into(),
        digest: bytes.checksum().into(),
        span: crate::ByteSpan {
            start,
            end: start + text.len() as u32,
        },
    }
}

fn declaration(source: &str, name: &str) -> Declaration {
    let ordinal = match name {
        "Extend" => 0,
        "Truncate" => 1,
        _ => 2,
    };
    Declaration {
        site: Site {
            definition: definition(&format!("prelude::{{use#0}}::{{use#{ordinal}}}"), true),
            declaration: span(
                source,
                if source.contains("Widen as _") {
                    "pub use crate::{Extend as _, Truncate as _, Widen as _};"
                } else {
                    "pub use crate::{Extend as _, Truncate as _};"
                },
            ),
            leaf: span(source, name),
            owner: definition("prelude", true),
            scope: vec!["prelude".into()],
            path: vec!["crate".into(), name.into()],
            binding: Binding::Anonymous,
            public: true,
            target: definition(name, true),
            target_kind: "Trait".into(),
        },
    }
}

fn census(source: &str) -> Arc<Census> {
    let manifest: crate::SourceSnapshot = "manifest".into();
    let bytes: crate::SourceSnapshot = source.into();
    Arc::new(Census {
        identity: crate::nominal_boundary::census::IDENTITY.into(),
        context: UnitContext {
            crate_name: "num_conv".into(),
            root_source: "/producer/src/lib.rs".into(),
            package: PackageContext {
                identity: PACKAGE_IDENTITY.into(),
                package_id: "selected-producer".into(),
                name: "num-conv".into(),
                version: "0.2.0".into(),
                manifest_path: "/producer/Cargo.toml".into(),
                manifest_digest: manifest.checksum().into(),
                manifest_bytes: manifest,
            },
        },
        files: vec![UnitFile {
            path: "/producer/src/lib.rs".into(),
            digest: bytes.checksum().into(),
            bytes,
        }],
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    })
}

fn fixture(source: &str) -> Fixture {
    let names: &[&str] = if source.contains("Widen as _") {
        &["Extend", "Truncate", "Widen"]
    } else {
        &["Extend", "Truncate"]
    };
    let declarations: Vec<_> = names.iter().map(|name| declaration(source, name)).collect();
    let exports = declarations
        .iter()
        .map(|record| Export {
            binding: Binding::Anonymous,
            target: foreign(&record.site.target),
            target_kind: "Trait".into(),
            public: true,
            route: vec![Route {
                kind: RouteKind::Single,
                declaration: foreign(&record.site.definition),
            }],
        })
        .collect();
    let mut site = declarations[0].site.clone();
    site.binding = Binding::Glob;
    site.target = definition("prelude", false);
    site.target_kind = "Mod".into();

    Fixture {
        census: census(source),
        capture: UnitCapture {
            identity: capture::IDENTITY.into(),
            declarations,
            imports: vec![],
            unsupported: vec![],
        },
        import: Import {
            site,
            compiled_dependency: serde_json::Value::Null,
            exports,
            complete: false,
        },
    }
}

fn foreign(definition: &Definition) -> Definition {
    Definition {
        local: false,
        path: format!("{}::{}", definition.crate_name, definition.path),
        ..definition.clone()
    }
}

fn verify(fixture: &Fixture) -> Result<AnonymousPrelude, ProofError> {
    verify_anonymous_prelude(
        Arc::clone(&fixture.census),
        &fixture.capture,
        &fixture.import,
    )
}

#[test]
fn exact_original_two_trait_footprint_proves_anonymous_import_without_artifact_authority() {
    let fixture = fixture(SOURCE);
    let receipt = verify(&fixture).unwrap();
    assert_eq!(
        receipt
            .traits()
            .each_ref()
            .map(|trait_| trait_.path.as_str()),
        ["Extend", "Truncate"]
    );
    assert_eq!(receipt.import().binding, Binding::Glob);
    assert!(Arc::ptr_eq(receipt.census(), &fixture.census));
    assert_eq!(receipt.module(), &fixture.import.site.target);
    assert!(!fixture.import.complete);
}

#[test]
fn identities_routes_inventory_and_original_coordinates_are_independent_requirements() {
    let mutations: &[fn(&mut Fixture)] = &[
        |f| f.import.exports.reverse(),
        |f| {
            f.import.exports.pop();
        },
        |f| f.import.exports.push(f.import.exports[0].clone()),
        |f| f.import.exports[0].binding = Binding::Named("_".into()),
        |f| f.import.exports[0].target.definition_identity.push('x'),
        |f| f.import.exports[0].target.path = "Extend".into(),
        |f| f.import.exports[0].target.path = "other_crate::Extend".into(),
        |f| f.import.exports[0].target.path = "num_conv::num_conv::Extend".into(),
        |f| f.import.exports[0].route[0].declaration.path = "prelude::{use#0}::{use#0}".into(),
        |f| f.capture.declarations[0].site.owner.path = "num_conv::prelude".into(),
        |f| f.capture.declarations[0].site.target.path = "num_conv::Extend".into(),
        |f| f.capture.declarations[0].site.definition.path = "other::prelude::{use#0}".into(),
        |f| f.import.exports[0].route[0].kind = RouteKind::Glob,
        |f| f.import.exports[0].route[0].declaration.local = true,
        |f| f.import.exports[0].route.clear(),
        |f| f.import.exports[0].public = false,
        |f| f.import.site.target.stable_crate_id.push('x'),
        |f| f.capture.declarations[0].site.leaf.span.start += 1,
        |f| f.capture.declarations[0].site.leaf.span.end += " as _".len() as u32,
        |f| f.capture.declarations[0].site.leaf.digest.push('x'),
        |f| f.capture.declarations[0].site.path.reverse(),
        |f| f.capture.declarations[0].site.target_kind = "Macro".into(),
        |f| {
            f.capture
                .declarations
                .push(f.capture.declarations[0].clone())
        },
    ];
    for mutation in mutations {
        let mut fixture = fixture(SOURCE);
        mutation(&mut fixture);
        assert!(verify(&fixture).is_err());
    }
}

#[test]
fn complete_raw_flag_cannot_cover_missing_source_or_extra_module_contents() {
    for addition in [
        "pub struct Hidden;",
        "macro_rules! hidden { () => {} }",
        "hidden!();",
        "use crate::Extend;",
        "mod inner {}",
    ] {
        let source = SOURCE.replacen("; }", &format!("; {addition} }}"), 1);
        let mut fixture = fixture(&source);
        fixture.import.complete = true;
        assert!(verify(&fixture).is_err(), "{addition}");
    }
    let mut fixture = fixture(SOURCE);
    Arc::make_mut(&mut fixture.census).files.clear();
    assert!(matches!(verify(&fixture), Err(ProofError::Missing(_))));
}

#[test]
fn selected_unsupported_evidence_blocks_and_unrelated_findings_are_not_discharged() {
    let mut fixture = fixture(SOURCE);
    fixture.capture.unsupported.push(Unsupported {
        definition: definition("other", true),
        reason: "unrelated".into(),
    });
    assert!(verify(&fixture).is_ok());
    assert_eq!(fixture.capture.unsupported.len(), 1);
    fixture.capture.unsupported.push(Unsupported {
        definition: definition("Extend", true),
        reason: "unsupported target".into(),
    });
    assert!(matches!(verify(&fixture), Err(ProofError::Unsupported(_))));
}

#[test]
fn conditional_private_or_generated_namespace_declarations_remain_unsupported() {
    for source in [
        SOURCE.replace("pub mod prelude", "#[cfg(any())] pub mod prelude"),
        SOURCE.replace("pub use crate", "use crate"),
        SOURCE.replace("pub trait Extend", "#[cfg(any())] pub trait Extend"),
        SOURCE.replace("Extend as _", "Extend"),
        SOURCE.replace("Extend as _", "Extend as Named"),
        SOURCE.replace("Truncate as _", "Truncate"),
    ] {
        // Records retain the original frozen source; altered producer bytes must
        // never become a supported-prefix proof even when export names agree.
        let mut fixture = fixture(SOURCE);
        fixture.census = census(&source);
        assert!(verify(&fixture).is_err());
    }
}

#[test]
fn original_root_attributes_require_literal_documentation_or_no_std() {
    let original = format!("#![no_std]\n//! literal docs\n{SOURCE}");
    assert!(verify(&fixture(&original)).is_ok());
    for attribute in [
        "#![cfg(any())]",
        "#![cfg_attr(all(), no_std)]",
        "#![custom::transform]",
        "#![feature(custom_inner_attributes)]",
        "#![no_implicit_prelude]",
        "#![doc = include_str!(\"hidden.rs\")]",
        "#![doc(hidden)]",
    ] {
        // Recompute original coordinates and consistency claims, so rejection
        // exercises attribute semantics rather than stale transport evidence.
        let original = format!("{attribute}\n{SOURCE}");
        assert!(
            matches!(verify(&fixture(&original)), Err(ProofError::Unsupported(_))),
            "{attribute}"
        );
    }
}

fn widening_source() -> String {
    SOURCE.replace("Truncate as _", "Truncate as _, Widen as _")
        .replace("pub use crate", "#[allow(deprecated)] pub use crate")
        .replace("pub trait Extend", "#[deprecated(since=\"0.2.2\", note=\"use Widen\")] #[allow(deprecated)] pub trait Extend")
        + "pub trait Widen {}\n"
}

fn verify_widening(fixture: &Fixture) -> Result<AnonymousPrelude, ProofError> {
    verify_widening_anonymous_prelude(
        Arc::clone(&fixture.census),
        &fixture.capture,
        &fixture.import,
    )
}

#[test]
fn widening_footprint_requires_all_three_routes_and_preserves_legacy_boundary() {
    let fixture = fixture(&widening_source());
    assert!(verify(&fixture).is_err());
    let receipt = verify_widening(&fixture).unwrap();
    assert_eq!(receipt.widening_trait().unwrap().path, "Widen");
    assert!(verify_widening(&self::fixture(SOURCE)).is_err());
    let attacks: &[fn(&mut Fixture)] = &[
        |f| {
            f.import.exports.pop();
        },
        |f| f.import.exports.swap(1, 2),
        |f| f.import.exports[2].route.clear(),
        |f| f.import.exports[2].target.definition_identity.push('x'),
        |f| f.capture.declarations[2].site.leaf.span.end += 1,
        |f| f.capture.declarations[2].site.declaration.span.start -= 1,
        |f| f.capture.declarations[2].site.owner = definition("other", true),
        |f| f.capture.declarations[2].site.target = f.capture.declarations[0].site.target.clone(),
        |f| {
            f.capture
                .declarations
                .push(f.capture.declarations[2].clone())
        },
        |f| {
            f.capture.unsupported.push(Unsupported {
                definition: definition("Widen", true),
                reason: "unproved".into(),
            })
        },
    ];
    for attack in attacks {
        let mut changed = self::fixture(&widening_source());
        attack(&mut changed);
        assert!(verify_widening(&changed).is_err());
    }
}

#[test]
fn widening_metadata_does_not_authorize_cfg_expansions_or_hidden_items() {
    let original = widening_source();
    for source in [
        original.replace("allow(deprecated)", "allow(unused)"),
        original.replace("allow(deprecated)", "cfg(all())"),
        original.replace("note=\"use Widen\"", "note=concat!(\"use Widen\")"),
        original.replace("note=\"use Widen\"", "since=\"0.2.3\""),
        original.replace("pub mod prelude", "#[allow(deprecated)] pub mod prelude"),
        original.replace("; }", "; pub struct Hidden; }"),
        original.replace("pub trait Widen", "#[cfg(all())] pub trait Widen"),
    ] {
        assert!(verify_widening(&fixture(&source)).is_err(), "{source}");
    }
}
