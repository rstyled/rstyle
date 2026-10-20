//! End-to-end tests: real `cargo check` through the real rustc driver.

mod byte_windows;
#[path = "../src/diagnostic_process.rs"]
mod diagnostic_process;
mod loop_merge_target;
mod shared_evidence;
use shared_evidence::{load as data_unit, reject_helper};

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// `cargo test -p cargo-rstyle` does not build sibling binaries, so build the driver once.
fn driver() -> &'static Path {
    static DRIVER: OnceLock<PathBuf> = OnceLock::new();
    DRIVER.get_or_init(|| {
        if let Some(path) = std::env::var_os("RSTYLE_TEST_DRIVER") {
            let path = PathBuf::from(path);
            assert!(
                path.is_file(),
                "prebuilt test driver is missing: {}",
                path.display()
            );
            return path;
        }
        let status = Command::new("cargo")
            .args(["build", "--quiet", "-p", "rstyle-driver"])
            .current_dir(workspace_root())
            .status()
            .unwrap();
        assert!(status.success(), "building rstyle-driver failed");
        cargo_bin("cargo-rstyle").with_file_name("rstyle-driver")
    })
}

/// Copies a fixture into a fresh temp dir so runs never share build state.
fn fixture(name: &str) -> TempDir {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&source, dir.path());
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn rstyle(dir: &Path, args: &[&str]) -> Output {
    diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args(args)
            .current_dir(dir)
            .env("RSTYLE_DRIVER", driver())
            // Fixtures assert on `<fixture>/target/rstyle/facts`; a shared/inherited
            // CARGO_TARGET_DIR (worktree cache, CI volume) would move it.
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn json(dir: &Path) -> Value {
    let output = rstyle(dir, &["--format", "json"]);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "bad json ({e}): {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn diagnostics_for<'a>(report: &'a Value, function: &str) -> Vec<&'a Value> {
    let all = report["diagnostics"].as_array().unwrap();
    all.iter()
        .filter(|d| {
            d["message"]
                .as_str()
                .unwrap()
                .contains(&format!("`{function}`"))
        })
        .collect()
}

fn rules_for(report: &Value, function: &str) -> Vec<String> {
    let found = diagnostics_for(report, function);
    found
        .iter()
        .map(|d| d["rule_id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn clean_crate_exits_zero() {
    let dir = fixture("clean");
    let output = rstyle(dir.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout(&output).contains("0 violations"));
    let report = json(dir.path());
    assert!(report["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(report["summary"]["functions"], 4);
}

#[test]
fn violations_exit_one_and_match_golden_output() {
    let dir = fixture("violations");
    let output = rstyle(dir.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    insta::assert_snapshot!(stdout(&output));
}

#[test]
fn json_output_reports_resolved_mutations_and_effects() {
    let dir = fixture("violations");
    let report = json(dir.path());
    // 4 mutations over 4 statements is also a density of 1000, above the default 500.
    assert_eq!(
        rules_for(&report, "Account::churn"),
        ["mutation_per_target", "side_effect_density"]
    );
    let churn = diagnostics_for(&report, "Account::churn")[0];
    // `self.bump()` is an autoref'd `&mut self` call: seen by HIR, invisible to name heuristics.
    assert_eq!(churn["details"][0]["label"], "self");
    assert_eq!(churn["details"][0]["count"], 4);
    assert!(rules_for(&report, "spread").contains(&"mutated_targets".to_string()));
    assert!(rules_for(&report, "too_many").contains(&"fn_params".to_string()));
    assert!(rules_for(&report, "very_long").contains(&"fn_length".to_string()));
    assert!(rules_for(&report, "Account::peek").is_empty());
    assert!(rules_for(&report, "pure").is_empty());
}

#[test]
fn effects_are_classified_from_resolved_paths() {
    let dir = fixture("violations");
    let report = json(dir.path());
    let labels = |function: &str| -> Vec<String> {
        diagnostics_for(&report, function)
            .iter()
            .filter(|d| d["rule_id"] == "side_effect_density")
            .flat_map(|d| d["details"].as_array().unwrap().clone())
            .map(|d| d["label"].as_str().unwrap().to_string())
            .collect()
    };
    let noisy = labels("noisy");
    assert!(
        noisy.contains(&"io: std::fs::write".to_string()),
        "{noisy:?}"
    );
    assert!(
        noisy.iter().any(|l| l.starts_with("io: std::io::")),
        "{noisy:?}"
    );
    let dirty = labels("dirty");
    assert!(
        dirty.contains(&"unsafe: unsafe block".to_string()),
        "{dirty:?}"
    );
    assert!(
        dirty.iter().any(|l| l.starts_with("global: static mut")),
        "{dirty:?}"
    );
    assert!(
        dirty.contains(&"unknown: <fn pointer>".to_string()),
        "{dirty:?}"
    );
}

#[test]
fn suppressions_apply_only_with_a_reason() {
    let report = json(fixture("violations").path());
    assert!(rules_for(&report, "allowed").is_empty());
    let missing = rules_for(&report, "missing_reason");
    assert!(missing.contains(&"fn_params".to_string()), "{missing:?}");
    assert!(
        missing.contains(&"invalid_suppression".to_string()),
        "{missing:?}"
    );
}

#[test]
fn derived_code_is_not_analyzed() {
    let report = json(fixture("violations").path());
    let summary = report["summary"]["functions"].as_u64().unwrap();
    // 11 hand-written fns; derive(Debug, Clone, PartialEq) would add more if leaked.
    assert_eq!(summary, 11);
}

#[test]
fn two_runs_are_byte_identical() {
    let dir = fixture("violations");
    let first = rstyle(dir.path(), &["--format", "json"]);
    let second = rstyle(dir.path(), &["--format", "json"]);
    assert_eq!(first.stdout, second.stdout);
    assert!(!first.stdout.is_empty());
}

#[test]
fn fn_spacing_is_configurable() {
    let dir = fixture("violations");
    let off = json(dir.path());
    assert!(!rules_for(&off, "noisy").contains(&"fn_spacing".to_string()));
    std::fs::write(
        dir.path().join("rstyle.toml"),
        "[fn_spacing]\nmin_statements = 3\n",
    )
    .unwrap();
    let on = json(dir.path());
    assert!(rules_for(&on, "noisy").contains(&"fn_spacing".to_string()));
}

#[test]
fn config_change_reevaluates_without_recompiling() {
    let dir = fixture("violations");
    let before = json(dir.path());
    let facts = dir.path().join("target/rstyle/facts/violations-lib.json");
    let modified = std::fs::metadata(&facts).unwrap().modified().unwrap();
    std::fs::write(
        dir.path().join("rstyle.toml"),
        "[fn_params]\nmax_params = 10\n",
    )
    .unwrap();
    let after = json(dir.path());
    assert!(rules_for(&before, "too_many").contains(&"fn_params".to_string()));
    assert!(!rules_for(&after, "too_many").contains(&"fn_params".to_string()));
    assert_eq!(
        std::fs::metadata(&facts).unwrap().modified().unwrap(),
        modified
    );
}

#[test]
fn compile_errors_exit_two() {
    let dir = fixture("broken");
    let output = rstyle(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cargo check failed"));
}

#[test]
fn bad_config_exits_two() {
    let dir = fixture("clean");
    std::fs::write(dir.path().join("rstyle.toml"), "typo = 1\n").unwrap();
    let output = rstyle(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("rstyle.toml"));
}

#[test]
fn this_workspace_passes_its_own_rules() {
    let output = rstyle(&workspace_root(), &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unknown_package_exits_two() {
    let dir = fixture("clean");
    let output = rstyle(dir.path(), &["-p", "nope"]);
    assert_eq!(output.status.code(), Some(2));
}

fn facts_for(dir: &Path, file: &str) -> Vec<Value> {
    let path = dir.join("target/rstyle/facts").join(file);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn fact<'a>(facts: &'a [Value], name: &str) -> &'a Value {
    facts
        .iter()
        .find(|f| f["name"].as_str().unwrap().ends_with(name))
        .unwrap_or_else(|| panic!("no fn {name}"))
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect()
}

#[test]
fn facts_record_call_graph_ids_and_mutated_types() {
    let dir = fixture("graph");
    assert!(rstyle(dir.path(), &[]).status.success());
    let facts = facts_for(dir.path(), "graph-lib.json");

    let direct = fact(&facts, "direct");
    assert_eq!(direct["id"], "graph::direct");
    let edges: Vec<&str> = direct["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["callee"].as_str().unwrap())
        .collect();
    // Same-crate callee and the trait method; no std calls like `Vec::new`.
    assert_eq!(edges, ["graph::side", "graph::Shape::area"]);
    assert_eq!(strings(&direct["mutated_types"]), ["graph::Counter"]);

    // Trait impl methods point back at the trait method they implement.
    let area = facts
        .iter()
        .find(|f| f["implements"] == "graph::Shape::area")
        .expect("impl of Shape::area");
    assert_eq!(area["edges"][0]["callee"], "graph::side");

    // `dyn` calls are flagged, with the trait method as target.
    let total = fact(&facts, "total");
    assert_eq!(total["edges"][0]["callee"], "graph::Shape::area");
    assert_eq!(total["edges"][0]["dynamic"], true);
}

#[test]
fn facts_record_params_and_argument_provenance() {
    let dir = fixture("params");
    assert!(rstyle(dir.path(), &[]).status.code().is_some());
    let facts = facts_for(dir.path(), "params-lib.json");

    let leaf = fact(&facts, "leaf");
    let named: Vec<(&str, &str)> = leaf["param_list"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["type"].as_str().unwrap()))
        .collect();
    assert_eq!(
        named,
        [
            ("root", "&str"),
            ("verbose", "bool"),
            ("depth", "u32"),
            ("tag", "u8"),
            ("extra", "u8")
        ]
    );

    // `mid` only forwards its first three parameters; it also reads `opts`.
    let mid = fact(&facts, "mid");
    let uses: Vec<u64> = mid["param_list"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["uses"].as_u64().unwrap())
        .collect();
    assert_eq!(uses, [1, 1, 1, 1]);
    let args = &mid["edges"][0]["args"];
    assert_eq!(
        args,
        &serde_json::json!([{"param": 0}, {"param": 1}, {"param": 2}, "const", "field"])
    );

    let top_a = fact(&facts, "top_a");
    assert_eq!(
        top_a["edges"][0]["args"],
        serde_json::json!(["const", "const", "const", {"param": 0}])
    );
}

#[test]
fn effects_propagate_through_calls_and_stop_at_boundaries() {
    let dir = fixture("transitive");
    let report = json(dir.path());
    let top = diagnostics_for(&report, "top");
    assert_eq!(top.len(), 1, "{report}");
    assert_eq!(top[0]["actual"], 750); // 3 (io, two hops up) * 1000 / 4 statements
    let label = top[0]["details"][0]["label"].as_str().unwrap();
    assert_eq!(label, "io: std::fs::read_to_string (via mid)");
    // `leaf` and `mid` are direct/one hop IO, `via_boundary` sits behind a declared boundary.
    assert!(!rules_for(&report, "leaf").is_empty());
    assert!(!rules_for(&report, "mid").is_empty());
    assert!(rules_for(&report, "via_boundary").is_empty(), "{report}");
    assert!(rules_for(&report, "boundary").is_empty());

    let off = dir.path().join("rstyle.toml");
    std::fs::write(
        &off,
        "[fn_spacing]\nmin_statements = 0\n[side_effects]\nmin_lines = 0\nmin_statements = 1\ntransitive_depth = 0\n",
    )
    .unwrap();
    let report = json(dir.path());
    assert!(
        rules_for(&report, "top").is_empty(),
        "depth 0 disables propagation"
    );
}

#[test]
fn fix_groups_stack_signature_changes_and_leave_the_rest_independent() {
    let dir = fixture("groups");
    let report = json(dir.path());
    let groups = report["fix_groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2, "{report}");

    // `caller` calls `callee`, so changing `callee`'s signature edits `caller`:
    // one group, callee first. `lonely` has no callers and stands alone.
    let names = |g: &Value| -> Vec<String> {
        g["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(names(&groups[0]), ["callee", "caller"]);
    assert_eq!(names(&groups[1]), ["lonely"]);
    assert_eq!(groups[0]["functions"][0]["rules"][0], "fn_params");

    let text = stdout(&rstyle(dir.path(), &[]));
    assert!(
        text.contains("fix groups: 2 (1 stacked, 1 independent)"),
        "{text}"
    );
    assert!(text.contains("group 1 (2 functions"), "{text}");
}

#[test]
fn param_plan_orders_callee_then_callers_with_bundle_and_pass_through() {
    let dir = fixture("params");
    let report = json(dir.path());
    let group = &report["fix_groups"][0];
    let plan = &group["plan"];
    let steps: Vec<(&str, bool, u64)> = plan["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["name"].as_str().unwrap(),
                s["signature"].as_bool().unwrap(),
                s["incoming_sites"].as_u64().unwrap(),
            )
        })
        .collect();
    // `mid` (4 params, not a violation) must follow `leaf`; `top_a`/`top_b` call `mid`, not `leaf`.
    assert_eq!(steps, [("leaf", true, 1), ("mid", false, 2)], "{plan}");
    assert_eq!(
        strings(&plan["steps"][1]["pass_through"]),
        ["root", "verbose", "depth"]
    );
    let bundle = &plan["bundles"][0];
    assert_eq!(bundle["sites"], 1);
    assert_eq!(
        bundle["params"][0],
        serde_json::json!({"name": "depth", "type": "u32"})
    );
    assert_eq!(bundle["params"].as_array().unwrap().len(), 3);

    let text = stdout(&rstyle(dir.path(), &[]));
    assert!(text.contains("1. signature leaf"), "{text}");
    assert!(text.contains("2. call sites mid"), "{text}");
    assert!(text.contains("bundle (1 sites)"), "{text}");
}

fn sig_of<'a>(facts: &'a [Value], name: &str) -> &'a Value {
    &fact(facts, name)["sig"]
}

#[test]
fn facts_record_sig() {
    let dir = fixture("animals");
    assert!(rstyle(dir.path(), &[]).status.code().is_some());
    let facts = facts_for(dir.path(), "animals-lib.json");

    let bark = sig_of(&facts, "Dog::bark");
    assert_eq!(bark["self_ty"], "animals::Dog");
    assert_eq!(bark["receiver"], "ref");
    assert_eq!(bark["ret"]["adt"]["id"], "alloc::string::String");

    // `Dog::chase(&self, &Cat) -> Dog`: raw shapes, no normalization in the driver.
    let chase = sig_of(&facts, "Dog::chase");
    assert_eq!(
        chase["params"][0]["ref"]["inner"]["adt"]["id"],
        "animals::Cat"
    );
    assert_eq!(chase["ret"]["adt"]["id"], "animals::Dog");

    assert_eq!(sig_of(&facts, "Dog::new")["receiver"], "none");
    assert_eq!(
        sig_of(&facts, "Dog::name")["ret"]["ref"]["inner"],
        serde_json::json!({"prim": "str"})
    );
    // Trait impls keep their self type and link to the trait method.
    assert_eq!(
        sig_of(&facts, "<Cow as Speak>::speak")["self_ty"],
        "animals::Cow"
    );
    assert!(fact(&facts, "<Cow as Speak>::speak")["implements"].is_string());
    // Free functions have no signature facts.
    assert!(fact(&facts, "emit_dogs")["sig"].is_null());
}

#[test]
fn patterns_lists_signature_groups_with_spans() {
    let dir = fixture("animals");
    let output = rstyle(dir.path(), &["patterns", "--format", "json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let groups = report["signature_groups"].as_array().unwrap();
    let find = |key: &str| {
        groups
            .iter()
            .find(|g| g["key"] == key)
            .unwrap_or_else(|| panic!("no group {key}"))
    };

    let strings = find("fn(&Self) -> String");
    let names: Vec<&str> = strings["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["Dog::bark", "Cat::meow", "Rock::label", "Pebble::tag"]
    );
    assert_eq!(strings["traited"].as_array().unwrap().len(), 2);
    assert!(strings["members"][0]["line"].as_u64().unwrap() > 0);

    assert_eq!(find("fn(&Self) -> &str")["same_name"], true);
    assert_eq!(find("fn(&Self, &A) -> Self")["self_types"], 2);

    let text = stdout(&rstyle(dir.path(), &["patterns"]));
    assert!(text.contains("fn(&Self) -> String"), "{text}");
    assert!(text.contains("src/lib.rs:"), "{text}");
}

#[test]
fn patterns_group_impls_that_differ_only_in_concrete_type_arguments() {
    let dir = fixture("animals");
    let report = patterns_json(dir.path(), &["--min-score", "0"]);
    let groups = report["signature_groups"].as_array().unwrap();
    let g = groups
        .iter()
        .find(|g| g["key"] == "fn(&Self) -> u32")
        .expect("Wrapper<u8> / Wrapper<u16> group");
    assert_eq!(g["self_types"], 2);
    assert_eq!(g["members"].as_array().unwrap().len(), 2);
}

fn patterns_json(dir: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["patterns", "--format", "json"];
    args.extend(extra);
    let output = rstyle(dir, &args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn site_names(item: &Value, field: &str) -> Vec<String> {
    let sites = item[field].as_array().unwrap();
    sites
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect()
}

/// The candidate that has a caller named `caller` among its sites.
fn candidate_at<'a>(report: &'a Value, caller: &str) -> Option<&'a Value> {
    let all = report["candidates"].as_array().unwrap();
    all.iter()
        .find(|c| site_names(c, "sites").iter().any(|n| n == caller))
}

#[test]
fn patterns_for_loops_and_for_each_form_one_trait_candidate() {
    let dir = fixture("animals");
    let report = patterns_json(dir.path(), &[]);
    let c = candidate_at(&report, "emit_dogs").expect("dog/cat loop candidate");
    assert_eq!(c["kind"], "trait_method");
    // `for` loops and the `iter().for_each` variant align into one cluster.
    assert_eq!(
        site_names(c, "sites"),
        ["emit_dogs", "emit_cats", "emit_cats_each"]
    );
    assert_eq!(site_names(c, "definitions"), ["Dog::bark", "Cat::meow"]);
    assert_eq!(c["breakdown"]["support"], 3);
    assert_eq!(c["breakdown"]["coverage_milli"], 1000);
    assert!(
        c["observation"]
            .as_str()
            .unwrap()
            .contains("M0 = Dog::bark | Cat::meow")
    );
    assert!(c["sites"][0]["line"].as_u64().unwrap() > 0);
}

fn observation_of(report: &Value, caller: &str) -> String {
    let c = candidate_at(report, caller).expect("candidate");
    c["observation"].as_str().unwrap().to_string()
}

#[test]
fn patterns_literal_holes_show_whole_values() {
    let dir = fixture("literals");
    let report = patterns_json(dir.path(), &[]);
    let seen = observation_of(&report, "tag_dog");
    assert!(seen.contains(r#"L0 = "alpha" | "beta""#), "{seen}");
}

#[test]
fn patterns_macro_literals_never_show_partial_tokens() {
    let dir = fixture("literals");
    let report = patterns_json(dir.path(), &[]);
    let seen = observation_of(&report, "fmt_dog");
    // Inside macros the span covers the call: such holes must be absent, not garbled.
    assert!(!seen.contains("format!"), "{seen}");
    assert!(!seen.contains("L0 = "), "{seen}");
}

#[test]
fn patterns_enum_dispatch_is_a_candidate() {
    let dir = fixture("animals");
    let report = patterns_json(dir.path(), &[]);
    let c = candidate_at(&report, "pet_sound").expect("dispatch candidate");
    assert_eq!(c["kind"], "enum_dispatch");
    assert_eq!(site_names(c, "definitions"), ["Dog::bark", "Cat::meow"]);
}

#[test]
fn patterns_getters_never_become_candidates_and_fall_below_min_score() {
    let dir = fixture("animals");
    let report = patterns_json(dir.path(), &[]);
    for c in report["candidates"].as_array().unwrap() {
        let touched = [site_names(c, "sites"), site_names(c, "definitions")].concat();
        for getter in ["Rock::label", "Pebble::tag", "Dog::name", "Cat::name"] {
            assert!(!touched.iter().any(|n| n == getter), "{getter} in {c}");
        }
    }
    // The shared `(&Self) -> String` signature is common, so its group scores low.
    let strict = patterns_json(dir.path(), &["--min-score", "1.0"]);
    let keys: Vec<&str> = strict["signature_groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["key"].as_str().unwrap())
        .collect();
    assert!(!keys.contains(&"fn(&Self) -> String"), "{keys:?}");
    assert!(candidate_at(&strict, "emit_dogs").is_some());
}

#[test]
fn patterns_already_behind_a_trait_is_suppressed_not_proposed() {
    let dir = fixture("animals");
    let report = patterns_json(dir.path(), &[]);
    assert!(candidate_at(&report, "emit_cow").is_none());
    let hidden = report["suppressed"].as_array().unwrap();
    assert_eq!(hidden.len(), 1);
    assert_eq!(site_names(&hidden[0], "sites"), ["emit_cow", "emit_hen"]);
    assert!(
        hidden[0]["reason"]
            .as_str()
            .unwrap()
            .contains("Speak::speak")
    );
}

#[test]
fn patterns_hide_implements_recovers_the_trait_as_a_candidate() {
    let dir = fixture("animals");
    let report = patterns_json(dir.path(), &["--hide-implements", "Speak"]);
    let c = candidate_at(&report, "emit_cow").expect("oracle recovers Speak");
    assert_eq!(site_names(c, "sites"), ["emit_cow", "emit_hen"]);
    assert!(report["suppressed"].as_array().unwrap().is_empty());
}

#[test]
fn patterns_text_golden() {
    let dir = fixture("animals");
    let output = rstyle(dir.path(), &["patterns"]);
    assert_eq!(output.status.code(), Some(0));
    insta::assert_snapshot!(stdout(&output));
}

#[test]
fn patterns_output_is_byte_identical_across_runs_and_copies() {
    let (one, two) = (fixture("animals"), fixture("animals"));
    let first = stdout(&rstyle(one.path(), &["patterns", "--format", "json"]));
    let again = stdout(&rstyle(one.path(), &["patterns", "--format", "json"]));
    let fresh = stdout(&rstyle(two.path(), &["patterns", "--format", "json"]));
    assert!(first.contains("\"candidates\""));
    assert_eq!(first, again);
    assert_eq!(first, fresh);
}

#[test]
fn pdg_is_only_recorded_for_patterns_and_modes_do_not_clobber() {
    let dir = fixture("animals");
    let style_facts = |dir: &Path| facts_for(dir, "animals-lib.json");
    assert!(rstyle(dir.path(), &[]).status.code().is_some());
    assert!(style_facts(dir.path()).iter().all(|f| f["pdg"].is_null()));

    patterns_json(dir.path(), &[]);
    let sem = dir.path().join("target/rstyle/facts-sem/animals-lib.json");
    let sem: Vec<Value> = serde_json::from_slice(&std::fs::read(sem).unwrap()).unwrap();
    let dogs = fact(&sem, "emit_dogs");
    assert!(dogs["pdg"]["nodes"].as_array().unwrap().len() >= 4);
    let kinds: Vec<&str> = dogs["pdg"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"iterate") && kinds.contains(&"call"),
        "{kinds:?}"
    );

    // Style facts stay PDG-free after a patterns run, and a second style run still works.
    assert!(rstyle(dir.path(), &[]).status.code().is_some());
    assert!(style_facts(dir.path()).iter().all(|f| f["pdg"].is_null()));
    assert!(
        patterns_json(dir.path(), &[])["candidates"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
}

fn first_candidate_id(dir: &Path) -> String {
    let report = patterns_json(dir, &[]);

    report["candidates"][0]["id"].as_str().unwrap().to_string()
}

fn ignore_ok(dir: &Path, args: &[&str]) -> String {
    let mut all = vec!["ignore"];
    all.extend(args);
    let output = rstyle(dir, &all);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    stdout(&output)
}

#[test]
fn ignore_hides_a_candidate_and_patterns_says_so() {
    let dir = fixture("animals");
    let before = patterns_json(dir.path(), &[]);
    let id = first_candidate_id(dir.path());
    let count = before["candidates"].as_array().unwrap().len();

    let said = ignore_ok(dir.path(), &[&id, "--reason", "reviewed, coincidence"]);
    assert!(said.contains(&id), "{said}");
    let file = std::fs::read_to_string(dir.path().join("rstyle-ignore.toml")).unwrap();
    assert!(file.contains(&format!("[ignore.{id}]")), "{file}");

    let after = patterns_json(dir.path(), &[]);
    assert_eq!(after["candidates"].as_array().unwrap().len(), count - 1);
    assert_eq!(after["ignored_count"], 1);
    assert_eq!(after["ignored"][0]["id"], id.as_str());
    assert_eq!(after["ignored"][0]["reason"], "reviewed, coincidence");

    let text = stdout(&rstyle(dir.path(), &["patterns", "--show-ignored"]));
    assert!(
        text.contains("1 candidates hidden by the ignore ledger"),
        "{text}"
    );
    assert!(text.contains("\"reviewed, coincidence\""), "{text}");

    let full = patterns_json(dir.path(), &["--no-ignore"]);
    assert_eq!(full["candidates"].as_array().unwrap().len(), count);
}

#[test]
fn ignore_status_and_prune_manage_stale_entries() {
    let dir = fixture("animals");
    let id = first_candidate_id(dir.path());
    ignore_ok(dir.path(), &[&id, "--allow-growth"]);
    let path = dir.path().join("rstyle-ignore.toml");
    let live = std::fs::read_to_string(&path).unwrap();
    let stale = "\n[ignore.deadbeef0000]\nkind = \"generic_fn\"\nfns = [\"k::gone\"]\n";
    std::fs::write(&path, format!("{live}{stale}")).unwrap();

    let status = ignore_ok(dir.path(), &["--status"]);
    assert!(
        status.contains(&id) && status.contains("active"),
        "{status}"
    );
    assert!(
        status.contains("deadbeef0000") && status.contains("stale"),
        "{status}"
    );

    let dry = ignore_ok(dir.path(), &["--prune", "--dry-run"]);
    assert!(dry.contains("would remove deadbeef0000"), "{dry}");
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("deadbeef0000")
    );

    ignore_ok(dir.path(), &["--prune"]);
    let kept = std::fs::read_to_string(&path).unwrap();
    assert!(!kept.contains("deadbeef0000"), "{kept}");
    assert!(kept.contains(&format!("[ignore.{id}]")), "{kept}");
}

#[test]
fn ignore_rejects_unknown_ids_and_missing_modes() {
    let dir = fixture("animals");
    let unknown = rstyle(dir.path(), &["ignore", "ffff"]);
    let nothing = rstyle(dir.path(), &["ignore"]);

    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("no candidate"));
    assert_eq!(nothing.status.code(), Some(2));
    assert!(!dir.path().join("rstyle-ignore.toml").exists());
}

#[test]
fn ignore_keeps_hand_written_comments_when_it_rewrites_the_file() {
    let dir = fixture("animals");
    let all = patterns_json(dir.path(), &[]);
    let (first, second) = (&all["candidates"][0]["id"], &all["candidates"][1]["id"]);
    let (first, second) = (first.as_str().unwrap(), second.as_str().unwrap());
    ignore_ok(dir.path(), &[first]);
    let path = dir.path().join("rstyle-ignore.toml");
    let header = format!("[ignore.{first}]");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replace(&header, &format!("# loops differ by closure\n{header}")),
    )
    .unwrap();

    ignore_ok(dir.path(), &[second]);
    let after = std::fs::read_to_string(&path).unwrap();

    assert!(
        after.contains(&format!("# loops differ by closure\n{header}")),
        "{after}"
    );
    assert!(after.contains(&format!("[ignore.{second}]")), "{after}");
}

#[test]
fn fail_on_candidates_exits_one_until_every_candidate_is_dismissed() {
    let dir = fixture("animals");
    let all = patterns_json(dir.path(), &[]);
    let ids: Vec<String> = all["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect();
    let gate = |dir: &Path| rstyle(dir, &["patterns", "--fail-on-candidates"]);

    let failing = gate(dir.path());
    assert_eq!(failing.status.code(), Some(1));
    assert!(
        stdout(&failing).contains("candidates"),
        "output is still printed"
    );

    ids.iter().for_each(|id| {
        ignore_ok(dir.path(), &[id]);
    });
    let clean = gate(dir.path());
    assert_eq!(clean.status.code(), Some(0), "{}", stdout(&clean));
    assert_eq!(rstyle(dir.path(), &["patterns"]).status.code(), Some(0));
}

fn clone_workspace() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("src")).unwrap();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nname='clone_fixture'\nversion='0.1.0'\nedition='2021'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn first(a: i32) -> i32 { a }\npub fn second(a: i32) -> i32 { a }\npub fn small() {}\npub fn unsupported(a: i32) -> i32 { first(a) }\n").unwrap();
    directory
}

fn clones(directory: &Path, arguments: &[&str]) -> (Output, Value) {
    let mut args = vec!["clones", "--format", "json"];
    args.extend_from_slice(arguments);
    let output = rstyle(directory, &args);
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

#[test]
#[cfg(unix)]
fn clones_runs_with_execute_only_binaries_without_fingerprint_fields() {
    use std::os::unix::fs::PermissionsExt;
    let directory = clone_workspace();
    let executable = directory.path().join("running-cli");
    std::fs::copy(cargo_bin("cargo-rstyle"), &executable).unwrap();
    let producer = directory.path().join("execute-only-driver");
    std::fs::copy(driver(), &producer).unwrap();
    let baseline = Command::new(&executable)
        .args(["clones", "--format", "json"])
        .current_dir(directory.path())
        .env("RSTYLE_DRIVER", &producer)
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap();
    assert!(baseline.status.success());
    let expected: Value = serde_json::from_slice(&baseline.stdout).unwrap();
    for path in [&executable, &producer] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o111)).unwrap();
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o111
        );
    }
    let child = Command::new(&executable)
        .args(["clones", "--format", "json"])
        .current_dir(directory.path())
        .env("RSTYLE_DRIVER", &producer)
        .env("RSTYLE_PERF", "1")
        .env_remove("CARGO_TARGET_DIR")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(expected["matching"], actual["matching"]);
    assert_unfingerprinted_reports(&expected, &actual);
    assert_no_binary_hash_events(&output.stderr);
}

fn assert_no_binary_hash_events(stderr: &[u8]) {
    let events: Vec<Value> = String::from_utf8_lossy(stderr)
        .lines()
        .filter_map(|line| line.strip_prefix("rstyle-perf "))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(events.iter().any(|event| event["stage"] == "cli.prepare"));
    assert!(
        !events
            .iter()
            .any(|event| event["stage"] == "capture.binary_hash")
    );
}

fn assert_unfingerprinted_reports(expected: &Value, actual: &Value) {
    for report in [expected, actual] {
        assert_persisted_report(report);
        for field in ["producer_binary_sha256", "orchestration_binary_sha256"] {
            assert!(report.get(field).is_none(), "{field}");
        }
        for field in [
            "capture_complete",
            "assessment_complete",
            "pair_enumeration_complete",
            "pair_evaluation_complete",
        ] {
            assert_eq!(report[field], true, "{field}");
        }
        assert_eq!(report["compiler_identity"], expected["compiler_identity"]);
        assert_eq!(report["contract_revision"], expected["contract_revision"]);
        let plan = read_capture_plan(report);
        assert!(plan.get("driver_sha256").is_none());
        assert!(plan.get("orchestrator_sha256").is_none());
    }
}

fn read_capture_plan(report: &Value) -> Value {
    let directory = Path::new(report["run_directory"].as_str().unwrap());
    serde_json::from_slice(&std::fs::read(directory.join("plan.json")).unwrap()).unwrap()
}

#[test]
fn clones_rejects_retired_expanded_evidence_option() {
    let directory = clone_workspace();
    let output = rstyle(directory.path(), &["clones", "--expanded-evidence"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("unexpected argument '--expanded-evidence'")
    );
    assert!(!directory.path().join("target/rstyle").exists());
}

#[test]
fn clones_shared_fanout_keeps_transitive_analysis_bounded() {
    let directory = clone_workspace();
    let mut source = String::from("pub fn level0(x: i32) -> bool { x > 0 }\n");
    for level in 1..8 {
        let calls = std::iter::repeat_n(format!("level{}(x)", level - 1), 6)
            .collect::<Vec<_>>()
            .join(" && ");
        source.push_str(&format!(
            "pub fn level{level}(x: i32) -> bool {{ {calls} }}\n"
        ));
    }
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-helpers-v1",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8);
    assert!(
        functions
            .iter()
            .all(|function| function["outcome"] == "admitted"),
        "{functions:#?}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 28);
}

fn assert_persisted_report(report: &Value) {
    let persisted: Value = serde_json::from_slice(
        &std::fs::read(Path::new(report["run_directory"].as_str().unwrap()).join("report.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(&persisted, report);
}

#[test]
fn clones_shared_bundle_is_the_default_and_keeps_calls_as_ids() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn leaf(x: i32) -> bool { x > 0 }\npub fn first(x: i32) -> bool { leaf(x) }\npub fn second(x: i32) -> bool { leaf(x) }\n").unwrap();
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-helpers-v1",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_persisted_report(&report);
    assert_eq!(report["unit_bundle_schema"], "rstyle.shared-unit/4");
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 3);
    for function in report["functions"].as_array().unwrap() {
        assert!(function.get("artifact_prefix").is_none());
        assert!(function.get("document_manifest").is_none());
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let mut calls = Vec::new();
        shared_call_nodes(&facts.body, &mut calls);
        for call in calls {
            assert!(
                rstyle_pdg::helper_calls::direct_resolution(call).unwrap()
                    [rstyle_pdg::helper_calls::KEY]
                    .is_null()
            );
        }
    }
    assert_shared_bundle_tables(&report);
}

fn assert_matching_build_metadata(graph: &Value, build: &Value) {
    for kind in ["pdg", "cfg"] {
        let configuration = &graph[kind]["producer"]["configuration"];
        assert!(configuration.get("captured_build").is_none());
        let matching = &configuration["matching_build"];
        assert_eq!(matching["schema"], "rstyle.matching-build/1");
        assert_eq!(matching.as_object().unwrap().len(), 4);
        for key in ["overflow_checks", "debug_assertions", "panic_strategy"] {
            assert_eq!(matching[key], build[key]);
        }
    }
}

fn assert_bundle_source_references(wire: &Value, store: &rstyle_pdg::unit_store::UnitStore) {
    for (index, function) in store.functions().iter().enumerate() {
        let reference = &wire["functions"][index]["source"];
        assert!(reference.get("bytes").is_none());
        let source = wire["sources"][reference["snapshot"].as_u64().unwrap() as usize]
            .as_str()
            .unwrap();
        let range = &reference["range"];
        assert_eq!(
            source.get(
                range["start"].as_u64().unwrap() as usize..range["end"].as_u64().unwrap() as usize
            ),
            Some(function.source.bytes.as_str())
        );
        assert_eq!(reference["digest"], function.source.digest);
        let graph = &wire["graphs"][wire["roots"][index].as_u64().unwrap() as usize];
        assert_matching_build_metadata(graph, &wire["functions"][index]["build"]);
    }
}

fn assert_shared_bundle_tables(report: &Value) {
    let unit = &report["observed_units"][0];
    let path = Path::new(unit["directory"].as_str().unwrap());
    let bytes = std::fs::read(path.join("unit.evidence.json")).unwrap();
    let manifest: rstyle_pdg::unit_store::BundleManifest =
        serde_json::from_slice(&std::fs::read(path.join("unit.evidence.manifest.json")).unwrap())
            .unwrap();
    manifest.verify(&bytes).unwrap();
    let store = rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes).unwrap();
    assert_eq!(store.helper_count(), 3);
    assert_eq!(store.graph_count(), 3);
    assert!(!path.join("facts.json").exists());
    assert!(!path.join("readable").exists());
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wire["functions"].as_array().unwrap().len(), 3);
    assert_eq!(wire["helpers"].as_array().unwrap().len(), 3);
    assert_eq!(wire["graphs"].as_array().unwrap().len(), 3);
    assert_eq!(wire["sources"].as_array().unwrap().len(), 1);
    assert_bundle_source_references(&wire, &store);
    let mut dangling = wire.clone();
    dangling["functions"][0]["source"]["snapshot"] = u32::MAX.into();
    assert!(
        rstyle_pdg::unit_store::UnitStore::from_bundle(&serde_json::to_vec(&dangling).unwrap())
            .is_err()
    );
    let mut unused = wire;
    unused["sources"]
        .as_array_mut()
        .unwrap()
        .push("unreferenced".into());
    assert!(
        rstyle_pdg::unit_store::UnitStore::from_bundle(&serde_json::to_vec(&unused).unwrap())
            .is_err()
    );
    let mut edited = bytes;
    edited.push(b' ');
    assert!(manifest.verify(&edited).is_err());
}

#[test]
fn clones_real_source_handoff_has_independent_features_and_gate_expectations() {
    let directory = clone_workspace();
    let (output, report) = clones(directory.path(), &["--trace"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], true);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 4);
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        3
    );
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "excluded")
            .count(),
        1
    );
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(
        pairs.iter().filter(|p| p["decision"] == "Accepted").count(),
        1
    );
    check_clone_identity_features(&report);
    check_clone_pair_gates(pairs);
}

fn check_clone_pair_gates(pairs: &[Value]) {
    for pair in pairs {
        if pair["decision"] == "Accepted" {
            assert!(
                pair["stages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|s| s["outcome"] == "Passed")
            );
            assert_eq!(pair["stages"][6]["detail"]["score"], "1/1");
            assert_eq!(pair["stages"][4]["detail"]["dot"], "15");
            assert_eq!(pair["stages"][4]["detail"]["norm_a_squared"], "15");
            assert_eq!(pair["stages"][4]["detail"]["norm_b_squared"], "15");
            assert_eq!(pair["stages"][5]["detail"]["name"], "0/1");
            assert_eq!(pair["stages"][5]["detail"]["mean"], "2/3");
            assert_eq!(pair["stages"][6]["detail"]["depth"], "2");
            assert_eq!(pair["stages"][6]["detail"]["weights"], "[3, 2, 1]");
            assert_eq!(
                pair["stages"][6]["detail"]["overlap_per_round"],
                "[3, 3, 3]"
            );
        } else {
            assert!(pair["stages"][3]["outcome"]["Rejected"].is_object());
            assert_eq!(pair["stages"][4]["outcome"], "NotEvaluated");
        }
    }
}

#[test]
fn clones_warm_runs_recapture_and_do_not_keep_deleted_functions() {
    let directory = clone_workspace();
    let (first_output, first) = clones(directory.path(), &[]);
    let old_unit = PathBuf::from(first["observed_units"][0]["directory"].as_str().unwrap());
    std::fs::write(old_unit.join("facts.json"), "stale corrupt facts").unwrap();
    let (second_output, second) = clones(directory.path(), &[]);
    assert!(first_output.status.success() && second_output.status.success());
    assert_ne!(first["run_directory"], second["run_directory"]);
    assert_eq!(first["matching"], second["matching"]);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn first(a: i32) -> i32 { a }\n",
    )
    .unwrap();
    let (output, edited) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(edited["functions"].as_array().unwrap().len(), 1);
    assert_eq!(edited["matching"]["pairs"], serde_json::json!([]));
}

#[test]
fn clones_failure_is_not_a_complete_zero_pair_run() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn broken() { absent(); }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["assessment_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(!report["capture_failures"].as_array().unwrap().is_empty());
}

#[test]
fn clones_exact_threshold_ingestion_and_explicit_cosine() {
    let directory = clone_workspace();
    let config = directory.path().join("matching.json");
    std::fs::write(
        &config,
        r#"{"string_threshold":0.50000000000000001,"scoring":"cross_product_cosine"}"#,
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--matching-config", config.to_str().unwrap()],
    );
    assert!(output.status.success());
    assert_eq!(
        report["matching"]["effective_config"]["string_threshold"],
        "50000000000000001/100000000000000000"
    );
    assert_eq!(report["matching"]["scoring"], "cross_product_cosine");
    std::fs::write(&config, r#"{"final_threshold":1.00000000000000001}"#).unwrap();
    let output = rstyle(
        directory.path(),
        &["clones", "--matching-config", config.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(2));
}

fn report_function_unit<'a>(report: &'a Value, function: &Value) -> &'a Value {
    let unit_id = &function["unit_evidence"]["report_unit"];
    let context = report["unit_contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|context| &context["id"] == unit_id)
        .unwrap();
    &report["observed_units"][context["observed_unit"].as_u64().unwrap() as usize]
}

fn assert_report_function_handles(report: &Value) {
    assert_eq!(
        report["function_identity_schema"],
        "rstyle.report-function/2"
    );
    assert_eq!(
        report["unit_contexts"].as_array().unwrap().len(),
        report["observed_units"].as_array().unwrap().len()
    );
    for function in report["functions"].as_array().unwrap() {
        let evidence = &function["unit_evidence"];
        assert_eq!(
            function["key"],
            format!(
                "function:u{}:f{}",
                evidence["report_unit"], evidence["function"]
            )
        );
        let unit = report_function_unit(report, function);
        assert_eq!(
            Path::new(evidence["bundle"].as_str().unwrap())
                .parent()
                .unwrap(),
            Path::new(unit["directory"].as_str().unwrap())
        );
        assert_eq!(
            function["compilation_unit"],
            format!(
                "{}:{}:test={}",
                unit["target"]["package"].as_str().unwrap(),
                unit["target"]["name"].as_str().unwrap(),
                unit["test"]
            )
        );
    }
}

fn assert_selected_release_handles(report: &Value, manifest_directory: &Path) {
    assert_report_function_handles(report);
    assert_eq!(report["scope"]["build_options"]["release"], true);
    let expected_directory = manifest_directory.canonicalize().unwrap();
    for function in report["functions"].as_array().unwrap() {
        let unit = report_function_unit(report, function);
        let actual_directory = Path::new(unit["target"]["manifest_dir"].as_str().unwrap());
        assert_eq!(actual_directory.canonicalize().unwrap(), expected_directory);
        assert_eq!(
            function["facts"]["build"]["opt_level"]["value"],
            "Aggressive"
        );
    }
}

#[cfg(unix)]
fn assert_selected_release_alias(report: &Value, manifest_directory: &Path) {
    let directory = tempfile::tempdir().unwrap();
    let alias = directory.path().join("manifest-alias");
    std::os::unix::fs::symlink(manifest_directory, &alias).unwrap();

    assert_selected_release_handles(report, &alias);
}

#[test]
fn clones_all_targets_keep_compilation_contexts_distinct() {
    let directory = clone_workspace();
    let (output, report) = clones(directory.path(), &["--all-targets", "--jobs", "2"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["observed_units"].as_array().unwrap().len(), 2);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["facts"]["name"] != "main")
            .count(),
        8
    );
    assert!(
        functions
            .iter()
            .filter(|f| f["facts"]["name"] == "main")
            .all(|f| f["outcome"] == "excluded")
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
    assert_report_function_handles(&report);
    let keys: std::collections::BTreeSet<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys.len(), functions.len());
}

#[test]
fn inherited_hir_destinations_do_not_change_legacy_commands() {
    let directory = clone_workspace();
    let poison = directory.path().join("must-not-exist");
    for args in [
        vec!["--format", "json"],
        vec!["patterns", "--format", "json"],
    ] {
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args(args)
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_HIR_DOCUMENT_DIR", &poison)
            .env("RSTYLE_HIR_FACTS_FILE", poison.join("facts.json"))
            .env_remove("CARGO_TARGET_DIR")
            .output()
            .unwrap();
        assert_ne!(
            output.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let _: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(!poison.exists());
    }
}

#[test]
fn clones_identical_source_in_packages_and_modules_survives_concurrent_capture() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nmembers=['a','b']\nresolver='2'\n",
    )
    .unwrap();
    for package in ["a", "b"] {
        let root = directory.path().join(package);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='{package}'\nversion='0.1.0'\nedition='2021'\nbuild='build.rs'\n"
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("build.rs"),
            "fn main() { println!(\"cargo:rerun-if-changed=build.rs\"); }",
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub mod one { pub fn r#type(x: i32) -> i32 { x } }\npub mod two { pub fn r#type(x: i32) -> i32 { x } }\n").unwrap();
    }
    let (output, report) = clones(directory.path(), &["--jobs", "2"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["observed_units"].as_array().unwrap().len(), 2);
    assert_eq!(report["functions"].as_array().unwrap().len(), 4);
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 6);
    assert!(pairs.iter().all(|p| p["decision"] == "Accepted"));
    let (output, serial) = clones(directory.path(), &["--jobs", "1"]);
    assert!(output.status.success(), "{serial}");
    assert_eq!(report["matching"], serial["matching"]);
    let (output, selected) = clones(directory.path(), &["-p", "a", "--release"]);
    assert!(output.status.success(), "{selected}");
    assert_eq!(selected["functions"].as_array().unwrap().len(), 2);
    assert_eq!(selected["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert_selected_release_handles(&selected, &directory.path().join("a"));
    #[cfg(unix)]
    assert_selected_release_alias(&selected, &directory.path().join("a"));
}

#[test]
fn clones_traces_and_fail_on_pairs_preserve_decisions() {
    let directory = clone_workspace();
    let (output, ordinary) = clones(directory.path(), &[]);
    assert!(output.status.success());
    let (output, traced) = clones(directory.path(), &["--trace", "--fail-on-accepted-pairs"]);
    assert_eq!(output.status.code(), Some(1));
    let pairs = |report: &Value| {
        report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["first"].clone(),
                    p["second"].clone(),
                    p["decision"].clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(pairs(&ordinary), pairs(&traced));
    let accepted = traced["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["decision"] == "Accepted")
        .unwrap();
    assert_eq!(accepted["wl_label_partitions"].as_array().unwrap().len(), 3);
    assert!(
        ordinary["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["projection_trace"].is_null())
    );
    assert!(
        traced["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .all(|f| !f["projection_trace"].is_null())
    );
}

#[test]
fn clones_unsupported_workspace_completes_assessment_with_zero_admitted_functions() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn unsupported() { loop {} }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["functions"][0]["outcome"], "excluded");
    assert_eq!(report["matching"]["functions"], serde_json::json!([]));
}

#[test]
fn clones_distinct_macro_function_owners_do_not_invalidate_ordinary_functions() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"macro_rules! make {
            () => { { fn f() {} } { fn f() {} } };
        }
        pub fn outer() { make!(); }
        pub fn first(a: i32) -> i32 { a }
        pub fn second(a: i32) -> i32 { a }
        "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}: {report}");
    }
    let functions = report["functions"].as_array().unwrap();
    check_distinct_expanded_functions(functions);
    assert_eq!(
        functions
            .iter()
            .filter(|function| function["outcome"] == "admitted")
            .count(),
        2
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 1);
}

fn check_distinct_expanded_functions(functions: &[Value]) {
    let generated: Vec<_> = functions
        .iter()
        .filter(|function| function["facts"]["name"].as_str().unwrap().ends_with("::f"))
        .collect();
    assert_eq!(generated.len(), 2);
    assert_eq!(generated[0]["facts"]["name"], generated[1]["facts"]["name"]);
    assert_eq!(
        generated[0]["facts"]["fn_span"],
        generated[1]["facts"]["fn_span"]
    );
    assert_ne!(
        generated[0]["facts"]["body"]["hir"]["owner"],
        generated[1]["facts"]["body"]["hir"]["owner"]
    );
    assert!(
        generated
            .iter()
            .all(|function| function["outcome"] == "excluded")
    );
}

#[test]
fn clones_resource_failure_is_explicit_and_incomplete() {
    let directory = clone_workspace();
    let (output, report) = clones(directory.path(), &["--max-pairs", "2"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert_eq!(report["matching"]["incomplete"], true);
    assert_eq!(report["pair_enumeration_complete"], false);
    assert!(
        report["matching"]["diagnostics"][0]
            .as_str()
            .unwrap()
            .contains("requires 3 pairs")
    );
    assert_eq!(report["matching"]["pairs"], serde_json::json!([]));
}

#[test]
fn clones_missing_corrupt_and_mismatched_serialized_artifacts_fail_closed() {
    use std::os::unix::fs::PermissionsExt;
    let directory = clone_workspace();
    let shim = directory.path().join("driver-shim");
    let real = driver().to_string_lossy().replace('\'', "'\\''");
    for operation in [
        "rm \"$RSTYLE_HIR_DOCUMENT_DIR/unit.evidence.json\"",
        "printf '{}' > \"$RSTYLE_HIR_DOCUMENT_DIR/unit.evidence.json\"",
        "printf '{\"documents\":[]}' > \"$RSTYLE_HIR_DOCUMENT_DIR/unit.evidence.manifest.json\"",
        "printf ' ' >> \"$RSTYLE_HIR_DOCUMENT_DIR/unit.evidence.json\"",
    ] {
        let script = format!(
            "#!/bin/sh\n'{real}' \"$@\"\ncode=$?\nif [ \"$code\" = 0 ] && [ -n \"$RSTYLE_HIR_DOCUMENT_DIR\" ]; then\n{operation}\nfi\nexit \"$code\"\n"
        );
        std::fs::write(&shim, script).unwrap();
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args(["clones", "--format", "json"])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", &shim)
            .env_remove("CARGO_TARGET_DIR")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{operation}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["assessment_complete"], false);
        assert_eq!(report["pair_evaluation_complete"], false);
        assert!(
            !report["capture_failures"].as_array().unwrap().is_empty()
                || !report["handoff_failures"].as_array().unwrap().is_empty()
        );
    }
}

#[test]
fn clones_unused_inputs_and_reachable_merges_reach_real_matching() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn unused(a: i32) {}\npub fn merge(c: bool) -> i32 { if c { 1 } else { 2 } }\npub fn copied(c: bool) -> i32 { if c { 1 } else { 2 } }\n").unwrap();
    let (output, report) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        3
    );
    for function in report["matching"]["functions"].as_array().unwrap() {
        let projection = &function["projection"];
        if projection["name"] == "unused" {
            assert_eq!(projection["projected_nodes"], 2);
            assert_eq!(projection["reference_count"], 0);
            assert_eq!(
                projection["features"],
                serde_json::json!([0, 0, 0, 0, 0, 0, 0, 2, 0])
            );
        } else {
            // One input, read, predicate, two literals, merge, and actual return.
            assert_eq!(projection["projected_nodes"], 7);
            assert_eq!(projection["reference_count"], 1);
        }
    }
    assert_eq!(
        report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["decision"] == "Accepted")
            .count(),
        1
    );
}

#[test]
fn clones_arithmetic_failure_is_not_an_ordinary_rejection() {
    let directory = clone_workspace();
    let config = directory.path().join("matching.json");
    std::fs::write(
        &config,
        r#"{"numerical_threshold":"0.90000000000000000001"}"#,
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--matching-config", config.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], false);
    let failed = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["evaluation_status"] == "failed")
        .collect::<Vec<_>>();
    assert_eq!(failed.len(), 1);
    assert_eq!(report["pair_enumeration_complete"], true);
    assert!(failed[0]["decision"].is_null());
    assert!(
        failed[0]["stages"][4]["outcome"]["Error"]["reason"]
            .as_str()
            .unwrap()
            .contains("overflow")
    );
}

fn check_clone_identity_features(report: &Value) {
    for function in report["matching"]["functions"].as_array().unwrap() {
        if function["projection"]["name"] == "first" || function["projection"]["name"] == "second" {
            assert_eq!(
                function["projection"]["features"],
                serde_json::json!([0, 1, 2, 0, 0, 0, 0, 3, 1])
            );
            assert_eq!(function["projection"]["reference_count"], 1);
        }
    }
}

#[test]
fn clones_build_features_and_required_targets_follow_cargo_unit_inventory() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("Cargo.toml"), "[package]\nname='clone_fixture'\nversion='0.1.0'\nedition='2021'\n[workspace]\n[features]\nextra=[]\n[[bin]]\nname='optional'\npath='src/optional.rs'\nrequired-features=['extra']\n").unwrap();
    std::fs::write(directory.path().join("src/optional.rs"), "fn main() {}\n").unwrap();
    let (output, default) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(default["observed_units"].as_array().unwrap().len(), 1);
    let (output, feature) = clones(directory.path(), &["--features", "extra"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(feature["observed_units"].as_array().unwrap().len(), 2);
    assert!(
        feature["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["facts"]["build"]["features"] == serde_json::json!(["extra"]))
    );
    assert_eq!(
        feature["expected_compilation_units"]["units"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn clones_literal_edits_preserve_matching_labels_and_disclose_overflow_limits() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn first(a: i32) -> i32 { a + 1 }\npub fn second(a: i32) -> i32 { a + 2 }\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(report["matching"]["pairs"][0]["decision"], "Accepted");
    assert_eq!(
        report["matching"]["pairs"][0]["stages"][6]["detail"]["score"],
        "1/1"
    );
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["extraction_evidence"]["cfg"]["status"], "partial");
        assert!(
            !function["extraction_evidence"]["cfg"]["limitations"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn clones_conditional_source_and_inherited_modes_remain_fail_closed() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn conditional(a: i32) -> i32 { #[cfg(any())] let removed = 1; a }",
    )
    .unwrap();
    let poison = directory.path().join("inherited-output");
    let output = Command::new(cargo_bin("cargo-rstyle"))
        .args(["clones", "--format", "json"])
        .current_dir(directory.path())
        .env("RSTYLE_DRIVER", driver())
        .env("RSTYLE_HIR_DOCUMENT_DIR", &poison)
        .env("RSTYLE_HIR_FACTS_FILE", poison.join("facts.json"))
        .env("RSTYLE_OUT_DIR", &poison)
        .env("RSTYLE_PDG", "1")
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["functions"][0]["outcome"], "unknown");
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["matching"]["pairs"], serde_json::json!([]));
    assert!(!poison.exists());
}

#[test]
fn concurrent_clone_commands_keep_independent_run_evidence() {
    let directory = clone_workspace();
    let _ = driver();
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| clones(directory.path(), &[]));
        let second = scope.spawn(|| clones(directory.path(), &[]));
        (first.join().unwrap(), second.join().unwrap())
    });
    assert!(first.0.status.success() && second.0.status.success());
    assert_ne!(first.1["run_directory"], second.1["run_directory"]);
    assert_eq!(first.1["matching"], second.1["matching"]);
}

#[test]
fn interrupted_compiler_evidence_is_never_consumed_or_reused() {
    use std::os::unix::fs::PermissionsExt;
    let directory = clone_workspace();
    let shim = directory.path().join("interrupted-driver");
    let real = driver().to_string_lossy().replace('\'', "'\\''");
    let script = format!(
        "#!/bin/sh\n'{real}' \"$@\"\ncode=$?\nif [ -n \"$RSTYLE_HIR_DOCUMENT_DIR\" ]; then kill -TERM $$; fi\nexit \"$code\"\n"
    );
    std::fs::write(&shim, script).unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(cargo_bin("cargo-rstyle"))
        .args(["clones", "--format", "json"])
        .current_dir(directory.path())
        .env("RSTYLE_DRIVER", &shim)
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let interrupted: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(interrupted["capture_complete"], false);
    assert_eq!(interrupted["pair_evaluation_complete"], false);
    assert_eq!(interrupted["functions"], serde_json::json!([]));
    let (output, fresh) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_ne!(interrupted["run_directory"], fresh["run_directory"]);
    assert_eq!(fresh["matching"]["pairs"].as_array().unwrap().len(), 3);
}

#[test]
fn selected_package_build_dependency_compilation_is_outside_matching_scope() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nmembers=['helper','app']\nresolver='2'\n",
    )
    .unwrap();
    for package in ["helper", "app"] {
        std::fs::create_dir_all(directory.path().join(package).join("src")).unwrap();
    }
    std::fs::write(
        directory.path().join("helper/Cargo.toml"),
        "[package]\nname='helper'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("helper/src/lib.rs"),
        "pub fn number() -> i32 { 1 }\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("app/Cargo.toml"), "[package]\nname='app'\nversion='0.1.0'\nedition='2021'\n[build-dependencies]\nhelper={path='../helper'}\n").unwrap();
    std::fs::write(
        directory.path().join("app/build.rs"),
        "fn main() { assert_eq!(helper::number(), 1); }\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("app/src/lib.rs"),
        "pub fn idle() {}\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--jobs", "2"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["observed_units"].as_array().unwrap().len(), 2);
    assert_eq!(
        report["intended_primary_units"].as_array().unwrap().len(),
        2
    );
    assert_eq!(report["functions"].as_array().unwrap().len(), 2);
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert!(
        report["expected_compilation_units"]["units"]
            .as_array()
            .unwrap()
            .len()
            > 2
    );
}

#[test]
fn clones_const_expansion_is_explicit_and_inherited_selection_cannot_activate_it() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub const fn f(x: i32) -> i32 { x }\npub fn g(x: i32) -> i32 { x }\n",
    )
    .unwrap();
    let (output, baseline) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(
        baseline["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        1
    );
    let (output, expanded) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1", "--trace"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        expanded["extraction_policy"]["id"],
        "rstyle.rust.hir-coverage"
    );
    assert_eq!(expanded["extraction_policy"]["version"], "0.7.0");
    assert_eq!(expanded["extraction_selection"], "hir-expanded-v1");
    assert_eq!(baseline["extraction_selection"], "r0");
    assert_eq!(
        expanded["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        2
    );
    assert_eq!(expanded["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert_eq!(expanded["matching"]["pairs"][0]["displayed_score"], "1");
    assert_inherited_expanded_selection_is_ignored(directory.path());
}

#[test]
fn clones_rejects_retired_staged_selections_before_capture() {
    let directory = clone_workspace();
    for profile in [
        "hir-const-v1",
        "hir-casts-v1",
        "hir-match-v1",
        "hir-guards-v1",
        "hir-conditional-v1",
        "hir-loops-v1",
        "hir-calls-v1",
    ] {
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args(["clones", "--extraction-profile", profile])
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("r0 or hir-expanded-v1"));
        assert!(!directory.path().join("target/rstyle/clones").exists());
    }
}

#[test]
fn clones_integer_cast_profile_captures_verified_operations_and_preserves_defaults() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub const fn f(x: i32) -> u8 { x as u8 }\npub fn g(x: i32) -> u8 { x as u8 }\npub fn h(x: i32) -> f64 { x as f64 }\n").unwrap();
    let (output, expanded) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(expanded["extraction_policy"]["version"], "0.7.0");
    assert_eq!(
        expanded["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        2
    );
    assert_eq!(expanded["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert_eq!(expanded["matching"]["pairs"][0]["displayed_score"], "1");
    let (output, baseline) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert_eq!(baseline["matching"]["pairs"].as_array().unwrap().len(), 0);
    assert!(
        baseline["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

#[test]
fn clones_data_cfg_removal_resolves_blocks_locals_macros_items_and_match_arms() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("Cargo.toml"), "[package]\nname='cfg_fixture'\nversion='0.1.0'\nedition='2021'\n[features]\nextra=[]\n[workspace]\n").unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "const fn identity(x: i32) -> i32 { x }\npub const fn choose(x: i32) -> i32 { #[cfg(feature=\"extra\")] { identity(x) } #[cfg(not(feature=\"extra\"))] { x } }\npub fn local(x: i32) -> i32 { #[cfg(any())] let removed = missing; x }\npub fn discarded() { #[cfg(any())] missing!(); }\npub fn block() { #[cfg_attr(all(), cfg(any()))] { unsafe { missing(); } } }\npub fn item() { #[cfg(any())] fn nested() { missing(); } }\npub fn arm(x: i32) -> i32 { match x { #[cfg(any())] 0 => missing(), 1 => 2, _ => 3 } }\npub fn attr(x: i32) -> i32 { #[cfg_attr(any(), cfg(any()))] let y = x; y }\npub fn field(x: &(i32, i32)) -> i32 { #[cfg(any())] { let y = x.0; } x.1 }\n").unwrap();
    for options in [vec![], vec!["--features", "extra"]] {
        let args: Vec<_> = ["--extraction-profile", "hir-data-v3"]
            .into_iter()
            .chain(options)
            .collect();
        let (output, report) = clones(directory.path(), &args);
        assert!(output.status.success(), "{report}");
        let functions = report["functions"].as_array().unwrap();
        for name in [
            "choose",
            "local",
            "discarded",
            "block",
            "item",
            "arm",
            "attr",
            "field",
        ] {
            assert_eq!(
                function_named(functions, name)["outcome"],
                "admitted",
                "{}",
                function_named(functions, name)
            );
        }
        check_cfg_removal_evidence(functions);
    }
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert_ne!(
        function_named(legacy["functions"].as_array().unwrap(), "choose")["outcome"],
        "admitted"
    );
}

fn check_cfg_removal_evidence(functions: &[Value]) {
    let facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(function_named(functions, "choose")["facts"].clone()).unwrap();
    let correspondence = rstyle_pdg::source_evidence::capture(&facts).unwrap();
    let removed: Vec<_> = correspondence
        .entries
        .iter()
        .filter(|entry| entry.kind == "cfg_inactive")
        .collect();
    assert_eq!(removed.len(), 1);
    assert!(removed[0].hir.is_none());
    rstyle_pdg::source_evidence::verify(&facts).unwrap();
    let mut changed = facts;
    let feature = "feature=\"extra\"".to_string();
    if changed.build.cfgs.contains(&feature) {
        changed.build.cfgs.retain(|value| value != &feature);
    } else {
        changed.build.cfgs.push(feature);
    }
    let regenerated = rstyle_pdg::source_evidence::capture(&changed).unwrap();
    assert!(
        regenerated
            .entries
            .iter()
            .any(|entry| entry.kind == "cfg_inactive" && entry.hir.is_some())
    );
    changed.build.frontend_options.insert(
        rstyle_pdg::source_evidence::KEY.into(),
        serde_json::to_value(regenerated).unwrap(),
    );
    assert!(rstyle_pdg::source_evidence::verify(&changed).is_err());
}

#[test]
fn clones_data_assertions_preserve_condition_order_panic_scope_and_debug_settings() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn check(b: bool) { assert!((b)); }\npub fn once(mut x: u32) -> u32 { assert!({ x = x & 1; x == 0 }); x }\npub fn debug_once(mut x: u32) -> u32 { debug_assert!({ x = x & 1; x == 0 }); x }\npub fn qualified(b: bool) { core::assert!(b,); }\npub fn certain() { assert!(true); }\npub fn certain_wrapper() { certain() }\npub fn falsehood() { assert!(false); }\npub fn wrapper(b: bool) { check(b); }\npub fn formatted(b: bool) { assert!(b, \"bad\"); }\npub fn equal(x: u32) { assert_eq!(x, 0); }\nmod custom { macro_rules! assert { ($value:expr) => { () } } pub fn counterfeit(b: bool) { assert!(b); } }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    check_assertion_admissions(functions);
    check_assertion_graph(functions, true);
    check_assertion_mutations(functions);

    let (output, release) = clones(
        directory.path(),
        &["--release", "--extraction-profile", "hir-data-v3"],
    );
    assert!(output.status.success(), "{release}");
    check_assertion_graph(release["functions"].as_array().unwrap(), false);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] != "admitted")
    );
}

fn check_assertion_admissions(functions: &[Value]) {
    for name in [
        "check",
        "once",
        "debug_once",
        "qualified",
        "certain",
        "certain_wrapper",
        "falsehood",
    ] {
        assert_eq!(
            function_named(functions, name)["outcome"],
            "admitted",
            "{}",
            function_named(functions, name)
        );
    }
    for name in ["wrapper", "formatted", "equal", "custom::counterfeit"] {
        assert_ne!(
            function_named(functions, name)["outcome"],
            "admitted",
            "{name}"
        );
    }
    assert_eq!(
        function_named(functions, "check")["extraction_evidence"]["cfg"]["status"],
        "partial"
    );
    assert_eq!(
        function_named(functions, "certain")["extraction_evidence"]["cfg"]["status"],
        "complete"
    );
}

fn check_assertion_graph(functions: &[Value], debug: bool) {
    for (name, active) in [("once", true), ("debug_once", debug)] {
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function_named(functions, name)["facts"].clone()).unwrap();
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let assertions: Vec<_> = pair
            .pdg
            .nodes
            .iter()
            .filter(|node| {
                node.attributes
                    .operator
                    .as_ref()
                    .and_then(rstyle_pdg::StringFact::known_value)
                    == Some("assert")
            })
            .collect();
        let writes: Vec<_> = pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.write_form.as_deref() == Some("reassignment"))
            .collect();
        assert_eq!(assertions.len(), usize::from(active), "{name}");
        assert_eq!(writes.len(), usize::from(active), "{name}");
        assert_eq!(
            pair.cfg.extraction_evidence.status == rstyle_pdg::ExtractionStatus::Complete,
            !active
        );
        if active {
            assert!(
                pair.pdg.edges.iter().any(|edge| edge.target == assertions[0].id && edge.operand_position == Some(0))
            );
            assert!(
                pair.cfg
                    .extraction_evidence
                    .limitations
                    .iter()
                    .any(|limitation| limitation.contains("Assertion")
                        && limitation.contains("omitted"))
            );
        }
    }
}

fn check_assertion_mutations(functions: &[Value]) {
    let facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(function_named(functions, "debug_once")["facts"].clone()).unwrap();
    let mut changed = facts.clone();
    changed.build.debug_assertions.value = Some(false);
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    let mut changed = facts.clone();
    changed.body.children[0].children[0]
        .res
        .as_mut()
        .unwrap()
        .description = "local::debug_assert".into();
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    let mut changed = facts;
    changed.body.children[0].children[0].children[0].span.start += 1;
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
}

#[test]
fn clones_data_constant_range_bounds_preserve_values_and_dependency_edits() {
    let directory = clone_workspace();
    let source = "mod config { pub const LOW: i32 = -3; } use config::LOW as START; struct Limits; impl Limits { const HIGH: i32 = 5; } const MAX: u128 = u128::MAX;\npub fn both(x: i32) -> bool { match x { START..=Limits::HIGH => true, _ => false } }\npub fn lower(x: i32) -> bool { match x { START.. => true, _ => false } }\npub fn upper(x: i32) -> bool { match x { ..Limits::HIGH => true, _ => false } }\npub fn mixed(x: i32, b: bool) -> i32 { match x { START..0 | 1..=Limits::HIGH if b => 1, _ => 2 } }\npub fn wide(x: u128) -> bool { match x { 0..=MAX => true, _ => false } }\npub fn external(x: u128) -> bool { match x { 0..=u128::MAX => true, _ => false } }\n";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    for name in ["both", "lower", "upper", "mixed", "wide"] {
        assert_eq!(
            function_named(functions, name)["outcome"],
            "admitted",
            "{}",
            function_named(functions, name)
        );
    }
    assert_ne!(function_named(functions, "external")["outcome"], "admitted");
    check_constant_bound_graph(functions);

    std::fs::write(
        directory.path().join("src/lib.rs"),
        source.replace("LOW: i32 = -3", "LOW: i32 = -4"),
    )
    .unwrap();
    let (output, edited) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{edited}");
    assert_ne!(
        function_named(functions, "both")["facts"]["body"],
        function_named(edited["functions"].as_array().unwrap(), "both")["facts"]["body"]
    );
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] != "admitted")
    );
}

fn check_constant_bound_graph(functions: &[Value]) {
    let facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(function_named(functions, "both")["facts"].clone()).unwrap();
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let selector = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("match.range"))
        .unwrap();
    let range: rstyle_pdg::scalar_patterns::IntegerRange = serde_json::from_str(
        selector
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(range.start.as_deref(), Some("-3"));
    assert_eq!(range.end.as_deref(), Some("5"));
    assert!(range.inclusive);
    let mut changed = facts.clone();
    changed.body.children[0].children[1].children[0].children[0]
        .span
        .start += 1;
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
}

#[test]
fn clones_data_constant_patterns_compose_with_guards_and_alternatives() {
    let directory = clone_workspace();
    let source = "mod config { pub const LOW: i32 = -3; } use config::LOW as FLOOR;\nstruct Limits; impl Limits { const TOP: i32 = 10; }\nconst FLAG: bool = true; const BIG: u128 = u128::MAX;\npub fn select(x: i32, b: bool) -> i32 { match x { FLOOR | 0 | 1..=2 if b => 1, Limits::TOP => 2, _ => 3 } }\npub fn flag(x: bool) -> i32 { match x { FLAG => 1, _ => 2 } }\npub fn big(x: u128) -> bool { match x { BIG => true, _ => false } }\npub fn external(x: u128) -> bool { match x { u128::MAX => true, _ => false } }\npub fn range(x: i32) -> bool { match x { FLOOR..=10 => true, _ => false } }\n";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    for name in ["select", "flag", "big"] {
        let function = function_named(functions, name);
        assert_eq!(function["outcome"], "admitted", "{function}");
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        assert!(
            pair.pdg
                .nodes
                .iter()
                .any(|node| node.attributes.construct.as_deref() == Some("match.constant"))
        );
    }
    assert_eq!(function_named(functions, "range")["outcome"], "admitted");
    assert_ne!(function_named(functions, "external")["outcome"], "admitted");
    let mut changed: rstyle_pdg::HirFnFacts =
        serde_json::from_value(function_named(functions, "flag")["facts"].clone()).unwrap();
    let pattern = &mut changed.body.children[0].children[1].children[0];
    assert_eq!(pattern.kind, "pat_constant");
    pattern.lit = Some("false".into());
    assert!(rstyle_pdg::producer::produce(&changed).is_err());

    std::fs::write(
        directory.path().join("src/lib.rs"),
        source.replace("LOW: i32 = -3", "LOW: i32 = -4"),
    )
    .unwrap();
    let (output, edited) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{edited}");
    assert_ne!(
        function_named(functions, "select")["facts"]["body"],
        function_named(edited["functions"].as_array().unwrap(), "select")["facts"]["body"]
    );
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] != "admitted")
    );
}

#[test]
fn clones_data_resolves_scalar_constants_and_updates_callers_after_edits() {
    let directory = clone_workspace();
    let source = "mod config { pub const DELTA: i32 = -3; }\nuse config::DELTA as CHANGE;\nstruct Limits; impl Limits { const MASK: u32 = (1 << 4) - 1; }\nconst LOW: i128 = i128::MIN; const TOP: u128 = u128::MAX; const EMPTY: () = (); const FLAG: bool = true;\npub fn above(x: i32) -> bool { x > CHANGE }\npub fn mask(x: u32) -> u32 { x & Limits::MASK }\npub fn low() -> i128 { LOW }\npub fn top() -> u128 { TOP }\npub fn unit() { EMPTY }\npub fn flag(b: bool) -> bool { b && FLAG }\npub fn wrapper(x: i32) -> bool { above(x) }\nstatic GLOBAL: u32 = 1; pub fn global() -> u32 { GLOBAL }\nstruct Generic<T>(T); impl<T> Generic<T> { const VALUE: u32 = 1; } pub fn generic() -> u32 { Generic::<u8>::VALUE }\n";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|function| function["outcome"] == "admitted")
            .count(),
        7,
        "{:?}",
        functions
            .iter()
            .map(|function| (&function["facts"]["name"], &function["reasons"]))
            .collect::<Vec<_>>()
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 21);
    check_constant_graphs(functions, "-3");
    std::fs::write(
        directory.path().join("src/lib.rs"),
        source.replace("DELTA: i32 = -3", "DELTA: i32 = -4"),
    )
    .unwrap();
    let (output, edited) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{edited}");
    let actual = edited["functions"].as_array().unwrap();
    assert_eq!(
        function_named(functions, "wrapper")["facts"]["body"],
        function_named(actual, "wrapper")["facts"]["body"]
    );
    assert_ne!(
        function_named(functions, "above")["facts"]["body"],
        function_named(actual, "above")["facts"]["body"]
    );
    check_constant_graphs(actual, "-4");
    assert_eq!(function_named(actual, "wrapper")["outcome"], "admitted");
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] != "admitted")
    );
}

fn function_named<'a>(functions: &'a [Value], name: &str) -> &'a Value {
    functions
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

fn check_constant_graphs(functions: &[Value], threshold: &str) {
    for (name, expected) in [
        ("above", threshold),
        ("mask", "15"),
        ("low", "-170141183460469231731687303715884105728"),
        ("top", "340282366920938463463374607431768211455"),
        ("unit", "()"),
        ("flag", "true"),
    ] {
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function_named(functions, name)["facts"].clone()).unwrap();
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let constants: Vec<_> = pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some("constant.value"))
            .collect();
        assert_eq!(constants.len(), 1, "{name}");
        assert_eq!(
            constants[0]
                .attributes
                .literal
                .as_ref()
                .unwrap()
                .known_value()
                .unwrap(),
            expected
        );
        assert_eq!(constants[0].category, rstyle_pdg::NodeCategory::Computation);
    }
}

#[test]
fn clones_data_integer_ranges_compose_with_alternatives_guards_and_open_bounds() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn classify(x: i128, b: bool) -> i32 { match x { ..=-6 if b => 1, -5..0 => 2, 0 | 1..=3 if b => 3, 4.. => 4, _ => 5 } }\npub fn wide(x: u128) -> bool { match x { 0x80..=340282366920938463463374607431768211455u128 => true, _ => false } }\npub fn narrow(x: i8) -> bool { match x { -128..127 => true, _ => false } }\npub fn characters(x: char) -> bool { match x { 'a'..='z' => true, _ => false } }\nconst LIMIT: i32 = 9;\npub fn constant_range(x: i32) -> bool { match x { 0..=LIMIT => true, _ => false } }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|function| function["outcome"] == "admitted")
            .count(),
        4,
        "{:?}",
        functions
            .iter()
            .map(|function| (&function["facts"]["name"], &function["reasons"]))
            .collect::<Vec<_>>()
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 6);
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(
        functions
            .iter()
            .find(|function| function["facts"]["name"] == "classify")
            .unwrap()["facts"]
            .clone(),
    )
    .unwrap();
    check_range_pattern_graph(&facts);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded")
    );
}

fn check_range_pattern_graph(facts: &rstyle_pdg::HirFnFacts) {
    let mut pair = rstyle_pdg::producer::produce(facts).unwrap();
    let ranges: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("match.range"))
        .collect();
    assert_eq!(ranges.len(), 3);
    assert_eq!(
        pair.pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some("match.or-patterns"))
            .count(),
        1
    );
    assert_eq!(
        pair.pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some("match.guard"))
            .count(),
        2
    );
    check_range_descriptor(
        ranges[0]
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value()
            .unwrap(),
    );
    let range = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("match.range"))
        .unwrap();
    range.attributes.literal = Some(rstyle_pdg::StringFact::known("{}"));
    assert!(rstyle_pdg::producer::verify(&pair, facts).is_err());
    check_range_source_payload(facts);
}

fn check_range_descriptor(value: &str) {
    let first: rstyle_pdg::scalar_patterns::IntegerRange = serde_json::from_str(value).unwrap();
    assert_eq!(
        first,
        rstyle_pdg::scalar_patterns::IntegerRange {
            start: None,
            end: Some("-6".into()),
            inclusive: true,
        }
    );
}

fn check_range_source_payload(facts: &rstyle_pdg::HirFnFacts) {
    let mut changed = facts.clone();
    let pattern = &mut changed.body.children[0].children[1].children[0];
    assert_eq!(pattern.kind, "pat_range");
    pattern.lit = Some(
        rstyle_pdg::scalar_patterns::IntegerRange {
            start: None,
            end: Some("-6".into()),
            inclusive: false,
        }
        .text(),
    );
    assert_eq!(
        rstyle_pdg::classify(&changed).status,
        rstyle_pdg::EligibilityVerdict::Ineligible
    );
}

#[test]
fn clones_data_literal_or_patterns_compose_with_guards_and_preserve_legacy_boundaries() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn choose(x: i32, b: bool) -> i32 { match x { -1 | 2 | 3 if b => 7, 2 | 3 => 8, _ => 9 } }\npub fn other(x: i32, b: bool) -> i32 { match x { -1 | 2 | 3 if b => 7, 2 | 3 => 8, _ => 9 } }\npub fn boolean(x: bool) -> i32 { match x { true | false => 1, _ => 2 } }\npub fn wildcard(x: i32) -> i32 { match x { 0 | _ => 1, _ => 2 } }\npub fn range(x: i32) -> i32 { match x { 1..=3 => 1, _ => 2 } }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["extraction_policy"]["version"], "3.35.0");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|function| function["outcome"] == "admitted")
            .count(),
        5,
        "{report}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
    assert_eq!(report["pair_evaluation_complete"], true);
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(
        functions
            .iter()
            .find(|function| function["facts"]["name"] == "choose")
            .unwrap()["facts"]
            .clone(),
    )
    .unwrap();
    check_or_pattern_graph(&facts);
    let (output, warm) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{warm}");
    assert_eq!(report["matching"], warm["matching"]);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded")
    );
}

fn check_or_pattern_graph(facts: &rstyle_pdg::HirFnFacts) {
    let mut pair = rstyle_pdg::producer::produce(facts).unwrap();
    assert_eq!(
        pair.pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some("match.guard"))
            .count(),
        1
    );
    let selectors: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("match.or"))
        .collect();
    assert_eq!(selectors.len(), 2);
    assert_eq!(
        selectors[0]
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value()
            .unwrap(),
        "[\"-1\",\"2\",\"3\"]"
    );
    let selector = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("match.or"))
        .unwrap();
    selector.attributes.literal = Some(rstyle_pdg::StringFact::known("[\"4\"]"));
    assert!(rstyle_pdg::producer::verify(&pair, facts).is_err());
}

#[test]
fn clones_scalar_match_is_versioned_and_evaluates_every_admitted_pair() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn f(x: i32) -> i32 { match x { 1 => 7, _ => 9 } }\npub fn g(x: i32) -> i32 { match x { 1 => 7, _ => 9 } }\npub fn h(x: i32) -> i32 { match x { 1 => 7, _ => 9 } }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-expanded-v1",
            "--all-targets",
            "--jobs",
            "2",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["extraction_policy"]["version"], "0.7.0");
    assert_eq!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        6
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
    assert_eq!(report["pair_enumeration_complete"], true);
    let (output, baseline) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert!(
        baseline["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

#[test]
fn clones_guarded_match_capture_is_explicit_and_trace_invariant() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn f(x: i32, b: bool) -> i32 { match x { 1 if b => 7, 1 if !b => 8, _ => 9 } }\npub fn g(x: i32, b: bool) -> i32 { match x { 1 if b => 7, 1 if !b => 8, _ => 9 } }\n").unwrap();
    let (output, ordinary) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let (output, traced) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1", "--trace"],
    );
    assert!(output.status.success());
    assert_eq!(ordinary["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert_eq!(
        ordinary["matching"]["pairs"][0]["decision"],
        traced["matching"]["pairs"][0]["decision"]
    );
    let (output, excluded) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert!(
        excluded["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

#[test]
fn clones_conditional_profile_records_active_resolution_and_removed_unknowns() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("Cargo.toml"), "[package]\nname='conditional_fixture'\nversion='0.1.0'\nedition='2021'\n[features]\nextra=[]\n[workspace]\n").unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "/// documented\n#[inline]\npub const fn f(x: i32) -> i32 { #[cfg(feature=\"extra\")] let y = x; x }\n#[cfg(feature=\"extra\")] pub fn present(x: i32) -> i32 { x }\npub fn unsupported() { #[cfg(any())] missing(); }\n").unwrap();
    let args = ["--extraction-profile", "hir-expanded-v1"];
    let (output, inactive) = clones(directory.path(), &args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_conditional_removed_unknowns(&inactive);
    let (output, active) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-expanded-v1",
            "--features",
            "extra",
        ],
    );
    assert!(
        output.status.success(),
        "stderr={} failures={} capture={}",
        String::from_utf8_lossy(&output.stderr),
        active["handoff_failures"],
        active["capture_failures"]
    );
    assert_eq!(
        active["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        2
    );
    assert_eq!(active["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert!(
        active["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["facts"]["name"] == "unsupported" && f["outcome"] == "excluded")
    );
}

#[test]
fn clones_scalar_loops_use_verified_cycles_and_exhaustive_concurrent_capture() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn f(mut x: i32, b: bool) -> i32 { while b { x = 1; if x > 2 { break; } continue; } x }\npub fn g(mut x: i32, b: bool) -> i32 { while b { x = 1; if x > 2 { break; } continue; } x }\npub fn forever() { loop {} }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-expanded-v1",
            "--all-targets",
            "--jobs",
            "2",
            "--trace",
        ],
    );
    assert!(
        output.status.success(),
        "failures={} capture={} stderr={}",
        report["handoff_failures"],
        report["capture_failures"],
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        6
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
    assert_eq!(report["pair_evaluation_complete"], true);
    let (output, legacy) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

#[test]
fn clones_named_primitive_summaries_cross_real_capture_and_preserve_r0_exclusions() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn f(x: i32, y: u32) -> u32 { x.unsigned_abs().wrapping_mul(y) }\npub fn g(x: i64, y: u64) -> u64 { x.unsigned_abs().wrapping_mul(y) }\npub fn unsupported(x: u32) -> u32 { x.wrapping_add(1) }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1", "--trace"],
    );
    assert!(
        output.status.success(),
        "failures={} capture={} stderr={}",
        report["handoff_failures"],
        report["capture_failures"],
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        2
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["facts"]["name"] == "unsupported" && f["outcome"] == "excluded")
    );
    let (output, plain) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1"],
    );
    assert!(output.status.success());
    assert_eq!(
        report["matching"]["pairs"][0]["decision"],
        plain["matching"]["pairs"][0]["decision"]
    );
    let (output, legacy) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

fn assert_inherited_expanded_selection_is_ignored(directory: &std::path::Path) {
    let inherited = Command::new(cargo_bin("cargo-rstyle"))
        .args(["clones", "--format", "json"])
        .current_dir(directory)
        .env("RSTYLE_DRIVER", driver())
        .env("RSTYLE_HIR_EXTRACTION_PROFILE", "hir-calls-v1")
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap();
    assert!(inherited.status.success());
    let inherited: Value = serde_json::from_slice(&inherited.stdout).unwrap();
    assert_eq!(inherited["extraction_policy"]["id"], "compat.rust.hir-r0");
    assert_eq!(inherited["matching"]["pairs"].as_array().unwrap().len(), 0);
}

fn assert_conditional_removed_unknowns(inactive: &Value) {
    assert!(
        inactive["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["facts"]["name"] == "f" && f["outcome"] == "unknown")
    );
    assert!(
        !inactive["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["facts"]["name"] == "present")
    );
}

#[test]
fn clones_conditional_bom_inventory_preserves_original_file_offsets() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "\u{feff}pub fn f(x: i32) -> i32 { x }\r\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let definitions = report["owned_source_inventory"][0]["source_functions"]
        .as_array()
        .unwrap();
    let definition = definitions.iter().find(|f| f["name"] == "f").unwrap();
    assert_eq!(definition["span"]["start"], 3);
    assert_eq!(definition["presence"], "active");
    assert_eq!(definition["active_functions"], serde_json::json!(["f"]));
}

#[test]
fn clones_source_inventory_keeps_nested_function_ownership_separate() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "#[inline]\npub fn outer() { fn inner() {} inner(); }\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-expanded-v1"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let definitions = report["owned_source_inventory"][0]["source_functions"]
        .as_array()
        .unwrap();
    assert_eq!(definitions.len(), 2);
    let outer = definitions.iter().find(|f| f["name"] == "outer").unwrap();
    let inner = definitions.iter().find(|f| f["name"] == "inner").unwrap();
    assert_eq!(outer["active_functions"], serde_json::json!(["outer"]));
    assert_eq!(
        inner["active_functions"],
        serde_json::json!(["outer::inner"])
    );
    assert_eq!(outer["function_span"]["start"], 10);
    assert_eq!(inner["function_span"]["start"], 27);
}

#[test]
fn clones_helpers_verify_complete_acyclic_scalar_families() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub mod predicates { pub fn positive(x: i32) -> bool { x > 0 } }\nuse predicates::positive as alias;\npub fn first(x: i32) -> bool { alias(x) }\npub fn second(x: i32) -> bool { predicates::positive(x) }\npub fn third(x: i32) -> bool { first(x) }\nfn unit_helper() {}\npub fn conditional(b: bool) { if b { unit_helper(); } }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1", "--trace"],
    );
    assert!(
        output.status.success(),
        "capture={}, handoff={}",
        report["capture_failures"],
        report["handoff_failures"]
    );
    assert_eq!(report["extraction_selection"], "hir-helpers-v1");
    assert_eq!(report["extraction_policy"]["id"], "rstyle.rust.hir-helpers");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 6);
    assert!(
        functions
            .iter()
            .all(|function| function["outcome"] == "admitted"),
        "{report}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["pair_enumeration_complete"], true);
    for function in functions.iter().filter(|function| {
        matches!(
            function["facts"]["name"].as_str(),
            Some("first" | "second" | "third")
        )
    }) {
        let key = &function["key"];
        let projected = report["matching"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| &record["key"] == key)
            .unwrap();
        assert_eq!(
            projected["projection"]["features"],
            serde_json::json!([0, 2, 3, 0, 0, 0, 1, 3, 1])
        );
    }
}

#[test]
fn clones_helpers_verify_serialized_dependencies_and_transitive_edits() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub mod predicates;\npub fn first(x: i32) -> bool { predicates::positive(x) }\npub fn second(x: i32) -> bool { first(x) }\n").unwrap();
    let callee = directory.path().join("src/predicates.rs");
    std::fs::write(&callee, "pub fn positive(x: i32) -> bool { x > 0 }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success());
    let wrapper = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "second")
        .unwrap();
    for corruption in ["manifest", "source", "signature", "effects"] {
        reject_helper(wrapper, corruption);
    }
    std::fs::write(&callee, "pub fn positive(x: i32) -> bool { x >= 0 }\n").unwrap();
    let (output, edited) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success());
    let changed = edited["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "second")
        .unwrap();
    assert_eq!(wrapper["facts"]["body"], changed["facts"]["body"]);
    assert_ne!(
        wrapper["unit_evidence"]["namespace"],
        changed["unit_evidence"]["namespace"]
    );
    check_native_helper(directory.path(), &callee);
}

fn check_native_helper(directory: &Path, callee: &Path) {
    std::fs::write(
        callee,
        "pub fn positive(x: i32) -> bool { let n = x as usize; n > 0 }\n",
    )
    .unwrap();
    let (output, native) = clones(directory, &["--extraction-profile", "hir-helpers-v1"]);
    assert!(output.status.success());
    let function = native["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "second")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let imported = rstyle_ccgraph::admit_produced(unit.pair(index).unwrap(), &facts).unwrap();
    assert!(imported.native_width_involved);
}

#[test]
fn clones_helpers_exclude_unsupported_recursive_and_panicking_callees() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn overflow(x: i32) -> i32 { x + 1 }\npub fn wrapper(x: i32) -> i32 { overflow(x) }\npub fn recursive(x: i32) -> i32 { recursive(x) }\npub fn diverges() -> i32 { loop {} }\npub fn divergent_wrapper() -> i32 { diverges() }\nfn unsupported(x: &i32) -> i32 { *x }\npub fn unknown(x: i32) -> i32 { unsupported(&x) }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(
        output.status.success(),
        "capture={}, handoff={}",
        report["capture_failures"],
        report["handoff_failures"]
    );
    for name in ["wrapper", "recursive", "divergent_wrapper", "unknown"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}

#[test]
fn clones_helpers_are_order_independent_and_keep_argument_positions() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "mod left { pub fn same(x: i32, y: i32) -> bool { x < y } }\nmod right { pub fn same(x: i32, y: i32) -> bool { x > y } }\npub fn wrapper(b: bool, x: i32, y: i32) -> bool { left::same(if b { x } else { y }, y) }\npub fn other(x: i32, y: i32) -> bool { right::same(x, y) }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{report}");
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 6);
    let facts: Vec<rstyle_pdg::HirFnFacts> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|function| serde_json::from_value(function["facts"].clone()).unwrap())
        .collect();
    assert_capture_order(&facts);
    let (output, traced) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1", "--trace"],
    );
    assert!(output.status.success(), "{traced}");
    assert_eq!(
        report["matching"]["functions"],
        traced["matching"]["functions"]
    );
    let outcomes = |report: &Value| {
        report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair["first"].clone(),
                    pair["second"].clone(),
                    pair["decision"].clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(outcomes(&report), outcomes(&traced));
}

fn assert_capture_order(facts: &[rstyle_pdg::HirFnFacts]) {
    let mut forward = facts.to_vec();
    for function in &mut forward {
        clear_helper_dependencies(&mut function.body);
    }
    let mut reversed = forward.clone();
    reversed.reverse();
    let forward = rstyle_pdg::unit_store::UnitStore::capture(forward).unwrap();
    let reversed = rstyle_pdg::unit_store::UnitStore::capture(reversed).unwrap();
    assert_eq!(forward.helper_count(), reversed.helper_count());
    assert_eq!(admitted_unit(&forward), admitted_unit(&reversed));
}

fn admitted_unit(
    store: &rstyle_pdg::unit_store::UnitStore,
) -> Vec<rstyle_ccgraph::ImportedFunction> {
    let analysis = store.analysis();
    let mut admitted: Vec<_> = (0..store.functions().len())
        .filter_map(|index| {
            let pair = store.pair(index)?;
            Some(rstyle_ccgraph::admit_produced(pair, &analysis.function(index).unwrap()).unwrap())
        })
        .collect();
    admitted.sort_by(|a, b| a.key.cmp(&b.key));
    admitted
}

#[test]
fn shared_unit_owns_diamond_helpers_once() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn leaf(x: i32) -> bool { x > 0 }\npub fn left(x: i32) -> bool { leaf(x) }\npub fn right(x: i32) -> bool { leaf(x) }\npub fn diamond(x: i32) -> bool { left(x) && right(x) }\npub fn repeated(x: i32) -> bool { leaf(x) && leaf(x) && leaf(x) }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{report}");
    let mut facts: Vec<rstyle_pdg::HirFnFacts> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|function| serde_json::from_value(function["facts"].clone()).unwrap())
        .collect();
    for function in &mut facts {
        clear_helper_dependencies(&mut function.body);
    }
    let mut reversed = facts.clone();
    reversed.reverse();
    let store = rstyle_pdg::verification_policy::generated(|| {
        rstyle_pdg::unit_store::UnitStore::capture(facts)
    })
    .unwrap();
    let reordered = rstyle_pdg::unit_store::UnitStore::capture(reversed).unwrap();
    assert_eq!(store.helper_count(), 5);
    assert_eq!(store.graph_count(), 5);
    assert_eq!(reordered.helper_count(), 5);
    assert_shared_unit(&store);
    assert_shared_unit(&reordered);
    assert_unit_bundle(&store);
}

fn assert_unit_bundle(store: &rstyle_pdg::unit_store::UnitStore) {
    let mut bytes = Vec::new();
    store.write_bundle(&mut bytes).unwrap();
    let imported = rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes).unwrap();
    assert_shared_unit(&imported);
    let mut round_trip = Vec::new();
    imported.write_bundle(&mut round_trip).unwrap();
    assert_eq!(bytes, round_trip);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    assert_renumbered_unit(store, &wire);
    for corruption in [
        "schema", "helper", "function", "kind", "link", "depth", "summary", "missing", "graph",
        "pair",
    ] {
        let mut changed = wire.clone();
        corrupt_unit_bundle(&mut changed, corruption);
        let edited = serde_json::to_vec(&changed).unwrap();
        assert!(
            rstyle_pdg::unit_store::UnitStore::from_bundle(&edited).is_err(),
            "{corruption}"
        );
        assert!(
            rstyle_pdg::verification_policy::generated(|| {
                rstyle_pdg::unit_store::UnitStore::from_bundle(&edited)
            })
            .is_err(),
            "generated import: {corruption}"
        );
    }
}

fn assert_renumbered_unit(store: &rstyle_pdg::unit_store::UnitStore, wire: &Value) {
    let mut renumbered = wire.clone();
    let helpers = renumbered["helpers"].as_array().unwrap().len() as u64;
    let graphs = renumbered["graphs"].as_array().unwrap().len() as u64;
    renumbered["helpers"].as_array_mut().unwrap().reverse();
    renumbered["graphs"].as_array_mut().unwrap().reverse();
    for helper in renumbered["helpers"].as_array_mut().unwrap() {
        helper["graph"] = (graphs - 1 - helper["graph"].as_u64().unwrap()).into();
    }
    renumber_ids(&mut renumbered["certified"], helpers);
    renumber_ids(&mut renumbered["roots"], graphs);
    renumber_helper_references(&mut renumbered["graphs"], helpers);
    let imported =
        rstyle_pdg::unit_store::UnitStore::from_bundle(&serde_json::to_vec(&renumbered).unwrap())
            .unwrap();
    assert_shared_unit(&imported);
    for index in 0..store.functions().len() {
        let before = store.pair(index).unwrap();
        let after = imported.pair(index).unwrap();
        assert_eq!(
            serde_json::to_value(&before.pdg.nodes).unwrap(),
            serde_json::to_value(&after.pdg.nodes).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&before.pdg.edges).unwrap(),
            serde_json::to_value(&after.pdg.edges).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&before.cfg.edges).unwrap(),
            serde_json::to_value(&after.cfg.edges).unwrap()
        );
    }
}

fn renumber_ids(ids: &mut Value, count: u64) {
    for id in ids.as_array_mut().unwrap() {
        if let Some(index) = id.as_u64() {
            *id = (count - 1 - index).into();
        }
    }
}

fn renumber_helper_references(value: &mut Value, count: u64) {
    match value {
        Value::Object(fields) => {
            if fields.get("identity").and_then(Value::as_str)
                == Some("rstyle.helper.unit-reference/1")
            {
                let index = fields["helper"].as_u64().unwrap();
                fields.insert("helper".into(), (count - 1 - index).into());
            }
            for field in fields.values_mut() {
                renumber_helper_references(field, count);
            }
        }
        Value::Array(values) => {
            for value in values {
                renumber_helper_references(value, count);
            }
        }
        _ => {}
    }
}

fn corrupt_unit_bundle(wire: &mut Value, corruption: &str) {
    match corruption {
        "schema" => wire["schema"] = "unknown version".into(),
        "helper" => wire["certified"][0] = u32::MAX.into(),
        "function" => wire["helpers"][0]["function"] = u32::MAX.into(),
        "kind" => wire["helpers"][0]["function"] = serde_json::json!({"helper": 0}),
        "link" => {
            let links = wire["links"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|links| !links.as_array().unwrap().is_empty())
                .unwrap();
            links[0]["function"] = u32::MAX.into();
        }
        "graph" => wire["roots"][0] = u32::MAX.into(),
        "pair" => wire["graphs"][0]["cfg"]["function"]["id"] = "disagreement".into(),
        "depth" => wire["helpers"][0]["depth"] = u32::MAX.into(),
        "summary" => wire["helpers"][0]["summary"]["always_returns"] = false.into(),
        "missing" => {
            wire["helpers"] = serde_json::json!([]);
            wire["certified"].as_array_mut().unwrap().fill(Value::Null);
        }
        _ => unreachable!(),
    }
}

fn assert_shared_unit(store: &rstyle_pdg::unit_store::UnitStore) {
    use rstyle_pdg::unit_store::EvidenceFacts;
    let analysis = store.analysis();
    for (index, facts) in store.functions().iter().enumerate() {
        let view = analysis.function(index).unwrap();
        rstyle_pdg::producer::verify(store.pair(index).unwrap(), &view).unwrap();
        let imported = rstyle_ccgraph::admit_produced(store.pair(index).unwrap(), &view).unwrap();
        let produced = rstyle_pdg::producer::produce(&view).unwrap();
        let reproduced = rstyle_ccgraph::admit_produced(&produced, &view).unwrap();
        assert_eq!(imported, reproduced);
        let generated =
            rstyle_pdg::verification_policy::generated(|| rstyle_pdg::producer::produce(&view))
                .unwrap();
        let strict = rstyle_pdg::producer::produce(&view).unwrap();
        assert_shared_reference_checks(&strict, &view);
        assert_eq!(
            generated.serialize().unwrap().pdg_bytes,
            strict.serialize().unwrap().pdg_bytes
        );
        assert!(!rstyle_pdg::helper_calls::native_width(&view).unwrap());
        if facts.name.ends_with("repeated") {
            let mut calls = Vec::new();
            shared_call_nodes(&facts.body, &mut calls);
            assert_eq!(calls.len(), 3);
            let first = view.helper(calls[0]).unwrap().unwrap();
            for call in calls {
                let helper = view.helper(call).unwrap().unwrap();
                assert!(std::ptr::eq(first.summary(), helper.summary()));
                assert!(
                    rstyle_pdg::helper_calls::direct_resolution(call).unwrap()
                        [rstyle_pdg::helper_calls::KEY]
                        .is_null()
                );
            }
        }
    }
}

fn assert_shared_reference_checks(
    pair: &rstyle_pdg::producer::Pair,
    view: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    let key = rstyle_pdg::extraction_profile::EVIDENCE_KEY;
    assert!(rstyle_pdg::producer::verify(pair, view.facts()).is_err());
    let mut contradictory = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    contradictory.pdg.extensions.get_mut(key).unwrap()["frontend"] =
        serde_json::to_value(view.facts()).unwrap();
    contradictory
        .cfg
        .extensions
        .insert(key.into(), contradictory.pdg.extensions[key].clone());
    assert!(
        rstyle_pdg::verification_policy::generated(|| rstyle_pdg::producer::verify(
            &contradictory,
            view
        ))
        .is_err()
    );
    let mut wrong_unit = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    wrong_unit.pdg.extensions.get_mut(key).unwrap()["frontend_ref"]["namespace"] =
        Value::String("other unit".into());
    wrong_unit
        .cfg
        .extensions
        .insert(key.into(), wrong_unit.pdg.extensions[key].clone());
    assert!(
        rstyle_pdg::verification_policy::generated(|| rstyle_pdg::producer::verify(
            &wrong_unit,
            view
        ))
        .is_err()
    );
}

fn shared_call_nodes<'a>(node: &'a rstyle_pdg::HirNode, calls: &mut Vec<&'a rstyle_pdg::HirNode>) {
    if rstyle_pdg::helper_calls::direct_resolution(node).is_some() {
        calls.push(node);
    }
    for child in &node.children {
        shared_call_nodes(child, calls);
    }
}

fn clear_helper_dependencies(node: &mut rstyle_pdg::HirNode) {
    if let Some(mut resolution) = rstyle_pdg::helper_calls::direct_resolution(node) {
        resolution
            .as_object_mut()
            .unwrap()
            .remove(rstyle_pdg::helper_calls::KEY);
        node.res.as_mut().unwrap().description = serde_json::to_string(&resolution).unwrap();
    }
    for child in &mut node.children {
        clear_helper_dependencies(child);
    }
}

#[test]
fn clones_helpers_resource_budget_is_incomplete_execution() {
    let directory = clone_workspace();
    let mut source = "pub fn leaf(x: i32) -> bool { x > 0 }\n".to_owned();
    for depth in 0..10 {
        let callee = if depth == 0 {
            "leaf".to_owned()
        } else {
            format!("level{}", depth - 1)
        };
        source.push_str(&format!(
            "pub fn level{depth}(x: i32) -> bool {{ {callee}(x) }}\n"
        ));
    }
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(!report["capture_failures"].as_array().unwrap().is_empty());
    let diagnostic = format!("{} {}", String::from_utf8_lossy(&output.stderr), report);
    assert!(diagnostic.contains("budget"), "{diagnostic}");
}

#[test]
fn clones_helpers_capture_concrete_interfaces_without_admitting_data_functions() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        format!("\u{feff}{}", "pub struct Inner { pub value: i32 }\npub struct Outer {\n pub inner: Inner,\n pub pair: (i16, bool),\n pub bytes: [u8; 3]\n}\nimpl Outer { pub fn get(&self) -> i32 { self.inner.value } pub fn borrow(&self) -> &Inner { &self.inner } }\npub fn alias(a: &Outer, b: &Outer) -> bool { a.inner.value == b.inner.value }\npub struct NeedsDrop;\nimpl Drop for NeedsDrop { fn drop(&mut self) {} }\npub fn drop_input(_: &NeedsDrop) {}\npub fn cell_input(_: &std::cell::Cell<i32>) {}\npub fn bytes(x: &[u8]) -> usize { x.len() }\npub fn text(x: &str) -> usize { x.len() }\n".replace('\n', "\r\n"))).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert!(
        functions
            .iter()
            .all(|function| function["outcome"] == "excluded"),
        "{report}"
    );
    for function in functions {
        let value =
            &function["facts"]["build"]["frontend_options"][rstyle_pdg::concrete_types::KEY];
        let observations: rstyle_pdg::concrete_types::InterfaceObservations =
            serde_json::from_value(value.clone()).unwrap();
        observations.validate().unwrap();
        check_concrete_interface(&function["facts"]["name"], &observations);
        check_formal_field_observations(function);
        check_nominal_unit_binding(function, &report);
    }
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["matching"]["pairs"], serde_json::json!([]));
}

fn check_nominal_unit_binding(function: &Value, report: &Value) {
    use rstyle_pdg::nominal_binding::{self, PackageContext};
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let context: PackageContext =
        serde_json::from_value(report["expected_targets"][0]["package_context"].clone()).unwrap();
    assert_eq!(
        context.manifest_digest,
        rstyle_pdg::concrete_types::digest(context.manifest_bytes.as_bytes())
    );
    let actual = nominal_binding::bind_unit(std::slice::from_ref(&facts), &context).unwrap();
    checked_build_context(function, report);
    assert_eq!(
        serde_json::to_value(&actual.bindings[0]).unwrap(),
        function["nominal_unit_binding"]
    );
    check_nominal_evidence_corruption(
        &facts,
        &context,
        !actual.bindings[0].as_ref().unwrap().declarations.is_empty(),
    );
    for change in ["name", "version", "manifest", "root"] {
        let mut changed = context.clone();
        match change {
            "name" => changed.name.push('x'),
            "version" => changed.version.push('x'),
            "manifest" => {
                changed.manifest_bytes = format!("{}\n# changed", changed.manifest_bytes).into();
                changed.manifest_digest =
                    rstyle_pdg::concrete_types::digest(changed.manifest_bytes.as_bytes());
            }
            "root" => changed.manifest_path = PathBuf::from("/different/package/Cargo.toml"),
            _ => unreachable!(),
        }
        if !actual.bindings[0].as_ref().unwrap().declarations.is_empty() {
            assert!(
                nominal_binding::bind_unit(std::slice::from_ref(&facts), &changed).is_err(),
                "{change}"
            );
        }
    }
}

fn check_nominal_evidence_corruption(
    facts: &rstyle_pdg::HirFnFacts,
    context: &rstyle_pdg::nominal_binding::PackageContext,
    has_declarations: bool,
) {
    use rstyle_pdg::{concrete_types, declaration_origins, nominal_binding};
    for key in [concrete_types::KEY, declaration_origins::KEY] {
        let mut changed = facts.clone();
        changed.build.frontend_options.remove(key);
        assert!(nominal_binding::bind_unit(&[changed], context).is_err());
    }
    if has_declarations {
        let mut changed = facts.clone();
        changed.fn_span.start += 1;
        assert!(nominal_binding::bind_unit(&[changed], context).is_err());
        let mut changed = facts.clone();
        let value = changed
            .build
            .frontend_options
            .get_mut(declaration_origins::KEY)
            .unwrap();
        let file = value["declarations"][0]["file_bytes"]
            .as_str()
            .unwrap()
            .to_owned()
            + "\n// changed file";
        value["declarations"][0]["file_digest"] =
            serde_json::json!(concrete_types::digest(file.as_bytes()));
        value["declarations"][0]["file_bytes"] = serde_json::json!(file);
        assert!(nominal_binding::bind_unit(&[facts.clone(), changed], context).is_err());
    }
}

fn check_formal_field_observations(function: &Value) {
    use rstyle_pdg::field_locations::{self, FieldObservations, Location};
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let observations: FieldObservations =
        serde_json::from_value(facts.build.frontend_options[field_locations::KEY].clone()).unwrap();
    let interface = serde_json::from_value(
        facts.build.frontend_options[rstyle_pdg::concrete_types::KEY].clone(),
    )
    .unwrap();
    let origins = serde_json::from_value(
        facts.build.frontend_options[rstyle_pdg::declaration_origins::KEY].clone(),
    )
    .unwrap();
    rstyle_pdg::declaration_origins::verify(&interface, &origins).unwrap();
    let expected = match function["facts"]["name"].as_str().unwrap() {
        name if name.ends_with("get") => 2,
        name if name.ends_with("borrow") => 1,
        "alias" => 4,
        _ => 0,
    };
    assert_eq!(observations.operations.len(), expected, "{observations:?}");
    field_locations::verify(&facts, &observations).unwrap();
    assert!(
        observations
            .operations
            .iter()
            .all(|operation| matches!(operation.location, Location::Captured { .. }))
    );
    if expected > 0 {
        check_field_observation_tampering(&facts, &observations);
    }
    if expected == 4 {
        assert_eq!(
            observations.possibly_aliasing_formals,
            [[0, 0], [0, 1], [1, 1]]
        );
    }
}

fn check_field_observation_tampering(
    facts: &rstyle_pdg::HirFnFacts,
    original: &rstyle_pdg::field_locations::FieldObservations,
) {
    use rstyle_pdg::field_locations;
    for change in [
        "identity",
        "missing",
        "duplicate",
        "order",
        "snippet",
        "span",
        "root",
        "type",
        "field",
        "nominal",
        "alias",
    ] {
        let mut observations = original.clone();
        match change {
            "identity" => observations.identity.push_str("-unknown"),
            "missing" => {
                observations.operations.pop();
            }
            "duplicate" => observations
                .operations
                .push(observations.operations[0].clone()),
            "order" if observations.operations.len() > 1 => observations.operations.swap(0, 1),
            "order" => observations.operations[0].hir.local += 1,
            "snippet" => observations.operations[0].snippet.push(' '),
            "span" => observations.operations[0].span.start += 1,
            "alias" => {
                observations.possibly_aliasing_formals.pop();
            }
            _ => change_field_location(&mut observations.operations[0].location, change),
        }
        assert!(
            field_locations::verify(facts, &observations).is_err(),
            "{change}"
        );
    }
}

fn change_field_location(location: &mut rstyle_pdg::field_locations::Location, change: &str) {
    use rstyle_pdg::field_locations::{Location, Projection};
    let Location::Captured {
        parameter,
        path,
        value,
    } = location
    else {
        unreachable!()
    };
    match change {
        "root" => *parameter += 1,
        "type" => *value = rstyle_pdg::concrete_types::TypeTree::Scalar("u128".into()),
        "field" => path.push(Projection::TupleField { position: 9 }),
        "nominal" => {
            let projection = path
                .iter_mut()
                .find(|p| matches!(p, Projection::StructField { .. }))
                .unwrap();
            let Projection::StructField { nominal, .. } = projection else {
                unreachable!()
            };
            nominal.definition_path.push_str("Changed");
        }
        _ => unreachable!(),
    }
}

fn check_concrete_interface(
    name: &Value,
    observations: &rstyle_pdg::concrete_types::InterfaceObservations,
) {
    use rstyle_pdg::concrete_types::{Observation, TypeTree};
    let name = name.as_str().unwrap();
    let observed = &observations.parameters[0];
    if name == "drop_input" || name == "cell_input" || name.ends_with("::drop") {
        assert!(
            matches!(observed, Observation::Unsupported { .. }),
            "{observations:?}"
        );
    } else {
        let Observation::Captured {
            ty: TypeTree::Shared(referent),
        } = observed
        else {
            panic!("Missing shared concrete interface: {observations:?}");
        };
        match name {
            "bytes" => assert_eq!(
                **referent,
                TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into())))
            ),
            "text" => assert_eq!(**referent, TypeTree::Str),
            _ => check_nested_concrete_type(referent),
        }
    }
}

fn check_nested_concrete_type(tree: &rstyle_pdg::concrete_types::TypeTree) {
    use rstyle_pdg::concrete_types::TypeTree;
    let TypeTree::Struct {
        nominal,
        source,
        fields,
    } = tree
    else {
        panic!("{tree:?}")
    };
    assert_eq!(nominal.definition_path.rsplit("::").next(), Some("Outer"));
    assert!(source.contains("\r\n"));
    assert_eq!(
        nominal.declaration_digest,
        rstyle_pdg::concrete_types::digest(source.as_bytes())
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["inner", "pair", "bytes"]
    );
    let TypeTree::Struct { fields: inner, .. } = &fields[0].ty else {
        panic!("{tree:?}")
    };
    assert_eq!(inner[0].name, "value");
    assert_eq!(inner[0].ty, TypeTree::Scalar("i32".into()));
    assert_eq!(
        fields[1].ty,
        TypeTree::Tuple(vec![
            TypeTree::Scalar("i16".into()),
            TypeTree::Scalar("bool".into())
        ])
    );
    assert_eq!(
        fields[2].ty,
        TypeTree::Array {
            element: Box::new(TypeTree::Scalar("u8".into())),
            length: 3
        }
    );
}

#[test]
fn clones_data_derived_methods_do_not_require_an_original_function_item() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "#[derive(Debug)] pub struct Record { pub value: u32 }\npub fn scalar(value: u32) -> u32 { value }\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert!(report["handoff_failures"].as_array().unwrap().is_empty());
    assert!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|function| {
                function["facts"]["name"].as_str().unwrap().contains("fmt")
                    && function["outcome"] == "excluded"
            })
    );
}

#[test]
fn clones_data_normalizes_concrete_array_method_results_before_verification() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn bytes(value: u16) -> [u8; 2] { value.to_le_bytes() }\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["functions"][0]["outcome"], "excluded");
    assert!(report["handoff_failures"].as_array().unwrap().is_empty());
}

#[test]
fn clones_data_assertion_argument_receivers_compose_with_partial_flow() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn check(bytes: &[u8]) -> usize { assert!(bytes.len() > 0); bytes.len() }\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    for function in check_data_family(&report, &["check"]) {
        assert_eq!(
            function["extraction_evidence"]["cfg"]["status"], "partial",
            "{function}"
        );
    }
    assert!(report["handoff_failures"].as_array().unwrap().is_empty());
}

#[test]
fn clones_data_large_array_lengths_do_not_expand_capture_work() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn length(_bytes: &[u8; 65536]) -> u32 { 1 }\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    let function = &report["functions"][0];
    assert_eq!(function["outcome"], "admitted", "{function}");
}

#[test]
fn clones_helpers_concrete_capture_budget_is_not_an_exclusion() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("pub fn deep(_: {}u8) {{}}\n", "&".repeat(17)),
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(!output.status.success());
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["assessment_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(
        format!("{} {report}", String::from_utf8_lossy(&output.stderr)).contains("budget"),
        "{report}"
    );
}

#[test]
fn clones_expanded_multi_package_targets_are_exhaustive_and_schedule_invariant() {
    let directory = expanded_multi_package_workspace();
    let (output, concurrent) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-expanded-v1",
            "--all-targets",
            "--jobs",
            "2",
        ],
    );
    assert!(
        output.status.success(),
        "capture={} handoff={}",
        concurrent["capture_failures"],
        concurrent["handoff_failures"]
    );
    assert_eq!(concurrent["observed_units"].as_array().unwrap().len(), 4);
    assert_eq!(
        concurrent["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        8
    );
    assert_eq!(
        concurrent["matching"]["pairs"].as_array().unwrap().len(),
        28
    );
    assert_eq!(concurrent["pair_evaluation_complete"], true);
    let (output, serial) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-expanded-v1",
            "--all-targets",
            "--jobs",
            "1",
        ],
    );
    assert!(output.status.success());
    assert_eq!(concurrent["matching"], serial["matching"]);
    assert_expanded_release_package_scope(directory.path());
}

fn expanded_multi_package_workspace() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nmembers=['a','b']\nresolver='2'\n",
    )
    .unwrap();
    for package in ["a", "b"] {
        let root = directory.path().join(package);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname='{package}'\nversion='0.1.0'\nedition='2021'\n"),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub const fn f(x: i32, b: bool) -> u8 { #[cfg(all())] let y = x as u8; match y { 0 if b => 1, _ => y } }\npub fn g(mut x: u32, b: bool) -> u32 { while b { x = x.wrapping_mul(3); if x == 0 { break; } continue; } x }\npub fn forbidden(x: u32) -> u32 { x.wrapping_add(1) }\n").unwrap();
    }
    directory
}

fn assert_expanded_release_package_scope(directory: &Path) {
    let (output, release) = clones(
        directory,
        &[
            "--extraction-profile",
            "hir-expanded-v1",
            "--all-targets",
            "-p",
            "a",
            "--release",
        ],
    );
    assert!(output.status.success());
    assert_eq!(release["matching"]["pairs"].as_array().unwrap().len(), 6);
    assert_eq!(release["pair_evaluation_complete"], true);
}

#[test]
fn clones_formal_field_observations_verify_tuple_paths_and_reject_unproven_roots() {
    let directory = formal_field_workspace();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{report}");
    let mut verified = Vec::new();
    let mut unavailable = Vec::new();
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "excluded");
        let (name, supported) = check_formal_field_fixture(function);
        if supported {
            verified.push(name);
        } else {
            unavailable.push(name);
        }
    }
    verified.sort();
    unavailable.sort();
    assert_eq!(
        verified,
        [
            "deref",
            "nested",
            "nested_shared",
            "nested_tuple",
            "owned",
            "stored_shared",
            "tuple",
            "unicode"
        ]
    );
    assert_eq!(
        unavailable,
        [
            "local",
            "macro_rebind",
            "mutable_borrow",
            "overloaded",
            "rebind",
            "removed",
            "shadow"
        ]
    );
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["matching"]["pairs"], serde_json::json!([]));
}

fn check_formal_field_fixture(function: &Value) -> (String, bool) {
    use rstyle_pdg::field_locations::{self, FieldObservations};
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let observations: FieldObservations =
        serde_json::from_value(facts.build.frontend_options[field_locations::KEY].clone()).unwrap();
    let name = facts.name.rsplit("::").next().unwrap();
    let supported = !matches!(
        name,
        "local"
            | "shadow"
            | "removed"
            | "overloaded"
            | "rebind"
            | "mutable_borrow"
            | "macro_rebind"
    );
    assert_eq!(
        field_locations::verify(&facts, &observations).is_ok(),
        supported,
        "{name}"
    );
    if name == "tuple" {
        check_tuple_field_expectation(&observations);
        check_field_resolution_tampering(&facts, &observations);
    }

    (name.to_owned(), supported)
}

fn formal_field_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
pub struct Inner { pub value: i32 }
pub struct Outer { pub inner: Inner, pub pair: (i16, bool) }
pub fn tuple(x: &(i32, bool)) -> i32 { x.0 }
pub fn nested_tuple(x: &Outer) -> i16 { x.pair.0 }
pub fn local(x: &Outer) -> i32 { let y = x; y.inner.value }
pub fn shadow(x: &Outer) -> i32 { let x = Inner { value: 3 }; x.value }
pub fn nested(x: &Outer) -> i32 { fn owned(y: &Inner) -> i32 { y.value } x.inner.value }
pub fn removed(x: &Outer) -> i32 { #[cfg(any())] { let _ = x.inner.value; } x.inner.value }
pub struct Wrapper { pub inner: Inner }
impl std::ops::Deref for Wrapper { type Target = Inner; fn deref(&self) -> &Inner { &self.inner } }
pub fn overloaded(x: &Wrapper) -> i32 { x.value }
pub struct 名 { pub 値: i32 }
pub fn unicode(x: &名) -> i32 {
    x.値
}
pub fn nested_shared(x: &&Outer) -> i32 { x.inner.value }
pub struct Borrowed { pub inner: &'static Inner }
pub fn stored_shared(x: &Borrowed) -> i32 { x.inner.value }
pub fn rebind(mut x: &'static Outer, y: &'static Outer) -> i32 { x = y; x.inner.value }
pub fn mutable_borrow(mut x: &Outer) -> i32 { let _ = &mut x; x.inner.value }
pub fn macro_rebind(mut x: &'static Outer, y: &'static Outer) -> i32 { macro_rules! alter { () => { x = y } } alter!(); x.inner.value }
"#,
    )
    .unwrap();
    directory
}

fn check_tuple_field_expectation(observations: &rstyle_pdg::field_locations::FieldObservations) {
    use rstyle_pdg::field_locations::{Location, Projection};
    assert_eq!(observations.operations.len(), 1);
    assert_eq!(
        observations.operations[0].location,
        Location::Captured {
            parameter: 0,
            path: vec![
                Projection::SharedReferent,
                Projection::TupleField { position: 0 }
            ],
            value: rstyle_pdg::concrete_types::TypeTree::Scalar("i32".into()),
        }
    );
}

fn check_field_resolution_tampering(
    facts: &rstyle_pdg::HirFnFacts,
    observations: &rstyle_pdg::field_locations::FieldObservations,
) {
    for change in ["owner", "position", "root", "type", "missing", "digest"] {
        let mut changed = facts.clone();
        if change == "digest" {
            changed.source.digest.push('0');
        } else {
            let field = changed
                .body
                .children
                .iter_mut()
                .find(|node| node.kind == "expr_field")
                .unwrap();
            match change {
                "owner" => {
                    field.res.as_mut().unwrap().description =
                        serde_json::json!({"position":0,"owner_path":"unrelated::Type"}).to_string()
                }
                "position" => {
                    field.res.as_mut().unwrap().description =
                        serde_json::json!({"position":1,"owner_path":null}).to_string()
                }
                "root" => {
                    field.children[0]
                        .res
                        .as_mut()
                        .unwrap()
                        .target
                        .as_mut()
                        .unwrap()
                        .local += 1
                }
                "type" => field.ty.as_mut().unwrap().normalized = Some("u32".into()),
                "missing" => field.children.clear(),
                _ => unreachable!(),
            }
        }
        assert!(
            rstyle_pdg::field_locations::verify(&changed, observations).is_err(),
            "{change}"
        );
    }
}

#[test]
fn clones_formal_field_operation_budget_reports_incomplete_capture() {
    let directory = clone_workspace();
    let reads = "let _ = input.0;\n".repeat(rstyle_pdg::field_locations::MAX_OPERATIONS + 1);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("pub fn large(input: &(i32,)) {{\n{reads}}}\n"),
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["assessment_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(
        format!("{} {report}", String::from_utf8_lossy(&output.stderr))
            .contains("Formal field observation exceeds operation budget")
    );
}

#[test]
fn clones_formal_field_byte_budget_reports_incomplete_capture() {
    let directory = clone_workspace();
    let declaration_padding = "x".repeat(60_000);
    let reads = "let _ = &input.inner;\n".repeat(150);
    std::fs::write(directory.path().join("src/lib.rs"), format!("pub struct Inner {{\n// {declaration_padding}\npub value: i32 }}\npub struct Outer {{ pub inner: Inner }}\npub fn large(input: &Outer) {{\n{reads}}}\n")).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["assessment_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(
        format!("{} {report}", String::from_utf8_lossy(&output.stderr))
            .contains("Formal field observation exceeds evidence byte budget")
    );
}

#[test]
fn clones_nominal_bindings_preserve_inherited_packages_and_target_contexts() {
    let directory = nominal_binding_workspace();
    let (output, report) = clones(
        directory.path(),
        &[
            "--all-targets",
            "--jobs",
            "4",
            "--extraction-profile",
            "hir-helpers-v1",
        ],
    );
    assert!(output.status.success(), "{report}");
    assert_eq!(report["observed_units"].as_array().unwrap().len(), 4);
    let mut packages = std::collections::BTreeSet::new();
    let mut builds = std::collections::BTreeSet::new();
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 10, "{report}");
    for function in functions {
        if let Some(binding) = checked_getter_binding(function) {
            packages.insert(binding["package_id"].as_str().unwrap().to_owned());
            let contexts = checked_build_context(function, &report);
            builds.insert((
                function["unit_evidence"]["bundle"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                binding["build_id"].as_u64().unwrap(),
            ));
            assert_eq!(contexts["features"], function["facts"]["build"]["features"]);
        }
    }
    assert_eq!(packages.len(), 2);
    assert_eq!(builds.len(), 4);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["pair_enumeration_complete"], true);
}

fn checked_build_context<'a>(function: &Value, report: &'a Value) -> &'a Value {
    let binding = &function["nominal_unit_binding"];
    assert!(binding.get("build_digest").is_none());
    let tables = report["build_contexts"].as_array().unwrap();
    let table = tables
        .iter()
        .find(|table| table["unit_bundle"] == function["unit_evidence"]["bundle"])
        .unwrap();
    assert_eq!(
        table["builds"]["identity"],
        "rstyle.rust.unit-build-contexts/1"
    );
    assert_eq!(table["builds"]["contexts"].as_array().unwrap().len(), 1);
    &table["builds"]["contexts"][binding["build_id"].as_u64().unwrap() as usize]
}

fn checked_getter_binding(function: &Value) -> Option<&Value> {
    let name = function["facts"]["name"].as_str().unwrap();
    if name.ends_with("get") {
        assert_eq!(function["outcome"], "excluded");
        let binding = &function["nominal_unit_binding"];
        assert_eq!(binding["identity"], rstyle_pdg::nominal_binding::IDENTITY);
        assert_eq!(
            binding["declarations"][0]["nominal"]["package_version"],
            "2.3.4"
        );
        return Some(binding);
    }
    if name.ends_with("scalar") {
        assert_eq!(function["outcome"], "admitted");
        assert!(function.get("nominal_unit_binding").is_none());
    } else {
        assert!(name.ends_with("main"), "{function}");
        assert_eq!(function["outcome"], "excluded", "{function}");
        assert_eq!(
            function["nominal_unit_binding"]["declarations"],
            serde_json::json!([])
        );
    }

    None
}

fn nominal_binding_workspace() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("Cargo.toml"), "[workspace]\nmembers = ['left', 'right']\nresolver = '2'\n[workspace.package]\nversion = '2.3.4'\n").unwrap();
    for package in ["left", "right"] {
        let root = directory.path().join(package);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname = '{package}'\nversion.workspace = true\nedition = '2021'\n"),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub struct Counter { pub value: i32 }\nimpl Counter { pub fn get(&self) -> i32 { self.value } }\npub fn scalar(x: i32) -> i32 { x + 1 }\n").unwrap();
    }

    directory
}

#[test]
fn clones_nominal_bindings_refresh_after_manifest_source_and_build_changes() {
    let directory = clone_workspace();
    write_nominal_getter(directory.path(), "i32");
    let (output, before) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{before}");
    let manifest = directory.path().join("Cargo.toml");
    let revised = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("0.1.0", "0.2.0");
    std::fs::write(manifest, revised).unwrap();
    write_nominal_getter(directory.path(), "u32");
    let (output, after) = clones(
        directory.path(),
        &["--release", "--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{after}");
    check_nominal_refresh(&before, &after);
}

fn check_nominal_refresh(before: &Value, after: &Value) {
    assert_ne!(before["run_directory"], after["run_directory"]);
    for report in [before, after] {
        assert_eq!(report["functions"].as_array().unwrap().len(), 1);
        assert_eq!(report["functions"][0]["outcome"], "excluded");
        assert_eq!(report["pair_evaluation_complete"], true);
    }
    let old = &before["functions"][0]["nominal_unit_binding"];
    let new = &after["functions"][0]["nominal_unit_binding"];
    for key in ["package_id", "manifest_digest"] {
        assert_ne!(old[key], new[key], "{key}");
    }
    let old_build = checked_build_context(&before["functions"][0], before);
    let new_build = checked_build_context(&after["functions"][0], after);
    assert_ne!(old_build["debug_assertions"], new_build["debug_assertions"]);
    assert_ne!(old_build["frontend_options"], new_build["frontend_options"]);
    assert_ne!(
        old["declarations"][0]["file_digest"],
        new["declarations"][0]["file_digest"]
    );
    assert_eq!(
        new["declarations"][0]["nominal"]["package_version"],
        "0.2.0"
    );
    assert_eq!(
        after["functions"][0]["facts"]["build"]["debug_assertions"]["value"],
        false
    );
    check_refreshed_nominal_field(after);
}

fn write_nominal_getter(root: &Path, scalar: &str) {
    std::fs::write(root.join("src/lib.rs"), format!("pub struct Counter {{ pub value: {scalar} }}\nimpl Counter {{ pub fn get(&self) -> {scalar} {{ self.value }} }}\n")).unwrap();
}

fn check_refreshed_nominal_field(report: &Value) {
    use rstyle_pdg::concrete_types::{self, InterfaceObservations, Observation, TypeTree};
    let observed: InterfaceObservations = serde_json::from_value(
        report["functions"][0]["facts"]["build"]["frontend_options"][concrete_types::KEY].clone(),
    )
    .unwrap();
    let Observation::Captured {
        ty: TypeTree::Shared(referent),
    } = &observed.parameters[0]
    else {
        panic!("{observed:?}")
    };
    let TypeTree::Struct { fields, .. } = referent.as_ref() else {
        panic!("{referent:?}")
    };
    assert_eq!(fields[0].ty, TypeTree::Scalar("u32".into()));
}

const DATA_TUPLE_FAMILY: &str = "pub fn positive(x: i32) -> bool { x > 0 }\npub fn first(pair: &(i32, bool)) -> i32 { pair.0 }\npub fn enabled(pair: &(i32, bool)) -> bool { pair.1 }\npub fn compare(a: &(i32, bool), b: &(i32, bool)) -> bool { positive(a.0) && positive(b.0) }\npub fn identity(pair: &(i32, bool)) -> &(i32, bool) { pair }\npub fn select<'a>(a: &'a (i32, bool), b: &'a (i32, bool), choose: bool) -> &'a (i32, bool) { if choose { a } else { b } }\n";

#[test]
fn clones_data_tuple_family_reaches_registered_projection_and_exhaustive_pairs() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), DATA_TUPLE_FAMILY).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    check_data_registration(&report);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 6);
    assert!(
        functions
            .iter()
            .all(|function| function["outcome"] == "admitted"),
        "{report}"
    );
    assert!(
        report["matching"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["eligible"] == true),
        "{report}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["pair_enumeration_complete"], true);
    let getter = functions
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let projection = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["key"] == getter["key"])
        .unwrap();
    // One input, binding read, field read and return; four value edges and
    // two sequential execution edges; binding and field are distinct reads.
    assert_eq!(
        projection["projection"]["features"],
        serde_json::json!([0, 2, 4, 0, 0, 0, 0, 4, 2])
    );
    check_data_document_tampering(getter);
    check_data_origins_tampering(
        functions
            .iter()
            .find(|function| function["facts"]["name"] == "select")
            .unwrap(),
    );
}

fn check_data_registration(report: &Value) {
    assert_eq!(report["extraction_selection"], "hir-data-v1");
    assert_eq!(report["extraction_policy"]["id"], "rstyle.rust.hir-data");
    assert_eq!(report["matching"]["profile_id"], "rstyle.ccgraph.data");
    assert_eq!(report["matching"]["profile_version"], "1.0.0");
}

fn copy_pair(pair: &rstyle_pdg::producer::Pair) -> rstyle_pdg::producer::Pair {
    rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    }
}

fn check_data_document_tampering(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    for change in [
        "path", "read", "alias", "count", "policy", "write", "operand", "root",
    ] {
        let mut changed = copy_pair(pair);
        tamper_data_document(&mut changed, change);
        assert!(
            rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
            "{change}"
        );
    }
}

fn tamper_data_document(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    match change {
        "path" => {
            pair.pdg.memory_locations[0]
                .extensions
                .get_mut(rstyle_pdg::data_documents::LOCATION_KEY)
                .unwrap()["path"][1]["position"] = serde_json::json!(1);
        }
        "read" => {
            data_read(pair).attributes.reads[0].location_id = Some("missing".into());
        }
        "alias" => {
            pair.pdg.extensions.insert(
                "rstyle.data:possible-formal-aliases".into(),
                serde_json::json!([]),
            );
        }
        "count" => {
            pair.pdg.function.reference_count.value = Some(0);
        }
        "policy" => {
            pair.cfg
                .extensions
                .remove(rstyle_pdg::data_documents::POLICY_KEY);
        }
        "write" => {
            let read = data_read(pair);
            read.attributes
                .writes
                .push(read.attributes.reads[0].clone());
        }
        "operand" => {
            let field = data_read(pair).id.clone();
            pair.pdg
                .edges
                .retain(|edge| edge.target != field || edge.subkind != "value_operand");
        }
        "root" => {
            data_read(pair).attributes.reads[0].symbol_id = Some("unrelated".into());
        }
        _ => unreachable!(),
    }
}

fn data_read(pair: &mut rstyle_pdg::producer::Pair) -> &mut rstyle_pdg::PdgNode {
    pair.pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.attributes
                .reads
                .iter()
                .any(|read| read.location_id.is_some())
        })
        .unwrap()
}

fn check_data_origins_tampering(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    for change in ["origin", "value", "type"] {
        let mut changed = copy_pair(pair);
        match change {
            "origin" => {
                changed
                    .pdg
                    .nodes
                    .iter_mut()
                    .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
                    .unwrap()
                    .annotations
                    .insert(rstyle_pdg::data_origins::KEY.into(), serde_json::json!([]));
            }
            "value" => {
                changed.pdg.edges.retain(|edge| {
                    edge.subkind != "value_alternative" || edge.outcome.as_deref() != Some("false")
                });
            }
            "type" => {
                changed
                    .pdg
                    .nodes
                    .iter_mut()
                    .find(|node| node.category == rstyle_pdg::NodeCategory::FormalInput)
                    .unwrap()
                    .annotations
                    .get_mut("compat.rust.hir-r0:type")
                    .unwrap()["value"] = serde_json::json!("i32");
            }
            _ => unreachable!(),
        }
        assert!(
            rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
            "{change}"
        );
    }
}

#[test]
fn clones_data_is_opt_in_and_unsupported_type_effect_mixtures_remain_excluded() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), format!("{DATA_TUPLE_FAMILY}\npub fn cells(x: &std::cell::Cell<i32>) -> i32 {{ x.get() }}\npub fn mutable(x: &mut i32) -> i32 {{ *x }}\npub fn local(x: &i32) -> &i32 {{ let y = x; y }}\n")).unwrap();
    let (output, data) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{data}");
    let functions = data["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|function| function["outcome"] == "admitted")
            .count(),
        6,
        "{data}"
    );
    assert_eq!(
        functions
            .iter()
            .filter(|function| function["outcome"] == "excluded")
            .count(),
        3,
        "{data}"
    );
    assert_eq!(data["matching"]["pairs"].as_array().unwrap().len(), 15);
    let (output, helpers) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{helpers}");
    assert_eq!(
        helpers["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|function| function["outcome"] == "admitted")
            .count(),
        1,
        "{helpers}"
    );
    assert_eq!(helpers["matching"]["profile_id"], "compat.ccgraph.strict");
}

#[test]
fn clones_data_trace_collection_preserves_pair_decisions_on_fresh_warm_runs() {
    for source in [
        DATA_TUPLE_FAMILY,
        DATA_BORROW_FAMILY,
        DATA_STORED_REFERENCE_FAMILY,
        DATA_LIFETIME_FAMILY,
    ] {
        check_data_trace_invariance(source);
    }
}

fn check_data_trace_invariance(source: &str) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, ordinary) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{ordinary}");
    let (output, traced) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{traced}");
    assert_ne!(ordinary["run_directory"], traced["run_directory"]);
    let decisions = |report: &Value| {
        report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair["first"].clone(),
                    pair["second"].clone(),
                    pair["decision"].clone(),
                    pair["displayed_score"].clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(decisions(&ordinary), decisions(&traced));
    assert_eq!(
        ordinary["matching"]["effective_config"],
        traced["matching"]["effective_config"]
    );
}

const DATA_NOMINAL_API: &str = "mod model;\nuse crate::model::Outer as Api;\npub fn first(a: &Api) -> i32 { a.inner.value }\npub fn compare(a: &Api, b: &Api) -> bool { a.inner.value < b.inner.value }\npub fn identity(a: &Api) -> &Api { a }\nimpl Api { pub fn value(&self) -> i32 { self.inner.value } pub fn flag(&self) -> bool { self.inner.flag } }\n";

#[test]
fn clones_data_nominal_modules_aliases_receivers_and_nested_fields_reach_all_pairs() {
    let directory = clone_workspace();
    let source = format!(
        "\u{feff}{}\r\nmod other {{ pub struct Inner {{ pub value: i32, pub flag: bool }} }}\r\n",
        DATA_NOMINAL_API.replace('\n', "\r\n")
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("src/model.rs"), "\u{feff}pub struct Inner { pub value: i32, pub flag: bool }\r\npub struct Outer { pub inner: Inner }\r\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    check_data_registration(&report);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 5);
    assert!(
        functions.iter().all(|f| f["outcome"] == "admitted"),
        "{report}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
    assert_eq!(report["pair_evaluation_complete"], true);
    let method = functions
        .iter()
        .find(|f| {
            f["facts"]["name"]
                .as_str()
                .is_some_and(|name| name.ends_with("::value"))
        })
        .unwrap();
    check_nominal_projection(&report, method);
}

fn check_nominal_projection(report: &Value, method: &Value) {
    let projection = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == method["key"])
        .unwrap();
    // Input, binding read, aggregate place, terminal field read and return.
    // Only the binding and terminal field are read; the place keeps its base edge.
    assert_eq!(
        projection["projection"]["features"],
        serde_json::json!([0, 3, 5, 0, 0, 0, 0, 5, 2])
    );
    let (unit, index) = data_unit(method);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    assert_eq!(
        pair.pdg.function.inputs[0].role,
        rstyle_pdg::ParamRole::Receiver
    );
    rstyle_pdg::data_frontend::verify(&facts).unwrap();
    check_nominal_proof_tampering(&facts);
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    changed.pdg.function.outputs[0].extensions.insert(
        "compat.rust.hir-r0:resolved-type".into(),
        serde_json::to_value(rstyle_pdg::StringFact::known("bool")).unwrap(),
    );
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    let mut changed = copy_pair(pair);
    changed.pdg.function.inputs[0].role = rstyle_pdg::ParamRole::Parameter;
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
}

fn check_nominal_proof_tampering(facts: &rstyle_pdg::HirFnFacts) {
    for change in [
        "missing",
        "duplicate",
        "orphan",
        "context",
        "bytes",
        "trait",
        "receiver",
    ] {
        let mut changed = facts.clone();
        tamper_nominal_proof(&mut changed, change);
        assert!(
            rstyle_pdg::data_frontend::verify(&changed).is_err(),
            "{change}"
        );
    }
}

fn tamper_nominal_proof(changed: &mut rstyle_pdg::HirFnFacts, change: &str) {
    use rstyle_pdg::{
        concrete_types::digest,
        nominal_boundary::{CONTEXT_KEY, census},
    };
    let original = changed.clone();
    let options = &mut changed.build.frontend_options;
    match change {
        "context" => {
            options.get_mut(CONTEXT_KEY).unwrap()["package"]["package_id"] =
                "different-package".into();
        }
        "missing" => {
            options.get_mut(census::KEY).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        "duplicate" | "orphan" => duplicate_census_file(options, &original, change),
        "bytes" | "trait" => {
            let file = &mut options.get_mut(census::KEY).unwrap()["files"][0];
            let bytes = if change == "trait" {
                format!(
                    "{}\nstruct Hidden; impl Drop for Hidden {{ fn drop(&mut self) {{}} }}",
                    file["bytes"].as_str().unwrap()
                )
            } else {
                "".into()
            };
            file["digest"] = digest(bytes.as_bytes()).into();
            file["bytes"] = bytes.into();
        }
        "receiver" => changed.signature.has_self = false,
        _ => unreachable!(),
    }
}

fn duplicate_census_file(
    options: &mut rstyle_pdg::FrontendOptions,
    facts: &rstyle_pdg::HirFnFacts,
    change: &str,
) {
    use rstyle_pdg::nominal_boundary::{CONTEXT_KEY, census};
    let files = options.get_mut(census::KEY).unwrap()["files"]
        .as_array_mut()
        .unwrap();
    let mut extra = files[0].clone();
    if change == "orphan" {
        extra["path"] = format!(
            "{}orphan.rs",
            facts.build.frontend_options[CONTEXT_KEY]["root_source"]
                .as_str()
                .unwrap()
                .trim_end_matches("lib.rs")
        )
        .into();
    }
    files.push(extra);
}

#[test]
fn clones_data_nominal_hidden_trait_and_expansion_items_remain_unavailable() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub struct Data { pub x: i32 }\npub fn get(x: &Data) -> i32 { x.x }\npub fn scalar(x: i32) -> i32 { x }\nfn hidden() { struct Hidden; impl Drop for Hidden { fn drop(&mut self) {} } }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == "get")
            .unwrap()["outcome"],
        "excluded"
    );
    assert_eq!(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == "scalar")
            .unwrap()["outcome"],
        "admitted"
    );
    assert_eq!(report["assessment_complete"], true);
}

const DATA_BORROW_FAMILY: &str = "pub struct Inner { pub x: i32, pub y: i32, pub ok: bool }\npub struct Outer { pub inner: Inner }\nimpl Outer { pub fn inner(&self) -> &Inner { &self.inner } pub fn x(&self) -> &i32 { &self.inner.x } pub fn y(&self) -> &i32 { &self.inner.y } pub fn ok(&self) -> bool { self.inner.ok } }\npub fn choose<'a>(a: &'a Outer, b: &'a Outer, x: i32) -> &'a i32 { if positive(x) { &a.inner.x } else { &b.inner.x } }\npub fn choose_inner<'a>(a: &'a Outer, b: &'a Outer, flag: bool) -> &'a Inner { if flag { &a.inner } else { &b.inner } }\npub fn tuple_first(a: &(i32, bool)) -> &i32 { &a.0 }\npub fn positive(x: i32) -> bool { x > 0 }\n";

#[test]
fn clones_data_shared_field_borrows_preserve_places_origins_and_all_pairs() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), DATA_BORROW_FAMILY).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8);
    assert!(
        functions.iter().all(|f| f["outcome"] == "admitted"),
        "{report}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 28);
    assert_eq!(report["pair_evaluation_complete"], true);
    let getter = functions
        .iter()
        .find(|f| f["facts"]["name"] == "Outer::x")
        .unwrap();
    let projection = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == getter["key"])
        .unwrap();
    // One binding read, two address projections, borrow and return, plus input.
    // Only the binding is read; all five value edges and four execution edges remain.
    assert_eq!(
        projection["projection"]["features"],
        serde_json::json!([0, 4, 5, 0, 0, 0, 0, 6, 1])
    );
    check_borrow_pair(&report, getter);
    check_borrow_document_tampering(getter);
    check_data_origins_tampering(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == "choose")
            .unwrap(),
    );
}

const DATA_STORED_REFERENCE_FAMILY: &str = "pub struct View { pub text: &'static str, pub bytes: &'static [u8], pub value: &'static i32 }\npub struct Outer { pub inner: View }\nimpl View { pub fn text(&self) -> &'static str { self.text } pub fn bytes(&self) -> &'static [u8] { self.bytes } }\npub fn value(view: View) -> &'static i32 { view.value }\npub fn inner(outer: Outer) -> View { outer.inner }\npub fn nested(outer: Outer) -> &'static str { outer.inner.text }\npub fn choose(a: Outer, b: Outer, flag: bool) -> View { if flag { a.inner } else { b.inner } }\npub fn identity(view: View) -> View { view }\n";

#[test]
fn clones_data_stored_reference_fields_preserve_nested_origins_and_all_pairs() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        DATA_STORED_REFERENCE_FAMILY,
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 7);
    assert!(
        functions
            .iter()
            .all(|function| function["outcome"] == "admitted"),
        "{report}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 21);
    assert_eq!(report["pair_evaluation_complete"], true);
    for name in ["inner", "nested", "choose", "identity"] {
        let function = functions
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        check_stored_reference_origins(function, name);
    }
    let getter = functions
        .iter()
        .find(|f| f["facts"]["name"] == "View::text")
        .unwrap();
    let projection = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == getter["key"])
        .unwrap();
    assert_eq!(
        projection["projection"]["features"],
        serde_json::json!([0, 2, 4, 0, 0, 0, 0, 4, 2])
    );
    check_data_document_tampering(getter);
    check_data_origins_tampering(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == "choose")
            .unwrap(),
    );
}

fn check_stored_reference_origins(function: &Value, name: &str) {
    use rstyle_pdg::{NodeCategory, data_origins};
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    let returned = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == NodeCategory::Return)
        .unwrap();
    let origins: Vec<data_origins::Origin> =
        serde_json::from_value(returned.annotations[data_origins::KEY].clone()).unwrap();
    assert_eq!(
        origins.len(),
        match name {
            "nested" => 1,
            "choose" => 6,
            _ => 3,
        }
    );
    for origin in &origins {
        check_stored_reference_path(origin, name);
    }
    if name == "choose" {
        assert_eq!(
            pair.pdg.extensions["rstyle.data:possible-formal-aliases"],
            serde_json::json!([[0, 0], [0, 1], [1, 1]])
        );
    }
}

fn check_stored_reference_path(origin: &rstyle_pdg::data_origins::Origin, name: &str) {
    use rstyle_pdg::field_locations::Projection;
    assert!(matches!(
        origin.path.last(),
        Some(Projection::SharedReferent)
    ));
    assert_eq!(origin.path.len(), if name == "identity" { 2 } else { 3 });
    let fields: Vec<_> = origin
        .path
        .iter()
        .filter_map(|projection| match projection {
            Projection::StructField { name, position, .. } => Some((name.as_str(), *position)),
            _ => None,
        })
        .collect();
    let expected = if name == "nested" {
        vec![("inner", 0), ("text", 0)]
    } else {
        let last = *fields.last().unwrap();
        assert!([("text", 0), ("bytes", 1), ("value", 2)].contains(&last));
        if name == "identity" {
            vec![last]
        } else {
            vec![("inner", 0), last]
        }
    };
    assert_eq!(fields, expected);
    assert!(origin.parameter == 0 || name == "choose" && origin.parameter == 1);
}

#[test]
fn clones_data_reference_element_and_type_generic_boundaries_fail_closed() {
    for (source, count) in [
        (
            "pub struct Mixed { pub direct: &'static i32, pub elements: [&'static i32; 2] }\npub fn mixed(value: Mixed) -> Mixed { value }\npub fn unused(value: Mixed) -> i32 { 1 }\npub fn scalar(x: i32) -> i32 { x }\n",
            3,
        ),
        (
            "pub struct Generic<T> { pub value: T }\npub fn generic(value: Generic<i32>) -> i32 { value.value }\npub fn scalar(x: i32) -> i32 { x }\n",
            2,
        ),
    ] {
        check_unavailable_reference_fields(source, count);
    }
}

const DATA_LIFETIME_FAMILY: &str = "mod model { pub struct View<'a> { pub text: &'a str, pub bytes: &'a [u8], pub value: &'a i32 } pub struct Outer<'a> { pub inner: View<'a> } impl<'a> View<'a> { pub fn text(&self) -> &'a str { self.text } pub fn bytes(&self) -> &'a [u8] { self.bytes } } }\nuse model::{View as Alias, Outer};\npub fn value(view: Alias<'_>) -> &i32 { view.value }\npub fn inner<'a>(outer: Outer<'a>) -> Alias<'a> { outer.inner }\npub fn nested(outer: Outer<'_>) -> &str { outer.inner.text }\npub fn choose<'a>(a: Outer<'a>, b: Outer<'a>, flag: bool) -> Alias<'a> { if flag { a.inner } else { b.inner } }\npub fn identity(view: Alias<'_>) -> Alias<'_> { view }\npub fn static_value(view: Alias<'static>) -> &'static i32 { view.value }\n";

#[test]
fn clones_data_lifetime_nominals_resolve_aliases_receivers_and_all_pairs() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), DATA_LIFETIME_FAMILY).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8);
    assert!(
        functions
            .iter()
            .all(|function| function["outcome"] == "admitted"),
        "{report}"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 28);
    assert_eq!(report["pair_evaluation_complete"], true);
    for name in ["inner", "nested", "choose", "identity"] {
        check_stored_reference_origins(
            functions
                .iter()
                .find(|f| f["facts"]["name"] == name)
                .unwrap(),
            name,
        );
    }
    check_lifetime_interface_pair(&report);
    let getter = functions
        .iter()
        .find(|f| f["facts"]["name"] == "value")
        .unwrap();
    check_lifetime_source_tampering(getter);
    check_data_document_tampering(
        functions
            .iter()
            .find(|f| f["facts"]["name"].as_str().unwrap().ends_with("::text"))
            .unwrap(),
    );
}

#[test]
fn clones_data_lifetime_bounds_and_legacy_profiles_remain_unavailable() {
    for source in [
        "pub struct Bounds<'a, 'b> { pub a: &'a i32, pub b: &'b i32 }\nimpl<'a: 'b,'b> Bounds<'a,'b> { pub fn get(&self) -> &'a i32 { self.a } }\npub fn scalar(x: i32) -> i32 { x }\n",
        "pub struct Bounded<'a> where 'a: 'static { pub value: &'a i32 }\npub fn get(value: Bounded<'static>) -> &'static i32 { value.value }\npub fn scalar(x: i32) -> i32 { x }\n",
        "pub struct Generic<const N: usize> { pub bytes: [u8;N] }\npub fn get(value: Generic<8>) -> [u8;8] { value.bytes }\npub fn scalar(x: i32) -> i32 { x }\n",
    ] {
        check_unavailable_reference_fields(source, 2);
    }
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), DATA_LIFETIME_FAMILY).unwrap();
    for selection in ["hir-helpers-v1", "hir-expanded-v1", "r0"] {
        let (output, report) = clones(directory.path(), &["--extraction-profile", selection]);
        assert!(output.status.success(), "{report}");
        let functions = report["functions"].as_array().unwrap();
        assert_eq!(functions.len(), 8);
        assert!(
            functions.iter().all(|f| f["outcome"] == "excluded"),
            "{report}"
        );
    }
}

#[test]
fn clones_data_lifetime_argument_budget_is_incomplete_capture() {
    let directory = clone_workspace();
    let count = rstyle_pdg::concrete_types::MAX_FIELDS + 1;
    let parameters = (0..count)
        .map(|position| format!("'a{position}"))
        .collect::<Vec<_>>()
        .join(",");
    let fields = (0..count)
        .map(|position| format!("&'a{position} i32"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "pub struct Many<{parameters}> {{ pub refs: ({fields},) }}\nimpl<{parameters}> Many<{parameters}> {{ pub fn get(&self) -> &Self {{ self }} }}\n"
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["assessment_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(
        report["functions"].as_array().unwrap().is_empty(),
        "{report}"
    );
    assert!(
        format!("{} {report}", String::from_utf8_lossy(&output.stderr))
            .contains("Nominal lifetime arguments exceed count budget"),
        "{report}"
    );
}

#[test]
fn clones_helpers_preserve_permissive_concrete_observations_without_data_admission() {
    for source in [
        "pub struct Plain where i32: Copy { pub value: i32 }\npub fn read(value: &Plain) -> i32 { value.value }\n",
        "pub trait Value { type Output; }\nimpl Value for i32 { type Output = i32; }\npub struct Plain { pub value: <i32 as Value>::Output }\npub fn read(value: &Plain) -> i32 { value.value }\n",
    ] {
        let directory = clone_workspace();
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        for profile in ["hir-helpers-v1", "hir-data-v1"] {
            let (output, report) = clones(directory.path(), &["--extraction-profile", profile]);
            assert!(output.status.success(), "{report}");
            assert_eq!(report["capture_complete"], true);
            assert_eq!(report["assessment_complete"], true);
            let functions = report["functions"].as_array().unwrap();
            assert_eq!(functions.len(), 1);
            assert_eq!(functions[0]["outcome"], "excluded", "{report}");
            if profile == "hir-helpers-v1" {
                assert_eq!(
                    functions[0]["facts"]["build"]["frontend_options"]
                        [rstyle_pdg::concrete_types::KEY]["parameters"][0]["status"],
                    "Captured",
                    "{report}"
                );
            }
        }
    }
}

#[test]
fn clones_data_field_context_preserves_pointer_reads_and_aggregate_places() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub struct Leaf { pub value: i32 }\npub struct Owned { pub leaf: Leaf }\npub struct Linked<'a> { pub leaf: &'a Leaf }\nimpl Owned { pub fn get(&self) -> i32 { self.leaf.value } }\nimpl<'a> Linked<'a> { pub fn get(&self) -> i32 { self.leaf.value } pub fn borrow(&self) -> &i32 { &self.leaf.value } }\npub fn owned_inner(value: Owned) -> Leaf { value.leaf }\npub fn tuple_borrow<'a>(value: &'a (&'a Leaf, bool)) -> &'a i32 { &value.0.value }\npub fn consecutive(value: &&Leaf) -> i32 { value.value }\npub fn scalar(value: i32) -> i32 { value }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 7);
    for function in functions {
        assert_eq!(
            function["outcome"],
            if function["facts"]["name"] == "consecutive" {
                "excluded"
            } else {
                "admitted"
            },
            "{report}"
        );
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
    assert_eq!(report["pair_evaluation_complete"], true);
    for (name, features) in [
        ("Owned::get", [0, 3, 5, 0, 0, 0, 0, 5, 2]),
        ("owned_inner", [0, 2, 4, 0, 0, 0, 0, 4, 2]),
        ("tuple_borrow", [0, 4, 6, 0, 0, 0, 0, 6, 2]),
    ] {
        let function = functions
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        let projection = report["matching"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["key"] == function["key"])
            .unwrap();
        assert_eq!(
            projection["projection"]["features"],
            serde_json::json!(features)
        );
    }
    let borrowed = functions
        .iter()
        .find(|f| f["facts"]["name"].as_str().unwrap().ends_with("::borrow"))
        .unwrap();
    check_stored_pointer_read_tampering(borrowed);
}

fn check_stored_pointer_read_tampering(function: &Value) {
    use rstyle_pdg::{NodeCategory, data_places};
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    let mut changed = facts.clone();
    assert!(tamper_pointer_value_type(&mut changed.body));
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
    let pointer = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == NodeCategory::Read
                && node.original_kind.known_value() == Some("expr_field")
        })
        .unwrap();
    for change in ["read", "role", "operand"] {
        let mut changed = copy_pair(pair);
        if change == "operand" {
            changed
                .pdg
                .edges
                .retain(|edge| !(edge.source == pointer.id && edge.subkind == "value_operand"));
        } else {
            let node = changed
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.id == pointer.id)
                .unwrap();
            if change == "read" {
                node.attributes.reads.clear();
            } else {
                node.category = NodeCategory::Computation;
                node.attributes.construct = Some("data.place".into());
                node.annotations.insert(
                    data_places::KEY.into(),
                    serde_json::json!(pointer.attributes.reads[0].location_id),
                );
            }
        }
        assert!(
            rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
            "{change}"
        );
    }
}

fn tamper_pointer_value_type(node: &mut rstyle_pdg::HirNode) -> bool {
    if node.kind == "expr_field"
        && node
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            .and_then(|token| rstyle_pdg::data_types::parse(token).ok())
            .is_some_and(|ty| matches!(ty, rstyle_pdg::data_types::TypeToken::Shared(_)))
    {
        node.ty.as_mut().unwrap().normalized = Some("bool".into());
        return true;
    }
    node.children.iter_mut().any(tamper_pointer_value_type)
}

fn check_lifetime_interface_pair(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let value = functions
        .iter()
        .find(|f| f["facts"]["name"] == "value")
        .unwrap();
    let other = functions
        .iter()
        .find(|f| f["facts"]["name"] == "static_value")
        .unwrap();
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (pair["first"] == value["key"] && pair["second"] == other["key"])
                || (pair["first"] == other["key"] && pair["second"] == value["key"])
        })
        .unwrap();
    assert_eq!(pair["decision"], "Accepted");
    assert_eq!(pair["stages"][6]["detail"]["score"], "1/1");
}

fn check_lifetime_source_tampering(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let _ = unit.pair(index).unwrap();
    rstyle_pdg::data_frontend::verify(&facts).unwrap();
    let observed: rstyle_pdg::concrete_types::InterfaceObservations = serde_json::from_value(
        facts.build.frontend_options[rstyle_pdg::concrete_types::KEY].clone(),
    )
    .unwrap();
    let rstyle_pdg::concrete_types::Observation::Captured { ty } = &observed.parameters[0] else {
        panic!("{observed:?}")
    };
    let resolution = rstyle_pdg::nominal_boundary::resolution::verify(&facts, &observed).unwrap();
    for path in ["Alias<'a,'b>", "Alias<i32>", "model<'a>::View", "Alias<3>"] {
        assert!(
            rstyle_pdg::concrete_types::source_proof::verify_top_spelling(path, ty, &resolution)
                .is_err(),
            "{path}"
        );
    }
}

fn check_unavailable_reference_fields(source: &str, count: usize) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), count);
    for function in functions {
        assert_eq!(
            function["outcome"],
            if function["facts"]["name"] == "scalar" {
                "admitted"
            } else {
                "excluded"
            },
            "{report}"
        );
    }
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_borrow_pair(report: &Value, getter: &Value) {
    let other = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "Outer::y")
        .unwrap();
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (pair["first"] == getter["key"] && pair["second"] == other["key"])
                || (pair["first"] == other["key"] && pair["second"] == getter["key"])
        })
        .unwrap();
    assert_eq!(pair["decision"], "Accepted");
    assert_eq!(pair["stages"][6]["detail"]["score"], "1/1");
}

fn check_borrow_document_tampering(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    for change in ["span", "type", "child"] {
        let mut changed = facts.clone();
        assert!(tamper_source_borrow(&mut changed.body, change));
        assert!(
            rstyle_pdg::data_frontend::verify(&changed).is_err(),
            "{change}"
        );
    }
    let place = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.annotations.contains_key(rstyle_pdg::data_places::KEY))
        .unwrap();
    let borrow = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == rstyle_pdg::NodeCategory::Computation
                && node.original_kind.known_value() == Some("expr_addr_of")
        })
        .unwrap();
    for change in ["place", "read", "operator", "origin", "operand", "position"] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        tamper_borrow_document(&mut changed, place, borrow, change);
        assert!(
            rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
            "{change}"
        );
    }
}

fn tamper_source_borrow(node: &mut rstyle_pdg::HirNode, change: &str) -> bool {
    if node.kind == "expr_addr_of" {
        match change {
            "span" => node.span.start += 1,
            "type" => {
                node.ty.as_mut().unwrap().normalized =
                    Some(r#"{"kind":"shared","value":{"kind":"scalar","value":"bool"}}"#.into())
            }
            "child" => node.children[0].kind = "expr_path".into(),
            _ => unreachable!(),
        }
        return true;
    }
    node.children
        .iter_mut()
        .any(|child| tamper_source_borrow(child, change))
}

fn tamper_borrow_document(
    pair: &mut rstyle_pdg::producer::Pair,
    place: &rstyle_pdg::PdgNode,
    borrow: &rstyle_pdg::PdgNode,
    change: &str,
) {
    if matches!(change, "operand" | "position") {
        let edge = pair
            .pdg
            .edges
            .iter_mut()
            .find(|edge| edge.target == borrow.id && edge.subkind == "value_operand")
            .unwrap();
        if change == "operand" {
            edge.source = place.id.clone();
        } else {
            edge.operand_position = Some(1);
        }
        return;
    }
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            &node.id
                == if change == "place" || change == "read" {
                    &place.id
                } else {
                    &borrow.id
                }
        })
        .unwrap();
    match change {
        "place" => {
            node.annotations
                .insert(rstyle_pdg::data_places::KEY.into(), "false-location".into());
        }
        "read" => node.attributes.reads.push(rstyle_pdg::Access {
            symbol_id: Some("invented".into()),
            location_id: None,
            operand_position: Some(0),
            unresolved_reason: None,
        }),
        "operator" => node.attributes.operator = Some(rstyle_pdg::StringFact::known("+")),
        "origin" => {
            node.annotations
                .insert(rstyle_pdg::data_origins::KEY.into(), serde_json::json!([]));
        }
        _ => unreachable!(),
    }
}

#[test]
fn clones_data_unavailable_borrow_places_and_legacy_profiles_fail_closed() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub struct Ref { pub value: &'static i32 }\npub fn reference_field(value: &Ref) -> &&'static i32 { &value.value }\npub fn temporary(x: i32) -> i32 { let reference = &x; *reference }\npub fn mutable(value: &mut i32) -> &i32 { &*value }\npub fn scalar(x: i32) -> i32 { x }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        1
    );
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "excluded")
            .count(),
        3
    );
    assert_eq!(report["assessment_complete"], true);
    std::fs::write(directory.path().join("src/lib.rs"), DATA_BORROW_FAMILY).unwrap();
    let (output, legacy) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{legacy}");
    let functions = legacy["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "excluded")
            .count(),
        7
    );
    assert_eq!(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == "positive")
            .unwrap()["outcome"],
        "admitted"
    );
}

#[test]
fn clones_data_field_borrows_preserve_array_slice_and_str_referents() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub struct Fixed { pub bytes: [u8; 8] }\npub struct Slice { pub bytes: [u8] }\npub struct Text { pub text: str }\nimpl Fixed { pub fn bytes(&self) -> &[u8; 8] { &self.bytes } }\nimpl Slice { pub fn bytes(&self) -> &[u8] { &self.bytes } }\nimpl Text { pub fn text(&self) -> &str { &self.text } }\npub fn unsize(value: &Fixed) -> &[u8] { &value.bytes }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        3,
        "{report}"
    );
    assert_eq!(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == "unsize")
            .unwrap()["outcome"],
        "excluded"
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 3);
    assert_eq!(report["pair_evaluation_complete"], true);
    let (unit, _) = data_unit(&functions[0]);
    let analysis = unit.analysis();
    for function in functions.iter().filter(|f| f["outcome"] == "admitted") {
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
        assert_eq!(pair.pdg.function.reference_count.value, Some(1));
        assert!(
            pair.pdg
                .nodes
                .iter()
                .filter(
                    |node| node.original_kind.known_value() == Some("expr_field")
                        && node.category == rstyle_pdg::NodeCategory::Computation
                )
                .all(|node| node.attributes.reads.is_empty())
        );
    }
}

// Library summaries require a reviewed compiler/target authority. Other hosts
// still capture receivers and must exclude calls whose semantics are unproved.
fn has_length_authority(report: &Value) -> bool {
    let facts = serde_json::from_value(report["functions"][0]["facts"].clone()).unwrap();
    rstyle_pdg::data_library::registry_for(&facts).is_some()
}

fn check_data_family<'a>(report: &'a Value, length_dependent: &[&str]) -> Vec<&'a Value> {
    let registered = has_length_authority(report);
    let mut admitted = Vec::new();
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let excluded = !registered && length_dependent.contains(&name);
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
        if !excluded {
            admitted.push(function);
        }
    }

    check_complete_data_pairs(report, admitted.len());
    admitted
}

fn check_complete_data_pairs(report: &Value, admitted: usize) {
    assert_eq!(
        report["matching"]["pairs"].as_array().unwrap().len(),
        admitted * admitted.saturating_sub(1) / 2
    );
    assert_eq!(report["pair_evaluation_complete"], true);
}

#[test]
fn clones_data_records_resolved_receivers_and_verified_length_calls() {
    use rstyle_pdg::data_receivers;
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn slice_len(x: &[u16]) -> usize { x.len() }\npub fn slice_empty(x: &[u16]) -> bool { x.is_empty() }\npub fn str_len(x: &str) -> usize { x.len() }\npub fn str_empty(x: &str) -> bool { x.is_empty() }\npub fn array_len(x: &[u8; 2]) -> usize { x.len() }\npub fn owned_len(x: &String) -> usize { x.len() }\npub struct View<'a> { pub text: &'a str }\nimpl<'a> View<'a> { pub fn length(&self) -> usize { self.text.len() } }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 7);
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        if has_length_authority(&report) { 5 } else { 0 }
    );
    check_complete_data_pairs(&report, if has_length_authority(&report) { 5 } else { 0 });
    for function in functions {
        check_length_receiver_capture(function);
    }
    let (output, legacy) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{legacy}");
    assert!(legacy["functions"].as_array().unwrap().iter().all(|f| {
        f["facts"]["build"]["frontend_options"]
            .get(data_receivers::KEY)
            .is_none()
    }));
}

fn check_length_receiver_capture(function: &Value) {
    use rstyle_pdg::{concrete_types::TypeTree, data_receivers};
    let capture: data_receivers::Capture = serde_json::from_value(
        function["facts"]["build"]["frontend_options"][data_receivers::KEY].clone(),
    )
    .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    assert_eq!(data_receivers::verify(&facts).unwrap(), capture);
    if facts.name == "str_len" {
        check_receiver_evidence_tampering(&facts);
    }
    assert_eq!(capture.identity, data_receivers::IDENTITY);
    assert_eq!(capture.methods.len(), 1);
    let method = &capture.methods[0];
    let name = function["facts"]["name"].as_str().unwrap();
    if name == "array_len" {
        assert!(
            method
                .adjustments
                .iter()
                .any(|a| a.kind == data_receivers::Kind::Unsupported("pointer_coercion".into()))
        );
    } else if name == "owned_len" {
        assert!(matches!(
            method.receiver_type,
            rstyle_pdg::concrete_types::Observation::Unsupported { .. }
        ));
    } else {
        let referent = if name.starts_with("slice_") {
            TypeTree::Slice(Box::new(TypeTree::Scalar("u16".into())))
        } else {
            TypeTree::Str
        };
        check_shared_length_receiver(method, referent, name.ends_with("empty"));
    }
}

fn check_shared_length_receiver(
    method: &rstyle_pdg::data_receivers::Method,
    referent: rstyle_pdg::concrete_types::TypeTree,
    empty: bool,
) {
    use rstyle_pdg::{
        concrete_types::{Observation, TypeTree},
        data_receivers::Kind,
    };
    let shared = Observation::Captured {
        ty: TypeTree::Shared(Box::new(referent.clone())),
    };
    assert_eq!(method.receiver_type, shared);
    assert_eq!(method.adjusted_type, shared);
    assert_eq!(method.signature_inputs, [shared]);
    assert_eq!(
        method.signature_result,
        Observation::Captured {
            ty: TypeTree::Scalar(if empty { "bool" } else { "usize" }.into()),
        }
    );
    assert_eq!(
        method
            .adjustments
            .iter()
            .map(|a| &a.kind)
            .collect::<Vec<_>>(),
        [&Kind::BuiltinDereference, &Kind::SharedBorrow]
    );
    assert_eq!(
        method.adjustments[0].target,
        Observation::Captured { ty: referent }
    );
    assert_eq!(method.adjustments[1].target, method.adjusted_type);
}

#[test]
fn clones_data_keeps_overloaded_receiver_transitions_unavailable() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub struct Wrapped<'a> { pub value: &'a str }\nimpl<'a> core::ops::Deref for Wrapped<'a> { type Target = str; fn deref(&self) -> &str { self.value } }\npub fn overloaded_len(x: &Wrapped<'_>) -> usize { x.len() }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "overloaded_len")
        .unwrap();
    assert_eq!(function["outcome"], "excluded");
    let capture: rstyle_pdg::data_receivers::Capture = serde_json::from_value(
        function["facts"]["build"]["frontend_options"][rstyle_pdg::data_receivers::KEY].clone(),
    )
    .unwrap();
    assert_eq!(capture.methods.len(), 1);
    assert!(capture.methods[0].adjustments.iter().any(|a| {
        a.kind == rstyle_pdg::data_receivers::Kind::Unsupported("overloaded_dereference".into())
    }));
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_receiver_evidence_tampering(facts: &rstyle_pdg::HirFnFacts) {
    use rstyle_pdg::data_receivers::*;
    for change in [
        "identity",
        "missing",
        "duplicate",
        "call",
        "receiver",
        "span",
        "type",
        "result",
        "endpoint",
        "order",
        "target",
        "reason",
        "mutable",
    ] {
        let mut changed = facts.clone();
        let mut capture = verify(facts).unwrap();
        match change {
            "identity" => capture.identity = "unregistered".into(),
            "missing" => capture.methods.clear(),
            "duplicate" => capture.methods.push(capture.methods[0].clone()),
            _ => tamper_receiver_method(&mut capture.methods[0], change),
        }
        changed
            .build
            .frontend_options
            .insert(KEY.into(), serde_json::to_value(capture).unwrap());
        assert!(verify(&changed).is_err(), "{change}");
    }
    let mut changed = facts.clone();
    changed.build.frontend_options.remove(KEY);
    assert!(verify(&changed).is_err());
}

fn tamper_receiver_method(method: &mut rstyle_pdg::data_receivers::Method, change: &str) {
    use rstyle_pdg::{
        concrete_types::{Observation, TypeTree},
        data_receivers::Kind,
    };
    let scalar = Observation::Captured {
        ty: TypeTree::Scalar("u8".into()),
    };
    match change {
        "call" => method.call.local += 1,
        "receiver" => method.receiver.local += 1,
        "span" => method.source_span.start += 1,
        "type" => method.receiver_type = scalar,
        "result" => method.signature_result = scalar,
        "endpoint" => method.adjusted_type = scalar,
        "order" => method.adjustments.swap(0, 1),
        "target" => method.adjustments[0].target = scalar,
        "reason" => method.adjustments[0].kind = Kind::Unsupported("fake".into()),
        "mutable" => method.adjustments[1].kind = Kind::MutableBorrow,
        _ => unreachable!(),
    }
}

#[test]
fn clones_data_checks_serialized_receiver_evidence_without_reemission() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn first(x: i32) -> u32 { x.unsigned_abs() }\npub fn second(x: i32) -> u32 { x.unsigned_abs() }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 1);
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted");
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        rstyle_pdg::data_receivers::verify_documents(pair, &facts).unwrap();
        let mut bad_signature = facts.clone();
        bad_signature
            .build
            .frontend_options
            .get_mut(rstyle_pdg::data_receivers::KEY)
            .unwrap()["methods"][0]["signature_inputs"] = serde_json::json!([]);
        assert!(rstyle_pdg::data_receivers::verify(&bad_signature).is_err());
        shared_evidence::reject_facts(function, &bad_signature);
        for document in ["pdg", "cfg"] {
            let mut changed = rstyle_pdg::producer::Pair {
                pdg: pair.pdg.clone(),
                cfg: pair.cfg.clone(),
            };
            let extensions = if document == "pdg" {
                &mut changed.pdg.extensions
            } else {
                &mut changed.cfg.extensions
            };
            extensions
                .get_mut(rstyle_pdg::extraction_profile::EVIDENCE_KEY)
                .unwrap()["frontend_ref"]["function"] = u32::MAX.into();
            assert!(
                rstyle_pdg::data_receivers::verify_documents(&changed, &facts).is_err(),
                "{document}"
            );
        }
    }
}

#[test]
fn clones_data_corrupt_receiver_inventory_is_incomplete_handoff() {
    use std::os::unix::fs::PermissionsExt;
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn length(x: &str) -> usize { x.len() }\n",
    )
    .unwrap();
    let shim = directory.path().join("driver-shim");
    let real = driver().to_string_lossy().replace('\'', "'\\''");
    for mutation in [
        "options.pop('data_method_receivers')",
        "options['data_method_receivers']['methods'] = []",
        "options['data_method_receivers']['methods'].append(options['data_method_receivers']['methods'][0].copy())",
        "options['data_method_receivers']['methods'][0]['receiver']['local'] += 1",
    ] {
        let script = format!(
            "#!/bin/sh\n'{real}' \"$@\"\ncode=$?\nif [ \"$code\" = 0 ] && [ -n \"$RSTYLE_HIR_DOCUMENT_DIR\" ]; then\npython3 - <<'PY'\nimport os, json\nfrom pathlib import Path\np = Path(os.environ['RSTYLE_HIR_DOCUMENT_DIR']) / 'unit.evidence.json'\nbundle = json.loads(p.read_text())\nfor function in bundle['functions']:\n    options = function['build']['frontend_options']\n    {mutation}\np.write_text(json.dumps(bundle))\nPY\nfi\nexit \"$code\"\n"
        );
        std::fs::write(&shim, script).unwrap();
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--extraction-profile",
                "hir-data-v1",
                "--format",
                "json",
            ])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", &shim)
            .env_remove("CARGO_TARGET_DIR")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{mutation}: {}",
            stdout(&output)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["assessment_complete"], false);
        assert_eq!(report["pair_evaluation_complete"], false);
        assert!(!report["handoff_failures"].as_array().unwrap().is_empty());
    }
}

const DATA_LENGTH_FAMILY: &str = "pub struct View<'a> { pub bytes: &'a [u8], pub text: &'a str }\nimpl<'a> View<'a> {\n pub fn bytes_len(&self) -> usize { self.bytes.len() }\n pub fn bytes_empty(&self) -> bool { self.bytes.is_empty() }\n pub fn text_len(&self) -> usize { self.text.len() }\n pub fn text_empty(&self) -> bool { self.text.is_empty() }\n pub fn text(&self) -> &str { self.text }\n pub fn bytes(&self) -> &[u8] { self.bytes }\n}\npub struct Text { pub text: str }\npub fn explicit_borrow(value: &Text) -> usize { (&value.text).len() }\npub fn slice_len(value: &[u8]) -> usize { value.len() }\npub fn slice_empty(value: &[u8]) -> bool { value.is_empty() }\npub fn str_len(value: &str) -> usize { value.len() }\npub fn str_empty(value: &str) -> bool { value.is_empty() }\npub fn under(value: usize) -> bool { value < 8 }\npub fn short(value: &[u8]) -> bool { under(value.len()) }\npub fn counted(value: &[u8]) -> usize { let mut index = 0usize; while index < value.len() { index += 1; } index }\npub fn choose<'a>(first: &'a str, second: &'a str) -> &'a str { if first.is_empty() { second } else { first } }\n";

#[test]
fn clones_data_length_family_composes_fields_calls_loops_and_borrowed_branches() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("\u{feff}{}", DATA_LENGTH_FAMILY.replace('\n', "\r\n")),
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 15);
    let functions = check_data_family(
        &report,
        &[
            "View::<'a>::bytes_len",
            "View::<'a>::bytes_empty",
            "View::<'a>::text_len",
            "View::<'a>::text_empty",
            "explicit_borrow",
            "slice_len",
            "slice_empty",
            "str_len",
            "str_empty",
            "short",
            "counted",
            "choose",
        ],
    );
    assert_eq!(report["pair_evaluation_complete"], true);
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in &functions {
        let index = shared_evidence::index(function);
        let facts = analysis.function(index).unwrap();
        let pair = unit.pair(index).unwrap();
        rstyle_pdg::data_library::verify_documents(pair, &facts).unwrap();
        if facts.name == "str_len" {
            check_length_document_tampering(pair, &facts);
            check_length_frontend_tampering(&facts);
            check_library_toolchain_contract(&facts);
            check_length_projection(&report, function);
        }
    }
    let (output, warm) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{warm}");
    assert_eq!(pair_decisions(&report), pair_decisions(&warm));
}

fn check_length_projection(report: &Value, function: &Value) {
    let projected = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == function["key"])
        .unwrap();
    // One formal, receiver read, source CALL and return. Three value
    // dependences, two execution edges, one call and one read location.
    assert_eq!(
        projected["projection"]["features"],
        serde_json::json!([0, 2, 3, 0, 0, 0, 1, 3, 1])
    );
}

#[test]
fn clones_data_verifies_memory_free_shared_argument_helper_families() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub mod predicates { pub fn short(value: &[u8]) -> bool { value.len() < 8 } }\nuse predicates::short as small;\npub fn first(value: &[u8]) -> bool { predicates::short(value) }\npub fn second(value: &[u8]) -> bool { small(value) }\npub fn third(value: &[u8]) -> bool { first(value) }\npub fn empty(value: &str) -> bool { value.is_empty() }\npub fn text(value: &str) -> bool { empty(value) }\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 6);
    let functions = check_data_family(
        &report,
        &[
            "predicates::short",
            "first",
            "second",
            "third",
            "empty",
            "text",
        ],
    );
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in &functions {
        let index = shared_evidence::index(function);
        let facts = analysis.function(index).unwrap();
        let pair = unit.pair(index).unwrap();
        rstyle_pdg::helper_calls::verify_documents(pair, &facts).unwrap();
    }
    assert_eq!(report["pair_evaluation_complete"], true);
    if has_length_authority(&report) {
        check_shared_helper_dependency(&report);
    }
    let (output, warm) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{warm}");
    assert_eq!(pair_decisions(&report), pair_decisions(&warm));
}

fn check_shared_helper_dependency(report: &Value) {
    let wrapper = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "third")
        .unwrap();
    let (unit, index) = data_unit(wrapper);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let _ = unit.pair(index).unwrap();
    assert!(rstyle_pdg::helper_calls::native_width(&facts).unwrap());
    for corruption in ["manifest", "source", "signature", "effects"] {
        reject_helper(wrapper, corruption);
    }
}

#[test]
fn clones_data_shared_helpers_keep_memory_reads_and_recursion_closed() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub struct Data { pub field: u8 }\npub fn getter(value: &Data) -> u8 { value.field }\npub fn caller(value: &Data) -> u8 { getter(value) }\npub fn identity(value: &str) -> &str { value }\npub fn borrowed(value: &str) -> &str { identity(value) }\npub fn recursive(value: &[u8]) -> bool { recursive(value) }\npub fn pair(value: (&'static str, &'static str)) -> (&'static str, &'static str) { value }\npub fn pair_caller(value: (&'static str, &'static str)) -> (&'static str, &'static str) { pair(value) }\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        let expected = match function["facts"]["name"].as_str().unwrap() {
            "getter" | "identity" | "borrowed" | "pair" => "admitted",
            "caller" | "recursive" | "pair_caller" => "excluded",
            name => panic!("Unexpected function {name}"),
        };
        assert_eq!(function["outcome"], expected, "{function}");
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 6);
    assert_eq!(report["pair_evaluation_complete"], true);
}

const DATA_SHARED_RESULT_FAMILY: &str = "pub fn identity(value: &str) -> &str { value }\npub fn wrapper(value: &str) -> &str { identity(value) }\npub fn third(value: &str) -> &str { wrapper(value) }\npub fn first<'a>(left: &'a str, right: &'a str) -> &'a str { left }\npub fn second<'a>(left: &'a str, right: &'a str) -> &'a str { right }\npub fn swapped<'a>(left: &'a str, right: &'a str) -> &'a str { second(right, left) }\npub fn choose<'a>(flag: bool, left: &'a str, right: &'a str) -> &'a str { if flag { left } else { right } }\npub fn branch<'a>(flag: bool, left: &'a str, right: &'a str) -> &'a str { choose(flag, left, right) }\npub fn empty(value: &str) -> bool { identity(value).is_empty() }\npub struct View<'a> { pub text: &'a str }\npub fn field<'a>(value: &View<'a>) -> &'a str { identity(value.text) }\n";

#[test]
fn clones_data_shared_results_verify_selected_argument_origins_and_all_pairs() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        DATA_SHARED_RESULT_FAMILY,
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v1", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 10);
    let functions = check_data_family(&report, &["empty"]);
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in &functions {
        let index = shared_evidence::index(function);
        let facts = analysis.function(index).unwrap();
        let pair = unit.pair(index).unwrap();
        check_shared_result_origins(pair, &facts);
        if facts.name == "swapped" {
            check_shared_result_tampering(pair, &facts, function);
        }
    }
    assert_eq!(report["pair_evaluation_complete"], true);
    check_shared_result_projection(&report);
    let (output, warm) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{warm}");
    assert_eq!(pair_decisions(&report), pair_decisions(&warm));
    check_shared_result_edit(directory.path(), &report);
}

fn check_shared_result_edit(directory: &Path, previous: &Value) {
    let source = DATA_SHARED_RESULT_FAMILY.replace(
        "pub fn second<'a>(left: &'a str, right: &'a str) -> &'a str { right }",
        "pub fn second<'a>(left: &'a str, right: &'a str) -> &'a str { left }",
    );
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory, &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    let wrapper = |report: &Value| {
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == "swapped")
            .unwrap()
            .clone()
    };
    let before = wrapper(previous);
    let after = wrapper(&report);
    assert_eq!(after["outcome"], "admitted");
    assert_ne!(
        before["unit_evidence"]["namespace"],
        after["unit_evidence"]["namespace"]
    );
    assert_eq!(before["facts"]["body"], after["facts"]["body"]);
    let (unit, index) = data_unit(&after);
    let pair = unit.pair(index).unwrap();
    for node in pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == rstyle_pdg::NodeCategory::Return)
    {
        assert_eq!(
            node.annotations[rstyle_pdg::data_origins::KEY],
            serde_json::json!([
                {"parameter": 1, "path": [{"kind": "SharedReferent"}]}
            ])
        );
    }
    check_data_family(&report, &["empty"]);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_shared_result_origins(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    rstyle_pdg::helper_calls::verify_documents(pair, facts).unwrap();
    if facts.name == "branch" {
        assert_eq!(
            pair.pdg.extensions["rstyle.data:possible-formal-aliases"],
            serde_json::json!([[1, 1], [1, 2], [2, 2]])
        );
    }
    let positions: &[u32] = match facts.name.as_str() {
        "identity" | "wrapper" | "third" | "first" | "swapped" => &[0],
        "second" => &[1],
        "choose" | "branch" => &[1, 2],
        _ => return,
    };
    let expected: Vec<_> = positions
        .iter()
        .map(|position| {
            serde_json::json!({
                "parameter": position, "path": [{"kind": "SharedReferent"}]
            })
        })
        .collect();
    for node in pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == rstyle_pdg::NodeCategory::Return)
    {
        assert_eq!(
            node.annotations[rstyle_pdg::data_origins::KEY],
            serde_json::json!(expected)
        );
    }
}

fn check_shared_result_projection(report: &Value) {
    assert!(
        report["call_summaries"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("rstyle.helper.shared-return/1"))
    );
    let wrapper = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "wrapper")
        .unwrap();
    let projected = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["key"] == wrapper["key"])
        .unwrap();
    // Formal input, binding read, source CALL and return: two CFG edges,
    // three value edges, one CALL, three OTHER nodes and one binding read key.
    assert_eq!(
        projected["projection"]["features"],
        serde_json::json!([0, 2, 3, 0, 0, 0, 1, 3, 1])
    );
}

#[test]
fn clones_data_v2_registers_separately_and_preserves_verified_data_paths() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub struct Data { pub field: u8 }\npub fn getter(value: &Data) -> u8 { value.field }\npub fn caller(value: &Data) -> u8 { getter(value) }\npub fn empty(value: &str) -> bool { value.is_empty() }\npub fn identity(value: &str) -> &str { value }\npub fn via(value: &str) -> &str { identity(value) }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v2", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    assert_eq!(report["extraction_selection"], "hir-data-v2");
    assert_eq!(report["extraction_policy"]["version"], "2.0.0");
    assert_eq!(
        report["matching"]["profile_id"],
        "rstyle.ccgraph.data-calls"
    );
    assert_eq!(report["functions"].as_array().unwrap().len(), 5);
    check_data_family(&report, &["empty"]);
    let (output, old) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{old}");
    assert_eq!(old["matching"]["profile_id"], "rstyle.ccgraph.data");
    assert_eq!(old["extraction_policy"]["version"], "1.0.0");
    check_complete_data_pairs(&old, if has_length_authority(&old) { 4 } else { 3 });
}

const DATA_READ_CALL_FAMILY: &str = "pub struct Inner { pub n: u8, pub other: u8 }\npub struct Data { pub inner: Inner, pub extra: u8 }\npub struct Holder<'a> { pub ptr: &'a Data }\npub fn get(v: &Data) -> u8 { v.inner.n }\npub fn wrap(v: &Data) -> u8 { get(v) }\npub fn third(v: &Data) -> u8 { wrap(v) }\npub fn equal(left: &Data, right: &Data) -> bool { get(left) == get(right) }\npub fn alias_inputs(v: &Data) -> bool { equal(v, v) }\npub fn repeated(v: &Data) -> bool { v.inner.n == v.inner.n }\npub fn repeat_wrap(v: &Data) -> bool { repeated(v) }\npub mod access { use super::Data; pub fn read(v: &Data) -> u8 { v.extra } }\nuse access::read as imported;\npub fn qualified(v: &Data) -> u8 { access::read(v) }\npub fn aliased(v: &Data) -> u8 { imported(v) }\npub fn read_inner(v: &Inner) -> u8 { v.n }\npub fn nested(v: &Data) -> u8 { read_inner(&v.inner) }\npub fn choose<'a>(flag: bool, left: &'a Data, right: &'a Data) -> &'a Data { if flag { left } else { right } }\npub fn choose_get(flag: bool, left: &Data, right: &Data) -> u8 { get(choose(flag, left, right)) }\npub fn stored(v: &Holder<'_>) -> u8 { get(v.ptr) }\npub fn consume(v: &Data) { get(v); }\npub fn conditional(v: &Data, flag: bool) { if flag { consume(v); } }\npub fn casted(v: &Data) -> u32 { get(v) as u32 }\npub fn with_length(v: &Data, text: &str) -> bool { get(v) == 0 && text.is_empty() }\npub fn looped(v: &Data) -> u8 { let mut n = 0u8; while n < get(v) { n += 1; } n }\n";

#[test]
fn clones_data_inherent_calls_reach_all_pairs_under_separate_selection() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub struct Data { pub field: u8 }\nimpl Data { pub fn get(&self) -> u8 { self.field } pub fn wrap(&self) -> u8 { self.get() } }\npub fn invoke(value: &Data) -> u8 { value.wrap() }\nuse Data as Alias;\npub fn ufcs(value: &Data) -> u8 { Data::get(value) }\npub fn alias(value: &Data) -> u8 { Alias::get(value) }\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    assert_eq!(report["extraction_policy"]["version"], "3.35.0");
    assert_eq!(
        report["matching"]["profile_id"],
        "rstyle.ccgraph.data-calls"
    );
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted", "{function}");
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        rstyle_pdg::helper_calls::verify_documents(pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
    assert_eq!(report["pair_evaluation_complete"], true);
    let (output, old) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{old}");
    assert_eq!(
        read_call_function(&old, "Data::wrap")["outcome"],
        "excluded"
    );
    assert_eq!(read_call_function(&old, "invoke")["outcome"], "excluded");
}

const DATA_INHERENT_FAMILY: &str = r#"pub mod model {
 pub struct Inner { pub n: u8, pub other: u8 }
 pub struct Data { pub inner: Inner, pub extra: u8 }
 impl Inner { pub fn read(&self) -> u8 { self.n } }
 impl Data {
  pub fn get(&self) -> u8 { self.inner.n }
  pub fn wrap(&self) -> u8 { self.get() }
  pub fn third(&self) -> u8 { self.wrap() }
  pub fn equal(&self, other: &Self) -> bool { self.get() == other.get() }
  pub fn consume(&self) { self.get(); }
  pub fn conditional(&self, flag: bool) { if flag { self.consume(); } }
  pub fn casted(&self) -> u32 { self.get() as u32 }
  pub fn with_length(&self, text: &str) -> bool { self.get() == 0 && text.is_empty() }
  pub fn identity(&self) -> &Self { self }
 }
}
use model::Data;
use model::Data as Alias;
pub struct Holder<'a> { pub value: &'a Data }
pub fn invoke(value: &Data) -> u8 { value.third() }
pub fn ufcs(value: &Data) -> u8 { model::Data::get(value) }
pub fn aliased_ufcs(value: &Data) -> u8 { Alias::get(value) }
pub fn nested(value: &Data) -> u8 { (&value.inner).read() }
pub fn alias_inputs(value: &Data) -> bool { value.equal(value) }
pub fn choose<'a>(flag: bool, left: &'a Data, right: &'a Data) -> &'a Data { if flag { left } else { right } }
pub fn chosen(flag: bool, left: &Data, right: &Data) -> u8 { choose(flag, left, right).get() }
pub fn looped(value: &Data) -> u8 { let mut n = 0u8; while n < value.get() { n += 1; } n }
pub fn via_identity(value: &Data) -> u8 { value.identity().get() }
pub fn field_and_call(value: &Data) -> bool { value.extra == value.get() }
pub fn stored(value: &Holder<'_>) -> u8 { value.value.get() }
"#;

#[test]
fn clones_data_inherent_family_composes_source_methods_fields_and_helpers() {
    let directory = clone_workspace();
    let source = format!("\u{feff}{}", DATA_INHERENT_FAMILY.replace('\n', "\r\n"));
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let initial =
        rstyle_pdg::diagnostics::Stage::new("test.inherent.initial", "family", None).enter();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    initial.finish(Ok::<(), String>(())).unwrap();
    let assertions =
        rstyle_pdg::diagnostics::Stage::new("test.inherent.assertions", "family", None).enter();
    assert_eq!(report["functions"].as_array().unwrap().len(), 21);
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in check_data_family(&report, &["model::Data::with_length"]) {
        let index = shared_evidence::index(function);
        let facts = analysis.function(index).unwrap();
        let pair = unit.pair(index).unwrap();
        rstyle_pdg::helper_calls::verify_documents(pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    }
    assert_eq!(report["pair_evaluation_complete"], true);
    check_inherent_call_evidence(&report, &unit);
    check_inherent_tampering(read_call_function(&report, "model::Data::wrap"));
    assertions.finish(Ok::<(), String>(())).unwrap();
    let warm_run =
        rstyle_pdg::diagnostics::Stage::new("test.inherent.warm", "family", None).enter();
    let (output, warm) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{warm}");
    assert_eq!(pair_decisions(&report), pair_decisions(&warm));
    warm_run.finish(Ok::<(), String>(())).unwrap();
    let edit = rstyle_pdg::diagnostics::Stage::new("test.inherent.edit", "family", None).enter();
    check_inherent_call_edit(directory.path(), &report);
    edit.finish(Ok::<(), String>(())).unwrap();
}

fn check_inherent_call_edit(directory: &Path, previous: &Value) {
    let source = DATA_INHERENT_FAMILY.replace("{ self.inner.n }", "{ self.inner.other }");
    let source = format!("\u{feff}{}", source.replace('\n', "\r\n"));
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory, &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let (unit, _) = data_unit(&report["functions"][0]);
    for name in ["model::Data::wrap", "model::Data::third", "invoke"] {
        let old = read_call_function(previous, name);
        let new = read_call_function(&report, name);
        assert_ne!(
            old["unit_evidence"]["namespace"],
            new["unit_evidence"]["namespace"]
        );
        let pair = shared_evidence::pair(&unit, new);
        assert_eq!(
            pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
                [2]["name"],
            "other"
        );
    }
    check_data_family(&report, &["model::Data::with_length"]);
    assert_eq!(report["pair_evaluation_complete"], true);
}

#[test]
fn clones_data_inherent_calls_keep_unproved_receivers_and_callees_closed() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"pub struct Data { pub n: u8 }
impl Data {
 pub fn get(&self) -> u8 { self.n }
 pub fn by_value(self) -> u8 { self.n }
 pub fn borrowed(&self) -> &u8 { &self.n }
 pub fn checked(&self) -> u8 { self.n + 1 }
 pub fn recursive(&self) -> u8 { self.recursive() }
 pub fn divergent(&self) -> u8 { loop {} }
 pub fn mutable(&mut self) -> u8 { self.n }
 pub fn generic<T>(&self, _unused: T) -> u8 { self.n }
}
pub fn supported(v: &Data) -> u8 { v.get() }
pub fn consumes(v: Data) -> u8 { v.by_value() }
pub fn autoref(v: Data) -> u8 { v.get() }
pub fn additional(v: &&Data) -> u8 { v.get() }
pub fn mutable_receiver(v: &mut Data) -> u8 { v.mutable() }
pub fn borrowed_call(v: &Data) -> &u8 { v.borrowed() }
pub fn panicking(v: &Data) -> u8 { v.checked() }
pub fn recursive_call(v: &Data) -> u8 { v.recursive() }
pub fn divergent_call(v: &Data) -> u8 { v.divergent() }
pub fn generic_call(v: &Data) -> u8 { v.generic(0u8) }
"#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(
        read_call_function(&report, "supported")["outcome"],
        "admitted"
    );
    check_unproved_inherent_calls(&report);
}

fn check_unproved_inherent_calls(report: &Value) {
    for name in [
        "consumes",
        "autoref",
        "additional",
        "mutable_receiver",
        "borrowed_call",
        "panicking",
        "recursive_call",
        "divergent_call",
        "generic_call",
    ] {
        assert_eq!(
            read_call_function(report, name)["outcome"],
            "excluded",
            "{name}: {report}"
        );
    }
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_inherent_call_evidence(report: &Value, unit: &rstyle_pdg::unit_store::UnitStore) {
    let function = read_call_function(report, "model::Data::wrap");
    let facts = &unit.functions()[shared_evidence::index(function)];
    let pair = shared_evidence::pair(unit, function);
    let call = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
        .unwrap();
    assert_eq!(call.original_kind.known_value(), Some("expr_method_call"));
    assert_eq!(
        call.attributes.call_form,
        Some(rstyle_pdg::CallForm::Direct)
    );
    assert_eq!(call.attributes.reads.len(), 1);
    assert_eq!(call.attributes.reads[0].operand_position, Some(0));
    assert_eq!(
        call.attributes.reads[0].symbol_id.as_deref(),
        Some(facts.signature.params[0].symbol.as_str())
    );
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    assert_eq!(
        pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
            [1]["name"],
        "inner"
    );
    let projection = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == function["key"])
        .unwrap();
    assert_eq!(
        projection["projection"]["features"],
        serde_json::json!([0, 2, 3, 0, 0, 0, 1, 3, 2])
    );
    let direct = shared_evidence::pair(unit, read_call_function(report, "ufcs"));
    let direct = direct
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
        .unwrap();
    assert_eq!(direct.original_kind.known_value(), Some("expr_call"));
}

fn check_inherent_tampering(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    for change in [
        "owner",
        "implementation",
        "span",
        "identity",
        "receiver",
        "adjustment",
        "signature",
    ] {
        let mut changed = facts.clone();
        tamper_inherent_evidence(&mut changed, change);
        shared_evidence::reject_facts(function, &changed);
    }
    for change in ["missing", "root", "location", "position", "write"] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        let call = changed
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
            .unwrap();
        tamper_read_call(call, change);
        assert!(
            rstyle_pdg::helper_calls::verify_documents(&changed, &facts).is_err(),
            "{change}"
        );
    }
}

fn tamper_inherent_evidence(facts: &mut rstyle_pdg::HirFnFacts, change: &str) {
    let own = facts
        .build
        .frontend_options
        .get_mut(rstyle_pdg::inherent_calls::KEY)
        .unwrap();
    match change {
        "owner" => own["owner_nominal"]["definition_path"] = serde_json::json!("forged"),
        "implementation" => own["implementation_identity"] = serde_json::json!("forged"),
        "span" => own["implementation_span"]["start"] = serde_json::json!(0),
        "identity" => own["identity"] = serde_json::json!("forged"),
        "receiver" => {
            facts
                .build
                .frontend_options
                .get_mut(rstyle_pdg::data_receivers::KEY)
                .unwrap()["methods"][0]["receiver"]["local"] = serde_json::json!(999)
        }
        "adjustment" => {
            facts
                .build
                .frontend_options
                .get_mut(rstyle_pdg::data_receivers::KEY)
                .unwrap()["methods"][0]["adjustments"] = serde_json::json!([])
        }
        "signature" => {
            facts
                .build
                .frontend_options
                .get_mut(rstyle_pdg::data_receivers::KEY)
                .unwrap()["methods"][0]["signature_inputs"] = serde_json::json!([])
        }
        _ => unreachable!(),
    }
}

#[test]
fn clones_data_read_calls_compose_complete_shared_field_helper_families() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), DATA_READ_CALL_FAMILY).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v2", "--trace"],
    );
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 20);
    let functions = check_data_family(&report, &["with_length"]);
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in &functions {
        let index = shared_evidence::index(function);
        let facts = analysis.function(index).unwrap();
        let pair = unit.pair(index).unwrap();
        rstyle_pdg::helper_calls::verify_documents(pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    }
    assert_eq!(report["pair_evaluation_complete"], true);
    check_read_call_locations(&report, &unit);
    check_read_call_tampering(read_call_function(&report, "wrap"));
    let (output, warm) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{warm}");
    assert_eq!(pair_decisions(&report), pair_decisions(&warm));
    check_read_call_edit(directory.path(), &report);
}

fn read_call_function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

#[test]
fn clones_data_read_calls_keep_unproved_callee_boundaries_closed() {
    let directory = clone_workspace();
    let source = "pub struct Data { pub n: u8 }\n\
pub fn get(v: &Data) -> u8 { v.n }\n\
pub fn supported(v: &Data) -> u8 { get(v) }\n\
pub fn by_value(v: Data) -> u8 { v.n }\n\
pub fn value_call(v: Data) -> u8 { by_value(v) }\n\
pub fn borrowed(v: &Data) -> &u8 { &v.n }\n\
pub fn borrowed_call(v: &Data) -> &u8 { borrowed(v) }\n\
pub fn checked(v: &Data) -> u8 { v.n + 1 }\n\
pub fn checked_call(v: &Data) -> u8 { checked(v) }\n\
pub fn recursive(v: &Data) -> u8 { recursive(v) }\n\
pub fn recursive_call(v: &Data) -> u8 { recursive(v) }\n\
pub fn divergent(v: &Data) -> u8 { loop {} }\n\
pub fn divergent_call(v: &Data) -> u8 { divergent(v) }\n";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(
        read_call_function(&report, "supported")["outcome"],
        "admitted"
    );
    for name in [
        "value_call",
        "borrowed_call",
        "checked_call",
        "recursive",
        "recursive_call",
        "divergent_call",
    ] {
        assert_eq!(
            read_call_function(&report, name)["outcome"],
            "excluded",
            "{name}: {report}"
        );
    }
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_read_call_locations(report: &Value, unit: &rstyle_pdg::unit_store::UnitStore) {
    let wrapper = read_call_function(report, "wrap");
    let projected = report["matching"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["key"] == wrapper["key"])
        .unwrap();
    // Input, argument READ, source CALL and return; the binding and referent
    // location are two distinct keys. No callee nodes appear in this graph.
    assert_eq!(
        projected["projection"]["features"],
        serde_json::json!([0, 2, 3, 0, 0, 0, 1, 3, 2])
    );
    for (name, reads, locations) in [
        ("wrap", 1, 1),
        ("third", 1, 1),
        ("alias_inputs", 2, 1),
        ("repeat_wrap", 2, 1),
        ("choose_get", 2, 2),
    ] {
        let pair = shared_evidence::pair(unit, read_call_function(report, name));
        assert_eq!(
            pair.pdg
                .nodes
                .iter()
                .filter(|node| node.category == rstyle_pdg::NodeCategory::Call)
                .map(|node| node.attributes.reads.len())
                .sum::<usize>(),
            reads,
            "{name}"
        );
        assert_eq!(pair.pdg.memory_locations.len(), locations, "{name}");
        for location in &pair.pdg.memory_locations {
            let evidence = &location.extensions[rstyle_pdg::data_documents::LOCATION_KEY];
            assert_eq!(evidence["path"][0]["kind"], "SharedReferent");
            assert_eq!(evidence["path"][1]["name"], "inner");
            assert_eq!(evidence["path"][2]["name"], "n");
            assert_eq!(
                evidence["type"],
                serde_json::json!({"kind":"scalar", "value":"u8"})
            );
        }
    }
    check_local_location_renumbering(report, unit);
    check_stored_read_path(shared_evidence::pair(
        unit,
        read_call_function(report, "stored"),
    ));
}

fn check_local_location_renumbering(report: &Value, unit: &rstyle_pdg::unit_store::UnitStore) {
    let function = read_call_function(report, "choose_get");
    let index = shared_evidence::index(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    let count = pair.pdg.memory_locations.len();
    assert!(count > 1);
    let renaming: std::collections::BTreeMap<_, _> = pair
        .pdg
        .memory_locations
        .iter()
        .enumerate()
        .map(|(index, location)| {
            (
                location.id.clone(),
                format!("location:local:{}", count - index - 1),
            )
        })
        .collect();
    let mut document = serde_json::to_value(pair).unwrap();
    rename_location_references(&mut document, &renaming);
    let mut changed: rstyle_pdg::producer::Pair = serde_json::from_value(document).unwrap();
    changed.pdg.memory_locations.reverse();
    rstyle_pdg::data_documents::verify(&changed, &facts).unwrap();
    rstyle_pdg::producer::verify(&changed, &facts).unwrap();
    changed.pdg.memory_locations[0].id = changed.pdg.memory_locations[1].id.clone();
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&changed, &facts).is_err());
}

fn rename_location_references(
    value: &mut Value,
    names: &std::collections::BTreeMap<String, String>,
) {
    match value {
        Value::String(name) => {
            if let Some(replacement) = names.get(name) {
                *name = replacement.clone();
            }
        }
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| rename_location_references(value, names)),
        Value::Object(values) => values
            .values_mut()
            .for_each(|value| rename_location_references(value, names)),
        _ => {}
    }
}

fn check_stored_read_path(stored: &rstyle_pdg::producer::Pair) {
    let path = &stored
        .pdg
        .memory_locations
        .iter()
        .find(|location| {
            location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
                .as_array()
                .unwrap()
                .len()
                == 5
        })
        .unwrap()
        .extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"];
    assert_eq!(path[1]["name"], "ptr");
    assert_eq!(path[2]["kind"], "SharedReferent");
    assert_eq!(path[3]["name"], "inner");
    assert_eq!(path[4]["name"], "n");
}

fn check_read_call_tampering(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    for change in [
        "missing",
        "extra",
        "root",
        "location",
        "position",
        "unresolved",
        "write",
        "binding",
    ] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        let call = changed
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
            .unwrap();
        tamper_read_call(call, change);
        assert!(
            rstyle_pdg::helper_calls::verify_documents(&changed, &facts).is_err(),
            "{change}"
        );
    }
    for change in ["removed", "duplicate", "path", "type", "binding-read"] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        tamper_read_location(&mut changed, change);
        assert!(
            rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
            "{change}"
        );
    }
    check_read_summary_tampering(function);
}

fn check_read_summary_tampering(function: &Value) {
    for change in [
        "missing",
        "empty",
        "extra",
        "operation",
        "access",
        "parameter",
        "path",
        "type",
        "effect",
        "identity",
    ] {
        shared_evidence::reject_read_summary(function, change);
    }
    for change in ["manifest", "source", "signature", "effects"] {
        reject_helper(function, change);
    }
}

fn tamper_read_call(call: &mut rstyle_pdg::PdgNode, change: &str) {
    match change {
        "missing" => call.attributes.reads.clear(),
        "extra" => call.attributes.reads.push(call.attributes.reads[0].clone()),
        "root" => call.attributes.reads[0].symbol_id = Some("other".into()),
        "location" => call.attributes.reads[0].location_id = None,
        "position" => call.attributes.reads[0].operand_position = Some(1),
        "unresolved" => call.attributes.reads[0].unresolved_reason = Some("unknown".into()),
        "write" => call
            .attributes
            .writes
            .push(call.attributes.reads[0].clone()),
        "binding" => call.attributes.symbol_ids.push("invented".into()),
        _ => unreachable!(),
    }
}

fn tamper_read_location(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    let key = rstyle_pdg::data_documents::LOCATION_KEY;
    match change {
        "removed" => pair.pdg.memory_locations.clear(),
        "duplicate" => pair
            .pdg
            .memory_locations
            .push(pair.pdg.memory_locations[0].clone()),
        "path" => {
            pair.pdg.memory_locations[0]
                .extensions
                .get_mut(key)
                .unwrap()["path"][2]["name"] = serde_json::json!("other")
        }
        "type" => {
            pair.pdg.memory_locations[0]
                .extensions
                .get_mut(key)
                .unwrap()["type"]["value"] = serde_json::json!("u16")
        }
        "binding-read" => {
            let location = pair.pdg.memory_locations[0].id.clone();
            pair.pdg
                .nodes
                .iter_mut()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Read)
                .unwrap()
                .attributes
                .reads[0]
                .location_id = Some(location);
        }
        _ => unreachable!(),
    }
}

fn check_read_call_edit(directory: &Path, previous: &Value) {
    let source = DATA_READ_CALL_FAMILY.replace(
        "pub fn get(v: &Data) -> u8 { v.inner.n }",
        "pub fn get(v: &Data) -> u8 { v.inner.other }",
    );
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory, &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    let (unit, _) = data_unit(&report["functions"][0]);
    for name in ["wrap", "third"] {
        let old = read_call_function(previous, name);
        let new = read_call_function(&report, name);
        assert_ne!(
            old["unit_evidence"]["namespace"],
            new["unit_evidence"]["namespace"]
        );
        let pair = shared_evidence::pair(&unit, new);
        assert_eq!(
            pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
                [2]["name"],
            "other"
        );
    }
    check_data_family(&report, &["with_length"]);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_shared_result_tampering(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
    function: &Value,
) {
    for corruption in [
        "output_missing",
        "output_parameter",
        "output_empty",
        "output_path",
        "output_identity",
        "unit_invocation",
        "unit_target",
    ] {
        reject_helper(function, corruption);
    }
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    let call = changed
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
        .unwrap();
    let call_id = call.id.clone();
    call.annotations.insert(
        rstyle_pdg::data_origins::KEY.into(),
        serde_json::json!([
            {"parameter": 1, "path": [{"kind": "SharedReferent"}]}
        ]),
    );
    assert!(rstyle_pdg::data_documents::verify(&changed, facts).is_err());
    changed.pdg = pair.pdg.clone();
    for edge in changed
        .pdg
        .edges
        .iter_mut()
        .filter(|edge| edge.target == call_id && edge.subkind == "value_operand")
    {
        edge.operand_position = edge.operand_position.map(|position| 1 - position);
    }
    assert!(rstyle_pdg::data_documents::verify(&changed, facts).is_err());
}

fn check_length_document_tampering(
    pair: &rstyle_pdg::producer::Pair,
    facts: &rstyle_pdg::HirFnFacts,
) {
    for change in [
        "callee", "effect", "type", "operand", "summary", "library", "extra", "removed",
    ] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        tamper_length_document(&mut changed, change);
        assert!(
            rstyle_pdg::data_library::verify_documents(&changed, facts).is_err(),
            "{change}"
        );
    }
}

fn tamper_length_document(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    use rstyle_pdg::{NodeCategory, call_summaries::KEY};
    let index = pair
        .pdg
        .nodes
        .iter()
        .position(|node| node.category == NodeCategory::Call)
        .unwrap();
    match change {
        "callee" => {
            pair.pdg.nodes[index].attributes.callee = Some(rstyle_pdg::StringFact::known("fake"))
        }
        "effect" => pair.pdg.nodes[index].attributes.defines.push("fake".into()),
        "type" => {
            pair.pdg.nodes[index]
                .annotations
                .get_mut("compat.rust.hir-r0:type")
                .unwrap()["value"] = serde_json::json!("bool")
        }
        "operand" => pair.pdg.edges.retain(|edge| {
            !(edge.target == pair.pdg.nodes[index].id && edge.kind == rstyle_pdg::EdgeFamily::Data)
        }),
        "removed" => {
            pair.pdg.nodes.remove(index);
        }
        _ => {
            let records = pair
                .pdg
                .extensions
                .get_mut(KEY)
                .unwrap()
                .as_array_mut()
                .unwrap();
            match change {
                "summary" => records[0]["summary"]["may_panic"] = serde_json::json!(true),
                "library" => {
                    records[0]["library"]["callee"]["definition_hash"] = serde_json::json!("fake")
                }
                "extra" => records.push(records[0].clone()),
                _ => unreachable!(),
            }
            pair.cfg
                .extensions
                .insert(KEY.into(), pair.pdg.extensions[KEY].clone());
        }
    }
}

fn check_length_frontend_tampering(facts: &rstyle_pdg::HirFnFacts) {
    for change in ["missing", "crate", "signature", "definition"] {
        let mut changed = facts.clone();
        match change {
            "missing" => {
                changed
                    .build
                    .frontend_options
                    .remove(rstyle_pdg::data_library::KEY);
            }
            "crate" => {
                changed
                    .build
                    .frontend_options
                    .get_mut(rstyle_pdg::data_library::KEY)
                    .unwrap()["libraries"][0]["crate_hash"] =
                    serde_json::json!("contradictory core identity")
            }
            "signature" => {
                changed
                    .build
                    .frontend_options
                    .get_mut(rstyle_pdg::data_receivers::KEY)
                    .unwrap()["methods"][0]["signature_inputs"] = serde_json::json!([])
            }
            "definition" => {
                assert!(tamper_length_definition(&mut changed.body));
            }
            _ => unreachable!(),
        }
        assert!(
            rstyle_pdg::data_library::verify_evidence(&changed).is_err(),
            "{change}"
        );
    }
}

fn check_library_toolchain_contract(facts: &rstyle_pdg::HirFnFacts) {
    use rstyle_pdg::data_library::{IDENTITY, KEY, resolve, verify_evidence};
    let libraries = serde_json::to_value(verify_evidence(facts).unwrap()).unwrap();
    assert_eq!(libraries["identity"], IDENTITY);
    assert!(libraries["libraries"][0].get("files").is_none());
    assert!(
        !facts
            .build
            .frontend_options
            .contains_key("data_library_artifacts")
    );
    for change in ["compiler", "target", "width"] {
        let mut changed = facts.clone();
        match change {
            "compiler" => changed.build.rustc_commit = rstyle_pdg::StringFact::known("unsupported"),
            "target" => changed.build.target_triple = rstyle_pdg::StringFact::known("unsupported"),
            "width" => changed.build.pointer_width.value = Some(128),
            _ => unreachable!(),
        }
        let method = first_library_method(&changed.body).unwrap();
        assert!(resolve(method, &changed).unwrap().is_none(), "{change}");
        assert!(changed.build.frontend_options.contains_key(KEY));
    }
}

fn first_library_method(node: &rstyle_pdg::HirNode) -> Option<&rstyle_pdg::HirNode> {
    if rstyle_pdg::data_library::operation(node).is_some() {
        Some(node)
    } else {
        node.children.iter().find_map(first_library_method)
    }
}

fn tamper_length_definition(node: &mut rstyle_pdg::HirNode) -> bool {
    if node.kind == "expr_method_call" {
        let resolution = node.res.as_mut().unwrap();
        let mut callee: serde_json::Value = serde_json::from_str(&resolution.description).unwrap();
        callee["definition_hash"] = serde_json::json!("fake");
        resolution.description = serde_json::to_string(&callee).unwrap();
        return true;
    }
    node.children.iter_mut().any(tamper_length_definition)
}

fn pair_decisions(report: &Value) -> Vec<Value> {
    report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            serde_json::json!([
                pair["first"],
                pair["second"],
                pair["decision"],
                pair["displayed_score"]
            ])
        })
        .collect()
}

#[test]
fn clones_data_lengths_keep_unsupported_combinations_and_legacy_profiles_closed() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub struct Shadow { pub value: usize }\nimpl Shadow { pub fn len(&self) -> usize { self.value } }\npub fn shadow(value: &Shadow) -> usize { value.len() }\npub fn array(value: &[u8; 8]) -> usize { value.len() }\npub fn mutable(value: &mut [u8]) -> usize { value.len() }\npub fn extra(value: &&str) -> usize { value.len() }\npub fn unsummarized(value: &str) -> usize { value.as_bytes().len() }\npub fn owned(value: &String) -> usize { value.len() }\npub fn index(value: &[u8]) -> u8 { value[0] }\npub fn slice(value: &[u8]) -> usize { value.len() }\npub fn text(value: &str) -> usize { value.len() }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 10);
    for function in functions {
        let admitted = matches!(function["facts"]["name"].as_str().unwrap(), "Shadow::len")
            || (has_length_authority(&report)
                && matches!(
                    function["facts"]["name"].as_str().unwrap(),
                    "slice" | "text"
                ));
        assert_eq!(
            function["outcome"],
            if admitted { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    check_complete_data_pairs(&report, if has_length_authority(&report) { 3 } else { 1 });
    assert_eq!(report["pair_evaluation_complete"], true);
    for profile in ["r0", "hir-expanded-v1", "hir-helpers-v1"] {
        let (output, legacy) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(output.status.success(), "{legacy}");
        assert!(
            legacy["functions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["outcome"] == "excluded")
        );
        assert!(legacy["functions"].as_array().unwrap().iter().all(|f| {
            f["facts"]["build"]["frontend_options"]
                .get(rstyle_pdg::data_library::KEY)
                .is_none()
        }));
    }
}

#[test]
fn clones_data_library_corruption_is_incomplete_execution() {
    use std::os::unix::fs::PermissionsExt;
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn length(value: &str) -> usize { value.len() }\n",
    )
    .unwrap();
    let shim = directory.path().join("driver-shim");
    let real = driver().to_string_lossy().replace('\'', "'\\''");
    for mutation in [
        "options.pop('data_library_resolution')",
        "options['data_library_resolution']['libraries'][0]['crate_hash'] = 'contradictory core identity'",
        "options['data_library_resolution']['identity'] = 'rstyle.rust.library-artifact-capture/1'",
        "function['build']['rustc_commit']['value'] = '0' * 40",
    ] {
        let script = format!(
            "#!/bin/sh\n'{real}' \"$@\"\ncode=$?\nif [ \"$code\" = 0 ] && [ -n \"$RSTYLE_HIR_DOCUMENT_DIR\" ]; then\npython3 - <<'PY'\nimport os, json\nfrom pathlib import Path\np = Path(os.environ['RSTYLE_HIR_DOCUMENT_DIR']) / 'unit.evidence.json'\nbundle = json.loads(p.read_text())\nfor function in bundle['functions']:\n    options = function['build']['frontend_options']\n    {mutation}\np.write_text(json.dumps(bundle))\nPY\nfi\nexit \"$code\"\n"
        );
        std::fs::write(&shim, script).unwrap();
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--extraction-profile",
                "hir-data-v1",
                "--format",
                "json",
            ])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", &shim)
            .env_remove("CARGO_TARGET_DIR")
            .output()
            .unwrap();
        check_library_corruption(&output, mutation);
    }
}

fn check_library_corruption(output: &Output, mutation: &str) {
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(2), "{mutation}: {report}");
    assert_eq!(report["assessment_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(!report["handoff_failures"].as_array().unwrap().is_empty());
}

#[test]
fn clones_capture_only_retains_raw_facts_without_complete_publication() {
    let directory = clone_workspace();
    let snapshots = tempfile::tempdir().unwrap();
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-helpers-v1",
            ])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY", "1")
            .env("RSTYLE_DIAGNOSTIC_FACTS_DIR", snapshots.path())
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap();
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(report["functions"].as_array().unwrap().is_empty());
    let files: Vec<_> = std::fs::read_dir(snapshots.path()).unwrap().collect();
    assert_eq!(files.len(), 1);
    let bytes = std::fs::read(files[0].as_ref().unwrap().path()).unwrap();
    let snapshot: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(snapshot["version"], 1);
    assert_eq!(snapshot["scope"], "diagnostic-raw-facts-only");
    assert_eq!(snapshot["phase"], "pre-helper-certification");
    assert_eq!(snapshot["facts"].as_array().unwrap().len(), 4);
}

#[test]
fn clones_generated_policy_preserves_documents_and_structural_validation() {
    let directory = clone_workspace();
    let (output, report) = clones(directory.path(), &[]);
    assert!(output.status.success());
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = copy_pair(unit.pair(index).unwrap());
    rstyle_pdg::producer::verify(&pair, &facts).unwrap();
    let strict = rstyle_pdg::producer::produce(&facts)
        .unwrap()
        .serialize()
        .unwrap();
    let generated = rstyle_pdg::verification_policy::generated(|| {
        rstyle_pdg::producer::produce(&facts)
            .unwrap()
            .serialize()
            .unwrap()
    });
    assert_eq!(strict.pdg_bytes, generated.pdg_bytes);
    assert_eq!(strict.cfg_bytes, generated.cfg_bytes);
    assert_eq!(strict.manifest, generated.manifest);
    check_borrowed_serialized_pair(&strict, &facts);
    pair.pdg.nodes[1].id = pair.pdg.nodes[0].id.clone();
    assert!(
        rstyle_pdg::verification_policy::generated(|| {
            rstyle_pdg::producer::verify(&pair, &facts)
        })
        .is_err()
    );
}

fn check_borrowed_serialized_pair(
    strict: &rstyle_pdg::producer::SerializedPair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    let borrowed = rstyle_pdg::producer::verify_serialized_view(strict.as_view(), facts)
        .unwrap()
        .serialize()
        .unwrap();
    assert_eq!(strict.pdg_bytes, borrowed.pdg_bytes);
    assert_eq!(strict.cfg_bytes, borrowed.cfg_bytes);
    let mut edited_cfg = strict.cfg_bytes.clone();
    edited_cfg.push(b' ');
    let edited = rstyle_pdg::producer::SerializedPairView {
        pdg_bytes: &strict.pdg_bytes,
        cfg_bytes: &edited_cfg,
        manifest: &strict.manifest,
    };
    assert!(rstyle_pdg::producer::verify_serialized_view(edited, facts).is_err());
    assert!(
        rstyle_pdg::verification_policy::generated(|| {
            rstyle_pdg::producer::verify_serialized_view(edited, facts)
        })
        .is_err()
    );
}

#[test]
fn clones_helpers_repeated_dependencies_preserve_strict_checks() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn leaf(x: i32) -> bool { x > 0 }\npub fn repeated(x: i32) -> bool { leaf(x) && leaf(x) }\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{report}");
    let function = read_call_function(&report, "repeated");
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    rstyle_pdg::producer::verify(pair, &facts).unwrap();
    let strict = rstyle_pdg::producer::produce(&facts)
        .unwrap()
        .serialize()
        .unwrap();
    let original = pair.serialize().unwrap();
    assert_eq!(strict.pdg_bytes, original.pdg_bytes);
    assert_eq!(strict.cfg_bytes, original.cfg_bytes);
    assert_eq!(strict.manifest, original.manifest);
    for corruption in ["manifest", "source", "signature", "effects"] {
        reject_helper(function, corruption);
    }
    rstyle_pdg::producer::verify(pair, &facts).unwrap();
}

#[test]
fn shared_unit_borrows_lexical_results_with_policy_and_context_isolation() {
    use rstyle_pdg::unit_store::EvidenceFacts;
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub struct Data { pub n: u8 }\nimpl Data { pub fn get(&self) -> u8 { self.n } }\npub fn first(v: &Data) -> u8 { v.get() }\npub fn second(v: &Data) -> u8 { v.get() }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let path = Path::new(report["observed_units"][0]["directory"].as_str().unwrap());
    let bytes = std::fs::read(path.join("unit.evidence.json")).unwrap();
    let store = rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes).unwrap();
    let index = store
        .functions()
        .iter()
        .position(|facts| facts.name.ends_with("::get"))
        .unwrap();
    let analysis = store.analysis();
    let view = analysis.function(index).unwrap();
    let first = view.nominal_resolution().unwrap();
    let second = view.nominal_resolution().unwrap();
    assert!(std::ptr::eq(&*first, &*second));
    let inherent = view.inherent_definition().unwrap().unwrap();
    assert!(matches!(inherent, std::borrow::Cow::Borrowed(_)));
    assert!(std::ptr::eq(
        inherent.as_ref(),
        view.inherent_definition().unwrap().unwrap().as_ref()
    ));
    rstyle_pdg::verification_policy::generated(|| {
        let generated = view.nominal_resolution().unwrap();
        assert!(!std::ptr::eq(&*first, &*generated));
        assert_eq!(first.namespace, generated.namespace);
        assert_eq!(first.receiver, generated.receiver);
    });
    let independent = store.analysis();
    let independent_view = independent.function(index).unwrap();
    assert!(!std::ptr::eq(
        &*first,
        &*independent_view.nominal_resolution().unwrap()
    ));
    let other_store = rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes).unwrap();
    let other_analysis = other_store.analysis();
    let other_view = other_analysis.function(index).unwrap();
    assert!(!std::ptr::eq(
        &*first,
        &*other_view.nominal_resolution().unwrap()
    ));
    let mut edited = view.facts().clone();
    edited.source.bytes = edited.source.bytes.replace("self.n", "self.n + 1").into();
    edited.source.digest = rstyle_pdg::concrete_types::digest(edited.source.bytes.as_bytes());
    assert!(edited.nominal_resolution().is_err());
    assert!(edited.inherent_definition().is_err());
    assert!(std::ptr::eq(&*first, &*view.nominal_resolution().unwrap()));
}

#[test]
fn clones_borrowed_typed_pairs_preserve_verification_and_strict_admission() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn leaf(x: usize) -> usize { x }\npub fn caller(x: usize) -> usize { leaf(x) }\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-helpers-v1"],
    );
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "caller")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    let borrowed = pair.as_view();
    assert!(std::ptr::eq(borrowed.pdg, &pair.pdg));
    assert!(std::ptr::eq(borrowed.cfg, &pair.cfg));
    assert_eq!(
        serde_json::to_vec(pair).unwrap(),
        serde_json::to_vec(&borrowed).unwrap()
    );
    assert_eq!(
        rstyle_pdg::producer::verify_view(borrowed, &facts),
        rstyle_pdg::producer::verify(pair, &facts)
    );
    let generated = rstyle_pdg::verification_policy::generated(|| {
        rstyle_pdg::producer::verify_view(borrowed, &facts)
    });
    assert!(generated.is_ok());
    let leaf = unit.functions().iter().find(|f| f.name == "leaf").unwrap();
    let standalone = rstyle_pdg::producer::produce(leaf).unwrap();
    check_borrowed_pair_admission(&standalone, leaf);
    check_borrowed_pair_tampering(pair, &facts);
}

fn check_borrowed_pair_admission(
    pair: &rstyle_pdg::producer::Pair,
    facts: &rstyle_pdg::HirFnFacts,
) {
    let borrowed = pair.as_view();
    let sources = pair
        .pdg
        .sources
        .iter()
        .map(|source| (source.id.clone(), facts.source.bytes.as_bytes().to_vec()))
        .collect();
    let context = rstyle_pdg::ValidationContext::CompilerEvidence {
        pointer_width_bits: u8::try_from(facts.build.pointer_width.value.unwrap()).unwrap(),
    };
    let standalone = rstyle_ccgraph::admit_strict(&rstyle_ccgraph::StrictBundle {
        pdg: borrowed.pdg,
        cfg: borrowed.cfg,
        source_bytes: &sources,
        ctx: &context,
    })
    .unwrap();
    assert!(standalone.native_width_involved);
    assert_eq!(
        standalone,
        rstyle_ccgraph::admit_produced(pair, facts).unwrap()
    );
}

fn check_borrowed_pair_tampering(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    let mut cfg = pair.cfg.clone();
    cfg.entry_node_id = "missing".into();
    let changed = rstyle_pdg::producer::PairView {
        pdg: &pair.pdg,
        cfg: &cfg,
    };
    assert!(rstyle_pdg::producer::verify_view(changed, facts).is_err());
    assert!(
        rstyle_pdg::verification_policy::generated(|| rstyle_pdg::producer::verify_view(
            changed, facts
        ))
        .is_err()
    );
}

#[test]
fn clones_cfg_definition_inventory_uses_actual_build_guards() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("Cargo.toml"), "[package]\nname='cfg_definitions'\nversion='0.1.0'\nedition='2021'\n[features]\nextra=[]\n[workspace]\n").unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "#[cfg(feature=\"extra\")] pub fn alternative() {}\n#[cfg(not(feature=\"extra\"))] pub fn alternative() {}\n#[cfg(any())] mod hidden { pub fn nested() { missing(); } }\nstruct S; #[cfg(any())] impl S { fn method() {} }\n#[cfg(any())] trait T { fn defaulted() {} }\npub fn active() {}\n").unwrap();
    for options in [vec![], vec!["--features", "extra"]] {
        let args: Vec<_> = ["--extraction-profile", "hir-data-v3"]
            .into_iter()
            .chain(options)
            .collect();
        let (output, report) = clones(directory.path(), &args);
        assert!(output.status.success(), "{report}");
        let definitions = report["owned_source_inventory"][0]["source_functions"]
            .as_array()
            .unwrap();
        let alternatives: Vec<_> = definitions
            .iter()
            .filter(|f| f["name"] == "alternative")
            .collect();
        assert_eq!(alternatives.len(), 2);
        assert_eq!(
            alternatives
                .iter()
                .filter(|f| f["presence"] == "active")
                .count(),
            1
        );
        assert_eq!(
            alternatives
                .iter()
                .filter(|f| f["presence"] == "inactive_by_cfg")
                .count(),
            1
        );
        for name in ["nested", "method", "defaulted"] {
            let definition = definitions.iter().find(|f| f["name"] == name).unwrap();
            assert_eq!(definition["presence"], "inactive_by_cfg");
            assert_eq!(definition["active_functions"], serde_json::json!([]));
        }
    }
}

#[test]
fn clones_cfg_inventory_retains_files_without_active_function_bodies() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "#[cfg(any())] pub fn inactive() { missing(); }\n",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let definitions = report["owned_source_inventory"][0]["source_functions"]
        .as_array()
        .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0]["presence"], "inactive_by_cfg");
    assert_eq!(definitions[0]["active_functions"], serde_json::json!([]));
}

#[test]
fn clones_data_exhaustive_scalar_matches_compose_without_wildcards() {
    let directory = exhaustive_scalar_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(
        functions
            .iter()
            .filter(|f| f["outcome"] == "admitted")
            .count(),
        15,
        "{report}"
    );
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == "boolean")
            .unwrap()["facts"]
            .clone(),
    )
    .unwrap();
    check_exhaustive_scalar_match(&facts);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

fn check_exhaustive_scalar_match(facts: &rstyle_pdg::HirFnFacts) {
    let pair = rstyle_pdg::producer::produce(facts).unwrap();
    assert_eq!(
        pair.pdg
            .nodes
            .iter()
            .filter(|n| n.attributes.construct.as_deref() == Some("match.literal"))
            .count(),
        2
    );
    check_exhaustive_match_edges(&pair, facts);
    check_guarded_coverage_gap(facts);
    check_exhaustive_literal_source(facts);
    let mut changed = facts.clone();
    let matched = &mut changed.body.children[0];
    assert_eq!(matched.kind, "expr_match");
    matched.children[2].children[0].lit = Some("true".into());
    let classified = rstyle_pdg::classify(&changed);
    assert!(
        classified
            .diagnostics
            .iter()
            .any(|d| d.construct == "match_boundary")
    );
    assert!(rstyle_pdg::producer::verify(&pair, &changed).is_err());
}

fn check_exhaustive_match_edges(pair: &rstyle_pdg::producer::Pair, facts: &rstyle_pdg::HirFnFacts) {
    let selector = pair
        .pdg
        .nodes
        .iter()
        .rfind(|n| n.attributes.construct.as_deref() == Some("match.literal"))
        .unwrap();
    let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let mapped = mapping
        .iter()
        .find(|entry| entry.pdg_node_id == selector.id)
        .unwrap();
    let edges: Vec<_> = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| edge.source == mapped.cfg_node_id)
        .collect();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].kind, rstyle_pdg::CfgEdgeKind::True);
    let mut impossible = edges[0].clone();
    impossible.kind = rstyle_pdg::CfgEdgeKind::False;
    let mut corrupted = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    corrupted.cfg.edges.push(impossible);
    assert!(rstyle_pdg::validate_value_sites(&corrupted.pdg, &corrupted.cfg).is_err());
    assert!(rstyle_pdg::producer::verify(&corrupted, facts).is_err());
}

fn check_guarded_coverage_gap(facts: &rstyle_pdg::HirFnFacts) {
    let mut changed = facts.clone();
    let matched = &mut changed.body.children[0];
    let guard = rstyle_pdg::HirNode {
        kind: "match_guard".into(),
        children: vec![matched.children[0].clone()],
        ..matched.children[0].clone()
    };
    matched.children[1].children.insert(1, guard);
    let classified = rstyle_pdg::classify(&changed);
    assert!(
        classified
            .diagnostics
            .iter()
            .any(|d| d.construct == "match_boundary")
    );
}

fn exhaustive_scalar_workspace() -> tempfile::TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        const ON: bool = true;
        const OFF: bool = false;
        const TOP: u128 = u128::MAX;
        pub fn boolean(x: bool) -> i32 { match x { true => 1, false => 2 } }
        pub fn caller(x: bool) -> i32 { boolean(x) }
        pub fn constants(x: bool) -> i32 { match x { ON => 1, OFF => 2 } }
        pub fn guarded(x: bool, gate: bool) -> i32 { match x { true if gate => 1, true => 2, false => 3 } }
        pub fn signed(x: i8) -> i32 { match x { ..=-1 => 1, 0.. => 2 } }
        pub fn unsigned(x: u8) -> i32 { match x { 0..128 => 1, 128.. => 2 } }
        pub fn wide(x: u128) -> i32 { match x { 0..TOP => 1, TOP => 2 } }
        pub fn minimum(x: i128) -> i32 { match x { -170141183460469231731687303715884105728..0 => 1, 0.. => 2 } }
        pub fn single(x: u8) -> i32 { match x { 0..=255 => 1 } }
        pub fn early(x: bool) -> i32 { match x { true | false => 1, _ if x => 2, _ => 3 } }
        pub fn returning(x: bool, y: i32) -> i32 { match x { true => return y, false => 2 } }
        pub fn conditional(x: bool) -> i32 { match x { #[cfg(any())] _ => missing(), true => 1, false => 2 } }
        pub fn mixed(x: u8, gate: bool) -> i32 { match x { 0 | 1..=3 if gate => 1, 0..=127 => 2, 128..=255 => 3 } }
        pub fn native(x: usize) -> i32 { match x { 0..=3 => 1, 4.. => 2 } }
        pub fn unreachable(x: bool) -> i32 { match x { true | false => 1, _ if x => 2 } }
        pub fn unsupported(x: char) -> i32 { match x { '\0'..='\u{10ffff}' => 1 } }
    "#).unwrap();
    directory
}

fn check_exhaustive_literal_source(facts: &rstyle_pdg::HirFnFacts) {
    let mut changed = facts.clone();
    let matched = &mut changed.body.children[0];
    matched.children[1].children[0].lit = Some("false".into());
    matched.children[2].children[0].lit = Some("true".into());
    let classified = rstyle_pdg::classify(&changed);
    assert_eq!(
        classified.status,
        rstyle_pdg::EligibilityVerdict::Ineligible
    );
    assert!(
        classified
            .diagnostics
            .iter()
            .any(|d| d.construct == "arm_shape")
    );
    assert!(
        !classified
            .diagnostics
            .iter()
            .any(|d| d.construct == "match_boundary")
    );
}

#[test]
fn clones_data_nested_scalar_patterns_compose_with_ranges_constants_and_guards() {
    let directory = nested_scalar_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    for name in [
        "nested", "flat", "mixed", "boolean", "wildcard", "grouped", "constant", "range",
        "leading", "repeated",
    ] {
        let function = functions
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "admitted", "{name}: {function}");
    }
    for name in ["payload", "binding", "reference"] {
        assert_ne!(
            functions
                .iter()
                .find(|f| f["facts"]["name"] == name)
                .unwrap()["outcome"],
            "admitted"
        );
    }
    check_nested_scalar_selector(functions);
    check_nested_scalar_payloads(functions);
    check_equivalent_matching(&report, ["nested", "flat"]);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded")
    );
}

fn nested_scalar_workspace() -> tempfile::TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        const LOW: i32 = -4;
        const HIGH: i32 = 4;
        pub fn nested(x: i32, gate: bool) -> i32 { match x { (1 | (2 | 3)) if gate => 7, _ => 9 } }
        pub fn flat(x: i32, gate: bool) -> i32 { match x { 1 | 2 | 3 if gate => 7, _ => 9 } }
        pub fn mixed(x: i32) -> i32 { match x { ((LOW | (0..=HIGH))) | 9 => 1, _ => 2 } }
        pub fn boolean(x: bool) -> i32 { match x { (/* outside */ true | (false)) => 1 } }
        pub fn wildcard(x: i32, gate: bool) -> i32 { match x { ((1 | _)) if gate => 1, _ => 2 } }
        pub fn grouped(x: i32) -> i32 { match x { (/* before */ (- /* minus */ 1) /* after */) => 1, _ => 2 } }
        pub fn constant(x: i32) -> i32 { match x { ((LOW)) => 1, _ => 2 } }
        pub fn range(x: i32) -> i32 { match x { (/* range */ (LOW..=HIGH)) => 1, _ => 2 } }
        pub fn leading(x: bool) -> i32 { match x { | (true | false) => 1 } }
        pub fn repeated(x: i32) -> i32 { match x { (1 | 1) | (2 | 2) => 1, _ => 2 } }
        pub fn payload(x: Option<i32>) -> i32 { match x { (Some(1 | 2) | None) => 1, _ => 2 } }
        pub fn binding(x: i32) -> i32 { match x { (value @ 1 | value @ 2) => value, _ => 0 } }
        pub fn reference(x: &i32) -> i32 { match x { (&1 | &2) => 1, _ => 0 } }
    "#).unwrap();
    directory
}

fn check_nested_scalar_selector(functions: &[Value]) {
    let facts = nested_scalar_facts(functions, "nested");
    let pattern = &facts.body.children[0].children[1].children[0];
    assert_eq!(pattern.kind, "pat_or");
    assert!(pattern.children.iter().any(|child| child.kind == "pat_or"));
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let selectors: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("match.or"))
        .collect();
    assert_eq!(selectors.len(), 1);
    assert_eq!(
        selectors[0]
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value(),
        Some("[\"1\",\"2\",\"3\"]")
    );
    assert_eq!(
        pair.pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some("match.guard"))
            .count(),
        1
    );
    let selector = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("match.or"))
        .unwrap();
    selector.attributes.literal = Some(rstyle_pdg::StringFact::known("[\"1\",\"2\"]"));
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
}

fn nested_scalar_facts(functions: &[Value], name: &str) -> rstyle_pdg::HirFnFacts {
    serde_json::from_value(
        functions
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap()["facts"]
            .clone(),
    )
    .unwrap()
}

fn check_nested_scalar_payloads(functions: &[Value]) {
    for (name, expected) in [
        ("flat", "[\"1\",\"2\",\"3\"]"),
        ("repeated", "[\"1\",\"1\",\"2\",\"2\"]"),
        ("wildcard", "[[\"pat_literal\",\"1\"],[\"pat_wild\",null]]"),
    ] {
        let facts = nested_scalar_facts(functions, name);
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let selector = pair
            .pdg
            .nodes
            .iter()
            .find(|node| {
                node.attributes
                    .construct
                    .as_deref()
                    .is_some_and(|construct| construct.starts_with("match.or"))
            })
            .unwrap();
        assert_eq!(
            selector.attributes.literal.as_ref().unwrap().known_value(),
            Some(expected)
        );
        if name == "wildcard" {
            let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
            let mapped = mapping
                .iter()
                .find(|entry| entry.pdg_node_id == selector.id)
                .unwrap();
            assert!(
                !pair
                    .cfg
                    .edges
                    .iter()
                    .any(|edge| edge.source == mapped.cfg_node_id
                        && edge.kind == rstyle_pdg::CfgEdgeKind::False)
            );
        }
    }
}

fn check_equivalent_matching(report: &Value, names: [&str; 2]) {
    let functions = report["functions"].as_array().unwrap();
    let keys: Vec<_> = names
        .iter()
        .map(|name| {
            &functions
                .iter()
                .find(|f| f["facts"]["name"] == *name)
                .unwrap()["key"]
        })
        .collect();
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (pair["first"] == *keys[0] && pair["second"] == *keys[1])
                || (pair["first"] == *keys[1] && pair["second"] == *keys[0])
        })
        .unwrap();
    assert_eq!(pair["evaluation_status"], "evaluated");
    assert_eq!(pair["decision"], "Accepted");
    assert_eq!(pair["displayed_score"], "1");
}

#[test]
fn clones_data_nominal_constants_compose_with_fields_patterns_and_helpers() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        mod model {
            pub const LIMIT: i32 = 4;
            pub struct Data { pub x: i32 }
            impl Data {
                pub const LOW: i32 = -2;
                pub fn get(&self) -> i32 { self.x }
                pub fn classify(&self) -> i32 {
                    match self.x { Self::LOW..=LIMIT => 1, _ => 0 }
                }
            }
        }
        use model::Data as Record;
        pub fn classify(data: &Record) -> i32 {
            match data.x { Record::LOW..=model::LIMIT => 1, _ => 0 }
        }
        pub fn forward(data: &Record) -> i32 { data.classify() }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = check_data_family(&report, &[]);
    assert_eq!(functions.len(), 4);
    let (unit, _) = data_unit(functions[0]);
    let analysis = unit.analysis();
    for function in functions {
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        rstyle_pdg::helper_calls::verify_documents(pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    }
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded"),
        "{legacy}"
    );
}

#[test]
fn clones_data_nominal_constant_initializers_cannot_hide_drop_implementations() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub struct Data { pub x: i32 }
        const _: () = {
            struct Hidden;
            impl Drop for Hidden { fn drop(&mut self) {} }
        };
        pub fn get(data: &Data) -> i32 { data.x }
        pub fn scalar(x: i32) -> i32 { x }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["assessment_complete"], true);
    for function in report["functions"].as_array().unwrap() {
        let expected = if function["facts"]["name"] == "scalar" {
            "admitted"
        } else {
            "excluded"
        };
        assert_eq!(function["outcome"], expected, "{function}");
    }
    assert!(report["handoff_failures"].as_array().unwrap().is_empty());
}

#[test]
fn clones_data_aggregate_literals_preserve_nested_values_and_component_order() {
    let directory = aggregate_literal_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["assessment_complete"], true);
    check_aggregate_admission(&report);
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["tuple", "other"]);
    check_aggregate_corruption(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn check_aggregate_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "array")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let constructor = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.original_kind.known_value() == Some("expr_array")
                && node.category == rstyle_pdg::NodeCategory::Computation
        })
        .unwrap()
        .id
        .clone();
    let edges: Vec<_> = pair
        .pdg
        .edges
        .iter_mut()
        .filter(|edge| edge.target == constructor && edge.subkind == "value_operand")
        .collect();
    assert_eq!(edges.len(), 3);
    for edge in edges {
        if edge.operand_position == Some(0) {
            edge.operand_position = Some(1);
        } else if edge.operand_position == Some(1) {
            edge.operand_position = Some(0);
        }
    }
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    let mut changed = facts.clone();
    changed.body.children[0].children.swap(0, 1);
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
}

fn check_aggregate_documents(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["outcome"] == "admitted")
    {
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
        rstyle_pdg::producer::verify(pair, &facts).unwrap();
    }
}

fn aggregate_literal_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn tuple(x: i32, b: bool) -> (i32, bool) { (x + 1, !b) }
        pub fn other(x: i32, b: bool) -> (i32, bool) { (x + 1, !b) }
        pub fn array(x: i32) -> [i32; 3] { [x, x + 1, x + 2] }
        pub fn nested(x: i32) -> ([i32; 2], (bool, i32)) { ([x, x + 1], (true, x)) }
        pub fn empty() -> [i32; 0] { [] }
        pub fn conditional(x: i32, b: bool) -> (i32, bool) { (if b { x } else { 0 }, b) }
        pub fn units(x: i32) -> ((), i32, ()) { ({}, x, { let _value = 1; }) }
        pub fn unit_array() -> [(); 2] { [{}, ()] }
        pub fn snapshots(mut x: i32) -> (i32, i32) { (x, { x = 3; x }) }
        pub fn local(x: i32) -> (i32, bool) { let y = (x, true); y }
        pub fn borrowed(x: &i32) -> (&i32, bool) { (x, true) }
        pub fn repeat(x: i32) -> [i32; 3] { [x; 3] }
    "#,
    )
    .unwrap();
    directory
}

fn check_aggregate_admission(report: &Value) {
    let names: Vec<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["facts"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 12, "{names:?}");
    for name in [
        "tuple",
        "other",
        "array",
        "nested",
        "empty",
        "conditional",
        "units",
        "unit_array",
        "snapshots",
        "local",
        "borrowed",
        "repeat",
    ] {
        assert!(names.contains(&name), "{name}: {names:?}");
    }
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
}

#[test]
fn clones_data_repeat_arrays_evaluate_once_without_expanding_large_lengths() {
    let directory = repeat_array_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["assessment_complete"], true);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "references" | "named" | "complex"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["repeat", "other"]);
    check_repeat_array_graphs(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn repeat_array_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        const N: usize = 4;
        pub fn repeat(x: i32) -> [i32; 4] { [x; 4] }
        pub fn other(x: i32) -> [i32; 4] { [x; 0x4usize] }
        pub fn huge(x: i32) -> [i32; 65536] { [x; 65_536] }
        pub fn zero_effect(mut x: i32) -> i32 { [{ x = 3; x }; 0]; x }
        pub fn nested(x: i32) -> [[i32; 2]; 3] { [[x; 2]; 3] }
        pub fn unit() -> [(); 0] { [{}; 0] }
        pub fn unitmany() -> [(); 7] { [{}; 7] }
        pub fn conditional(x: i32, b: bool) -> [i32; 3] { [if b { x } else { 0 }; 3] }
        pub fn references(x: &i32) -> [&i32; 4] { [x; 4] }
        pub fn named(x: i32) -> [i32; 4] { [x; N] }
        pub fn complex(x: i32) -> [i32; 4] { [x; 2 + 2] }
    "#,
    )
    .unwrap();
    directory
}

fn check_repeat_array_graphs(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let facts = |name| -> rstyle_pdg::HirFnFacts {
        serde_json::from_value(
            functions
                .iter()
                .find(|f| f["facts"]["name"] == name)
                .unwrap()["facts"]
                .clone(),
        )
        .unwrap()
    };
    let small = rstyle_pdg::producer::produce(&facts("repeat")).unwrap();
    let huge = rstyle_pdg::producer::produce(&facts("huge")).unwrap();
    assert_eq!(small.pdg.nodes.len(), huge.pdg.nodes.len());
    assert_eq!(small.pdg.edges.len(), huge.pdg.edges.len());
    let zero = facts("zero_effect");
    let pair = rstyle_pdg::producer::produce(&zero).unwrap();
    assert_eq!(
        pair.pdg
            .nodes
            .iter()
            .filter(|n| n.category == rstyle_pdg::NodeCategory::Assignment)
            .count(),
        1
    );
    let constructor = pair
        .pdg
        .nodes
        .iter()
        .find(|n| n.original_kind.known_value() == Some("expr_repeat"))
        .unwrap();
    assert_eq!(
        pair.pdg
            .edges
            .iter()
            .filter(|e| e.target == constructor.id && e.subkind == "value_operand")
            .count(),
        1
    );
    let mut changed = facts("repeat");
    let ty = changed.body.children[0].ty.as_mut().unwrap();
    let mut token: serde_json::Value =
        serde_json::from_str(ty.normalized.as_ref().unwrap()).unwrap();
    token["value"]["length"] = serde_json::json!(5);
    ty.normalized = Some(serde_json::to_string(&token).unwrap());
    let error = rstyle_pdg::data_frontend::verify(&changed).unwrap_err();
    assert!(error.contains("Repeat count disagrees"), "{error}");
}

#[test]
fn clones_data_named_struct_values_preserve_field_slots_and_evaluation_order() {
    let directory = struct_value_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_struct_slot_corruption(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn struct_value_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/model.rs"),
        r#"
        pub struct Data { pub left: i32, pub right: i32, pub stamp: () }
        impl Data { pub fn new(left: i32, right: i32) -> Self { Self { right, left, stamp: () } } }
        pub struct Outer { pub inner: Data, pub bytes: [i32; 2] }
        pub struct Empty {}
        pub struct Borrowed<'a> { pub value: &'a i32 }
    "#,
    )
    .unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        mod model;
        use model::Data as Record;
        pub fn first(x: i32, y: i32) -> Record { Record { right: y, left: x, stamp: () } }
        pub fn other(x: i32, y: i32) -> Record { Record { right: y, left: x, stamp: () } }
        pub fn shorthand(left: i32, right: i32) -> Record { Record { left, right, stamp: () } }
        pub fn effects(mut x: i32) -> Record { Record { left: x, right: { x = 7; x }, stamp: () } }
        pub fn nested(x: i32) -> model::Outer { model::Outer { bytes: [x; 2], inner: Record { left: x, right: x, stamp: () } } }
        pub fn units() -> Record { Record { stamp: {}, left: 1, right: 2 } }
        pub fn empty() -> model::Empty { model::Empty {} }
        pub fn update(old: Record) -> Record { Record { left: 1, ..old } }
        pub fn local(x: i32) -> Record { let r = Record { left: x, right: x, stamp: () }; r }
        pub fn borrowed<'a>(x: &'a i32) -> model::Borrowed<'a> { model::Borrowed { value: x } }
    "#).unwrap();
    directory
}

fn check_struct_slot_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let constructor = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.struct"))
        .unwrap();
    assert_eq!(
        constructor
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value(),
        Some("[1,0,2]")
    );
    constructor.attributes.literal = Some(rstyle_pdg::StringFact::known("[0,1,2]"));
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    let mut changed = facts;
    changed.body.children[0].children.swap(0, 1);
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
}

#[test]
fn clones_data_struct_updates_preserve_base_evaluation_and_field_overrides() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Record { pub left: i32, pub right: i32, pub stamp: () }
        pub fn first(old: Record, x: i32) -> Record { Record { right: x, ..old } }
        pub fn other(old: Record, x: i32) -> Record { Record { right: x, ..old } }
        pub fn identity(old: Record) -> Record { Record { ..old } }
        pub fn nested(x: i32) -> Record { Record { left: x, ..Record { left: 1, right: 2, stamp: () } } }
        pub fn effects(mut x: i32) -> Record { Record { right: { x = 7; x }, ..{ x = 9; Record { left: x, right: x, stamp: () } } } }
        pub fn complete(mut x: i32) -> Record { Record { left: x, right: x, stamp: {}, ..{ x = 7; Record { left: x, right: x, stamp: () } } } }
        pub fn conditional(a: Record, b: Record, pick: bool) -> Record { Record { left: 4, ..if pick { a } else { b } } }
        pub fn local(old: Record) -> Record { let base = old; Record { ..base } }
    "#).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8);
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_struct_update_corruption(&report);
    check_struct_update_effects(&report);
}

fn check_struct_update_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let constructor = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.struct"))
        .unwrap();
    assert_eq!(
        constructor
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value(),
        Some("[1,null]")
    );
    constructor.attributes.literal = Some(rstyle_pdg::StringFact::known("[0,null]"));
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    let mut changed = facts;
    changed.body.children[0].children.pop();
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
}

fn check_struct_update_effects(report: &Value) {
    for (name, expected) in [("effects", 2), ("complete", 1)] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        assert_eq!(
            pair.pdg
                .nodes
                .iter()
                .filter(|node| node.category == rstyle_pdg::NodeCategory::Assignment)
                .count(),
            expected,
            "{name}"
        );
        let constructor = pair
            .pdg
            .nodes
            .iter()
            .find(|node| {
                node.attributes
                    .literal
                    .as_ref()
                    .and_then(|literal| literal.known_value())
                    .is_some_and(|literal| literal.ends_with(",null]"))
            })
            .unwrap();
        assert_eq!(
            pair.pdg
                .edges
                .iter()
                .filter(|edge| edge.target == constructor.id && edge.subkind == "value_operand")
                .count(),
            if name == "complete" { 4 } else { 2 }
        );
    }
}

#[test]
fn clones_data_tuple_struct_values_compose_with_nominal_slots_and_units() {
    let directory = tuple_struct_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_tuple_struct_corruption(&report);
    let helper = functions
        .iter()
        .find(|function| function["facts"]["name"] == "helper")
        .unwrap();
    let mut facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(helper["facts"].clone()).unwrap();
    assert_eq!(facts.body.children[0].kind, "expr_call");
    facts.body.children[0].kind = "expr_struct".into();
    assert!(rstyle_pdg::data_frontend::verify(&facts).is_err());
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn check_tuple_struct_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let constructor = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.struct"))
        .unwrap();
    assert_eq!(
        constructor
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value(),
        Some("[0,1,2]")
    );
    constructor.attributes.literal = Some(rstyle_pdg::StringFact::known("[1,0,2]"));
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    let mut changed = facts;
    changed.body.children[0].children.swap(0, 1);
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
}

fn tuple_struct_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/model.rs"),
        r#"
        pub struct Pair(pub i32, pub i32, pub ());
        impl Pair { pub fn new(x: i32) -> Self { Self(x, x, ()) } }
        pub struct Empty();
        pub struct Nested(pub Pair, pub [i32; 2]);
        pub struct Borrowed<'a>(pub &'a i32);
        pub struct Named { pub value: i32 }
        #[allow(non_snake_case)] pub fn Named(value: i32) -> Named { Named { value } }
    "#,
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        mod model;
        use model::Pair as Alias;
        pub fn first(x: i32, y: i32) -> Alias { Alias(x, y, ()) }
        pub fn other(x: i32, y: i32) -> Alias { Alias(x, y, ()) }
        pub fn units(x: i32) -> Alias { Alias(x, x, {}) }
        pub fn empty() -> model::Empty { model::Empty() }
        pub fn nested(x: i32) -> model::Nested { model::Nested(Alias(x, x, ()), [x; 2]) }
        pub fn effects(mut x: i32) -> Alias { Alias(x, { x = 7; x }, ()) }
        pub fn borrowed<'a>(x: &'a i32) -> model::Borrowed<'a> { model::Borrowed(x) }
        pub fn local(x: i32) -> Alias { let p = Alias(x, x, ()); p }
        pub fn helper(x: i32) -> model::Named { model::Named(x) }
    "#,
    )
    .unwrap();
    directory
}

#[test]
fn clones_data_unit_struct_paths_preserve_nominal_constructor_identity() {
    let directory = unit_struct_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "constant" | "named_constant"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_unit_struct_corruption(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn unit_struct_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/model.rs"),
        r#"
        pub struct Unit;
        impl Unit { pub fn new() -> Self { Self } }
        pub struct Wrap { pub tag: Unit, pub value: i32 }
        pub struct Named {}
        pub const UNIT: Unit = Unit;
        #[allow(non_upper_case_globals)] pub const Named: Named = Named {};
    "#,
    )
    .unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        mod model;
        use model::Unit as Alias;
        pub fn first() -> Alias { Alias }
        pub fn other() -> Alias { Alias }
        pub fn grouped() -> Alias { (Alias) }
        pub fn nested(x: i32) -> model::Wrap { model::Wrap { tag: Alias, value: x } }
        pub fn repeat() -> [Alias; 0] { [Alias; 0] }
        pub fn conditional(pick: bool) -> Alias { if pick { Alias } else { Alias } }
        pub fn effects(mut x: i32) -> model::Wrap { model::Wrap { tag: { x = 7; Alias }, value: x } }
        pub fn local() -> Alias { let value = Alias; value }
        pub fn constant() -> Alias { model::UNIT }
        pub fn named_constant() -> model::Named { model::Named }
    "#).unwrap();
    directory
}

fn check_unit_struct_corruption(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let first = functions
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(first["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let constructor = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.struct"))
        .unwrap();
    assert_eq!(
        constructor
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value(),
        Some("[]")
    );
    constructor.attributes.literal = Some(rstyle_pdg::StringFact::known("[0]"));
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    let named = functions
        .iter()
        .find(|function| function["facts"]["name"] == "named_constant")
        .unwrap();
    let mut changed: rstyle_pdg::HirFnFacts =
        serde_json::from_value(named["facts"].clone()).unwrap();
    assert_eq!(changed.body.children[0].kind, "expr_path");
    changed.body.children[0].kind = "expr_struct".into();
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
}

#[test]
fn clones_data_local_aggregate_values_follow_definitions_across_branches_and_loops() {
    let directory = local_aggregate_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 13);
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_local_aggregate_corruption(&report);
    check_local_aggregate_reaching(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn local_aggregate_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Record { pub value: i32 }
        pub fn first(x: i32, y: i32) -> (i32, i32) { let v = (x, y); v }
        pub fn other(x: i32, y: i32) -> (i32, i32) { let v = (x, y); v }
        pub fn array(a: [i32; 2]) -> [i32; 2] { let b = a; let c = b; c }
        pub fn branch(old: Record, pick: bool) -> Record { let mut r = old; if pick { r = Record { value: 7 }; } r }
        pub fn conditional(x: i32, pick: bool) -> (i32, i32) { let v = if pick { (x, 1) } else { (2, x) }; v }
        pub fn cyclic(mut n: i32) -> (i32, i32) { let mut v = (n, n); while n > 0 { v = (n, 1); n -= 1; } v }
        pub fn shadow(x: i32) -> (i32, i32) { let v = (x, 1); { let v = v; v } }
        pub fn update(old: Record) -> Record { let base = old; let result = Record { value: 4, ..base }; result }
        pub fn mutable(mut old: Record) -> Record { old = Record { value: 7 }; old }
        pub fn unreachable(old: Record) -> Record { return old; let value = Record { value: 1 }; value }
        pub fn reference(x: &i32) -> &i32 { let v = x; v }
        pub fn field(old: Record) -> i32 { let value = old; value.value }
        pub fn mixed(old: Record, source: &Record) -> Record { let value = old; let x = source.value; Record { value: x, ..value } }
    "#).unwrap();
    directory
}

fn check_local_aggregate_corruption(report: &Value) {
    let first = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(first["facts"].clone()).unwrap();
    check_local_initializer_corruption(&facts);
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let mut changed = rstyle_pdg::producer::produce(&facts).unwrap();
    changed.pdg.edges.retain(|edge| {
        edge.subkind != "reaching_definition"
            || !changed.pdg.nodes.iter().any(|node| {
                node.id == edge.target
                    && node.category == rstyle_pdg::NodeCategory::Read
                    && node.attributes.symbol_ids.iter().any(|symbol| {
                        facts
                            .bindings
                            .iter()
                            .any(|binding| !binding.is_param && &binding.symbol == symbol)
                    })
            })
    });
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&changed, &facts).is_err());
    let mut changed = pair;
    let definition = changed
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Declaration)
        .unwrap()
        .id
        .clone();
    changed
        .pdg
        .edges
        .retain(|edge| edge.target != definition || edge.subkind != "value_operand");
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
}

fn check_local_initializer_corruption(facts: &rstyle_pdg::HirFnFacts) {
    let mut pair = rstyle_pdg::producer::produce(facts).unwrap();
    let definition = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Declaration)
        .unwrap()
        .id
        .clone();
    let read = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == rstyle_pdg::NodeCategory::Read
                && node.original_kind.known_value() == Some("expr_path")
                && node.annotations["compat.rust.hir-r0:type"]["value"] != "i32"
        })
        .unwrap()
        .id
        .clone();
    let edge = pair
        .pdg
        .edges
        .iter_mut()
        .find(|edge| edge.target == definition && edge.subkind == "value_operand")
        .unwrap();
    edge.source = read;
    let error = rstyle_pdg::data_documents::verify(&pair, facts).unwrap_err();
    assert!(error.contains("original component"), "{error}");
}

fn check_local_aggregate_reaching(report: &Value) {
    for (name, binding_name) in [("branch", "r"), ("cyclic", "v")] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let symbol = &facts
            .bindings
            .iter()
            .find(|binding| binding.name == binding_name)
            .unwrap()
            .symbol;
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let read = pair
            .pdg
            .nodes
            .iter()
            .find(|node| {
                node.category == rstyle_pdg::NodeCategory::Read
                    && node.attributes.symbol_ids == [symbol.clone()]
            })
            .unwrap();
        assert_eq!(
            pair.pdg
                .edges
                .iter()
                .filter(|edge| edge.target == read.id && edge.subkind == "reaching_definition")
                .count(),
            2,
            "{name}"
        );
    }
}

#[test]
fn clones_data_aggregate_value_fields_preserve_component_identity_and_value_flow() {
    let directory = value_field_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 15);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "borrow" | "write"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_value_field_corruption(&report);
    check_value_field_evaluation(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"]
                == if function["facts"]["name"] == "formal_read" {
                    "admitted"
                } else {
                    "excluded"
                }),
        "{legacy}"
    );
}

fn value_field_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Record { pub left: i32, pub right: i32, pub stamp: () }
        pub struct Outer { pub inner: Record }
        pub struct Borrowed<'a> { pub value: &'a i32 }
        pub fn first(old: Record) -> i32 { let value = old; value.right }
        pub fn other(old: Record) -> i32 { let value = old; value.right }
        pub fn tuple(old: (i32, i32)) -> i32 { let value = old; (value).0 }
        pub fn nested(old: Outer) -> i32 { let value = old; value.inner.left }
        pub fn computed(x: i32) -> i32 { (x, { 7 }).1 }
        pub fn effects(mut x: i32) -> i32 { ({ x = 7; (x, x) }).1 }
        pub fn conditional(a: Record, b: Record, pick: bool) -> i32 { (if pick { a } else { b }).left }
        pub fn reassigned(mut old: Record) -> i32 { old = Record { left: 1, right: 2, stamp: () }; old.right }
        pub fn snapshot(mut old: Record) -> (i32, i32) { let left = old.left; old = Record { left: 7, right: 9, stamp: () }; (left, old.right) }
        pub fn units(old: Record) -> ((), i32) { let value = old; (value.stamp, value.left) }
        pub fn mixed(old: Record, source: &Record) -> i32 { let value = old; value.right + source.left }
        pub fn formal_read(mut old: Record) -> i32 { old.left + old.right }
        pub fn borrow(old: Record) -> i32 { let value = old; let _r = &value.left; value.right }
        pub fn write(old: Record) -> i32 { let mut value = old; value.left = 4; value.left }
        pub fn references<'a>(old: Borrowed<'a>) -> &'a i32 { let value = old; value.value }
    "#).unwrap();
    directory
}

fn check_value_field_corruption(report: &Value) {
    let first = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(first["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    assert!(pair.pdg.memory_locations.is_empty());
    let projection = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.value-field"))
        .unwrap();
    assert_eq!(
        projection
            .attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value(),
        Some("1")
    );
    projection.attributes.literal = Some(rstyle_pdg::StringFact::known("0"));
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    let mut changed = facts;
    let field = changed.body.children.last_mut().unwrap();
    let resolution = field.res.as_mut().unwrap();
    let mut value: Value = serde_json::from_str(&resolution.description).unwrap();
    value["position"] = serde_json::json!(0);
    resolution.description = value.to_string();
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
}

fn check_value_field_evaluation(report: &Value) {
    for (name, locations) in [("effects", 0), ("mixed", 1)] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        assert_eq!(pair.pdg.memory_locations.len(), locations, "{name}");
        if name == "effects" {
            assert_eq!(
                pair.pdg
                    .nodes
                    .iter()
                    .filter(|node| node.category == rstyle_pdg::NodeCategory::Assignment)
                    .count(),
                1
            );
            let projection = pair
                .pdg
                .nodes
                .iter()
                .find(|node| node.attributes.construct.as_deref() == Some("data.value-field"))
                .unwrap();
            assert_eq!(
                pair.pdg
                    .edges
                    .iter()
                    .filter(|edge| edge.target == projection.id && edge.subkind == "value_operand")
                    .count(),
                1
            );
        }
    }
}

#[test]
fn clones_data_shared_reference_locals_preserve_initializer_and_formal_origins() {
    let directory = reference_local_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 14);
    for function in functions {
        let excluded = !has_length_authority(&report)
            && matches!(
                function["facts"]["name"].as_str().unwrap(),
                "length" | "empty"
            );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_reference_local_corruption(read_call_function(&report, "conditional"));
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn reference_local_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Record<'a> { pub value: &'a i32 }
        pub fn first(value: &i32) -> &i32 { let local = value; local }
        pub fn other(value: &i32) -> &i32 { let local = value; local }
        pub fn alias(value: &i32) -> &i32 { let local = value; let copy = local; copy }
        pub fn shadow<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let value = a; let saved = value; let value = b; if true { saved } else { value } }
        pub fn conditional<'a>(a: &'a i32, b: &'a i32, pick: bool) -> &'a i32 { let value = if pick { a } else { b }; value }
        pub fn stored<'a>(record: Record<'a>) -> &'a i32 { let value = record.value; value }
        pub fn borrowed(record: &(i32, i32)) -> &i32 { let value = &record.1; value }
        pub fn length(bytes: &[u8]) -> usize { let value = bytes; value.len() }
        pub fn empty(text: &str) -> bool { let value = text; value.is_empty() }
        pub fn mutable(value: &i32) -> &i32 { let mut local = value; local }
        pub fn rebound<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let mut local = a; local = b; local }
        pub fn formal_rebound<'a>(mut a: &'a i32, b: &'a i32) -> &'a i32 { a = b; a }
        pub fn local_field(record: &(i32, i32)) -> i32 { let local = record; local.0 }
        pub fn aggregate(value: &i32) -> (&i32,) { let local = (value,); local }
    "#).unwrap();
    directory
}

fn check_reference_local_corruption(function: &Value) {
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let read = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == rstyle_pdg::NodeCategory::Read
                && node.attributes.reads.iter().any(|access| {
                    access.symbol_id.as_ref().is_some_and(|symbol| {
                        facts
                            .bindings
                            .iter()
                            .any(|binding| !binding.is_param && binding.symbol == *symbol)
                    })
                })
        })
        .unwrap();
    let mut missing = rstyle_pdg::producer::produce(&facts).unwrap();
    missing
        .pdg
        .edges
        .retain(|edge| !(edge.target == read.id && edge.subkind == "reaching_definition"));
    assert!(rstyle_pdg::data_documents::verify(&missing, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&missing, &facts).is_err());
    let mut origins = rstyle_pdg::producer::produce(&facts).unwrap();
    let changed = origins
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.id == read.id)
        .unwrap();
    changed.annotations.insert(
        rstyle_pdg::data_origins::KEY.into(),
        serde_json::json!([{ "parameter": 0, "path": [{"kind":"SharedReferent"}] }]),
    );
    assert!(rstyle_pdg::data_documents::verify(&origins, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&origins, &facts).is_err());
}

#[test]
fn clones_data_shared_local_fields_retain_formal_locations_and_alias_value_flow() {
    let directory = shared_local_field_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 18);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "double" | "borrowlocal"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_shared_local_field_corruption(&report);
    check_shared_local_borrow_location(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn shared_local_field_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Record { pub left: i32, pub right: i32, pub stamp: () }
        pub struct Outer { pub inner: Record }
        pub struct Holder<'a> { pub record: &'a Record }
        pub fn first(record: &Record) -> i32 { let local = record; local.right }
        pub fn other(record: &Record) -> i32 { let local = record; local.right }
        pub fn grouped(record: &Record) -> i32 { let local = record; ((local)).right }
        pub fn chain(record: &Record) -> i32 { let a = record; let b = a; b.left }
        pub fn shadow(record: &Record, other: &Record) -> i32 { let local = record; let saved = local; let local = other; saved.left + local.right }
        pub fn nested(record: &Outer) -> i32 { let local = record; local.inner.right }
        pub fn stored(holder: Holder<'_>) -> i32 { let local = holder.record; local.right }
        pub fn borrowed(record: &Outer) -> i32 { let local = &record.inner; local.right }
        pub fn same_branch(record: &Record, pick: bool) -> i32 { let local = if pick { record } else { record }; local.left }
        pub fn block(record: &Record) -> i32 { let local = { let inner = record; inner }; local.right }
        pub fn reference_result<'a>(record: &'a (i32, &'a i32)) -> &'a i32 { let local = record; local.1 }
        pub fn alias_borrow(record: &Record) -> &i32 { let local = record; &local.right }
        pub fn units(record: &Record) -> ((), i32) { let local = record; (local.stamp, local.right) }
        pub fn multi(a: &Record, b: &Record, pick: bool) -> i32 { let local = if pick { a } else { b }; local.right }
        pub fn mutable(record: &Record) -> i32 { let mut local = record; local.right }
        pub fn double(record: &&Record) -> i32 { let local = record; local.right }
        pub fn borrowlocal(record: Record) -> i32 { let local = &record; local.right }
        pub fn aggregate(record: &Record) -> i32 { let local = (record,); local.0.right }
    "#).unwrap();
    directory
}

fn check_shared_local_field_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "shadow")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    assert_eq!(pair.pdg.memory_locations.len(), 2);
    pair.pdg.memory_locations[0]
        .extensions
        .get_mut(rstyle_pdg::data_documents::LOCATION_KEY)
        .unwrap()["parameter"] = serde_json::json!(1);
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    let mut changed = facts;
    let mut fields = changed
        .build
        .frontend_options
        .data::<rstyle_pdg::field_locations::FieldObservations>()
        .unwrap()
        .into_owned();
    let rstyle_pdg::field_locations::Location::Captured { parameter, .. } =
        &mut fields.operations[0].location
    else {
        panic!("captured alias field")
    };
    assert_eq!(*parameter, 0);
    *parameter = 1;
    changed.build.frontend_options.insert_data(fields);
    assert!(rstyle_pdg::data_frontend::verify(&changed).is_err());
}

fn check_shared_local_borrow_location(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "borrowed")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let location = pair
        .pdg
        .memory_locations
        .iter()
        .find(|location| {
            location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
                .as_array()
                .unwrap()
                .last()
                .is_some_and(|step| step["name"] == "right")
        })
        .unwrap();
    let path = location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
        .as_array()
        .unwrap();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0]["kind"], "SharedReferent");
    assert_eq!(path[1]["name"], "inner");
    assert_eq!(path[2]["name"], "right");
}

#[test]
fn clones_data_reference_bearing_values_preserve_origin_unions_and_exact_slots() {
    let directory = reference_literal_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 24);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "arrays" | "zero" | "helper_return"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_reference_literal_slot_corruption(&report);
    check_reference_literal_effects(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"]
                == if function["facts"]["name"] == "shared_identity" {
                    "admitted"
                } else {
                    "excluded"
                }),
        "{legacy}"
    );
}

fn reference_literal_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Pair<'a> { pub left: &'a i32, pub right: &'a i32, pub stamp: () }
        pub struct Tuple<'a>(pub &'a i32, pub (), pub &'a i32);
        pub struct Nested<'a> { pub inner: Pair<'a>, pub scalar: i32 }
        pub struct Holder<'a> { pub value: &'a i32 }
        pub fn first<'a>(a: &'a i32, b: &'a i32) -> (&'a i32, &'a i32) { (a, b) }
        pub fn other<'a>(a: &'a i32, b: &'a i32) -> (&'a i32, &'a i32) { (a, b) }
        pub fn named<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { Pair { right: b, stamp: (), left: a } }
        pub fn tuple<'a>(a: &'a i32, b: &'a i32) -> Tuple<'a> { Tuple(a, (), b) }
        pub fn nested<'a>(a: &'a i32, b: &'a i32) -> Nested<'a> { Nested { inner: Pair { left: a, right: b, stamp: () }, scalar: 7 } }
        pub fn aliases<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { let local = a; Pair { left: local, right: b, stamp: () } }
        pub fn borrowed(record: &(i32, i32)) -> (&i32, &i32) { (&record.0, &record.1) }
        pub fn stored<'a>(record: Holder<'a>, other: &'a i32) -> Pair<'a> { Pair { left: record.value, right: other, stamp: () } }
        pub fn conditional<'a>(a: &'a i32, b: &'a i32, pick: bool) -> (&'a i32, &'a i32) { (if pick { a } else { b }, a) }
        pub fn block<'a>(a: &'a i32, b: &'a i32) -> (&'a i32, &'a i32) { { (a, b) } }
        pub fn effects<'a>(a: &'a i32, b: &'a i32, mut n: i32) -> Pair<'a> { Pair { left: { n = 7; a }, right: { let check = n; b }, stamp: () } }
        pub fn wrap<'a>(pair: (&'a i32, &'a i32)) -> ((&'a i32, &'a i32),) { let saved = pair; (saved,) }
        pub fn mixed_unit<'a>(a: &'a i32, b: &'a i32) -> ((), &'a i32, (&'a i32, ())) { ((), a, (b, ())) }
        pub fn local<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { let value = Pair { left: a, right: b, stamp: () }; value }
        pub fn shadow<'a>(a: &'a i32, b: &'a i32) -> (Pair<'a>, Pair<'a>) { let pair = Pair { left: a, right: b, stamp: () }; let saved = pair; let pair = Pair { left: b, right: a, stamp: () }; (saved, pair) }
        pub fn mutable<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { let mut value = Pair { left: a, right: b, stamp: () }; value }
        pub fn reassigned<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { let mut value = Pair { left: a, right: b, stamp: () }; value = Pair { left: b, right: a, stamp: () }; value }
        pub fn temporary_field<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { (a, b).0 }
        pub fn update<'a>(old: Pair<'a>, value: &'a i32) -> Pair<'a> { Pair { left: value, ..old } }
        pub fn arrays(value: &i32) -> [&i32; 2] { [value, value] }
        pub fn zero(value: &i32) -> [&i32; 0] { [value; 0] }
        pub fn shared_identity(value: &i32) -> &i32 { value }
        pub fn helper_component<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { Pair { left: shared_identity(a), right: b, stamp: () } }
        pub fn helper_return<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { named(a, b) }
    "#).unwrap();
    directory
}

fn check_reference_literal_slot_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    assert!(pair.pdg.memory_locations.is_empty());
    let constructor = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.original_kind.known_value() == Some("expr_tup")
                && node.category == rstyle_pdg::NodeCategory::Computation
        })
        .unwrap();
    assert_eq!(
        constructor.annotations[rstyle_pdg::data_origins::KEY]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let operands: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.target == constructor.id && edge.subkind == "value_operand")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(operands.len(), 2);
    let first = pair.pdg.edges[operands[0]].source.clone();
    pair.pdg.edges[operands[0]].source = pair.pdg.edges[operands[1]].source.clone();
    pair.pdg.edges[operands[1]].source = first;
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
}

fn check_reference_literal_effects(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "effects")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    assert_eq!(
        pair.pdg
            .nodes
            .iter()
            .filter(|node| node.category == rstyle_pdg::NodeCategory::Assignment)
            .count(),
        1
    );
    let constructor = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("data.struct"))
        .unwrap();
    assert_eq!(
        pair.pdg
            .edges
            .iter()
            .filter(|edge| edge.target == constructor.id && edge.subkind == "value_operand")
            .count(),
        3
    );
}

#[test]
fn clones_data_reference_value_fields_select_exact_component_origins() {
    let directory = reference_value_field_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 22);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "borrow" | "helper_result" | "helper_field"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_reference_value_field_origins(&report);
    check_reference_value_field_paths(&report);
    check_reference_value_field_corruption(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"]
                == if function["facts"]["name"] == "identity" {
                    "admitted"
                } else {
                    "excluded"
                }),
        "{legacy}"
    );
}

fn reference_value_field_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Pair<'a> { pub left: &'a i32, pub right: &'a i32, pub stamp: i32 }
        pub struct Outer<'a> { pub inner: Pair<'a> }
        pub struct Tuple<'a>(pub &'a i32, pub &'a i32);
        pub fn first<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let pair = (a, b); pair.0 }
        pub fn other<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let pair = (a, b); pair.0 }
        pub fn second<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { (a, b).1 }
        pub fn named<'a>(a: &'a i32, b: &'a i32, old: Pair<'a>) -> &'a i32 { let pair = Pair { right: b, stamp: old.stamp, left: a }; pair.left }
        pub fn tuple<'a>(a: &'a i32, b: &'a i32, old: Tuple<'a>) -> &'a i32 { Tuple(a, b).1 }
        pub fn nested<'a>(a: &'a i32, b: &'a i32, old: Outer<'a>) -> &'a i32 { let pair = Outer { inner: Pair { left: a, right: b, stamp: old.inner.stamp } }; pair.inner.right }
        pub fn formal<'a>(pair: Pair<'a>) -> &'a i32 { let saved = pair; saved.right }
        pub fn loaded<'a>(pair: &((&'a i32, &'a i32),)) -> &'a i32 { let saved = pair.0; saved.0 }
        pub fn shadow<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let pair = (a, b); let saved = pair; let pair = (b, a); saved.0 }
        pub fn conditional<'a>(a: &'a i32, b: &'a i32, pick: bool) -> &'a i32 { let pair = if pick { (a, b) } else { (b, a) }; pair.0 }
        pub fn stable<'a>(a: &'a i32, b: &'a i32, pick: bool) -> &'a i32 { (if pick { (a, b) } else { (a, a) }).0 }
        pub fn scalar(a: &i32, b: &i32, old: Pair) -> i32 { Pair { left: a, right: b, stamp: 7 }.stamp }
        pub fn identity(value: &i32) -> &i32 { value }
        pub fn helper_component<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { (identity(a), b).0 }
        pub fn mutable<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let mut pair = (a, b); pair.0 }
        pub fn rebound<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let mut pair = (a, b); pair = (b, a); pair.0 }
        pub fn borrow(a: &i32, b: &i32, old: Pair) -> i32 { let pair = Pair { left: a, right: b, stamp: 7 }; let saved = &pair.stamp; pair.stamp }
        pub fn update<'a>(old: Pair<'a>, a: &'a i32) -> &'a i32 { Pair { left: a, ..old }.right }
        pub fn helper_result<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { identity_pair(a, b).0 }
        pub fn helper_field<'a>(pair: Pair<'a>) -> &'a i32 { formal(pair) }
        pub fn body_nominal<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { Pair { left: a, right: b, stamp: 7 }.left }
        fn identity_pair<'a>(a: &'a i32, b: &'a i32) -> (&'a i32, &'a i32) { (a, b) }
    "#).unwrap();
    directory
}

fn check_reference_value_field_origins(report: &Value) {
    for (name, parameters) in [
        ("first", vec![0]),
        ("second", vec![1]),
        ("named", vec![0]),
        ("nested", vec![1]),
        ("shadow", vec![0]),
        ("conditional", vec![0, 1]),
        ("stable", vec![0]),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let (unit, index) = data_unit(function);
        let pair = unit.pair(index).unwrap();
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        let actual: Vec<_> = returned.annotations[rstyle_pdg::data_origins::KEY]
            .as_array()
            .unwrap()
            .iter()
            .map(|origin| origin["parameter"].as_u64().unwrap())
            .collect();
        assert_eq!(actual, parameters, "{name}");
    }
}

fn check_reference_value_field_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    assert!(pair.pdg.memory_locations.is_empty());
    for corruption in ["origin", "slot", "base"] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        corrupt_reference_value_field(&mut changed, corruption);
        assert!(
            rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
            "{corruption}"
        );
        assert!(
            rstyle_pdg::producer::verify(&changed, &facts).is_err(),
            "{corruption}"
        );
    }
}

fn check_reference_value_field_paths(report: &Value) {
    for name in ["formal", "loaded"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let (unit, index) = data_unit(function);
        let pair = unit.pair(index).unwrap();
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        let origins = returned.annotations[rstyle_pdg::data_origins::KEY]
            .as_array()
            .unwrap();
        assert_eq!(origins.len(), 1, "{name}");
        assert_eq!(origins[0]["parameter"], 0);
        let path = origins[0]["path"].as_array().unwrap();
        if name == "formal" {
            assert_eq!(path.len(), 2);
            assert_eq!(path[0]["kind"], "StructField");
            assert_eq!(path[0]["name"], "right");
            assert_eq!(path[0]["position"], 1);
        } else {
            assert_eq!(path.len(), 4);
            assert_eq!(path[0]["kind"], "SharedReferent");
            for component in &path[1..3] {
                assert_eq!(component["kind"], "TupleField");
                assert_eq!(component["position"], 0);
            }
        }
        assert_eq!(path.last().unwrap()["kind"], "SharedReferent");
    }
}

fn corrupt_reference_value_field(pair: &mut rstyle_pdg::producer::Pair, corruption: &str) {
    let projection = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.value-field"))
        .unwrap();
    let id = projection.id.clone();
    match corruption {
        "origin" => {
            projection
                .annotations
                .get_mut(rstyle_pdg::data_origins::KEY)
                .unwrap()[0]["parameter"] = 1.into()
        }
        "slot" => projection.attributes.literal = Some(rstyle_pdg::StringFact::known("1")),
        _ => {
            let sibling = pair
                .pdg
                .nodes
                .iter()
                .find(|node| {
                    node.category == rstyle_pdg::NodeCategory::FormalInput
                        && node.attributes.position == Some(1)
                })
                .unwrap()
                .id
                .clone();
            pair.pdg
                .edges
                .iter_mut()
                .find(|edge| edge.target == id && edge.subkind == "value_operand")
                .unwrap()
                .source = sibling;
        }
    }
}

#[test]
fn clones_data_reference_struct_updates_remove_only_overwritten_origins() {
    let directory = reference_update_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 18);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "helper" | "array"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_reference_update_origins(&report);
    check_reference_update_nested_paths(&report);
    check_reference_update_corruption(&report);
    check_reference_update_effects(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded"),
        "{legacy}"
    );
}

fn reference_update_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub struct Pair<'a> { pub left: &'a i32, pub right: &'a i32, pub stamp: () }
        pub struct Outer<'a> { pub inner: Pair<'a>, pub tag: i32 }
        pub fn first<'a>(old: Pair<'a>, a: &'a i32) -> Pair<'a> { Pair { left: a, ..old } }
        pub fn other<'a>(old: Pair<'a>, a: &'a i32) -> Pair<'a> { Pair { left: a, ..old } }
        pub fn unchanged(old: Pair<'_>) -> Pair<'_> { Pair { ..old } }
        pub fn local<'a>(old: Pair<'a>, a: &'a i32) -> Pair<'a> { let saved = old; Pair { left: a, ..saved } }
        pub fn constructed<'a>(a: &'a i32, b: &'a i32, c: &'a i32) -> Pair<'a> { Pair { left: c, ..Pair { left: a, right: b, stamp: () } } }
        pub fn complete<'a>(a: &'a i32, b: &'a i32, c: &'a i32) -> Pair<'a> { Pair { right: b, left: a, stamp: (), ..Pair { left: c, right: c, stamp: () } } }
        pub fn shared_origin<'a>(a: &'a i32, b: &'a i32) -> Pair<'a> { Pair { left: b, ..Pair { left: a, right: a, stamp: () } } }
        pub fn nested<'a>(old: Outer<'a>, a: &'a i32) -> Outer<'a> { Outer { inner: Pair { left: a, ..old.inner }, ..old } }
        pub fn chain<'a>(a: &'a i32, b: &'a i32, c: &'a i32) -> Pair<'a> { let base = Pair { left: a, right: b, stamp: () }; let mid = Pair { left: c, ..base }; Pair { right: a, ..mid } }
        pub fn projected<'a>(old: Pair<'a>, a: &'a i32) -> &'a i32 { Pair { left: a, ..old }.right }
        pub fn overridden<'a>(old: Pair<'a>, a: &'a i32) -> &'a i32 { Pair { left: a, ..old }.left }
        pub fn conditional<'a>(old: Pair<'a>, a: &'a i32, b: &'a i32, pick: bool) -> Pair<'a> { Pair { left: a, ..if pick { Pair { left: b, ..old } } else { old } } }
        pub fn effects<'a>(old: Pair<'a>, a: &'a i32, mut n: i32) -> (Pair<'a>, i32) { let pair = Pair { left: { n = 7; a }, ..{ n = 9; old } }; (pair, n) }
        pub fn ignored_base<'a>(a: &'a i32, b: &'a i32, c: &'a i32, mut n: i32) -> (Pair<'a>, i32) { let pair = Pair { left: a, right: b, stamp: (), ..{ n = 7; Pair { left: c, right: c, stamp: () } } }; (pair, n) }
        pub fn unit_effect<'a>(old: Pair<'a>, mut n: i32) -> (Pair<'a>, i32) { let pair = Pair { stamp: { n = 7; () }, ..old }; (pair, n) }
        pub fn mutable<'a>(old: Pair<'a>, a: &'a i32) -> Pair<'a> { let mut saved = Pair { left: a, ..old }; saved }
        pub fn helper<'a>(old: Pair<'a>, a: &'a i32) -> Pair<'a> { first(old, a) }
        pub fn array<'a>(old: Pair<'a>, a: &'a i32) -> [Pair<'a>; 1] { [Pair { left: a, ..old }] }
    "#).unwrap();
    directory
}

fn check_reference_update_origins(report: &Value) {
    for (name, parameters) in [
        ("first", vec![0, 1]),
        ("constructed", vec![1, 2]),
        ("complete", vec![0, 1]),
        ("shared_origin", vec![0, 1]),
        ("chain", vec![0, 2]),
        ("projected", vec![0]),
        ("overridden", vec![1]),
        ("conditional", vec![0, 1]),
        ("ignored_base", vec![0, 1]),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let (unit, index) = data_unit(function);
        let pair = unit.pair(index).unwrap();
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        let origins = returned.annotations[rstyle_pdg::data_origins::KEY]
            .as_array()
            .unwrap();
        let actual: Vec<_> = origins
            .iter()
            .map(|origin| origin["parameter"].as_u64().unwrap())
            .collect();
        assert_eq!(actual, parameters, "{name}");
        if matches!(name, "first" | "projected" | "conditional") {
            assert_eq!(origins[0]["path"][0]["name"], "right", "{name}");
            assert_eq!(origins[0]["path"][0]["position"], 1, "{name}");
        }
    }
}

fn check_reference_update_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "constructed")
        .unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    for corruption in ["origin", "slot", "base"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt_reference_update(&mut pair, corruption);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{corruption}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{corruption}"
        );
    }
}

fn corrupt_reference_update(pair: &mut rstyle_pdg::producer::Pair, corruption: &str) {
    let update = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.attributes
                .literal
                .as_ref()
                .and_then(|literal| literal.known_value())
                == Some("[0,null]")
        })
        .unwrap();
    let id = update.id.clone();
    match corruption {
        "origin" => update
            .annotations
            .get_mut(rstyle_pdg::data_origins::KEY)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"parameter": 0, "path": [{"kind": "SharedReferent"}]})),
        "slot" => update.attributes.literal = Some(rstyle_pdg::StringFact::known("[1,null]")),
        _ => pair.pdg.edges.retain(|edge| {
            !(edge.target == id
                && edge.subkind == "value_operand"
                && edge.operand_position == Some(1))
        }),
    }
}

fn check_reference_update_effects(report: &Value) {
    for (name, assignments, operands) in [
        ("effects", 2, 2),
        ("ignored_base", 1, 4),
        ("unit_effect", 1, 2),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let (unit, index) = data_unit(function);
        let pair = unit.pair(index).unwrap();
        assert_eq!(
            pair.pdg
                .nodes
                .iter()
                .filter(|node| node.category == rstyle_pdg::NodeCategory::Assignment)
                .count(),
            assignments
        );
        let update = pair
            .pdg
            .nodes
            .iter()
            .find(|node| {
                node.attributes
                    .literal
                    .as_ref()
                    .and_then(|literal| literal.known_value())
                    .is_some_and(|literal| literal.contains("null"))
            })
            .unwrap();
        assert_eq!(
            pair.pdg
                .edges
                .iter()
                .filter(|edge| edge.target == update.id && edge.subkind == "value_operand")
                .count(),
            operands
        );
    }
}

fn check_reference_update_nested_paths(report: &Value) {
    for name in ["unchanged", "nested", "unit_effect"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let (unit, index) = data_unit(function);
        let pair = unit.pair(index).unwrap();
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        let origins = returned.annotations[rstyle_pdg::data_origins::KEY]
            .as_array()
            .unwrap();
        assert_eq!(origins.len(), 2, "{name}");
        assert_eq!(origins[0]["parameter"], 0);
        let path = origins[0]["path"].as_array().unwrap();
        if name == "nested" {
            assert_eq!(path.len(), 3);
            assert_eq!(path[0]["name"], "inner");
            assert_eq!(path[1]["name"], "right");
            assert_eq!(origins[1]["parameter"], 1);
        } else {
            assert_eq!(path.len(), 2);
            assert_eq!(path[0]["name"], "left");
            assert_eq!(origins[1]["parameter"], 0);
            assert_eq!(origins[1]["path"][0]["name"], "right");
        }
        assert_eq!(path.last().unwrap()["kind"], "SharedReferent");
    }
}

#[test]
fn clones_data_body_nominal_types_compose_without_signature_witnesses() {
    let directory = body_nominal_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 13);
    for function in functions {
        let excluded = matches!(function["facts"]["name"].as_str().unwrap(), "borrow");
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_body_nominal_corruption(&report);
    check_body_nominal_interface(&report);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    assert!(
        legacy["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["outcome"] == "excluded"),
        "{legacy}"
    );
}

const REFERENCE_REBINDING_FAMILY: &str = r#"
        pub struct Pair<'a> { pub left: &'a i32, pub right: &'a i32 }
        pub fn first<'a>(a: &'a i32, b: &'a i32) -> &'a i32 {
            let mut selected = a; selected = b; selected
        }
        pub fn other<'a>(a: &'a i32, b: &'a i32) -> &'a i32 {
            let mut selected = a; selected = b; selected
        }
        pub fn snapshot<'a>(a: &'a i32, b: &'a i32) -> &'a i32 {
            let mut selected = a; let saved = selected; selected = b; saved
        }
        pub fn branch<'a>(a: &'a i32, b: &'a i32, pick: bool) -> &'a i32 {
            let mut selected = a; if pick { selected = b; } selected
        }
        pub fn killed<'a>(a: &'a i32, b: &'a i32, pick: bool) -> &'a i32 {
            let mut selected = a; if pick { selected = b; } selected = a; selected
        }
        pub fn looped<'a>(a: &'a i32, b: &'a i32, mut count: u32) -> &'a i32 {
            let mut selected = a;
            while count > 0 { selected = b; count -= 1; }
            selected
        }
        pub fn swapped<'a>(a: &'a i32, b: &'a i32, mut count: u32) -> &'a i32 {
            let mut left = a; let mut right = b;
            while count > 0 { let saved = left; left = right; right = saved; count -= 1; }
            left
        }
        pub fn aggregate<'a>(a: &'a i32, b: &'a i32, pick: bool) -> &'a i32 {
            let mut pair = Pair { left: a, right: b };
            if pick { pair = Pair { left: b, right: a }; }
            pair.left
        }
        pub fn aggregate_snapshot<'a>(a: &'a i32, b: &'a i32) -> &'a i32 {
            let mut pair = Pair { left: a, right: b }; let saved = pair.left;
            pair = Pair { left: b, right: a }; saved
        }
        pub fn update<'a>(a: &'a i32, b: &'a i32) -> &'a i32 {
            let mut pair = Pair { left: a, right: b }; pair = Pair { left: b, ..pair }; pair.right
        }
        pub fn parameter<'a>(mut a: &'a i32, b: &'a i32) -> &'a i32 { a = b; a }
        pub fn stopped<'a>(a: &'a i32, b: &'a i32, flag: bool) -> &'a i32 {
            let mut selected = a; loop { selected = b; if flag { break; } continue; } selected
        }
        pub fn matched<'a>(a: &'a i32, b: &'a i32, flag: bool) -> &'a i32 {
            let mut selected = a; match flag { true => selected = b, false => {} } selected
        }
        pub fn guarded<'a>(a: &'a i32, b: &'a i32, flag: bool) -> &'a i32 {
            let mut selected = a; match flag { true if flag => selected = b, _ => {} } selected
        }
        pub fn whole<'a>(value: &'a i32) -> &'a i32 { value }
        pub fn helper<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let mut selected = a; selected = whole(b); selected }
        pub fn composed<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { first(a, b) }
        pub fn storage_borrow(value: i32) -> i32 { let reference = &value; *reference }
    "#;

#[test]
fn clones_data_reference_rebindings_preserve_snapshots_and_component_flow() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        REFERENCE_REBINDING_FAMILY,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 18);
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(
            function["outcome"],
            if function["facts"]["name"] == "storage_borrow" {
                "excluded"
            } else {
                "admitted"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_reference_rebinding_origins(&report);
    check_reference_local_corruption(read_call_function(&report, "branch"));
    check_reference_local_corruption(read_call_function(&report, "swapped"));
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    for name in [
        "first",
        "snapshot",
        "looped",
        "swapped",
        "aggregate",
        "update",
        "parameter",
    ] {
        assert_eq!(
            read_call_function(&legacy, name)["outcome"],
            "excluded",
            "{name}: {legacy}"
        );
    }
}

#[test]
fn clones_data_reference_rebindings_skip_disabled_assertion_effects() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn debug<'a>(a: &'a i32, b: &'a i32) -> &'a i32 {
            let mut selected = a; debug_assert!({ selected = b; true }); selected
        }
        pub fn active<'a>(a: &'a i32, b: &'a i32) -> &'a i32 {
            let mut selected = a; assert!({ selected = b; true }); selected
        }
    "#,
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--release", "--extraction-profile", "hir-data-v3"],
    );
    assert!(output.status.success(), "{report}");
    for name in ["debug", "active"] {
        assert_eq!(
            read_call_function(&report, name)["outcome"],
            "admitted",
            "{report}"
        );
    }
    check_aggregate_documents(&report);
    let (unit, _) = data_unit(read_call_function(&report, "debug"));
    for (name, parameter) in [("debug", 0), ("active", 1)] {
        let pair = shared_evidence::pair(&unit, read_call_function(&report, name));
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        assert_eq!(
            returned.annotations[rstyle_pdg::data_origins::KEY][0]["parameter"],
            parameter
        );
    }
}

fn check_reference_rebinding_origins(report: &Value) {
    let (unit, _) = data_unit(read_call_function(report, "first"));
    for (name, expected) in [
        ("first", vec![1]),
        ("snapshot", vec![0]),
        ("branch", vec![0, 1]),
        ("killed", vec![0]),
        ("looped", vec![0, 1]),
        ("swapped", vec![0, 1]),
        ("aggregate", vec![0, 1]),
        ("aggregate_snapshot", vec![0]),
        ("update", vec![1]),
        ("parameter", vec![1]),
        ("stopped", vec![1]),
        ("matched", vec![0, 1]),
        ("guarded", vec![0, 1]),
        ("helper", vec![1]),
        ("composed", vec![1]),
    ] {
        let function = read_call_function(report, name);
        let pair = shared_evidence::pair(&unit, function);
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        let actual: Vec<_> = returned.annotations[rstyle_pdg::data_origins::KEY]
            .as_array()
            .unwrap()
            .iter()
            .map(|origin| origin["parameter"].as_u64().unwrap())
            .collect();
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn clones_data_static_inherent_helpers_compose_without_owner_values() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        mod model {
            pub struct Owner { pub unrelated: String }
            impl Owner {
                pub fn identity(value: i32) -> i32 { value }
                pub fn chained(value: i32) -> i32 { Self::identity(value) }
                pub fn zero() -> i32 { 0 }
                pub fn nothing() {}
                pub fn unsupported(value: i32) -> i32 { let reference = &value; *reference }
            }
        }
        use model::Owner as Alias;
        pub fn first(value: i32) -> i32 { Alias::chained(value) }
        pub fn other(value: i32) -> i32 { Alias::chained(value) }
        pub fn qualified(value: i32) -> i32 { crate::model::Owner::identity(value) }
        pub fn zero_call() -> i32 { Alias::zero() }
        pub fn unit_call() { Alias::nothing() }
        pub fn rejected(value: i32) -> i32 { Alias::unsupported(value) }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "model::Owner::unsupported" | "rejected"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    check_static_inherent_tampering(read_call_function(&report, "model::Owner::chained"));
}

fn check_static_inherent_tampering(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for change in ["owner", "implementation", "span", "identity"] {
        let mut changed = facts.clone();
        tamper_inherent_evidence(&mut changed, change);
        shared_evidence::reject_facts(function, &changed);
    }
    for field in ["owner_nominal", "implementation_span"] {
        let mut changed = facts.clone();
        let key = rstyle_pdg::inherent_calls::KEY;
        if field == "owner_nominal" {
            changed.build.frontend_options.get_mut(key).unwrap()[field]["declaration_digest"] =
                serde_json::json!("forged");
        } else {
            changed.build.frontend_options.get_mut(key).unwrap()[field]["start"] =
                serde_json::json!(0);
        }
        let own = changed.build.frontend_options.get(key).unwrap().clone();
        let associations = changed
            .build
            .frontend_options
            .get_mut(rstyle_pdg::inherent_calls::UNIT_KEY)
            .unwrap()["definitions"]
            .as_array_mut()
            .unwrap();
        let association = associations
            .iter_mut()
            .find(|record| record["definition_identity"] == own["definition_identity"])
            .unwrap();
        association[field] = own[field].clone();
        shared_evidence::reject_facts(function, &changed);
    }
}

fn body_nominal_workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/model.rs"),
        r#"
        pub struct Pair<'a> { pub left: &'a i32, pub right: &'a i32 }
        pub struct Outer<'a> { pub inner: Pair<'a> }
        pub struct Tuple<'a>(pub &'a i32, pub &'a i32);
        pub struct Scalar { pub value: i32 }
        impl Scalar { pub fn extract(value: i32) -> i32 { Self { value }.value } }
        pub struct Unit;
    "#,
    )
    .unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        mod model;
        use model::Pair as Alias;
        pub fn first<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let pair = Alias { right: b, left: a }; pair.left }
        pub fn other<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let pair = Alias { right: b, left: a }; pair.left }
        pub fn nested<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let outer = model::Outer { inner: Alias { left: a, right: b } }; outer.inner.right }
        pub fn tuple<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { model::Tuple(a, b).1 }
        pub fn update<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let pair = Alias { left: a, right: b }; Alias { left: b, ..pair }.right }
        pub fn conditional<'a>(a: &'a i32, b: &'a i32, pick: bool) -> &'a i32 { (if pick { Alias { left: a, right: b } } else { Alias { left: b, right: a } }).left }
        pub fn scalar(value: i32) -> i32 { model::Scalar { value }.value }
        pub fn helper(value: i32) -> i32 { scalar(value) }
        pub fn self_call(value: i32) -> i32 { model::Scalar::extract(value) }
        pub fn unit(value: i32) -> i32 { let tag = model::Unit; value }
        pub fn borrow(value: i32) -> i32 { let saved = model::Scalar { value }; let reference = &saved.value; saved.value }
        pub fn mutable<'a>(a: &'a i32, b: &'a i32) -> &'a i32 { let mut pair = Alias { left: a, right: b }; pair.left }
    "#).unwrap();
    directory
}

fn check_body_nominal_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let view = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    for corruption in ["origin", "range", "field"] {
        let mut facts = (*view).clone();
        let mut origins = facts
            .build
            .frontend_options
            .data::<rstyle_pdg::declaration_origins::Origins>()
            .unwrap()
            .into_owned();
        assert_eq!(origins.declarations.len(), 1);
        match corruption {
            "origin" => origins.declarations[0]
                .nominal
                .definition_path
                .push_str("_other"),
            "range" => origins.declarations[0].declaration_span.end -= 1,
            _ => {
                let mut changed = serde_json::to_value(&facts).unwrap();
                change_body_nominal_fields(&mut changed["body"]);
                facts = serde_json::from_value(changed).unwrap();
            }
        }
        facts.build.frontend_options.insert_data(origins);
        assert!(
            rstyle_pdg::data_documents::verify(pair, &facts).is_err(),
            "{corruption}"
        );
        assert!(
            rstyle_pdg::producer::verify_view(pair.as_view(), &facts).is_err(),
            "{corruption}"
        );
    }
}

fn change_body_nominal_fields(node: &mut Value) {
    if let Some(text) = node["ty"]["normalized"].as_str()
        && let Ok(mut ty) = serde_json::from_str::<Value>(text)
        && ty["kind"] == "struct"
    {
        ty["value"]["fields"][0][0] = "wrong".into();
        node["ty"]["normalized"] = serde_json::to_string(&ty).unwrap().into();
    }
    if let Some(children) = node["children"].as_array_mut() {
        for child in children {
            change_body_nominal_fields(child);
        }
    }
}

fn check_body_nominal_interface(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let facts = &function["facts"];
    assert_eq!(facts["signature"]["params"].as_array().unwrap().len(), 2);
    let observations = &facts["build"]["frontend_options"][rstyle_pdg::concrete_types::KEY];
    assert_eq!(observations["parameters"].as_array().unwrap().len(), 2);
    assert_eq!(
        function["nominal_unit_binding"]["declarations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

const INDIRECT_FIELD_FAMILY: &str = r#"
    pub struct Inner { pub n: u8 }
    pub struct Record<'a> { pub inner: Inner, pub pointer: &'a Inner }
    pub fn first(a: &Inner, b: &Inner, pick: bool) -> u8 { let selected = if pick { a } else { b }; selected.n }
    pub fn composed(a: &Inner, b: &Inner, pick: bool) -> u8 { first(a, b, pick) }
    pub fn other(a: &Inner, b: &Inner, pick: bool) -> u8 { let selected = if pick { a } else { b }; selected.n }
    pub fn mutable(a: &Inner, b: &Inner) -> u8 { let mut selected = a; selected = b; selected.n }
    pub fn snapshot(a: &Inner, b: &Inner) -> u8 { let mut selected = a; let saved = selected; selected = b; saved.n }
    pub fn formal_snapshot<'a>(mut a: &'a Inner, b: &'a Inner) -> u8 { let saved = a; a = b; saved.n }
    pub fn looped(a: &Inner, b: &Inner, mut count: u32) -> u8 { let mut selected = a; while count > 0 { selected = b; count -= 1; } selected.n }
    pub fn nested(a: &Record<'_>, b: &Record<'_>, pick: bool) -> u8 { let selected = if pick { a } else { b }; selected.inner.n }
    pub fn stored(a: &Record<'_>, b: &Record<'_>, pick: bool) -> u8 { let selected = if pick { a } else { b }; selected.pointer.n }
    pub fn reference<'a>(a: &Record<'a>, b: &Record<'a>, pick: bool) -> &'a Inner { let selected = if pick { a } else { b }; selected.pointer }
    pub fn choose<'a>(a: &'a Inner, b: &'a Inner, pick: bool) -> &'a Inner { if pick { a } else { b } }
    pub fn helper(a: &Inner, b: &Inner, pick: bool) -> u8 { choose(a, b, pick).n }
    pub fn conditional(a: &Inner, b: &Inner, pick: bool) -> u8 { (if pick { a } else { b }).n }
    pub fn tuple(a: &(u8, u8), b: &(u8, u8), pick: bool) -> u8 { let selected = if pick { a } else { b }; selected.1 }
    pub fn same(a: &Inner, pick: bool) -> u8 { let selected = if pick { a } else { a }; selected.n }
    pub fn borrow_local(value: u8) -> u8 { let local = Inner { n: value }; let selected = &local; selected.n }
    pub fn pointee(a: &mut Inner) -> u8 { a.n = 1; a.n }
"#;

#[test]
fn clones_data_indirect_fields_compose_with_reference_flow_and_helpers() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), INDIRECT_FIELD_FAMILY).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 17);
    for function in report["functions"].as_array().unwrap() {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "borrow_local" | "pointee"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_indirect_field_locations(&report);
    check_indirect_field_paths(&report);
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    for function in legacy["functions"].as_array().unwrap() {
        if function["facts"]["name"] != "choose" {
            assert_eq!(function["outcome"], "excluded", "{function}");
        }
    }
}

fn check_indirect_field_locations(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for (name, roots) in [
        ("first", vec![0, 1]),
        ("mutable", vec![1]),
        ("snapshot", vec![0]),
        ("formal_snapshot", vec![0]),
        ("looped", vec![0, 1]),
        ("nested", vec![0, 1]),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        let actual: Vec<_> = pair
            .pdg
            .memory_locations
            .iter()
            .map(|location| {
                location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(actual, roots, "{name}");
        check_indirect_field_corruption(&facts);
    }
}

fn check_indirect_field_corruption(facts: &dyn rstyle_pdg::unit_store::EvidenceFacts) {
    check_indirect_field_edge_corruption(facts);
    for mutation in ["missing", "symbol", "position"] {
        let mut changed = rstyle_pdg::producer::produce(facts).unwrap();
        let node = changed
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.attributes.construct.as_deref() == Some("data.indirect-field"))
            .unwrap();
        match mutation {
            "missing" => node.attributes.reads.clear(),
            "symbol" => node.attributes.reads[0].symbol_id = Some("invented".into()),
            _ => node.attributes.reads[0].operand_position = Some(1),
        }
        assert!(
            rstyle_pdg::data_documents::verify(&changed, facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&changed, facts).is_err(),
            "{mutation}"
        );
    }
}

fn check_indirect_field_paths(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    for name in ["nested", "reference"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        if name == "nested" {
            for location in &pair.pdg.memory_locations {
                let path = &location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"];
                assert_eq!(path.as_array().unwrap().len(), 3);
                assert_eq!(path[0]["kind"], "SharedReferent");
                assert_eq!(path[1]["name"], "inner");
                assert_eq!(path[2]["name"], "n");
            }
        } else {
            let returned = pair
                .pdg
                .nodes
                .iter()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
                .unwrap();
            let origins = returned.annotations[rstyle_pdg::data_origins::KEY]
                .as_array()
                .unwrap();
            assert_eq!(origins.len(), 2);
            for (parameter, origin) in origins.iter().enumerate() {
                assert_eq!(origin["parameter"], parameter);
                assert_eq!(origin["path"][0]["kind"], "SharedReferent");
                assert_eq!(origin["path"][1]["name"], "pointer");
                assert_eq!(origin["path"][2]["kind"], "SharedReferent");
            }
        }
    }
}

fn check_indirect_field_edge_corruption(facts: &dyn rstyle_pdg::unit_store::EvidenceFacts) {
    for mutation in ["base", "lexical", "path"] {
        let mut pair = rstyle_pdg::producer::produce(facts).unwrap();
        let id = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.attributes.construct.as_deref() == Some("data.indirect-field"))
            .unwrap()
            .id
            .clone();
        if mutation == "path" {
            pair.pdg.memory_locations[0]
                .extensions
                .get_mut(rstyle_pdg::data_documents::LOCATION_KEY)
                .unwrap()["path"] = serde_json::json!([]);
        } else {
            let edge = pair
                .pdg
                .edges
                .iter_mut()
                .find(|edge| edge.target == id && edge.subkind == "value_operand")
                .unwrap();
            if mutation == "base" {
                edge.source = id;
            } else {
                let mut extra = edge.clone();
                extra.subkind = "reaching_definition".into();
                pair.pdg.edges.push(extra);
            }
        }
        assert!(
            rstyle_pdg::data_documents::verify(&pair, facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, facts).is_err(),
            "{mutation}"
        );
    }
}

const SHARED_DEREFERENCE_FAMILY: &str = r#"
    pub struct Record<'a> { pub n: u8, pub pointer: &'a u8 }
    pub fn first(a: &u8) -> u8 { *a }
    pub fn other(a: &u8) -> u8 { *a }
    pub fn nested(a: &&u8) -> u8 { **a }
    pub fn aggregate(a: &(u8, u8)) -> (u8, u8) { *a }
    pub fn field(a: &Record<'_>) -> u8 { (*a).n }
    pub fn nested_field(a: &&Record<'_>) -> u8 { (**a).n }
    pub fn stored(a: &Record<'_>) -> u8 { *a.pointer }
    pub fn reference<'a>(a: &&'a u8) -> &'a u8 { *a }
    pub fn conditional(a: &u8, b: &u8, pick: bool) -> u8 { *(if pick { a } else { b }) }
    pub fn mutable(a: &u8, b: &u8) -> u8 { let mut selected = a; selected = b; *selected }
    pub fn snapshot(a: &u8, b: &u8) -> u8 { let mut selected = a; let saved = selected; selected = b; *saved }
    pub fn looped(a: &u8, b: &u8, mut count: u32) -> u8 { let mut selected = a; while count > 0 { selected = b; count -= 1; } *selected }
    pub fn composed(a: &u8) -> u8 { first(a) }
    pub fn loop_read(a: &u8, mut count: u32) -> u8 { let mut value = 0; while count > 0 { value = *a; count -= 1; } value }
    pub fn choose<'a>(a: &'a u8, b: &'a u8, pick: bool) -> &'a u8 { if pick { a } else { b } }
    pub fn helper(a: &u8, b: &u8, pick: bool) -> u8 { *choose(a, b, pick) }
    pub fn flow_snapshot<'a>(a: &&'a u8, b: &&'a u8) -> &'a u8 { let mut selected = a; let saved = *selected; selected = b; saved }
    pub fn flow_rebound<'a>(a: &&'a u8, b: &&'a u8) -> &'a u8 { let mut selected = a; selected = b; *selected }
    pub fn flow_loop<'a>(a: &&'a u8, b: &&'a u8, mut count: u32) -> &'a u8 { let mut selected = a; while count > 0 { selected = b; count -= 1; } *selected }
    pub fn flow_component<'a>(a: &(&'a u8, u8), b: &(&'a u8, u8)) -> &'a u8 { let mut selected = a; let saved = *selected; selected = b; saved.0 }
    pub fn local(value: u8) -> u8 { let r = &value; *r }
    pub fn write(a: &mut u8) -> u8 { *a = 1; *a }
"#;

#[test]
fn clones_data_shared_dereferences_compose_physical_reads_and_reference_flow() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        SHARED_DEREFERENCE_FAMILY,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 22);
    for function in report["functions"].as_array().unwrap() {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "local" | "write"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_shared_dereference_locations(&report);
    check_dereference_flow_origins(&report);
    check_dereference_flow_component(&report);
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    for function in legacy["functions"].as_array().unwrap() {
        if function["facts"]["name"] != "choose" {
            assert_eq!(function["outcome"], "excluded", "{function}");
        }
    }
}

fn check_shared_dereference_locations(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for (name, roots) in [
        ("first", vec![0]),
        ("mutable", vec![1]),
        ("snapshot", vec![0]),
        ("conditional", vec![0, 1]),
        ("looped", vec![0, 1]),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        let actual: Vec<_> = pair
            .pdg
            .memory_locations
            .iter()
            .map(|location| {
                let record = &location.extensions[rstyle_pdg::data_documents::LOCATION_KEY];
                assert_eq!(
                    record["path"],
                    serde_json::json!([{"kind":"SharedReferent"}])
                );
                record["parameter"].as_u64().unwrap()
            })
            .collect();
        assert_eq!(actual, roots, "{name}");
        check_shared_dereference_corruption(&facts);
    }
    check_shared_dereference_paths(report);
}

fn check_shared_dereference_corruption(facts: &dyn rstyle_pdg::unit_store::EvidenceFacts) {
    for mutation in ["missing", "symbol", "operand", "dispatch"] {
        let mut pair = rstyle_pdg::producer::produce(facts).unwrap();
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.attributes.construct.as_deref() == Some("data.indirect-deref"))
            .unwrap();
        match mutation {
            "missing" => node.attributes.reads.clear(),
            "symbol" => node.attributes.reads[0].symbol_id = Some("invented".into()),
            "operand" => node.attributes.reads[0].operand_position = Some(1),
            _ => node.attributes.operator = Some(rstyle_pdg::StringFact::known("*")),
        }
        assert!(
            rstyle_pdg::data_documents::verify(&pair, facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, facts).is_err(),
            "{mutation}"
        );
    }
}

fn check_shared_dereference_paths(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    for (name, steps) in [("nested", 2), ("nested_field", 3), ("reference", 1)] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        let longest = pair
            .pdg
            .memory_locations
            .iter()
            .map(|location| {
                location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
                    .as_array()
                    .unwrap()
                    .len()
            })
            .max()
            .unwrap();
        assert_eq!(longest, steps, "{name}");
        if name == "nested_field" {
            assert_eq!(pair.pdg.memory_locations.len(), 2);
            assert!(pair.pdg.memory_locations.iter().any(|location| {
                location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"][2]["name"]
                    == "n"
            }));
        }
        if name == "reference" {
            let returned = pair
                .pdg
                .nodes
                .iter()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
                .unwrap();
            assert_eq!(
                returned.annotations[rstyle_pdg::data_origins::KEY],
                serde_json::json!([
                    {"parameter":0,"path":[{"kind":"SharedReferent"},{"kind":"SharedReferent"}]}
                ])
            );
        }
    }
}

fn check_dereference_flow_origins(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    for (name, roots) in [
        ("flow_snapshot", vec![0]),
        ("flow_rebound", vec![1]),
        ("flow_loop", vec![0, 1]),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
            .unwrap();
        let origins = returned.annotations[rstyle_pdg::data_origins::KEY]
            .as_array()
            .unwrap();
        let actual: Vec<_> = origins
            .iter()
            .map(|origin| {
                assert_eq!(
                    origin["path"],
                    serde_json::json!([
                        {"kind":"SharedReferent"}, {"kind":"SharedReferent"}
                    ])
                );
                origin["parameter"].as_u64().unwrap()
            })
            .collect();
        assert_eq!(actual, roots, "{name}");
    }
}

fn check_dereference_flow_component(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "flow_component")
        .unwrap();
    let (unit, index) = data_unit(function);
    let pair = unit.pair(index).unwrap();
    let returned = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
        .unwrap();
    assert_eq!(
        returned.annotations[rstyle_pdg::data_origins::KEY],
        serde_json::json!([
            {"parameter":0,"path":[
                {"kind":"SharedReferent"}, {"kind":"TupleField","position":0}, {"kind":"SharedReferent"}
            ]}
        ])
    );
}

const INDEX_READ_FAMILY: &str = r#"
    pub struct Holder<'a> { pub bytes: &'a [u8] }
    pub fn first(a: &[u8], i: usize) -> u8 { a[i] }
    pub fn other(a: &[u8], i: usize) -> u8 { a[i] }
    pub fn array(a: [u8; 3], i: usize) -> u8 { a[i] }
    pub fn local_array(a: u8, b: u8, i: usize) -> u8 { let values = [a, b]; values[i] }
    pub fn shared_array(a: &[u8; 3], i: usize) -> u8 { a[i] }
    pub fn field(h: &Holder<'_>, i: usize) -> u8 { h.bytes[i] }
    pub fn mutable(a: &[u8], b: &[u8], i: usize) -> u8 { let mut selected = a; selected = b; selected[i] }
    pub fn index_rebind<'a>(mut a: &'a [u8], b: &'a [u8]) -> u8 { a[{ a = b; 0 }] }
    pub fn array_rebind(mut a: [u8; 2], b: [u8; 2]) -> u8 { a[{ a = b; 0 }] }
    pub fn field_rebind<'a>(mut h: &'a Holder<'a>, other: &'a Holder<'a>) -> u8 { h.bytes[{ h = other; 0 }] }
    pub fn computed<'a>(mut selected: &'a [u8], other: &'a [u8]) -> u8 { ({ selected })[{ selected = other; 0 }] }
    pub fn computed_array(mut a: [u8; 2], b: [u8; 2]) -> u8 { ({ a })[{ a = b; 0 }] }
    pub fn snapshot(a: &[u8], b: &[u8], i: usize) -> u8 { let mut selected = a; let saved = selected; selected = b; saved[i] }
    pub fn looped(a: &[u8], b: &[u8], mut count: u32, i: usize) -> u8 { let mut selected = a; while count > 0 { selected = b; count -= 1; } selected[i] }
    pub fn loop_read(a: &[u8], mut i: usize) -> u8 { let mut value = 0; while i > 0 { i -= 1; value = a[i]; } value }
    pub fn choose<'a>(a: &'a [u8], b: &'a [u8], pick: bool) -> &'a [u8] { if pick { a } else { b } }
    pub fn helper_base(a: &[u8], b: &[u8], pick: bool, i: usize) -> u8 { choose(a, b, pick)[i] }
    pub fn twice(a: &[u8], i: usize, j: usize) -> u8 { a[i] + a[j] }
    pub fn reference_element<'a>(a: &[&'a u8], i: usize) -> &'a u8 { a[i] }
    pub fn aggregate_element(a: &[(u8, u8)], i: usize) -> (u8, u8) { a[i] }
    pub fn borrow_local(a: [u8; 2], i: usize) -> u8 { let r = &a; r[i] }
    pub fn write(a: &mut [u8], i: usize) -> u8 { a[i] = 1; a[i] }
    pub fn range(a: &[u8]) -> &[u8] { &a[1..] }
    pub fn index_helper(a: &[u8], i: usize) -> u8 { first(a, i) }
    pub fn array_helper(a: [u8; 3], i: usize) -> u8 { array(a, i) }
"#;

#[test]
fn clones_data_index_reads_preserve_places_snapshots_bounds_and_scalar_values() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), INDEX_READ_FAMILY).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 25);
    for function in report["functions"].as_array().unwrap() {
        let excluded = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "reference_element"
                | "aggregate_element"
                | "borrow_local"
                | "write"
                | "range"
                | "index_helper"
                | "array_helper"
        );
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_index_locations(&report);
    check_index_place_values(&report);
    check_index_corruption(&report);
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["first", "other"]);
    let (output, legacy) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{legacy}");
    for function in legacy["functions"].as_array().unwrap() {
        if function["facts"]["name"] != "choose" {
            assert_eq!(function["outcome"], "excluded", "{function}");
        }
    }
}

fn check_index_locations(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    for (name, expected) in [
        ("first", vec![0]),
        ("mutable", vec![1]),
        ("index_rebind", vec![1]),
        ("field_rebind", vec![1]),
        ("computed", vec![0]),
        ("snapshot", vec![0]),
        ("looped", vec![0, 1]),
        ("helper_base", vec![0, 1]),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        let mut actual: Vec<_> = pair
            .pdg
            .memory_locations
            .iter()
            .filter_map(|location| {
                let record = &location.extensions[rstyle_pdg::data_documents::LOCATION_KEY];
                let last = record["path"].as_array().unwrap().last().unwrap();
                if last["kind"] != "IndexedElement" {
                    return None;
                }
                assert_eq!(last["operand_position"], 1);
                assert_eq!(location.abstraction, "rstyle.formal-indexed-element/1");
                Some(record["parameter"].as_u64().unwrap())
            })
            .collect();
        actual.sort();
        assert_eq!(actual, expected, "{name}");
    }
}

fn check_index_place_values(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    for (name, category) in [
        ("array_rebind", rstyle_pdg::NodeCategory::Assignment),
        ("computed_array", rstyle_pdg::NodeCategory::FormalInput),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        assert!(pair.pdg.memory_locations.is_empty());
        let indexed = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.attributes.construct.as_deref() == Some("data.value-index"))
            .unwrap();
        let base = pair
            .pdg
            .edges
            .iter()
            .find(|edge| {
                edge.target == indexed.id
                    && edge.subkind == "value_operand"
                    && edge.operand_position == Some(0)
            })
            .unwrap();
        let reaching: Vec<_> = pair
            .pdg
            .edges
            .iter()
            .filter(|edge| edge.target == base.source && edge.subkind == "reaching_definition")
            .collect();
        assert_eq!(reaching.len(), 1);
        let producer = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.id == reaching[0].source)
            .unwrap();
        assert_eq!(producer.category, category, "{name}");
    }
    check_index_access_count(report);
}

fn check_index_access_count(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "twice")
        .unwrap();
    let pair = shared_evidence::pair(&unit, function);
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    assert_eq!(pair.pdg.function.reference_count.value, Some(5));
}

fn check_index_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "first")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["bounds", "missing", "symbol", "selector", "operand"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt_index(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
}

fn corrupt_index(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.index"))
        .unwrap();
    match mutation {
        "bounds" => {
            node.annotations.remove("rstyle.data:index-bounds");
        }
        "missing" => node.attributes.reads.clear(),
        "symbol" => node.attributes.reads[0].symbol_id = Some("invented".into()),
        "selector" => {
            pair.pdg.memory_locations[0]
                .extensions
                .get_mut(rstyle_pdg::data_documents::LOCATION_KEY)
                .unwrap()["path"][1]["operand_position"] = 0.into()
        }
        _ => {
            pair.pdg
                .edges
                .iter_mut()
                .find(|edge| {
                    edge.target == node.id
                        && edge.subkind == "value_operand"
                        && edge.operand_position == Some(1)
                })
                .unwrap()
                .operand_position = Some(0)
        }
    }
}

#[test]
fn clones_data_external_associated_calls_retain_distinct_authorities() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn native(bytes: [u8; 4]) -> u32 { u32::from_ne_bytes(bytes) }
        pub fn endian(bytes: [u8; 4]) -> u32 { u32::from_be_bytes(bytes) }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let mut identities = std::collections::BTreeSet::new();
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(
            function["outcome"],
            if function["facts"]["name"] == "native" {
                "admitted"
            } else {
                "excluded"
            }
        );
        let (unit, index) = data_unit(function);
        let analysis = unit.analysis();
        let facts = analysis.function(index).unwrap();
        let call = find_external_call(&facts.body).unwrap();
        let callee: rstyle_pdg::data_library::Callee =
            serde_json::from_str(&call.res.as_ref().unwrap().description).unwrap();
        assert_eq!(callee.crate_name, "core");
        assert_ne!(callee.crate_hash, "local-unencoded");
        let dispatch = call.dispatch.as_ref().unwrap();
        assert_eq!(dispatch.kind, "resolved_direct");
        assert_eq!(dispatch.op, callee.path);
        assert_eq!(dispatch.operand_tys.len(), 1);
        let operand: Value = serde_json::from_str(&dispatch.operand_tys[0]).unwrap();
        assert_eq!(
            operand,
            serde_json::json!({
                "kind": "array", "value": {
                    "element": {"kind": "scalar", "value": "u8"}, "length": 4
                }
            })
        );
        assert_eq!(dispatch.result_ty, "u32");
        assert!(identities.insert(callee.definition_hash));
    }
    assert_eq!(identities.len(), 2);
}

fn find_external_call(node: &rstyle_pdg::HirNode) -> Option<&rstyle_pdg::HirNode> {
    if node.kind == "expr_call"
        && node
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "resolved_callee")
    {
        return Some(node);
    }
    node.children.iter().find_map(find_external_call)
}

#[test]
fn clones_data_range_capture_distinguishes_core_desugaring_and_custom_structs() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn window(a: &[u8], lo: usize, hi: usize) -> &[u8] { &a[lo..hi] }
        pub fn suffix(a: &[u8], lo: usize) -> &[u8] { &a[lo..] }
        pub fn inclusive(a: &[u8], hi: usize) -> &[u8] { &a[..=hi] }
        pub struct Range { pub start: usize, pub end: usize }
        pub fn custom(start: usize, end: usize) -> Range { Range { start, end } }
        pub fn scalar(a: &[u8], i: usize) -> u8 { a[i] }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        let (unit, index) = data_unit(function);
        let analysis = unit.analysis();
        let facts = analysis.function(index).unwrap();
        match facts.name.as_str() {
            "window" | "suffix" | "inclusive" => {
                assert_eq!(
                    function["outcome"],
                    if facts.name == "window" {
                        "admitted"
                    } else {
                        "excluded"
                    }
                );
                check_range_capture(&facts.body);
            }
            "custom" => {
                let node = find_kind(&facts.body, "expr_struct").unwrap();
                assert!(node.res.is_none());
                assert!(node.expansion.is_none());
            }
            "scalar" => {
                assert_eq!(function["outcome"], "admitted");
                assert!(find_kind(&facts.body, "expr_index").unwrap().res.is_none());
            }
            _ => panic!("unexpected function"),
        }
    }
}

fn check_range_capture(body: &rstyle_pdg::HirNode) {
    let indexed = find_kind(body, "expr_index").unwrap();
    let resolution = indexed.res.as_ref().unwrap();
    assert_eq!(resolution.kind, "resolved_callee");
    let callee: rstyle_pdg::data_library::Callee =
        serde_json::from_str(&resolution.description).unwrap();
    assert_eq!(callee.crate_name, "core");
    assert!(callee.path.ends_with("::index"));
    assert_eq!(indexed.dispatch.as_ref().unwrap().kind, "overloaded");
    let range = &indexed.children[1];
    let resolution = range.res.as_ref().unwrap();
    assert_eq!(resolution.kind, "resolved_range_type");
    let callee: rstyle_pdg::data_library::Callee =
        serde_json::from_str(&resolution.description).unwrap();
    assert_eq!(callee.crate_name, "core");
    assert!(callee.path.contains("::ops::Range"), "{}", callee.path);
    assert!(range.expansion.is_some());
}

fn find_kind<'a>(node: &'a rstyle_pdg::HirNode, kind: &str) -> Option<&'a rstyle_pdg::HirNode> {
    if node.kind == kind {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_kind(child, kind))
}

#[test]
fn clones_data_subslice_flow_snapshots_base_before_endpoint_rebinding() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn snapshot<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] {
            let mut selected = a;
            &selected[{ selected = b; 0 }..1]
        }
        pub fn scalar(a: &[u8], b: &[u8]) -> u8 {
            let mut selected = a;
            selected[{ selected = b; 0 }]
        }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        let (unit, index) = data_unit(function);
        let analysis = unit.analysis();
        let facts = analysis.function(index).unwrap();
        let indexed = find_kind(&facts.body, "expr_index").unwrap();
        let base = &indexed.children[0];
        let roots = rstyle_pdg::data_origins::origins(base, &facts).unwrap();
        let scalar = facts.name == "scalar";
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].parameter, u32::from(scalar));
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_aggregate_documents(&report);
}

#[test]
fn clones_data_shared_subslice_views_preserve_endpoints_and_saved_origins() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn window(a: &[u8], lo: usize, hi: usize) -> &[u8] { &a[lo..hi] }
        pub fn same(a: &[u8], lo: usize, hi: usize) -> &[u8] { &a[lo..hi] }
        pub fn read(a: &[u8], lo: usize, hi: usize, i: usize) -> u8 {
            let view = &a[lo..hi]; view[i]
        }
        pub fn saved<'a>(a: &'a [u8], b: &'a [u8], lo: usize, hi: usize) -> &'a [u8] {
            let mut selected = a; let view = &selected[lo..hi]; selected = b; view
        }
        pub fn nested(a: &[u8], lo: usize, hi: usize) -> u8 { let view = &a[lo..hi]; (&view[0..1])[0] }
        pub fn choose<'a>(a: &'a [u8], b: &'a [u8], pick: bool) -> &'a [u8] { if pick { a } else { b } }
        pub fn helper_base<'a>(a: &'a [u8], b: &'a [u8], pick: bool) -> &'a [u8] { &choose(a, b, pick)[0..1] }
        pub fn caller(a: &[u8]) -> &[u8] { window(a, 0, 1) }
        pub fn array(a: &[u8; 4]) -> &[u8] { &a[0..1] }
        pub fn suffix(a: &[u8], lo: usize) -> &[u8] { &a[lo..] }
        pub fn inclusive(a: &[u8], hi: usize) -> &[u8] { &a[0..=hi] }
        pub fn write(a: &mut [u8], lo: usize, hi: usize) -> &mut [u8] { &mut a[lo..hi] }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "window" | "same" | "read" | "saved" | "nested" | "choose" | "helper_base"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_subslice_corruption(&report);
    check_equivalent_matching(&report, ["window", "same"]);
}

fn check_subslice_corruption(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "window")
        .unwrap();
    check_subslice_authorities(function);
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["bounds", "operand", "read"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt_subslice(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
}

fn corrupt_subslice(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.subslice-place"))
        .unwrap();
    match mutation {
        "bounds" => {
            node.annotations.remove("rstyle.data:subslice-bounds");
        }
        "read" => node.attributes.reads.push(rstyle_pdg::Access {
            symbol_id: Some("invented".into()),
            location_id: None,
            unresolved_reason: None,
            operand_position: Some(0),
        }),
        _ => {
            pair.pdg
                .edges
                .iter_mut()
                .find(|edge| {
                    edge.target == node.id
                        && edge.subkind == "value_operand"
                        && edge.operand_position == Some(2)
                })
                .unwrap()
                .operand_position = Some(1)
        }
    }
}

fn check_subslice_authorities(function: &Value) {
    for range in [false, true] {
        shared_evidence::reject_edit(function, |bundle, index| {
            let indexed = &mut bundle["functions"][index]["body"]["children"][0]["children"][0];
            let operation = if range {
                &mut indexed["children"][1]
            } else {
                indexed
            };
            let mut callee: Value =
                serde_json::from_str(operation["res"]["description"].as_str().unwrap()).unwrap();
            callee["definition_hash"] = "invented".into();
            operation["res"]["description"] = callee.to_string().into();
        });
    }
}
