use super::*;

fn workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"clone_fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let body = "let next = value; ".repeat(50);
    std::fs::write(directory.path().join("src/lib.rs"), format!(
        "pub fn large(value: u32, spare: u32) -> u32 {{ {body} value }}\npub async fn asynchronous(value: u32) -> u32 {{ value }}\n"
    )).unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_params]\nmax_params=1\n[fn_spacing]\n",
    )
    .unwrap();
    directory
}

fn assert_independent(report: &Value) {
    assert_eq!(
        report["analysis"]["execution"], "budget_exhausted",
        "{report}"
    );
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(guard(report, "large")["execution"], "budget_exhausted");
    let function = report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"].as_str().unwrap().ends_with("large"))
        .unwrap();
    let assessments = function["assessments"].as_array().unwrap();
    let arity = assessments
        .iter()
        .find(|item| item["rule_id"] == "fn_params")
        .unwrap();
    assert_eq!(arity["outcome"], "finding");
    assert_eq!(arity["actual"], 2);
    let spacing = assessments
        .iter()
        .find(|item| item["rule_id"] == "fn_spacing")
        .unwrap();
    assert_eq!(spacing["outcome"], "not_evaluated");
    assert_eq!(spacing["availability"], "unknown");
}

fn replay(directory: &Path, original: &Value, detail: &str) -> Value {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--source-rules",
            "--guard-clauses",
            "--analysis-config",
            "analysis.toml",
            "--report-detail",
            detail,
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn bounded_bodies_keep_parameter_findings_through_source_free_strict_replay() {
    let directory = workspace();
    let (output, original) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--locked",
            "--source-rules",
            "--guard-clauses",
            "--analysis-config",
            "analysis.toml",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_independent(&original);
    for file in ["Cargo.toml", "src/lib.rs"] {
        std::fs::remove_file(directory.path().join(file)).unwrap();
    }
    for detail in ["full", "compact"] {
        let replayed = replay(directory.path(), &original, detail);
        assert_eq!(replayed["verification_policy"], "strict-replay");
        assert_independent(&replayed);
        assert_eq!(original["analysis"], replayed["analysis"]);
        assert_eq!(original["matching"], replayed["matching"]);
    }
}

#[test]
fn header_only_native_async_and_opaque_bodies_keep_source_rules_available() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--locked",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
        ],
    );
    assert!(
        output.status.success(),
        "{report}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["analysis"]["execution"], "completed");
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    for function in report["analysis"]["functions"].as_array().unwrap() {
        for assessment in function["assessments"].as_array().unwrap() {
            assert_eq!(assessment["availability"], "known", "{function}");
        }
    }
}
