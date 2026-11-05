use super::propagation::checked;
use super::*;
use rstyle_core::cpg::model::Document;

const SOURCE: &str = "pub fn wide(a:u32,b:u32)->u32 { let total=a+b; total }\npub fn caller(value:u32)->u32 { wide(value,1) }\n";
const OPTIONS: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-advisory",
    "--compiler-diagnostic-scope",
    "authored",
    "--compiler-plans",
    "--fail-on-findings",
    "--compiler-advisory-config",
    "rstyle.toml",
    "--report-detail",
    "compact",
];

fn fixture() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    directory
}

fn root_capture(directory: &Path) -> Value {
    let output = rstyle(directory, &["--format", "json"]);
    checked(&output, 1);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary_complete"], true);
    assert_eq!(report["summary"]["functions"], 2);
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["rule_id"], "fn_params");
    assert_eq!(diagnostics[0]["actual"], 2);
    assert_eq!(diagnostics[0]["limit"], 1);
    report
}

pub(super) fn snapshot(report: &Value) -> Value {
    let archive = Path::new(report["run_directory"].as_str().unwrap()).join("report.json");
    let archive: Value = serde_json::from_slice(&std::fs::read(archive).unwrap()).unwrap();
    let bundle = archive["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap()
}

/// The documents a root capture published, read as the core reads them.
pub(super) fn documents(report: &Value) -> Vec<Document> {
    let archive = Path::new(report["run_directory"].as_str().unwrap()).join("report.json");
    let archive: Value = serde_json::from_slice(&std::fs::read(archive).unwrap()).unwrap();
    let evidence = &archive["functions"][0]["unit_evidence"];
    assert_eq!(evidence["artifact_kind"], "compat-cpg-documents");
    let manifest = Path::new(evidence["bundle"].as_str().unwrap());
    assert_eq!(
        manifest.file_name().unwrap(),
        rstyle_pdg::unit_store::DOCUMENTS_MANIFEST
    );
    let directory = manifest.parent().unwrap();
    assert!(!directory.join("unit.evidence.json").exists());
    assert!(!directory.join("source-inventory.json").exists());
    let manifest =
        rstyle_pdg::unit_store::DocumentManifest::read(&std::fs::read(manifest).unwrap()).unwrap();
    manifest
        .documents
        .iter()
        .map(|sealed| {
            let bytes = std::fs::read(directory.join(&sealed.file)).unwrap();
            sealed.verify(&bytes).unwrap();
            rstyle_core::cpg::layout::read(&bytes).unwrap()
        })
        .collect()
}

fn style_documents(documents: &[Document]) {
    let functions = documents.iter().flat_map(|document| {
        document
            .unit
            .as_ref()
            .unwrap()
            .functions
            .as_deref()
            .unwrap()
    });
    assert_eq!(functions.count(), 2);
}

fn declared(ty: &rstyle_core::cpg::records::DeclaredType, facts: &rstyle_pdg::ResolvedTy) {
    assert_eq!(ty.normalized, facts.normalized);
    assert_eq!(ty.observed, facts.observed);
    assert_eq!(ty.spelling, facts.spelling);
}

/// What the documents record of each function and its build is what a full capture
/// reads of them, so reviews and baselines identify the same owners on either route.
fn complete_documents(documents: &[Document], full: &Value) {
    let bytes = serde_json::to_vec(full).unwrap();
    let complete = rstyle_pdg::unit_store::read_diagnostic_snapshot_headers_phase(
        &bytes,
        "compiler-advisory-only",
    )
    .unwrap();
    shared_unit_options(full, &complete);
    assert_eq!(full["schema"], "rstyle.diagnostic.raw-facts/5");
    assert_eq!(full["phase"], "compiler-advisory-only");
    assert_eq!(full["build_settings"].as_array().unwrap().len(), 1);
    for document in documents {
        let build = rstyle_pdg::cpg_reader::unit_build(document).unwrap();
        for record in rstyle_core::cpg::records::records(document, Default::default()).unwrap() {
            let facts = complete
                .iter()
                .find(|fact| fact.name == record.name)
                .unwrap();
            assert_eq!(record.source, facts.source.bytes.as_str());
            assert_eq!(
                rstyle_pdg::unit_store::transport_checksum(record.source.as_bytes()),
                facts.source.digest
            );
            assert_eq!(record.located.start as u32, facts.fn_span.start);
            assert_eq!(record.located.end as u32, facts.fn_span.end);
            assert_eq!(
                record.root,
                format!("f{}.{}", facts.body.owner, facts.body.local)
            );
            let signature = &facts.signature;
            assert_eq!(record.signature.parameters.len(), signature.params.len());
            for (ty, param) in record.signature.parameters.iter().zip(&signature.params) {
                // This fixture explicitly annotates every parameter as u32.
                assert_eq!(ty.spelling, "u32");
                declared(ty, &param.ty);
            }
            declared(&record.signature.result, &signature.return_ty);
            assert_eq!(record.signature.has_self, signature.has_self);
            let names: Vec<_> = signature
                .generics
                .iter()
                .map(|generic| &generic.name)
                .collect();
            assert_eq!(record.signature.generics.iter().collect::<Vec<_>>(), names);
            assert_build(&build, &facts.build);
        }
    }
}

/// A document's unit settings are the settings a full capture records of each function.
fn assert_build(document: &rstyle_pdg::BuildFacts, facts: &rstyle_pdg::BuildFacts) {
    let mut contexts = rstyle_pdg::nominal_binding::BuildContexts::default();
    let first = contexts.intern(document).unwrap();
    assert_eq!(contexts.intern(facts).unwrap(), first);
}

fn shared_unit_options(full: &Value, complete: &[rstyle_pdg::HirFnFacts<rstyle_pdg::HirRef>]) {
    for key in ["invocation", rstyle_pdg::nominal_boundary::CONTEXT_KEY] {
        let first = complete[0].build.frontend_options.get(key).unwrap();
        for fact in complete {
            assert!(std::ptr::eq(
                first,
                fact.build.frontend_options.get(key).unwrap()
            ));
        }
    }
    assert!(full["facts"][0]["build"]["frontend_options"]["invocation"]["value"].is_array());
    assert_eq!(
        full["facts"][0]["build"]["frontend_options"]["invocation"]["kind"],
        "inline"
    );
    assert_eq!(
        full["facts"][1]["build"]["frontend_options"]["invocation"]["kind"],
        "shared_reference"
    );
}

fn offline(directory: &Path, report: &Value, extra: &[&str]) -> Output {
    let mut options = vec!["clones", "--format", "json"];
    options.extend_from_slice(OPTIONS);
    if extra.contains(&"--report-detail") {
        options.truncate(options.len() - 2);
    }
    replay(directory, report, &options, extra)
}

fn replay(directory: &Path, report: &Value, options: &[&str], extra: &[&str]) -> Output {
    diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory)
            .args(options)
            .args(["--replay-run", report["run_directory"].as_str().unwrap()])
            .args(extra)
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RUSTC", directory.join("no-rustc"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}

fn root_replay(directory: &Path, original: &Value) {
    const ROOT: &[&str] = &["--format", "json"];
    let output = replay(directory, original, ROOT, &[]);
    checked(&output, 1);
    let restored: Value = serde_json::from_slice(&output.stdout).unwrap();
    for field in [
        "diagnostics",
        "summary",
        "functions",
        "fix_groups",
        "compiler_advisory",
        "unit_contexts",
        "observed_units",
        "build_contexts",
        "summary_complete",
        "planning_input_complete",
        "planning_required_edits_complete",
        "matching_execution",
        "strict_assessment_execution",
    ] {
        assert_eq!(original[field], restored[field], "{field}");
    }
    for extra in [
        &["-p", "fixture"][..],
        &["--all-targets"][..],
        &["--manifest-path", "Cargo.toml"][..],
    ] {
        let output = replay(directory, original, ROOT, extra);
        checked(&output, 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains("original selection"));
    }
    let missing = serde_json::json!({"run_directory":directory.join("missing-run")});
    checked(&replay(directory, &missing, ROOT, &[]), 2);

    std::fs::write(
        directory.join("relaxed.toml"),
        "[fn_params]\nmax_params=2\n",
    )
    .unwrap();
    let output = replay(directory, original, ROOT, &["--config", "relaxed.toml"]);
    checked(&output, 0);
    let relaxed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(relaxed["summary"]["functions"], 2);
    assert_eq!(relaxed["summary"]["violations"], 0);
    assert_eq!(relaxed["diagnostics"], serde_json::json!([]));
    assert_eq!(relaxed["summary_complete"], true);
}

fn projected_numeric(root: &Value, replay: &Value) {
    let evaluated = &replay["compiler_advisory"]["units"][0]["evaluation"];
    for field in ["diagnostics", "summary", "functions"] {
        assert_eq!(root[field], evaluated[field], "{field}");
    }
    let groups: Vec<_> = replay["compiler_advisory"]["planning"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|group| &group["evaluation"])
        .collect();
    assert_eq!(root["fix_groups"], serde_json::json!(groups));
    assert_eq!(root["compiler_advisory"], replay["compiler_advisory"]);
}

/// A root capture's documents hold every derivation over them, so a later run asks
/// for graphs and bounds without capturing again.
fn derived_from_the_documents(directory: &Path, original: &Value) {
    let output = offline(
        directory,
        original,
        &["--compiler-patterns", "--compiler-runtime-bounds"],
    );
    checked(&output, 1);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["capture_failures"], serde_json::json!([]));
    assert_eq!(report["handoff_failures"], serde_json::json!([]));
    let unit = &report["compiler_advisory"]["units"][0];
    assert_eq!(unit["patterns"]["input_complete"], true);
    assert_eq!(unit["patterns"]["observed_graphs"], 2);
    assert_eq!(unit["runtime_bounds"]["input_complete"], true);
    let rows = unit["runtime_bounds"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row["capture_status"] == "captured"));

    let output = replay(directory, original, &["patterns", "--format", "json"], &[]);
    checked(&output, 0);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        unit["patterns"]["evaluation"],
        report["compiler_advisory"]["units"][0]["patterns"]["evaluation"]
    );
}

fn unavailable_body_consumers(directory: &Path, original: &Value) {
    for extra in [&["--report-detail", "full"][..], &["--source-rules"][..]] {
        let output = offline(directory, original, extra);
        checked(&output, 2);
        let text = format!(
            "{}\n{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains("style-facts")
                || text.contains("compiler-style-inputs")
                || text.contains("style inputs")
                || text.contains("style-only")
                || text.contains("full body"),
            "missing unavailable-payload explanation: {text}"
        );
        if let Ok(report) = serde_json::from_slice::<Value>(&output.stdout) {
            assert!(
                report["handoff_failures"]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
                    || report["capture_complete"] == false,
                "missing explicit failure: {report}"
            );
        }
    }
}

fn numeric_baselines(directory: &Path, numeric: &Value, full: &Value) {
    for (original, file) in [(numeric, "numeric.json"), (full, "full.json")] {
        let output = offline(directory, original, &["--write-compiler-baseline", file]);
        checked(&output, 1);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report["compiler_advisory"]["diagnostic_baseline"]["coverage"],
            "complete"
        );
    }
    let read = |file| -> Value {
        serde_json::from_slice(&std::fs::read(directory.join(file)).unwrap()).unwrap()
    };
    assert_eq!(read("numeric.json"), read("full.json"));
    let output = offline(
        directory,
        numeric,
        &["--compiler-baseline", "full.json", "--fail-on-new-compiler"],
    );
    checked(&output, 1); // The unchanged --fail-on-findings still sees the raw finding.
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let baseline = &report["compiler_advisory"]["diagnostic_baseline"];
    assert_eq!(baseline["coverage"], "complete");
    assert_eq!(baseline["existing_findings"], 1);
    assert_eq!(baseline["new_findings"], 0);
    assert_eq!(baseline["scope_changed"], false);
}

fn numeric_reviews(directory: &Path, numeric: &Value, full: &Value) {
    let output = offline(
        directory,
        numeric,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            "0",
            "--review-reason",
            "Independently reviewed parameter fixture",
        ],
    );
    checked(&output, 0);
    for original in [numeric, full] {
        let output = offline(
            directory,
            original,
            &["--review-file", "reviews.json", "--review-status"],
        );
        checked(&output, 0);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let advisory = &report["compiler_advisory"];
        assert_eq!(
            advisory["diagnostic_review"]["decisions"][0]["state"],
            "hidden_exact"
        );
        assert_eq!(
            report["review_status"]["records"][0]["state"],
            "hidden_exact"
        );
        assert_eq!(
            advisory["units"][0]["visible_diagnostics"],
            serde_json::json!([])
        );
        assert_eq!(advisory["findings"], 0);
        let evaluation = &advisory["units"][0]["evaluation"];
        assert_eq!(evaluation["diagnostics"], numeric["diagnostics"]);
        assert_eq!(evaluation["summary"], numeric["summary"]);
        assert_eq!(report["matching_execution"], "not_requested");
        assert_eq!(report["strict_assessment_execution"], "not_requested");
    }
}

#[test]
fn root_numeric_payload_is_distinct_from_full_clones_and_replays_without_sources_or_tools() {
    let directory = fixture();
    let first = root_capture(directory.path());
    let first_documents = documents(&first);
    style_documents(&first_documents);
    let (output, full) = clones(directory.path(), OPTIONS);
    checked(&output, 1);
    complete_documents(&first_documents, &snapshot(&full));
    projected_numeric(&first, &full);
    let repeated = root_capture(directory.path());
    style_documents(&documents(&repeated));
    assert_ne!(first["run_directory"], repeated["run_directory"]);
    for field in ["diagnostics", "summary", "functions", "fix_groups"] {
        assert_eq!(first[field], repeated[field], "{field}");
    }
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let output = offline(directory.path(), &repeated, &[]);
    checked(&output, 1);
    let replay: Value = serde_json::from_slice(&output.stdout).unwrap();
    projected_numeric(&repeated, &replay);
    root_replay(directory.path(), &repeated);
    numeric_baselines(directory.path(), &repeated, &full);
    numeric_reviews(directory.path(), &repeated, &full);
    derived_from_the_documents(directory.path(), &repeated);
    unavailable_body_consumers(directory.path(), &repeated);
}
