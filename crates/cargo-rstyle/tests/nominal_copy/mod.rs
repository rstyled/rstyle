//! Resolved Copy derive evidence is observational; nominal admission stays closed.
use super::*;

#[test]
fn builtin_copy_observations_bind_trait_macro_and_original_declaration() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("\u{feff}// π\r\n{}", source()),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8);
    check_observations(functions);
    reject_copy_corruptions(functions);
}

fn check_observations(functions: &[Value]) {
    for function in functions {
        assert_eq!(function["outcome"], "excluded", "{function}");
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let origins = facts
            .build
            .frontend_options
            .data::<rstyle_pdg::declaration_origins::Origins>()
            .unwrap();
        let copy = origins
            .declarations
            .iter()
            .find(|origin| origin.nominal.definition_path.rsplit("::").next() == Some("Copied"));
        if let Some(origin) = copy {
            check_positive(origin, &facts);
        } else {
            assert!(
                origins
                    .declarations
                    .iter()
                    .all(|origin| origin.builtin_copy.is_none()),
                "{function}"
            );
        }
    }
    let (unit, _) = data_unit(&functions[0]);
    assert_eq!(unit.functions().len(), 8);
    for (stored, function) in unit.functions().iter().zip(functions) {
        let actual = stored
            .build
            .frontend_options
            .data::<rstyle_pdg::declaration_origins::Origins>()
            .unwrap();
        let expected: rstyle_pdg::declaration_origins::Origins = serde_json::from_value(
            function["facts"]["build"]["frontend_options"]["concrete_declaration_origins"].clone(),
        )
        .unwrap();
        assert_eq!(actual.as_ref(), &expected);
    }
}

fn check_positive(
    origin: &rstyle_pdg::declaration_origins::Origin,
    facts: &rstyle_pdg::HirFnFacts,
) {
    let observed = origin
        .builtin_copy
        .as_ref()
        .expect("builtin Copy observation");
    assert_eq!(observed.trait_definition.crate_name, "core");
    assert_eq!(observed.derive_definition.crate_name, "core");
    let registry = rstyle_pdg::data_library::registry_for(facts).unwrap();
    assert_eq!(observed.trait_definition.crate_hash, registry.core_hash);
    assert_eq!(observed.derive_definition.crate_hash, registry.core_hash);
    assert_ne!(
        observed.trait_definition.definition_hash,
        observed.derive_definition.definition_hash
    );
    let bytes = &origin.file_bytes;
    assert_eq!(
        &bytes[observed.derive_span.start as usize..observed.derive_span.end as usize],
        "Copy"
    );
}

fn source() -> &'static str {
    r#"
    #[derive(Clone, Copy)] pub struct Copied { pub value: usize }
    pub struct Plain { pub value: usize }
    pub struct Manual { pub value: usize }
    impl Clone for Manual { fn clone(&self) -> Self { Manual { value: self.value } } }
    impl Copy for Manual {}
    pub struct Dropped { pub value: usize }
    impl Drop for Dropped { fn drop(&mut self) {} }
    pub fn copied(value: &Copied) -> usize { value.value }
    pub fn plain(value: &Plain) -> usize { value.value }
    pub fn manual(value: &Manual) -> usize { value.value }
    pub fn dropped(value: &Dropped) -> usize { value.value }
    impl Copied { pub fn copied_method(&self) -> usize { self.value } }
    "#
}

#[test]
fn builtin_copy_observations_preserve_std_and_no_std_ci_target_contexts() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let output = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(output.status.success());
        if !Path::new(String::from_utf8(output.stdout).unwrap().trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            let directory = clone_workspace();
            let prefix = if no_std { "#![no_std]\n" } else { "" };
            std::fs::write(
                directory.path().join("src/lib.rs"),
                format!("{prefix}{}", source()),
            )
            .unwrap();
            let (output, report) = clones(
                directory.path(),
                &["--target", target, "--extraction-profile", "hir-data-v3"],
            );
            assert!(output.status.success(), "{target}: {report}");
            check_observations(report["functions"].as_array().unwrap());
        }
    }
}

#[test]
fn historical_profiles_do_not_publish_builtin_copy_observations() {
    for profile in ["hir-data-v1", "hir-data-v2"] {
        let directory = clone_workspace();
        std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(output.status.success(), "{report}");
        for function in report["functions"].as_array().unwrap() {
            let origins = &function["facts"]["build"]["frontend_options"]["concrete_declaration_origins"]
                ["declarations"];
            for origin in origins.as_array().unwrap() {
                assert!(origin.get("builtin_copy").is_none());
            }
        }
    }
}

#[test]
fn a_procedural_derive_named_copy_cannot_supply_the_builtin_observation() {
    let directory = custom_derive_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    let functions = report["functions"].as_array().unwrap();
    let getter = functions
        .iter()
        .find(|f| f["facts"]["name"] == "read")
        .unwrap();
    assert_eq!(getter["outcome"], "excluded");
    let origins: rstyle_pdg::declaration_origins::Origins = serde_json::from_value(
        getter["facts"]["build"]["frontend_options"]["concrete_declaration_origins"].clone(),
    )
    .unwrap();
    assert_eq!(origins.declarations.len(), 1);
    assert!(origins.declarations[0].builtin_copy.is_none());
    let (unit, _) = data_unit(getter);
    let selected_unit = functions
        .iter()
        .filter(|function| function["compilation_unit"] == getter["compilation_unit"])
        .count();
    assert_eq!(unit.functions().len(), selected_unit);
    assert_eq!(selected_unit, 2);
    assert_eq!(functions.len(), 3);
}

fn custom_derive_workspace() -> TempDir {
    let directory = clone_workspace();
    let manifest = directory.path().join("Cargo.toml");
    let bytes = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        manifest,
        format!("{bytes}\n[dependencies]\ncustom = {{ path = 'custom' }}\n"),
    )
    .unwrap();
    let custom = directory.path().join("custom");
    std::fs::create_dir_all(custom.join("src")).unwrap();
    std::fs::write(
        custom.join("Cargo.toml"),
        "[package]\nname='custom'\nversion='0.1.0'\nedition='2021'\n[lib]\nproc-macro=true\n",
    )
    .unwrap();
    std::fs::write(custom.join("src/lib.rs"), r##"
        use proc_macro::TokenStream;
        #[proc_macro_derive(Copy)]
        pub fn copy(_: TokenStream) -> TokenStream {
            "#[automatically_derived] impl core::marker::Copy for Shadow {} impl Clone for Shadow { fn clone(&self) -> Self { *self } }".parse().unwrap()
        }
    "##).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        #[derive(custom::Copy)] pub struct Shadow { pub value: usize }
        pub fn read(value: &Shadow) -> usize { value.value }
    "#,
    )
    .unwrap();
    directory
}

#[test]
fn conditional_copy_derives_follow_actual_compiler_presence() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        #[cfg_attr(all(), derive(Clone, Copy))]
        pub struct Active { pub value: usize }
        #[cfg_attr(any(), derive(Clone, Copy))]
        pub struct Inactive { pub value: usize }
        pub fn active(value: &Active) -> usize { value.value }
        pub fn inactive(value: &Inactive) -> usize { value.value }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    reject_inactive_copy(&report);
    for name in ["active", "inactive"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "excluded");
        let origins: rstyle_pdg::declaration_origins::Origins = serde_json::from_value(
            function["facts"]["build"]["frontend_options"]["concrete_declaration_origins"].clone(),
        )
        .unwrap();
        assert_eq!(origins.declarations.len(), 1);
        assert_eq!(
            origins.declarations[0].builtin_copy.is_some(),
            name == "active"
        );
    }
}

fn reject_copy_corruptions(functions: &[Value]) {
    let selected = functions
        .iter()
        .find(|function| function["facts"]["name"] == "copied")
        .unwrap();
    let context: rstyle_pdg::nominal_binding::PackageContext = serde_json::from_value(
        selected["facts"]["build"]["frontend_options"]["rstyle.nominal_selected_unit"]["package"]
            .clone(),
    )
    .unwrap();
    let original = selected["facts"].clone();
    let facts = serde_json::from_value(original.clone()).unwrap();
    rstyle_pdg::nominal_binding::bind_unit(&[facts], &context).unwrap();
    for change in ["trait", "derive", "span", "path", "implementation"] {
        let mut changed = original.clone();
        let observed = &mut changed["build"]["frontend_options"]["concrete_declaration_origins"]["declarations"]
            [0]["builtin_copy"];
        match change {
            "trait" => observed["trait_definition"]["definition_hash"] = "unregistered".into(),
            "derive" => {
                observed["derive_definition"]["definition_hash"] =
                    observed["trait_definition"]["definition_hash"].clone()
            }
            "span" => observed["derive_span"]["start"] = 0.into(),
            "path" => observed["derive_definition"]["path"] = "custom::Copy".into(),
            "implementation" => {
                observed["trait_definition"]["implementation"] =
                    observed["derive_definition"].clone()
            }
            _ => unreachable!(),
        }
        let facts = serde_json::from_value(changed).unwrap();
        let error = rstyle_pdg::nominal_binding::bind_unit(&[facts], &context)
            .err()
            .unwrap();
        assert!(error.contains("Copy observation"), "{change}: {error}");
    }
}

fn reject_inactive_copy(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let active = functions
        .iter()
        .find(|f| f["facts"]["name"] == "active")
        .unwrap();
    let inactive = functions
        .iter()
        .find(|f| f["facts"]["name"] == "inactive")
        .unwrap();
    let context: rstyle_pdg::nominal_binding::PackageContext = serde_json::from_value(
        inactive["facts"]["build"]["frontend_options"]["rstyle.nominal_selected_unit"]["package"]
            .clone(),
    )
    .unwrap();
    let mut changed = inactive["facts"].clone();
    let copied = active["facts"]["build"]["frontend_options"]["concrete_declaration_origins"]
        ["declarations"][0]["builtin_copy"].clone();
    let origin = &mut changed["build"]["frontend_options"]["concrete_declaration_origins"]["declarations"]
        [0];
    origin["builtin_copy"] = copied;
    // Point at the inactive struct's own correctly spelled token, rather than
    // merely reusing the unrelated active struct's span.
    let bytes = origin["file_bytes"].as_str().unwrap();
    let start = bytes
        .find("#[cfg_attr(any(), derive(Clone, Copy))]")
        .unwrap()
        + "#[cfg_attr(any(), derive(Clone, ".len();
    origin["builtin_copy"]["derive_span"] = serde_json::json!({"start": start, "end": start + 4});
    let facts = serde_json::from_value(changed).unwrap();
    let error = rstyle_pdg::nominal_binding::bind_unit(&[facts], &context)
        .err()
        .unwrap();
    assert!(error.contains("active derive-token ownership"), "{error}");
}
