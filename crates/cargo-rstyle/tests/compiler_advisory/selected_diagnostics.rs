use super::propagation::checked;
use super::*;

const DERIVED: &str = "#[derive(Debug)] pub struct Generated(pub u32);\n";
const SOURCE: &str = "pub fn mixed(a:u128,b:u128)->u128 {\nlet mut x=a;\nlet mut y=b;\nx+=1;\nx+=2;\ny+=3;\nif a>0 {if b>0 {x+=1;}}\nstd::fs::write(\"/unused\",b\"x\").ok();\nx+y\n}\n";
const POLICY: &str = "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\n[fn_spacing]\nmin_statements=2\n[complexity]\nmax_cyclomatic=1\nmax_cognitive=0\n[mutation_budget]\nmax_mutations_per_target=1\nmax_mutated_targets=1\n[side_effects]\nmax_density_milli=1\nmin_statements=0\nmin_lines=0\n";
const BASE: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-advisory",
    "--compiler-advisory-config",
    "policy.toml",
];
const AUTHORED: &[&str] = &["--compiler-diagnostic-scope", "authored"];
const PARAMS: &[&str] = &["--compiler-diagnostic-rules", "fn_params"];
const ALL: &str = "fn_length,fn_params,fn_spacing,cyclomatic_complexity,cognitive_complexity,mutation_per_target,mutated_targets,side_effect_density";

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("policy.toml"), POLICY).unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut flags = BASE.to_vec();
    flags.extend_from_slice(extra);
    clones(directory, &flags)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .arg("clones")
        .args(BASE)
        .args([
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
    let value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, value)
}

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}
fn scope(report: &Value) -> &Value {
    &unit(report)["diagnostic_scope"]
}
fn diagnostics(report: &Value) -> &Vec<Value> {
    unit(report)["evaluation"]["diagnostics"]
        .as_array()
        .unwrap()
}

fn selected(report: &Value, rules: &[&str]) {
    let indices: Vec<_> = diagnostics(report)
        .iter()
        .enumerate()
        .filter(|(_, row)| rules.contains(&row["rule_id"].as_str().unwrap()))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        scope(report)["diagnostic_indices"],
        serde_json::json!(indices)
    );
    assert_eq!(
        unit(report)["visible_diagnostics"],
        serde_json::json!(indices)
    );
    assert_eq!(report["compiler_advisory"]["findings"], indices.len());
}

fn unchanged(before: &Value, after: &Value) {
    for field in [
        "evaluation",
        "observations",
        "unavailable",
        "unavailable_complexity",
        "generated_effects",
        "unresolved_callees",
    ] {
        assert_eq!(unit(before)[field], unit(after)[field], "{field}");
    }
}

#[test]
fn requested_numeric_rules_preserve_full_default_evaluation_and_original_diagnostic_indices() {
    let directory = fixture(SOURCE);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert!(unit(&original).get("diagnostic_scope").is_none());
    let rules: std::collections::BTreeSet<_> = diagnostics(&original)
        .iter()
        .map(|row| row["rule_id"].as_str().unwrap())
        .collect();
    assert_eq!(rules, ALL.split(',').collect());
    let (output, captured) = replay(
        directory.path(),
        &original,
        &["--compiler-diagnostic-scope", "captured"],
    );
    checked(&output, 0);
    unchanged(&original, &captured);
    assert!(unit(&captured).get("diagnostic_scope").is_none());
    assert_eq!(captured["compiler_advisory"], original["compiler_advisory"]);
    assert_eq!(
        captured["compiler_advisory"]["findings"],
        original["compiler_advisory"]["findings"]
    );
    let (output, filtered) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-diagnostic-rules",
            "fn_params,fn_length",
            "--fail-on-findings",
        ],
    );
    checked(&output, 1);
    unchanged(&original, &filtered);
    assert_eq!(
        scope(&filtered)["request"]["rules"],
        serde_json::json!(["fn_length", "fn_params"])
    );
    selected(&filtered, &["fn_length", "fn_params"]);
    let (output, all) = replay(
        directory.path(),
        &original,
        &["--compiler-diagnostic-rules", ALL],
    );
    checked(&output, 0);
    unchanged(&original, &all);
    selected(&all, &ALL.split(',').collect::<Vec<_>>());
}

#[test]
fn authored_requested_diagnostics_complete_without_erasing_generated_missing_owners_and_replay_source_free()
 {
    let directory = fixture(&format!("{DERIVED}{SOURCE}"));
    let (output, original) = capture(directory.path(), PARAMS);
    checked(&output, 2);
    assert_eq!(scope(&original)["input_complete"], false);
    let (output, authored) = replay(directory.path(), &original, &[AUTHORED, PARAMS].concat());
    checked(&output, 0);
    unchanged(&original, &authored);
    assert_eq!(scope(&authored)["input_complete"], true);
    assert_eq!(scope(&authored)["effect_inputs_complete"], true);
    assert_eq!(authored["compiler_advisory"]["complete"], true);
    assert_eq!(scope(&authored)["unavailable_owners"], 0);
    let outside = scope(&authored)["source_scope"]["outside_scope_functions"]
        .as_array()
        .unwrap();
    assert!(!outside.is_empty());
    let missing: Vec<_> = unit(&original)["unavailable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["key"].clone())
        .collect();
    assert_eq!(outside, &missing);
    selected(&authored, &["fn_params"]);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, saved) = replay(directory.path(), &original, &[AUTHORED, PARAMS].concat());
    checked(&output, 0);
    assert_eq!(saved["compiler_advisory"], authored["compiler_advisory"]);
}

fn raw_wire(report: &Value) -> Value {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn damage(wire: &Value, kind: &str) -> Value {
    let mut changed = wire.clone();
    for fact in changed["facts"].as_array_mut().unwrap() {
        if fact["name"] == "mixed" {
            let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
            if kind == "score" {
                options[rstyle_core::advisory::KEY]["facts"]["complexity"] = Value::Null;
            }
            if kind == "observation" {
                options.remove(rstyle_core::advisory::KEY);
            }
        } else if kind == "origin" {
            fact["source_diagnostics"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row["construct"] != "macro_generated_function");
            let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
            options.remove(rstyle_core::advisory::generated_effects::KEY);
            options.remove(rstyle_core::advisory::generated_effects::CONNECTED_KEY);
        }
    }
    changed
}

#[test]
fn requested_rules_require_only_their_measurements_and_never_hide_unknown_authored_inputs() {
    let directory = fixture(&format!("{DERIVED}{SOURCE}"));
    let (output, original) = capture(directory.path(), &[AUTHORED, PARAMS].concat());
    checked(&output, 0);
    let wire = raw_wire(&original);
    let ledger = only::ledger_path(&original);
    for kind in ["score", "observation", "origin"] {
        only::reseal_raw(&original, &ledger, &damage(&wire, kind));
        let (output, report) = replay(directory.path(), &original, &[AUTHORED, PARAMS].concat());
        checked(&output, if kind == "score" { 0 } else { 2 });
        assert_eq!(scope(&report)["input_complete"], kind == "score");
        if kind == "score" {
            let (output, complexity) = replay(
                directory.path(),
                &original,
                &[
                    AUTHORED,
                    &["--compiler-diagnostic-rules", "cyclomatic_complexity"],
                ]
                .concat(),
            );
            checked(&output, 2);
            assert_eq!(scope(&complexity)["unavailable_measurements"], 1);
            assert_eq!(scope(&complexity)["input_complete"], false);
        } else {
            assert_eq!(scope(&report)["unavailable_owners"], 1);
            if kind == "origin" {
                assert!(
                    scope(&report)["source_scope"]["outside_scope_functions"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
            }
        }
    }
    only::reseal_raw(&original, &ledger, &wire);
}

#[test]
fn requested_diagnostics_seed_existing_plans_without_changing_completed_metrics() {
    let directory = fixture(SOURCE);
    let (output, original) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    let (output, selected_report) = replay(
        directory.path(),
        &original,
        &[PARAMS, &["--compiler-plans"]].concat(),
    );
    checked(&output, 0);
    unchanged(&original, &selected_report);
    selected(&selected_report, &["fn_params"]);
    let planning = &selected_report["compiler_advisory"]["planning"];
    assert_eq!(planning["input_complete"], true);
    let groups = planning["groups"].as_array().unwrap();
    assert!(!groups.is_empty());
    for group in groups {
        for function in group["evaluation"]["functions"].as_array().unwrap() {
            assert_eq!(function["rules"], serde_json::json!(["fn_params"]));
        }
    }
    std::fs::write(
        directory.path().join("policy.toml"),
        POLICY.replace("max_params=1", "max_params=100"),
    )
    .unwrap();
    let (output, no_seed) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-plans",
            "--compiler-diagnostic-rules",
            "fn_params",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        no_seed["compiler_advisory"]["planning"]["groups"],
        serde_json::json!([])
    );
    assert_eq!(no_seed["compiler_advisory"]["findings"], 0);
    assert!(!diagnostics(&no_seed).is_empty());
    assert_eq!(
        unit(&no_seed)["observations"],
        unit(&original)["observations"]
    );
}

fn select(directory: &Path, paths: &[&str]) {
    std::fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&serde_json::json!({"schema":"rstyle.changed-files/1", "paths":paths}))
            .unwrap(),
    )
    .unwrap();
}

#[test]
fn requested_numeric_rules_intersect_changed_paths_and_keep_source_suppressions_and_invalid_errors()
{
    let directory = fixture("pub mod first; pub mod second;\n");
    std::fs::write(directory.path().join("src/first.rs"), SOURCE).unwrap();
    std::fs::write(directory.path().join("src/second.rs"), SOURCE).unwrap();
    let (output, original) = capture(directory.path(), PARAMS);
    checked(&output, 0);
    select(directory.path(), &["src/first.rs"]);
    let flags = [
        PARAMS,
        &["--changed-files", "selection.json", "--fail-on-findings"],
    ]
    .concat();
    let (output, changed) = replay(directory.path(), &original, &flags);
    checked(&output, 1);
    unchanged(&original, &changed);
    assert_eq!(
        changed["compiler_advisory"]["changed_scope"]["visible_findings"],
        1
    );
    assert_eq!(
        unit(&changed)["selected_diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    suppression_controls(directory.path(), &flags);
}

fn suppression_controls(directory: &Path, flags: &[&str]) {
    std::fs::write(
        directory.join("src/first.rs"),
        format!("// rstyle-allow(fn_params): deliberate boundary\n{SOURCE}"),
    )
    .unwrap();
    let (output, suppressed) = capture(directory, flags);
    checked(&output, 0);
    assert_eq!(
        suppressed["compiler_advisory"]["changed_scope"]["visible_findings"],
        0
    );
    std::fs::write(
        directory.join("src/second.rs"),
        format!("// rstyle-allow(unknown): malformed control\n{SOURCE}"),
    )
    .unwrap();
    let (output, invalid) = capture(directory, flags);
    checked(&output, 1);
    let indices = unit(&invalid)["selected_diagnostics"].as_array().unwrap();
    assert!(indices.iter().any(
        |index| diagnostics(&invalid)[index.as_u64().unwrap() as usize]["rule_id"]
            == "invalid_suppression"
    ));
}

#[test]
fn combined_numeric_and_pattern_changed_scope_excludes_only_shared_outside_owners() {
    let directory = fixture("pub mod generated; pub fn original(a:u128,b:u128)->u128 {a+b}\n");
    std::fs::write(directory.path().join("src/generated.rs"), DERIVED).unwrap();
    select(directory.path(), &["src/generated.rs"]);
    let flags = [
        AUTHORED,
        PARAMS,
        &[
            "--compiler-patterns",
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ],
    ]
    .concat();
    let (output, captured_patterns) = capture(directory.path(), &flags);
    checked(&output, 2);
    assert_eq!(scope(&captured_patterns)["input_complete"], true);
    assert_eq!(
        unit(&captured_patterns)["patterns"]["input_complete"],
        false
    );
    let path = &captured_patterns["compiler_advisory"]["changed_scope"]["paths"][0];
    assert_eq!(path["coverage"], "incomplete");
    assert_eq!(path["outside_scope_functions"].as_u64().unwrap_or(0), 0);
    let flags = [flags.as_slice(), &["--compiler-pattern-scope", "authored"]].concat();
    let (output, authored_both) = replay(directory.path(), &captured_patterns, &flags);
    checked(&output, 0);
    assert_eq!(unit(&authored_both)["patterns"]["input_complete"], true);
    let path = &authored_both["compiler_advisory"]["changed_scope"]["paths"][0];
    assert_eq!(path["coverage"], "complete");
    assert!(path["outside_scope_functions"].as_u64().unwrap() > 0);
    unchanged(&captured_patterns, &authored_both);
}

#[test]
fn diagnostic_selection_flags_require_numeric_advice_and_known_rule_names_before_capture() {
    let directory = fixture(SOURCE);
    for flags in [
        vec!["--compiler-diagnostic-rules", "fn_params"],
        vec!["--compiler-diagnostic-scope", "authored"],
        vec!["--compiler-diagnostic-scope", "captured"],
        vec![
            "--compiler-advisory",
            "--compiler-diagnostic-rules",
            "fn_param",
        ],
    ] {
        let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
        command
            .current_dir(directory.path())
            .arg("clones")
            .args(&flags)
            .env("PATH", directory.path().join("no-tools"))
            .env("CARGO", directory.path().join("no-cargo"));
        let output = diagnostic_process::output(&mut command).unwrap();
        checked(&output, 2);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("no-cargo"), "{error}");
        assert!(!directory.path().join("target/rstyle/clones").exists());
    }
}
