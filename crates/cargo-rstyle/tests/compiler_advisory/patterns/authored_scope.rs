use super::mining_options::{checked, remove_sources};
use super::*;

const DERIVED: &str = "#[derive(Debug)] pub struct Generated(pub u32);";
const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";
const AUTHORED: &[&str] = &["--compiler-pattern-scope", "authored"];
fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(output),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn raw(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["patterns", "--format", "json"];
    args.extend_from_slice(extra);
    rstyle(directory, &args)
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "patterns",
            "--format",
            "json",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let value = json(&output);
    (output, value)
}

fn direct(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--compiler-patterns",
        "--pattern-ledger",
        "rstyle-ignore.toml",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    advisory(directory, &args)
}

fn outside(report: &Value) -> &Vec<Value> {
    proposals(report)["source_scope"]["outside_scope_functions"]
        .as_array()
        .unwrap()
}

#[test]
fn authored_scope_distinguishes_derived_owners_from_explicit_captured_scope() {
    let directory = fixture(&format!("{DERIVED} pub fn original() {{}}"));
    let output = raw(directory.path(), &["--compiler-pattern-scope", "captured"]);
    checked(&output, 2);
    let original = json(&output);
    assert!(proposals(&original).get("source_scope").is_none());
    assert_eq!(proposals(&original)["input_complete"], false);
    let (output, authored) = saved(directory.path(), &original, &[]);
    checked(&output, 0);
    assert_eq!(proposals(&authored)["source_scope"]["kind"], "authored");
    assert_eq!(proposals(&authored)["input_complete"], true);
    assert_eq!(proposals(&authored)["unavailable_functions"], 0);
    assert!(!outside(&authored).is_empty());
    let unit = &authored["compiler_advisory"]["units"][0];
    let excluded: Vec<_> = unit["unavailable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["key"].clone())
        .collect();
    assert_eq!(outside(&authored), &excluded);
    assert_eq!(
        unit["unavailable"],
        original["compiler_advisory"]["units"][0]["unavailable"]
    );
    assert_eq!(
        proposals(&authored)["evaluation"],
        proposals(&original)["evaluation"]
    );
    let (output, captured) = saved(
        directory.path(),
        &original,
        &["--compiler-pattern-scope", "captured"],
    );
    checked(&output, 2);
    assert_eq!(captured["compiler_advisory"], original["compiler_advisory"]);
    let (output, explicit) = direct(directory.path(), &original, AUTHORED);
    checked(&output, 0);
    assert_eq!(explicit["compiler_advisory"], authored["compiler_advisory"]);
}

#[test]
fn authored_scope_preserves_findings_and_replays_without_sources_or_tools() {
    let directory = fixture(&format!("{DERIVED} {HELPERS}"));
    let output = raw(
        directory.path(),
        &[
            "--compiler-pattern-scope",
            "authored",
            "--fail-on-candidates",
        ],
    );
    checked(&output, 1);
    let original = json(&output);
    assert_eq!(proposals(&original)["input_complete"], true);
    assert_eq!(
        proposals(&original)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    remove_sources(directory.path());
    let (output, replayed) = saved(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--fail-on-candidates",
        ],
    );
    checked(&output, 1);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}

#[test]
fn authored_scope_preserves_numeric_incompleteness_and_strict_decisions() {
    let directory = fixture(&format!("{DERIVED} pub fn original() {{}}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let (output, combined) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--compiler-advisory",
        ],
    );
    checked(&output, 2);
    assert_eq!(proposals(&combined)["input_complete"], true);
    assert_eq!(combined["compiler_advisory"]["complete"], false);
    strict_decisions_agree(directory.path());
}

fn strict_decisions_agree(directory: &Path) {
    let (output, standalone) = advisory(directory, &[]);
    checked(&output, 0);
    let (output, strict) = advisory(
        directory,
        &[
            "--compiler-patterns",
            "--compiler-pattern-scope",
            "authored",
        ],
    );
    checked(&output, 0);
    let outcomes = |report: &Value| {
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|function| (function["name"].clone(), function["outcome"].clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(outcomes(&strict), outcomes(&standalone));
    assert!(
        !strict["compiler_advisory"]["units"][0]["unavailable"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn authored_scope_keeps_check_and_test_owner_populations_distinct() {
    let directory = fixture(&format!(
        "{DERIVED} pub fn original() {{}} #[cfg(test)] pub fn test_only() {{}}"
    ));
    let output = raw(
        directory.path(),
        &["--compiler-pattern-scope", "authored", "--all-targets"],
    );
    checked(&output, 0);
    let report = json(&output);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    let mut counts: Vec<_> = units
        .iter()
        .map(|unit| {
            assert_eq!(unit["patterns"]["input_complete"], true);
            assert_eq!(unit["patterns"]["source_scope"]["kind"], "authored");
            assert!(
                !unit["patterns"]["source_scope"]["outside_scope_functions"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            unit["patterns"]["observed_graphs"].as_u64().unwrap()
        })
        .collect();
    counts.sort();
    assert_eq!(counts, [1, 2]);
    let (output, replayed) = saved(directory.path(), &report, AUTHORED);
    checked(&output, 0);
    assert_eq!(replayed["compiler_advisory"], report["compiler_advisory"]);
}

#[test]
fn authored_scope_can_export_a_baseline_despite_expanded_owners() {
    let directory = fixture(&format!("{DERIVED} {HELPERS}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let (output, exported) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--write-compiler-pattern-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    let bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
    let baseline: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(baseline["source_scope"], "authored");
    assert_eq!(baseline["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        proposals(&exported)["evaluation"],
        proposals(&original)["evaluation"]
    );
    remove_sources(directory.path());
    let (output, compared) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--compiler-pattern-baseline",
            "baseline.json",
            "--fail-on-new-compiler-patterns",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["scope_changed"],
        false
    );
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["existing_findings"],
        1
    );
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        bytes
    );
}

#[test]
fn source_scope_does_not_change_a_positive_structural_review() {
    let directory = fixture(&format!("{DERIVED} {HELPERS}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let id = proposals(&original)["evaluation"]["candidates"][0]["id"]
        .as_str()
        .unwrap();
    let (output, reviewed) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            id,
            "--review-reason",
            "Independent authored-scope fixture review",
        ],
    );
    checked(&output, 0);
    let decisions = &reviewed["compiler_advisory"]["pattern_review"]["decisions"];
    assert_eq!(decisions[0]["state"], "hidden_exact");
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    remove_sources(directory.path());
    let (output, captured) = saved(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--review-status",
            "--compiler-pattern-scope",
            "captured",
        ],
    );
    checked(&output, 2);
    assert_eq!(
        &captured["compiler_advisory"]["pattern_review"]["decisions"],
        decisions
    );
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

#[test]
fn authored_and_captured_baseline_headers_cannot_establish_each_others_clean_scope() {
    let directory = fixture("pub fn original() {}");
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = json(&output);
    for (scope, filename) in [("captured", "captured.json"), ("authored", "authored.json")] {
        let (output, _) = direct(
            directory.path(),
            &original,
            &[
                "--compiler-pattern-scope",
                scope,
                "--write-compiler-pattern-baseline",
                filename,
            ],
        );
        checked(&output, 0);
        let baseline: Value =
            serde_json::from_slice(&std::fs::read(directory.path().join(filename)).unwrap())
                .unwrap();
        if scope == "captured" {
            assert!(baseline.get("source_scope").is_none());
        } else {
            assert_eq!(baseline["source_scope"], "authored");
        }
    }
    for (scope, filename) in [("authored", "captured.json"), ("captured", "authored.json")] {
        let (output, report) = direct(
            directory.path(),
            &original,
            &[
                "--compiler-pattern-scope",
                scope,
                "--compiler-pattern-baseline",
                filename,
                "--fail-on-new-compiler-patterns",
            ],
        );
        checked(&output, 1);
        assert_eq!(
            report["compiler_advisory"]["pattern_baseline"]["scope_changed"],
            true
        );
    }
}

fn select(directory: &Path, path: &str) {
    std::fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema": "rstyle.changed-files/1", "paths": [path],
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn authored_changed_scope_covers_expanded_only_files_but_not_unknown_paths() {
    let directory = fixture("pub mod generated; pub fn original() {}");
    std::fs::write(directory.path().join("src/generated.rs"), DERIVED).unwrap();
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    for (path, code) in [
        ("src/lib.rs", 0),
        ("src/generated.rs", 0),
        ("src/missing.rs", 2),
    ] {
        select(directory.path(), path);
        let (output, report) = direct(
            directory.path(),
            &original,
            &[
                "--compiler-pattern-scope",
                "authored",
                "--select-compiler-patterns",
                "--changed-files",
                "selection.json",
                "--fail-on-findings",
            ],
        );
        checked(&output, code);
        assert_eq!(proposals(&report)["input_complete"], true);
        let coverage = &report["compiler_advisory"]["changed_scope"]["paths"][0]["coverage"];
        assert_eq!(coverage, if code == 0 { "complete" } else { "incomplete" });
    }
}

/// A saved run's documents, with its manifest and ledger sealed over `edit`'s result.
fn resealed(original: &Value, edit: impl FnOnce(&mut Vec<Value>)) {
    use rstyle_pdg::unit_store::{
        DOCUMENTS_MANIFEST, DocumentManifest, SealedDocument, transport_checksum,
    };
    let path = original["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let path = Path::new(path);
    let directory = path.parent().unwrap();
    let manifest = DocumentManifest::read(&std::fs::read(path).unwrap()).unwrap();
    let mut documents: Vec<Value> = manifest
        .documents
        .iter()
        .map(|sealed| rstyle_core::cpg::transport::read_sealed(directory, sealed).unwrap())
        .collect();
    edit(&mut documents);
    let mut sealed = Vec::new();
    for document in &documents {
        let bytes = serde_json::to_vec(document).unwrap();
        let seal = SealedDocument::new(sealed.len(), &bytes);
        std::fs::write(directory.join(&seal.file), bytes).unwrap();
        sealed.push(seal);
    }
    let bytes = serde_json::to_vec(&DocumentManifest::of(sealed)).unwrap();
    std::fs::write(path, &bytes).unwrap();
    let ledger = super::super::only::ledger_path(original);
    let mut unit: Value = serde_json::from_slice(&std::fs::read(&ledger).unwrap()).unwrap();
    for artifact in unit["artifacts"].as_array_mut().unwrap() {
        if artifact["file"] == DOCUMENTS_MANIFEST {
            artifact["checksum"] = transport_checksum(&bytes).into();
        }
    }
    std::fs::write(ledger, serde_json::to_vec(&unit).unwrap()).unwrap();
}

/// Moves a function's signature to a file outside the workspace root, so that the core
/// has no facts for it though no macro wrote it.
fn outside_the_root(documents: &mut [Value], name: &str) {
    let document = documents
        .iter_mut()
        .find(|document| {
            document["unit"]["functions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|function| function["name"] == name)
        })
        .unwrap();
    let mut file = document["unit"]["files"][0].clone();
    file["id"] = "file:outside".into();
    file["path"] = "../outside.rs".into();
    document["unit"]["files"].as_array_mut().unwrap().push(file);
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    let function = functions
        .iter_mut()
        .find(|function| function["name"] == name)
        .unwrap();
    function["signature"]["span"]["file"] = "file:outside".into();
}

#[test]
fn documents_never_hide_a_function_the_core_cannot_measure() {
    let directory = fixture(&format!("{DERIVED} pub fn original() {{}}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let (_, complete) = saved(directory.path(), &original, AUTHORED);
    assert_eq!(proposals(&complete)["input_complete"], true);
    assert_eq!(outside(&complete).len(), 1);

    resealed(&original, |documents| {
        outside_the_root(documents, "original")
    });
    let (output, report) = saved(directory.path(), &original, AUTHORED);
    checked(&output, 2);
    assert_eq!(report["capture_failures"], serde_json::json!([]));
    assert_eq!(report["handoff_failures"], serde_json::json!([]));
    assert_eq!(proposals(&report)["input_complete"], false);
    // It stays unavailable rather than joining the generated functions out of scope.
    assert_eq!(proposals(&report)["unavailable_functions"], 1);
    assert_eq!(outside(&report).len(), 1);
}

#[test]
fn a_changed_document_or_manifest_fails_the_replay() {
    let directory = fixture(&format!("{DERIVED} pub fn original() {{}}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let manifest = original["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let manifest = Path::new(manifest);
    let seal = rstyle_pdg::unit_store::DocumentManifest::read(&std::fs::read(manifest).unwrap())
        .unwrap()
        .documents
        .remove(0);
    let document = manifest.with_file_name(&seal.file);
    let refused = |expected: &str| {
        let (output, report) = saved(directory.path(), &original, AUTHORED);
        checked(&output, 2);
        let failures = report["handoff_failures"].to_string();
        assert!(failures.contains(expected), "{failures}");
    };
    let (document_bytes, manifest_bytes) = (
        std::fs::read(&document).unwrap(),
        std::fs::read(manifest).unwrap(),
    );

    let mut changed = document_bytes.clone();
    changed[0] = b' ';
    std::fs::write(&document, &changed).unwrap();
    refused("disagrees with its manifest");

    std::fs::remove_file(&document).unwrap();
    refused("missing captured artifact document-000000.cpg.json.zst");
    std::fs::write(&document, &document_bytes).unwrap();

    let mut changed = manifest_bytes.clone();
    changed.push(b' ');
    std::fs::write(manifest, &changed).unwrap();
    refused("captured artifact checksum mismatch: documents.manifest.json");
    std::fs::write(manifest, &manifest_bytes).unwrap();

    let (output, _) = saved(directory.path(), &original, AUTHORED);
    checked(&output, 0);
}
