use super::*;

const RAW: &[&str] = &["--compiler-advisory-only"];
const GENERIC: &str = "pub struct Owner<T>(pub T);\n";
const SOURCE_POLICY: &str =
    "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\ncount_self=false\n";

fn fixture(external: bool, source: &str) -> TempDir {
    let directory = workspace(if external {
        "pub mod external;\n"
    } else {
        source
    });
    std::fs::write(directory.path().join("analysis.toml"), SOURCE_POLICY).unwrap();
    if external {
        std::fs::write(directory.path().join("src/external.rs"), source).unwrap();
    }
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> Value {
    let mut options = RAW.to_vec();
    options.extend_from_slice(extra);
    let (output, report) = requested(directory, &options);
    checked(&output, 0);
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["matching_execution"], "not_requested");
    assert!(report.get("compiler_advisory").is_none());
    report
}

fn compact(name: &str) -> String {
    name.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn locators(directory: &Path) -> Vec<String> {
    let saved = stored(directory);
    let mut names: Vec<_> = saved["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            assert_eq!(entry["finding"]["locator"]["declaration_kind"], "method");
            compact(
                entry["finding"]["locator"]["qualified_name"]
                    .as_str()
                    .unwrap(),
            )
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

fn parameter_owner(report: &Value) -> &Value {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["assessments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|assessment| {
                    assessment["rule_id"] == "fn_params" && assessment["outcome"] == "finding"
                })
        })
        .unwrap()
}

fn dismiss(directory: &Path, original: &Value) -> Value {
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--dismiss-parameter",
            parameter_owner(original)["key"].as_str().unwrap(),
            "--review-reason",
            "Reviewed generic method parameter boundary",
        ],
    );
    checked(&output, 0);
    let report = parsed(&output);
    assert_eq!(report["parameter_review"]["hidden_findings"], 1);
    report
}

fn source_free(directory: &Path, original: &Value, existing: usize) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    let external = directory.join("src/external.rs");
    if external.exists() {
        std::fs::remove_file(external).unwrap();
    }
    for detail in ["full", "compact"] {
        let mut options = [RAW, BASELINE].concat();
        options.extend(["--report-detail", detail, "--review-file", "reviews.json"]);
        let output = replay(directory, original, &options);
        checked(&output, 0);
        let report = parsed(&output);
        counts(&report, existing, 0);
        assert_eq!(report["verification_policy"], "raw-replay");
        assert_eq!(report["parameter_review"]["hidden_findings"], 1);
        assert_eq!(report["analysis"]["findings"], existing + 1);
    }
}

fn insert_unrelated_impl(directory: &Path, external: bool, source: &str) {
    let path = if external {
        "src/external.rs"
    } else {
        "src/lib.rs"
    };
    std::fs::write(
        directory.join(path),
        format!("pub struct Unrelated; impl Unrelated {{ pub fn clean() {{}} }}\n{source}"),
    )
    .unwrap();
}

#[test]
fn generic_root_and_external_method_baselines_and_reviews_survive_unrelated_impl_insertion() {
    let source = format!("{GENERIC}impl<T> Owner<T> {{\n{ORIGINAL}}}\n");
    for external in [false, true] {
        let directory = fixture(external, &source);
        let original = capture(
            directory.path(),
            &["--write-source-baseline", "baseline.json"],
        );
        let prefix = if external { "crate::external" } else { "crate" };
        assert_eq!(
            locators(directory.path()),
            [format!("{prefix}::Owner<T>::many")]
        );
        assert_eq!(
            stored(directory.path())["entries"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        dismiss(directory.path(), &original);
        let saved = std::fs::read(directory.path().join("reviews.json")).unwrap();
        insert_unrelated_impl(directory.path(), external, &source);
        let changed = capture(
            directory.path(),
            &[
                "--review-file",
                "reviews.json",
                "--review-status",
                "--source-baseline",
                "baseline.json",
                "--fail-on-new-source",
            ],
        );
        counts(&changed, 1, 0);
        assert_eq!(
            changed["review_status"]["records"][0]["state"],
            "hidden_exact"
        );
        source_free(directory.path(), &changed, 1);
        assert_eq!(
            std::fs::read(directory.path().join("reviews.json")).unwrap(),
            saved
        );
    }
}

fn specialized_source() -> String {
    let mut source = String::from("pub struct Owner<T>(pub T); pub struct Count<const N:usize>;\n");
    for owner in ["Owner<u8>", "Owner<u16>", "Count<1>", "Count<2>"] {
        source.push_str(&format!("impl {owner} {{\n{ORIGINAL}}}\n"));
    }
    source
}

#[test]
fn concrete_type_and_const_owner_arguments_keep_distinct_baselines_and_reviews() {
    let directory = fixture(false, &specialized_source());
    let original = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    assert_eq!(
        locators(directory.path()),
        [
            "crate::Count<1>::many",
            "crate::Count<2>::many",
            "crate::Owner<u16>::many",
            "crate::Owner<u8>::many",
        ]
    );
    assert_eq!(
        stored(directory.path())["entries"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    let reviewed = dismiss(directory.path(), &original);
    assert_eq!(reviewed["parameter_review"]["visible_findings"], 3);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-only",
            "--source-baseline",
            "baseline.json",
            "--fail-on-new-source",
            "--review-file",
            "reviews.json",
        ],
    );
    checked(&output, 0);
    counts(&parsed(&output), 7, 0);
    source_free(directory.path(), &original, 7);
}

#[test]
fn generic_local_trait_arguments_keep_same_named_method_reviews_separate() {
    let source = format!(
        "pub struct Owner; pub trait Local<T> {{fn many(a:u32,b:u32)->u32;}}\nimpl Local<u8> for Owner {{\n{ORIGINAL}}}\nimpl Local<u16> for Owner {{\n{ORIGINAL}}}\n"
    ).replace("pub fn many", "fn many");
    let directory = fixture(true, &source);
    let original = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    assert_eq!(
        locators(directory.path()),
        [
            "crate::external::<OwnerasLocal<u16>>::many",
            "crate::external::<OwnerasLocal<u8>>::many",
        ]
    );
    assert_eq!(
        stored(directory.path())["entries"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let reviewed = dismiss(directory.path(), &original);
    assert_eq!(reviewed["parameter_review"]["visible_findings"], 1);
    source_free(directory.path(), &original, 3);
}

#[test]
fn generic_local_trait_on_nongeneric_owner_has_root_and_external_review_identity() {
    let source = format!(
        "pub struct Unit; pub trait Local<T> {{fn many(a:u32,b:u32)->u32;}}\nimpl<T> Local<T> for Unit {{\n{ORIGINAL}}}\n"
    )
    .replace("pub fn many", "fn many");
    for external in [false, true] {
        let directory = fixture(external, &source);
        let original = capture(
            directory.path(),
            &["--write-source-baseline", "baseline.json"],
        );
        let prefix = if external { "crate::external" } else { "crate" };
        assert_eq!(
            locators(directory.path()),
            [format!("{prefix}::<UnitasLocal<T>>::many")]
        );
        let reviewed = dismiss(directory.path(), &original);
        assert_eq!(reviewed["parameter_review"]["visible_findings"], 0);
        source_free(directory.path(), &original, 1);
    }
}

#[test]
fn existing_root_function_review_ignores_unrelated_external_location_parser_budget() {
    let directory = fixture(false, &format!("pub mod external;\n{ORIGINAL}"));
    let comment = format!("/* {} */\n", "x".repeat(300 * 1024));
    std::fs::write(directory.path().join("src/external.rs"), comment).unwrap();
    let original = capture(directory.path(), &[]);
    let reviewed = dismiss(directory.path(), &original);
    assert_eq!(reviewed["parameter_review"]["coverage"], "complete");
    assert_eq!(reviewed["parameter_review"]["visible_findings"], 0);
    let store: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("reviews.json")).unwrap())
            .unwrap();
    let locator = &store["parameter_entries"][0]["finding"]["locator"];
    assert_eq!(locator["qualified_name"], "crate::many");
    assert_eq!(locator["source"], "src/lib.rs");
    assert_eq!(locator["declaration_kind"], "free-function");
}

#[test]
fn nongeneric_and_lifetime_method_locators_preserve_external_and_name_root_nominal_owners() {
    for external in [false, true] {
        for (declaration, implementation) in [
            ("pub struct Owner;", "impl Owner"),
            ("pub struct Owner<'a>(pub &'a u32);", "impl<'a> Owner<'a>"),
        ] {
            let directory = fixture(
                external,
                &format!("{declaration}{implementation} {{\n{ORIGINAL}}}\n"),
            );
            let original = capture(
                directory.path(),
                &["--write-source-baseline", "baseline.json"],
            );
            let prefix = if external { "crate::external" } else { "crate" };
            assert_eq!(
                locators(directory.path()),
                [format!("{prefix}::Owner::many")]
            );
            dismiss(directory.path(), &original);
            let store: Value = serde_json::from_slice(
                &std::fs::read(directory.path().join("reviews.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                store["parameter_entries"][0]["finding"]["locator"]["qualified_name"],
                format!("{prefix}::Owner::many")
            );
            assert_eq!(
                store["parameter_entries"][0]["finding"]["locator"]["declaration_kind"],
                "method"
            );
        }
    }
}

#[test]
fn malformed_old_root_lifetime_locator_neither_hides_current_finding_nor_becomes_prunable() {
    let source =
        format!("pub struct Owner<'a>(pub &'a u32); impl<'a> Owner<'a> {{\n{ORIGINAL}}}\n");
    let directory = fixture(false, &source);
    let original = capture(directory.path(), &[]);
    dismiss(directory.path(), &original);
    let path = directory.path().join("reviews.json");
    let mut store: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    store["parameter_entries"][0]["finding"]["locator"]["qualified_name"] = "crate::::many".into();
    let saved = serde_json::to_vec(&store).unwrap();
    std::fs::write(&path, &saved).unwrap();
    let output = replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--review-status",
            "--review-prune",
            "--review-prune-dry-run",
        ],
    );
    checked(&output, 2);
    let report = parsed(&output);
    assert_eq!(report["parameter_review"]["hidden_findings"], 0);
    assert_eq!(report["parameter_review"]["visible_findings"], 1);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "not_observed"
    );
    assert_eq!(report["review_prune"]["eligible_entries"], 0);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

fn raw_wire(report: &Value) -> Value {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn generic_method_missing_context_and_duplicate_captured_owner_cannot_export_or_dismiss() {
    let directory = fixture(
        false,
        &format!("{GENERIC}impl<T> Owner<T> {{\n{ORIGINAL}}}\n"),
    );
    let original = capture(directory.path(), &[]);
    let wire = raw_wire(&original);
    let ledger = super::super::compiler_advisory::only::ledger_path(&original);
    for duplicate in [false, true] {
        let mut changed = wire.clone();
        if duplicate {
            let owner = changed["facts"][0].clone();
            changed["facts"].as_array_mut().unwrap().push(owner);
        } else {
            changed["facts"][0]["build"]["frontend_options"]
                .as_object_mut()
                .unwrap()
                .remove(rstyle_pdg::nominal_boundary::CONTEXT_KEY);
        }
        super::super::compiler_advisory::only::reseal_raw(&original, &ledger, &changed);
        let output = replay(
            directory.path(),
            &original,
            &[
                "--compiler-advisory-only",
                "--write-source-baseline",
                "invalid.json",
            ],
        );
        checked(&output, 2);
        assert!(!directory.path().join("invalid.json").exists());
        let output = replay(
            directory.path(),
            &original,
            &[
                "--compiler-advisory-only",
                "--review-file",
                "reviews.json",
                "--dismiss-parameter",
                parameter_owner(&original)["key"].as_str().unwrap(),
                "--review-reason",
                "Unavailable original owner",
            ],
        );
        checked(&output, 2);
        assert!(!directory.path().join("reviews.json").exists());
    }
    super::super::compiler_advisory::only::reseal_raw(&original, &ledger, &wire);
    checked(&replay(directory.path(), &original, RAW), 0);
}
