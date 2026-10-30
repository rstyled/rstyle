use super::propagation::checked;
use super::*;

const BASE: &[&str] = &[
    "--compiler-advisory-only",
    "--compiler-plans",
    "--compiler-advisory-config",
    "policy.toml",
    "--compiler-diagnostic-scope",
    "authored",
];
const PARAMETERS: &str = r#"
pub fn left(a:u32,b:u32,c:u32,d:u32,e:u32)->u32 {a+b+c+d+e}
pub fn right(a:u32,b:u32,c:u32,d:u32,e:u32)->u32 {a+b+c+d+e}
"#;
const DENSITY: &str = r#"
pub fn left() {let _ = std::fs::read("/unused-left");}
pub fn right() {let _ = std::fs::read("/unused-right");}
"#;

fn fixture_directory(authored: &str, generated: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!(
            "{authored}\nmacro_rules! make {{()=>{{pub fn generated(){{{generated}}}}}}} make!();"
        ),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("policy.toml"),
        "[side_effects]\nmax_density_milli=1\nmin_lines=0\nmin_statements=0\ntransitive_depth=3\n",
    )
    .unwrap();
    directory
}

fn fixture(authored: &str, generated: &str, rule: &str) -> (TempDir, Value) {
    let directory = fixture_directory(authored, generated);
    let mut options = BASE.to_vec();
    options.extend_from_slice(&["--compiler-diagnostic-rules", rule]);
    let (output, report) = super::propagation::capture(directory.path(), &options);
    checked(&output, 0);
    (directory, report)
}

fn planning(report: &Value) -> &Value {
    &report["compiler_advisory"]["planning"]
}

fn groups(report: &Value) -> &[Value] {
    planning(report)["groups"].as_array().unwrap()
}

fn key(report: &Value, name: &str) -> Value {
    let unit = &report["compiler_advisory"]["units"][0];
    unit["observations"]
        .as_array()
        .unwrap()
        .iter()
        .chain(unit["generated_effects"].as_array().unwrap())
        .find(|function| function["name"].as_str().unwrap().ends_with(name))
        .unwrap()["key"]
        .clone()
}

fn source_gaps(report: &Value) {
    let unit = &report["compiler_advisory"]["units"][0];
    assert_eq!(unit["observations"].as_array().unwrap().len(), 2);
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), 1);
    assert_eq!(unit["unavailable"][0]["key"], key(report, "generated"));
    assert_eq!(planning(report)["known_functions"], 2);
    assert_eq!(planning(report)["known_call_graph_functions"], 3);
    assert_eq!(planning(report)["call_graph_complete"], true);
    assert_eq!(planning(report)["input_complete"], false);
}

fn authored_members(report: &Value, group: &Value, count: usize) {
    let generated = key(report, "generated");
    assert_eq!(group["members"].as_array().unwrap().len(), count);
    assert_eq!(
        group["evaluation"]["functions"].as_array().unwrap().len(),
        count
    );
    for field in ["members", "step_owners"] {
        assert!(!group[field].as_array().unwrap().contains(&generated));
    }
    for owners in group["bundle_owners"].as_array().unwrap() {
        assert!(!owners.as_array().unwrap().contains(&generated));
    }
    assert!(
        group["evaluation"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|function| !function["name"].as_str().unwrap().ends_with("generated"))
    );
}

fn replay(directory: &Path, original: &Value, rule: &str) -> Value {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(BASE)
        .args(["--compiler-diagnostic-rules", rule])
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 0);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn replay_without_sources(directory: &Path, original: &Value, rule: &str) {
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    assert_eq!(
        replay(directory, original, rule)["compiler_advisory"],
        original["compiler_advisory"]
    );
}

#[test]
fn common_generated_parameter_caller_joins_authored_edits_without_inventing_generated_steps() {
    let (directory, report) = fixture(
        PARAMETERS,
        "left(1,2,3,4,5); right(1,2,3,4,5);",
        "fn_params",
    );
    source_gaps(&report);
    assert_eq!(planning(&report)["required_edits_complete"], false);
    assert_eq!(groups(&report).len(), 1);
    let group = &groups(&report)[0];
    authored_members(&report, group, 2);
    for name in ["left", "right"] {
        assert!(
            group["members"]
                .as_array()
                .unwrap()
                .contains(&key(&report, name))
        );
    }
    assert_eq!(
        group["unavailable_owners"],
        serde_json::json!([key(&report, "generated")])
    );
    assert_eq!(group["step_owners"].as_array().unwrap().len(), 2);
    let steps = group["evaluation"]["plan"]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2);
    assert!(steps.iter().all(|step| step["incoming_sites"] == 1));
    replay_without_sources(directory.path(), &report, "fn_params");
}

#[test]
fn single_target_and_disconnected_generated_callers_do_not_join_unrelated_parameter_groups() {
    for (body, unavailable) in [("left(1,2,3,4,5);", 1), ("let _ = 7u32;", 0)] {
        let (_directory, report) = fixture(PARAMETERS, body, "fn_params");
        source_gaps(&report);
        assert_eq!(
            planning(&report)["required_edits_complete"],
            unavailable == 0
        );
        assert_eq!(groups(&report).len(), 2);
        let mut recorded = Vec::new();
        for group in groups(&report) {
            authored_members(&report, group, 1);
            recorded.extend(
                group["unavailable_owners"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .cloned(),
            );
        }
        assert_eq!(recorded.len(), unavailable);
        if unavailable == 1 {
            assert_eq!(recorded, vec![key(&report, "generated")]);
        }
    }
}

#[test]
fn generated_shared_caller_is_a_density_hint_without_becoming_a_required_local_edit() {
    let (directory, report) = fixture(DENSITY, "left(); right();", "side_effect_density");
    source_gaps(&report);
    assert_eq!(planning(&report)["required_edits_complete"], true);
    assert_eq!(groups(&report).len(), 2);
    for group in groups(&report) {
        authored_members(&report, group, 1);
        assert_eq!(group["unavailable_owners"], serde_json::json!([]));
        assert_eq!(group["step_owners"], serde_json::json!([]));
        assert!(group["evaluation"].get("plan").is_none());
        assert!(
            group["evaluation"]["hints"]
                .as_array()
                .unwrap()
                .iter()
                .any(|hint| {
                    hint["kind"] == "shared_caller"
                        && hint["item"].as_str().unwrap().ends_with("generated")
                        && hint["other_groups"] == 1
                })
        );
    }
    replay_without_sources(directory.path(), &report, "side_effect_density");
    density_caller_footprint(directory.path(), &report);
}

fn density_caller_footprint(directory: &Path, original: &Value) {
    std::fs::write(
        directory.join("policy.toml"),
        "[side_effects]\nmax_density_milli=1\nmin_lines=0\nmin_statements=0\ntransitive_depth=3\n[grouping]\ncaller_rules=['side_effect_density']\ncaller_hops=1\n",
    )
    .unwrap();
    let report = replay(directory, original, "side_effect_density");
    source_gaps(&report);
    assert_eq!(planning(&report)["required_edits_complete"], false);
    assert_eq!(groups(&report).len(), 1);
    let group = &groups(&report)[0];
    authored_members(&report, group, 2);
    assert_eq!(
        group["unavailable_owners"],
        serde_json::json!([key(&report, "generated")])
    );
    assert_eq!(group["step_owners"], serde_json::json!([]));
    assert!(group["evaluation"].get("plan").is_none());
    assert_eq!(
        replay(directory, &report, "side_effect_density")["compiler_advisory"],
        report["compiler_advisory"]
    );
}

#[test]
fn identical_generated_callers_keep_authored_plans_in_their_compilation_contexts() {
    let directory = fixture_directory(PARAMETERS, "left(1,2,3,4,5); right(1,2,3,4,5);");
    let mut options = BASE.to_vec();
    options.extend_from_slice(&["--compiler-diagnostic-rules", "fn_params", "--all-targets"]);
    let (output, report) = super::propagation::capture(directory.path(), &options);
    checked(&output, 0);
    assert_eq!(groups(&report).len(), 2);
    assert_eq!(planning(&report)["required_edits_complete"], false);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    let mut contexts = std::collections::BTreeSet::new();
    for unit in units {
        context_group(&report, unit);
        let ordinal = unit["unit"].as_u64().unwrap() as usize;
        let observed = report["unit_contexts"][ordinal]["observed_unit"]
            .as_u64()
            .unwrap() as usize;
        contexts.insert(
            report["observed_units"][observed]["test"]
                .as_bool()
                .unwrap(),
        );
    }
    assert_eq!(contexts, [false, true].into_iter().collect());
}

fn context_group(report: &Value, unit: &Value) {
    let members: Vec<_> = unit["observations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            ["left", "right"]
                .iter()
                .any(|name| row["name"].as_str().unwrap().ends_with(name))
        })
        .map(|row| row["key"].clone())
        .collect();
    assert_eq!(members.len(), 2);
    let matching: Vec<_> = groups(report)
        .iter()
        .filter(|group| group["members"].as_array().unwrap().contains(&members[0]))
        .collect();
    assert_eq!(matching.len(), 1);
    let group = matching[0];
    assert_eq!(group["members"].as_array().unwrap().len(), 2);
    assert!(
        members
            .iter()
            .all(|key| group["members"].as_array().unwrap().contains(key))
    );
    let generated = unit["generated_effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"].as_str().unwrap().ends_with("generated"))
        .unwrap();
    assert_eq!(
        group["unavailable_owners"],
        serde_json::json!([generated["key"]])
    );
    assert_eq!(group["step_owners"].as_array().unwrap().len(), 2);
    assert!(
        group["step_owners"]
            .as_array()
            .unwrap()
            .iter()
            .all(|key| members.contains(key))
    );
}
