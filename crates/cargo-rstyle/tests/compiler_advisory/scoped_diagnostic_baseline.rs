use super::propagation::checked;
use super::*;

const AUTHORED: &[&str] = &["--compiler-diagnostic-scope", "authored"];
const RULES: &[&str] = &["--compiler-diagnostic-rules", "fn_params,fn_length"];

fn baseline(report: &Value) -> &Value {
    &report["compiler_advisory"]["diagnostic_baseline"]
}

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory-only",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

fn saved(directory: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(directory.join("baseline.json")).unwrap()).unwrap()
}

#[test]
fn authored_numeric_baseline_replays_selected_core_findings_without_effect_completeness() {
    let (directory, original) = generated_caller();
    let exported = export_selected(directory.path(), &original);
    assert_selected_measurements(directory.path(), &original, &exported);
    replay_selected(directory.path(), &original);
}

fn generated_caller() -> (TempDir, Value) {
    let directory = propagation::workspace("pub fn leaf(){}");
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "macro_rules! make {()=>{pub fn generated(){}}} make!();\n\
         pub fn caller(a:u32,b:u32){\nlet mut x=a;\nx+=1;\nx+=2;\nrenamed::leaf();\nlet _=(x,b);\n}",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("policy.toml"),
        "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\n[fn_spacing]\nmin_statements=2\n",
    )
    .unwrap();
    let (output, original) = propagation::capture(
        directory.path(),
        &[
            "--package",
            "caller",
            "--compiler-advisory-only",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    checked(&output, 2);
    assert!(
        !unit(&original)["unavailable"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        unit(&original)["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    (directory, original)
}

fn export_selected(directory: &Path, original: &Value) -> Value {
    let export = [
        AUTHORED,
        RULES,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--write-compiler-baseline",
            "baseline.json",
        ],
    ]
    .concat();
    let (output, exported) = replay(directory, original, &export);
    checked(&output, 0);
    assert_eq!(baseline(&exported)["coverage"], "complete");
    exported
}

fn assert_selected_measurements(directory: &Path, original: &Value, exported: &Value) {
    let stored = saved(directory);
    assert_eq!(
        stored["request"],
        serde_json::json!({
            "rules": ["fn_length", "fn_params"], "source_scope": "authored"
        })
    );
    let entries = stored["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        let diagnostic = unit(original)["evaluation"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["rule_id"] == entry["rule_id"])
            .unwrap();
        assert_eq!(entry["actual"], diagnostic["actual"]);
        assert_eq!(entry["limit"], diagnostic["limit"]);
    }
    assert!(
        unit(original)["evaluation"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["rule_id"] == "fn_spacing")
    );
    for field in [
        "evaluation",
        "observations",
        "unavailable",
        "unresolved_callees",
        "generated_effects",
    ] {
        assert_eq!(unit(exported)[field], unit(original)[field], "{field}");
    }
}

fn replay_selected(directory: &Path, original: &Value) {
    let baseline_bytes = std::fs::read(directory.join("baseline.json")).unwrap();
    for path in ["caller/src/lib.rs", "caller/Cargo.toml", "Cargo.toml"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    let compare = [
        AUTHORED,
        RULES,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
        ],
    ]
    .concat();
    let (output, current) = replay(directory, original, &compare);
    checked(&output, 0);
    assert_eq!(baseline(&current)["coverage"], "complete");
    assert_eq!(baseline(&current)["new_findings"], 0);
    assert_eq!(baseline(&current)["existing_findings"], 2);
    assert_eq!(
        std::fs::read(directory.join("baseline.json")).unwrap(),
        baseline_bytes
    );
}

#[test]
fn empty_numeric_baseline_detects_rule_and_population_drift() {
    let (directory, output, original) = capture(
        "pub fn empty(){}",
        &[
            AUTHORED,
            RULES,
            &[
                "--compiler-advisory-only",
                "--write-compiler-baseline",
                "baseline.json",
            ],
        ]
        .concat(),
    );
    checked(&output, 0);
    assert_eq!(saved(directory.path())["entries"], serde_json::json!([]));
    let compare = &[
        "--compiler-baseline",
        "baseline.json",
        "--fail-on-new-compiler",
    ];
    for extra in [
        [
            AUTHORED,
            &["--compiler-diagnostic-rules", "fn_length"],
            compare,
        ]
        .concat(),
        [RULES, compare].concat(),
    ] {
        let (output, current) = replay(directory.path(), &original, &extra);
        checked(&output, 1);
        assert_eq!(baseline(&current)["scope_changed"], true);
        assert_eq!(baseline(&current)["new_findings"], 0);
        assert_eq!(baseline(&current)["coverage"], "complete");
    }
    let (output, same) = replay(
        directory.path(),
        &original,
        &[AUTHORED, RULES, compare].concat(),
    );
    checked(&output, 0);
    assert_eq!(baseline(&same)["scope_changed"], false);
}

#[test]
fn unrequested_retained_numeric_rule_cannot_become_a_negative() {
    let (directory, output, original) = capture(
        "pub fn many(a:u32,b:u32,c:u32,d:u32,e:u32){}",
        &[
            "--compiler-advisory-only",
            "--compiler-diagnostic-rules",
            "fn_params",
            "--write-compiler-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        saved(directory.path())["entries"].as_array().unwrap().len(),
        1
    );
    let bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
    let (output, current) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-diagnostic-rules",
            "fn_length",
            "--compiler-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    let retained = baseline(&current)["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0]["state"], "unselected");
    assert_eq!(baseline(&current)["new_findings"], 0);
    assert_eq!(baseline(&current)["scope_changed"], true);
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        bytes
    );
}
