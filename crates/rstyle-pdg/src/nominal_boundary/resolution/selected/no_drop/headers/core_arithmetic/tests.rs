use super::*;

fn scopes(import: &str) -> Vec<BTreeMap<String, bool>> {
    let item = syn::parse_str::<syn::ItemUse>(import).unwrap();
    let mut scope = BTreeMap::new();
    for (name, _) in source_imports::bindings(&item.tree) {
        source_imports::bind(&mut scope, name, false);
    }
    source_imports::record(&item.tree, &mut scope);
    vec![scope]
}

fn checked(path: &str, scopes: &[BTreeMap<String, bool>]) -> bool {
    resolve(
        &syn::parse_str::<syn::TraitBound>(path).unwrap().path,
        scopes,
        Some(&crate::data_library::registries()[0]),
    )
    .is_some()
}

#[test]
fn original_arithmetic_imports_prove_exact_identity_with_bounded_type_arguments() {
    for name in NAMES {
        let scopes = scopes(&format!("use core::ops::{name};"));
        assert!(checked(name, &scopes), "{name}");
        assert!(checked(&format!("core::ops::{name}"), &[]));
        if *name != "Neg" {
            assert!(checked(&format!("{name}<Duration>"), &scopes));
            assert!(checked(&format!("{name}<&'a [u8]>"), &scopes));
        }
    }
    assert!(checked(
        "Arithmetic<Duration>",
        &scopes("use core::ops::Add as Arithmetic;")
    ));
    assert!(checked(
        "operations::Add<Duration>",
        &scopes("use core::ops as operations;")
    ));
}

#[test]
fn unknown_shadowed_drop_or_malformed_trait_paths_have_no_receipt() {
    let imported = scopes("use core::ops::Add;");
    for path in [
        "Add<>",
        "Add<u8, u16>",
        "Add<'a>",
        "Add<3>",
        "Add<Output = u8>",
        "Add<Output: Copy>",
        "core::ops::Neg<u8>",
        "::core::ops::Add<u8>",
        "core::ops::<u8>::Add<u8>",
        "core::ops::Drop",
        "core::ops::Rem<u8>",
        "std::ops::Add<u8>",
        "external::ops::Add<u8>",
        "Add(u8)",
    ] {
        assert!(!checked(path, &imported), "{path}");
    }
    assert!(!checked("Add<u8>", &[]));
    for import in ["use core::ops::Drop as Add;", "use external::ops::Add;"] {
        assert!(!checked("Add<u8>", &scopes(import)), "{import}");
    }
    for name in ["Add", "*", "core", "std"] {
        let mut changed = imported.clone();
        changed.push(BTreeMap::from([(name.into(), false)]));
        assert!(!checked("Add<u8>", &changed), "{name}");
    }
}

#[test]
fn arithmetic_identity_requires_checked_revision_and_complete_existing_source_authority() {
    let path = syn::parse_str("core::ops::Add<u8>").unwrap();
    assert!(resolve(&path, &[], None).is_none());
    for registry in crate::data_library::registries() {
        assert!(resolve(&path, &[], Some(registry)).is_some());
    }
    let mut registry: Registry = serde_json::from_str(include_str!(
        "../../../../../../../contracts/read-only-lengths-v2.json"
    ))
    .unwrap();
    registry.rustc_commit = "unreviewed".into();
    assert!(resolve(&path, &[], Some(&registry)).is_none());
    registry.rustc_commit = REVIEWED_COMMIT.into();
    registry
        .cleanup_macro_source_digests
        .remove("core/src/prelude/v1.rs");
    assert!(resolve(&path, &[], Some(&registry)).is_none());
    registry
        .cleanup_macro_source_digests
        .insert("core/src/prelude/v1.rs".into(), "changed".into());
    assert!(resolve(&path, &[], Some(&registry)).is_none());
}

#[test]
fn original_reverse_arithmetic_impls_preserve_historical_and_hidden_cleanup_barriers() {
    let source = "struct Error(()); struct Duration; use core::time::Duration as StdDuration; use core::ops::{Add, Sub, Div}; impl Add<Duration> for StdDuration { type Output = Duration; fn add(self, _: Duration) -> Duration { Duration } }";
    let inspect = super::super::super::closure::inspect_external_source;
    inspect(source, true).unwrap();
    assert!(inspect(source, false).is_err());
    for extra in [
        "mod core {}",
        "use external::*;",
        "impl Drop for Error { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Error { fn drop(&mut self) {} } }",
        "macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
    ] {
        assert!(
            inspect(&format!("{source} {extra}"), true).is_err(),
            "{extra}"
        );
    }
    let hidden = source.replace(
        "{ Duration }",
        "{ impl Drop for Error { fn drop(&mut self) {} } Duration }",
    );
    assert!(inspect(&hidden, true).is_err());
}
