//! End-to-end tests: real `cargo check` through the real rustc driver.

#[path = "../src/diagnostic_process.rs"]
mod diagnostic_process;

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
    let mut args = vec!["clones", "--format", "json", "--expanded-evidence"];
    args.extend_from_slice(arguments);
    let output = rstyle(directory, &args);
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
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
    assert_eq!(report["unit_bundle_schema"], "rstyle.shared-unit/1");
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 3);
    for function in report["functions"].as_array().unwrap() {
        assert!(function["artifact_prefix"].is_null());
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
    assert!(output.status.success());
    assert_eq!(report["matching"], serial["matching"]);
    let (output, selected) = clones(directory.path(), &["-p", "a", "--release"]);
    assert!(output.status.success());
    assert_eq!(selected["functions"].as_array().unwrap().len(), 2);
    assert_eq!(selected["matching"]["pairs"].as_array().unwrap().len(), 1);
    assert!(selected["functions"].as_array().unwrap().iter().all(|f| {
        !report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|old| old["key"] == f["key"])
    }));
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
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(wrapper["facts"].clone()).unwrap();
    for corruption in ["manifest", "source", "signature", "effects"] {
        let mut changed = facts.clone();
        assert!(corrupt_helper(&mut changed.body, corruption));
        assert!(
            rstyle_pdg::producer::produce(&changed).is_err(),
            "{corruption}"
        );
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
    assert_ne!(wrapper["facts"]["body"], changed["facts"]["body"]);
    assert_ne!(wrapper["document_manifest"], changed["document_manifest"]);
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
    let native_facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(function["facts"].clone()).unwrap();
    let pair = rstyle_pdg::producer::produce(&native_facts).unwrap();
    let imported = rstyle_ccgraph::admit_produced(&pair, &native_facts).unwrap();
    assert!(imported.native_width_involved);
}

fn corrupt_helper(node: &mut rstyle_pdg::HirNode, corruption: &str) -> bool {
    if node.kind == "expr_call" {
        let resolution = node.res.as_mut().unwrap();
        let mut value: Value = serde_json::from_str(&resolution.description).unwrap();
        let dependency = &mut value[rstyle_pdg::helper_calls::KEY];
        match corruption {
            "effects" => dependency["summary"]["always_returns"] = Value::Bool(false),
            "manifest" => {
                dependency["manifest"]["documents"][0]["sha256"] =
                    Value::String("sha256:bad".into())
            }
            "source" => {
                dependency["frontend"]["source"]["bytes"] = Value::String("fn forged() {}".into())
            }
            "signature" => {
                dependency["frontend"]["signature"]["return_ty"]["normalized"] =
                    Value::String("i32".into())
            }
            "output_missing" => {
                dependency["summary"]
                    .as_object_mut()
                    .unwrap()
                    .remove("reference_outputs");
            }
            "output_parameter" => {
                dependency["summary"]["reference_outputs"][0]["parameter"] = serde_json::json!(0)
            }
            "output_empty" => dependency["summary"]["reference_outputs"] = serde_json::json!([]),
            "output_path" => {
                dependency["summary"]["reference_outputs"][0]["path"] = serde_json::json!([])
            }
            "output_identity" => {
                dependency["summary"]["identity"] = serde_json::json!("rstyle.helper.source-body/1")
            }
            "unit_invocation" => {
                dependency["frontend"]["build"]["frontend_options"]["invocation"] =
                    serde_json::json!(["forged"])
            }
            "unit_target" => {
                dependency["frontend"]["build"]["target_triple"]["value"] =
                    serde_json::json!("another-target")
            }
            _ => unreachable!(),
        }
        resolution.description = serde_json::to_string(&value).unwrap();
        return true;
    }

    node.children
        .iter_mut()
        .any(|child| corrupt_helper(child, corruption))
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
    rstyle_pdg::helper_calls::capture(&mut forward).unwrap();
    rstyle_pdg::helper_calls::capture(&mut reversed).unwrap();
    reversed.reverse();
    assert_eq!(forward, reversed);
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
    let standalone = store.expanded_frontends().unwrap();
    for (index, facts) in store.functions().iter().enumerate() {
        let view = analysis.function(index).unwrap();
        rstyle_pdg::producer::verify(store.pair(index).unwrap(), &view).unwrap();
        let imported = rstyle_ccgraph::admit_produced(store.pair(index).unwrap(), &view).unwrap();
        let legacy_pair = rstyle_pdg::producer::produce(&standalone[index]).unwrap();
        let legacy = rstyle_ccgraph::admit_produced(&legacy_pair, &standalone[index]).unwrap();
        assert_eq!(imported, legacy);
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
    wrong_unit.pdg.extensions.get_mut(key).unwrap()["frontend_ref"]["unit"] =
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
    assert_eq!(
        serde_json::to_value(&actual[0]).unwrap(),
        function["nominal_unit_binding"]
    );
    check_nominal_evidence_corruption(
        &facts,
        &context,
        !actual[0].as_ref().unwrap().declarations.is_empty(),
    );
    for change in ["name", "version", "manifest", "root"] {
        let mut changed = context.clone();
        match change {
            "name" => changed.name.push('x'),
            "version" => changed.version.push('x'),
            "manifest" => {
                changed.manifest_bytes.push_str("\n# changed");
                changed.manifest_digest =
                    rstyle_pdg::concrete_types::digest(changed.manifest_bytes.as_bytes());
            }
            "root" => changed.manifest_path = PathBuf::from("/different/package/Cargo.toml"),
            _ => unreachable!(),
        }
        if !actual[0].as_ref().unwrap().declarations.is_empty() {
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
fn clones_helpers_concrete_capture_budget_is_not_an_exclusion() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn large(_: &[u8; 4097]) {}\n",
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
            builds.insert(binding["build_digest"].as_str().unwrap().to_owned());
        }
    }
    assert_eq!(packages.len(), 2);
    assert_eq!(builds.len(), 4);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["pair_enumeration_complete"], true);
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
    for key in ["package_id", "manifest_digest", "build_digest"] {
        assert_ne!(old[key], new[key], "{key}");
    }
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

fn data_pair(function: &Value) -> (rstyle_pdg::producer::Pair, rstyle_pdg::HirFnFacts) {
    let prefix = function["artifact_prefix"].as_str().unwrap();
    let pdg =
        serde_json::from_slice(&std::fs::read(format!("{prefix}.pdg.json")).unwrap()).unwrap();
    let cfg =
        serde_json::from_slice(&std::fs::read(format!("{prefix}.cfg.json")).unwrap()).unwrap();
    let facts = serde_json::from_value(function["facts"].clone()).unwrap();
    (rstyle_pdg::producer::Pair { pdg, cfg }, facts)
}

fn check_data_document_tampering(function: &Value) {
    let (pair, facts) = data_pair(function);
    rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
    for change in [
        "path", "read", "alias", "count", "policy", "write", "operand", "root",
    ] {
        let (mut changed, _) = data_pair(function);
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
    let (pair, facts) = data_pair(function);
    rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
    for change in ["origin", "value", "type"] {
        let (mut changed, _) = data_pair(function);
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
    let (pair, facts) = data_pair(method);
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
    let mut changed = pair;
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
    let (pair, facts) = data_pair(function);
    rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
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
    let (pair, facts) = data_pair(function);
    rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
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
        let (mut changed, _) = data_pair(function);
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
    let (_, facts) = data_pair(function);
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
    let (pair, facts) = data_pair(function);
    rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
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
    for function in functions.iter().filter(|f| f["outcome"] == "admitted") {
        let (pair, facts) = data_pair(function);
        rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
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
        5
    );
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
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
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted");
        let (pair, facts) = data_pair(function);
        rstyle_pdg::data_receivers::verify_documents(&pair, &facts).unwrap();
        let mut bad_signature = facts.clone();
        bad_signature
            .build
            .frontend_options
            .get_mut(rstyle_pdg::data_receivers::KEY)
            .unwrap()["methods"][0]["signature_inputs"] = serde_json::json!([]);
        assert!(rstyle_pdg::data_receivers::verify(&bad_signature).is_err());
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
            let frontend = &mut extensions
                .get_mut(rstyle_pdg::extraction_profile::EVIDENCE_KEY)
                .unwrap()["frontend"];
            let inputs = &mut frontend["build"]["frontend_options"]
                [rstyle_pdg::data_receivers::KEY]["methods"][0]["signature_inputs"];
            *inputs = serde_json::json!([]);
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
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 105);
    assert_eq!(report["pair_evaluation_complete"], true);
    for function in functions {
        let (pair, facts) = data_pair(function);
        rstyle_pdg::data_library::verify_documents(&pair, &facts).unwrap();
        if facts.name == "str_len" {
            check_length_document_tampering(&pair, &facts);
            check_length_frontend_tampering(&facts);
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
    }
    let (output, warm) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{warm}");
    assert_eq!(pair_decisions(&report), pair_decisions(&warm));
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
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
        let (pair, facts) = data_pair(function);
        rstyle_pdg::helper_calls::verify_documents(&pair, &facts).unwrap();
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
    assert_eq!(report["pair_evaluation_complete"], true);
    check_shared_helper_dependency(&report);
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
    let (_, facts) = data_pair(wrapper);
    assert!(rstyle_pdg::helper_calls::native_width(&facts).unwrap());
    for corruption in ["manifest", "source", "signature", "effects"] {
        let mut changed = facts.clone();
        assert!(corrupt_helper(&mut changed.body, corruption));
        assert!(rstyle_pdg::producer::produce(&changed).is_err());
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
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
        let (pair, facts) = data_pair(function);
        check_shared_result_origins(&pair, &facts);
        if facts.name == "swapped" {
            check_shared_result_tampering(&pair, &facts);
        }
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 45);
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
    assert_ne!(before["document_manifest"], after["document_manifest"]);
    assert_ne!(before["facts"]["body"], after["facts"]["body"]);
    let (pair, _) = data_pair(&after);
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
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 45);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_shared_result_origins(pair: &rstyle_pdg::producer::Pair, facts: &rstyle_pdg::HirFnFacts) {
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
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
    assert_eq!(report["pair_evaluation_complete"], true);
    let (output, old) = clones(directory.path(), &["--extraction-profile", "hir-data-v1"]);
    assert!(output.status.success(), "{old}");
    assert_eq!(old["matching"]["profile_id"], "rstyle.ccgraph.data");
    assert_eq!(old["extraction_policy"]["version"], "1.0.0");
    assert_eq!(old["matching"]["pairs"].as_array().unwrap().len(), 6);
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
    assert_eq!(report["extraction_policy"]["version"], "3.0.0");
    assert_eq!(
        report["matching"]["profile_id"],
        "rstyle.ccgraph.data-calls"
    );
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted", "{function}");
        let (pair, facts) = data_pair(function);
        rstyle_pdg::helper_calls::verify_documents(&pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
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
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted", "{function}");
        let (pair, facts) = data_pair(function);
        rstyle_pdg::helper_calls::verify_documents(&pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 210);
    assert_eq!(report["pair_evaluation_complete"], true);
    check_inherent_call_evidence(&report);
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
    for name in ["model::Data::wrap", "model::Data::third", "invoke"] {
        let old = read_call_function(previous, name);
        let new = read_call_function(&report, name);
        assert_ne!(old["document_manifest"], new["document_manifest"]);
        let (pair, _) = data_pair(new);
        assert_eq!(
            pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
                [2]["name"],
            "other"
        );
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 210);
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

fn check_inherent_call_evidence(report: &Value) {
    let (pair, facts) = data_pair(read_call_function(report, "model::Data::wrap"));
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
    let function = read_call_function(report, "model::Data::wrap");
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
    let (direct, _) = data_pair(read_call_function(report, "ufcs"));
    let direct = direct
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
        .unwrap();
    assert_eq!(direct.original_kind.known_value(), Some("expr_call"));
}

fn check_inherent_tampering(function: &Value) {
    let (pair, facts) = data_pair(function);
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
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{change}");
        assert!(
            rstyle_pdg::helper_calls::verify_documents(&pair, &changed).is_err(),
            "{change}"
        );
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
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
        let (pair, facts) = data_pair(function);
        rstyle_pdg::helper_calls::verify_documents(&pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 190);
    assert_eq!(report["pair_evaluation_complete"], true);
    check_read_call_locations(&report);
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

fn check_read_call_locations(report: &Value) {
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
        let (pair, _) = data_pair(read_call_function(report, name));
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
    check_stored_read_path(read_call_function(report, "stored"));
}

fn check_stored_read_path(function: &Value) {
    let (stored, _) = data_pair(function);
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
    let (pair, facts) = data_pair(function);
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
    check_read_summary_tampering(&facts);
}

fn check_read_summary_tampering(facts: &rstyle_pdg::HirFnFacts) {
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
        let mut changed = facts.clone();
        assert!(tamper_read_summary(&mut changed.body, change));
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{change}");
    }
    for change in ["manifest", "source", "signature", "effects"] {
        let mut changed = facts.clone();
        assert!(corrupt_helper(&mut changed.body, change));
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{change}");
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

fn tamper_read_summary(node: &mut rstyle_pdg::HirNode, change: &str) -> bool {
    if node.kind != "expr_call" {
        return node
            .children
            .iter_mut()
            .any(|child| tamper_read_summary(child, change));
    }
    let resolution = node.res.as_mut().unwrap();
    let mut value: Value = serde_json::from_str(&resolution.description).unwrap();
    let summary = &mut value[rstyle_pdg::helper_calls::KEY]["summary"];
    match change {
        "missing" => {
            summary.as_object_mut().unwrap().remove("memory_reads");
        }
        "empty" => summary["memory_reads"] = serde_json::json!([]),
        "extra" => {
            let read = summary["memory_reads"][0].clone();
            summary["memory_reads"].as_array_mut().unwrap().push(read);
        }
        "operation" => summary["memory_reads"][0]["operation"] = serde_json::json!("forged"),
        "access" => summary["memory_reads"][0]["access"] = serde_json::json!(1),
        "parameter" => summary["memory_reads"][0]["parameter"] = serde_json::json!(1),
        "path" => summary["memory_reads"][0]["path"] = serde_json::json!([]),
        "type" => summary["memory_reads"][0]["value"]["value"] = serde_json::json!("u16"),
        "effect" => summary["memory_effects"] = serde_json::json!(false),
        "identity" => summary["identity"] = serde_json::json!("rstyle.helper.source-body/1"),
        _ => unreachable!(),
    }
    resolution.description = serde_json::to_string(&value).unwrap();
    true
}

fn check_read_call_edit(directory: &Path, previous: &Value) {
    let source = DATA_READ_CALL_FAMILY.replace(
        "pub fn get(v: &Data) -> u8 { v.inner.n }",
        "pub fn get(v: &Data) -> u8 { v.inner.other }",
    );
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory, &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    for name in ["wrap", "third"] {
        let old = read_call_function(previous, name);
        let new = read_call_function(&report, name);
        assert_ne!(old["document_manifest"], new["document_manifest"]);
        let (pair, _) = data_pair(new);
        assert_eq!(
            pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["path"]
                [2]["name"],
            "other"
        );
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 190);
    assert_eq!(report["pair_evaluation_complete"], true);
}

fn check_shared_result_tampering(
    pair: &rstyle_pdg::producer::Pair,
    facts: &rstyle_pdg::HirFnFacts,
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
        let mut changed = facts.clone();
        assert!(corrupt_helper(&mut changed.body, corruption));
        assert!(
            rstyle_pdg::producer::produce(&changed).is_err(),
            "{corruption}"
        );
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
    for change in ["missing", "artifact", "signature", "definition"] {
        let mut changed = facts.clone();
        match change {
            "missing" => {
                changed
                    .build
                    .frontend_options
                    .remove(rstyle_pdg::data_library::KEY);
            }
            "artifact" => {
                changed
                    .build
                    .frontend_options
                    .get_mut(rstyle_pdg::data_library::KEY)
                    .unwrap()["libraries"][0]["files"][0]["sha256"] =
                    serde_json::json!(format!("sha256:{}", "0".repeat(64)))
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
        let admitted = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Shadow::len" | "slice" | "text"
        );
        assert_eq!(
            function["outcome"],
            if admitted { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 3);
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
        "options.pop('data_library_artifacts')",
        "options['data_library_artifacts']['libraries'][0]['files'][0]['sha256'] = 'sha256:' + '0' * 64",
        "options['data_library_artifacts']['libraries'][0]['files'][0]['bytes'] = 128 * 1024 * 1024 + 1",
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
    let (mut pair, facts) = data_pair(function);
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
    facts: &rstyle_pdg::HirFnFacts,
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
    let (pair, facts) = data_pair(function);
    rstyle_pdg::producer::verify(&pair, &facts).unwrap();
    let strict = rstyle_pdg::producer::produce(&facts)
        .unwrap()
        .serialize()
        .unwrap();
    let original = pair.serialize().unwrap();
    assert_eq!(strict.pdg_bytes, original.pdg_bytes);
    assert_eq!(strict.cfg_bytes, original.cfg_bytes);
    assert_eq!(strict.manifest, original.manifest);
    for corruption in ["manifest", "source", "signature", "effects"] {
        let mut changed = facts.clone();
        assert!(corrupt_helper(&mut changed.body, corruption));
        assert!(
            rstyle_pdg::producer::produce(&changed).is_err(),
            "{corruption}"
        );
    }
    rstyle_pdg::producer::verify(&pair, &facts).unwrap();
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
    edited.source.bytes = edited.source.bytes.replace("self.n", "self.n + 1");
    edited.source.digest = rstyle_pdg::concrete_types::digest(edited.source.bytes.as_bytes());
    assert!(edited.nominal_resolution().is_err());
    assert!(edited.inherent_definition().is_err());
    assert!(std::ptr::eq(&*first, &*view.nominal_resolution().unwrap()));
}
