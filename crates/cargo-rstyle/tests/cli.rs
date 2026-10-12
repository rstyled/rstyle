//! End-to-end tests: real `cargo check` through the real rustc driver.

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
    Command::new(cargo_bin("cargo-rstyle"))
        .args(args)
        .current_dir(dir)
        .env("RSTYLE_DRIVER", driver())
        // Fixtures assert on `<fixture>/target/rstyle/facts`; a shared/inherited
        // CARGO_TARGET_DIR (worktree cache, CI volume) would move it.
        .env_remove("CARGO_TARGET_DIR")
        .output()
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
        "rm \"$RSTYLE_HIR_DOCUMENT_DIR/function-0.cfg.json\"",
        "printf '{}' > \"$RSTYLE_HIR_DOCUMENT_DIR/function-0.pdg.json\"",
        "printf '{\"documents\":[]}' > \"$RSTYLE_HIR_DOCUMENT_DIR/function-0.manifest.json\"",
        "printf ' ' >> \"$RSTYLE_HIR_DOCUMENT_DIR/function-0.cfg.json\"",
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
