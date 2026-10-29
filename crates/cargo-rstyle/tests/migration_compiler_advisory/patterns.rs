use super::*;

mod baseline;
mod reviews;

const ANIMALS: &str = include_str!("../fixtures/animals/src/lib.rs");

const FAMILIES: &str = r#"
pub struct Dog; pub struct Cat;
impl Dog { pub fn sound(&self)->String {"woof".into()} pub fn steps(&self)->u32 {4} }
impl Cat { pub fn noise(&self)->String {"meow".into()} pub fn stride(&self)->u32 {2} }
pub fn dogs(values:&[Dog],out:&mut Vec<String>)->u32 {let mut n=0; for value in values {out.push(value.sound()); n+=value.steps();} n}
pub fn cats(values:&[Cat],out:&mut Vec<String>)->u32 {let mut n=0; for value in values {out.push(value.noise()); n+=value.stride();} n}
pub fn inspect_dog(values:&[Dog]) { for value in values {std::hint::black_box(value); std::hint::black_box(value);} }
pub fn inspect_cat(values:&[Cat]) { for value in values {std::hint::black_box(value); std::hint::black_box(value);} }
pub fn tag_left(value:u128)->u128 {std::hint::black_box(value.wrapping_add(7)).wrapping_mul(3)}
pub fn tag_right(value:u128)->u128 {std::hint::black_box(value.wrapping_add(8)).wrapping_mul(3)}
"#;

fn proposals(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]["patterns"]
}

#[test]
fn compiler_proposer_reuses_capability_generic_and_helper_families() {
    let (_directory, output, report) =
        capture(FAMILIES, &["--compiler-patterns", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let evaluation = &proposals(&report)["evaluation"];
    assert_eq!(
        candidate_at(evaluation, "dogs").unwrap()["kind"],
        "capability_set"
    );
    assert_eq!(
        candidate_at(evaluation, "inspect_dog").unwrap()["kind"],
        "generic_fn"
    );
    assert_eq!(
        candidate_at(evaluation, "tag_left").unwrap()["kind"],
        "parameterize"
    );
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-patterns",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

#[test]
fn compiler_graphs_reuse_trait_dispatch_and_existing_trait_suppression_policy() {
    let (directory, _output, report) = capture(
        ANIMALS,
        &["--compiler-patterns", "--report-detail", "compact"],
    );
    assert_eq!(report["capture_failures"], serde_json::json!([]));
    assert_eq!(report["handoff_failures"], serde_json::json!([]));
    let patterns = proposals(&report);
    assert_eq!(patterns["input_complete"], true);
    let evaluation = &patterns["evaluation"];
    let dogs = candidate_at(evaluation, "emit_dogs").expect("trait proposal");
    assert_eq!(dogs["kind"], "trait_method");
    assert_eq!(site_names(dogs, "definitions"), ["Dog::bark", "Cat::meow"]);
    assert_eq!(
        candidate_at(evaluation, "pet_sound").unwrap()["kind"],
        "enum_dispatch"
    );
    assert!(candidate_at(evaluation, "emit_cow").is_none());
    for getter in ["Rock::label", "Pebble::tag", "Dog::name", "Cat::name"] {
        assert!(
            !evaluation["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| site_names(row, "sites")
                    .iter()
                    .chain(site_names(row, "definitions").iter())
                    .any(|name| name == getter))
        );
    }
    assert!(
        evaluation["suppressed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| site_names(row, "sites") == ["emit_cow", "emit_hen"])
    );
    assert_eq!(evaluation, &patterns_json(directory.path(), &[]));
    let raw = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "emit_dogs")
        .unwrap();
    assert_ne!(raw["outcome"], "admitted");
}

#[test]
fn optional_compiler_graphs_do_not_change_strict_pairs_or_propose_bare_forwarders() {
    let source = "pub fn first(a:u32)->u32{a} pub fn second(a:u32)->u32{a} pub fn forward_a(a:u32)->u32{first(a)} pub fn forward_b(a:u32)->u32{second(a)}";
    let (directory, output, original) = capture(source, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(!original["accepted_pairs"].as_array().unwrap().is_empty());
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--fail-on-findings",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(report["accepted_pairs"], original["accepted_pairs"]);
    assert_eq!(report["matching"], original["matching"]);
    assert_eq!(
        proposals(&report)["evaluation"]["candidates"],
        serde_json::json!([])
    );
}

#[test]
fn captured_proposals_replay_without_sources_and_preserve_strict_results() {
    let (directory, _output, original) = capture(ANIMALS, &["--compiler-patterns"]);
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, report) = replay(
        directory.path(),
        &original,
        &["--fail-on-findings", "--report-detail", "compact"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert_eq!(proposals(&report), proposals(&original));
    assert_eq!(report["accepted_pairs"], original["accepted_pairs"]);
    let text = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    assert!(text.status.success(), "{}", stdout(&text));
    assert!(stdout(&text).contains("Compiler patterns for"));
    assert!(stdout(&text).contains("emit_dogs"));
}

#[test]
fn graph_capture_is_opt_in_and_old_advisory_runs_remain_incomplete() {
    let (directory, output, original) = capture(GCD, &[]);
    assert!(output.status.success());
    assert!(original["compiler_advisory"]["units"][0]["patterns"].is_null());
    let (output, report) = replay(directory.path(), &original, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["compiler_advisory"]["complete"], false);
    let patterns = proposals(&report);
    assert_eq!(patterns["input_complete"], false);
    assert_eq!(patterns["observed_graphs"], 0);
    assert_eq!(patterns["unavailable_graphs"].as_array().unwrap().len(), 1);
    assert_eq!(patterns["evaluation"]["candidates"], serde_json::json!([]));
    assert_eq!(report["accepted_pairs"], original["accepted_pairs"]);
}

#[test]
fn known_empty_graphs_are_observed_and_generated_functions_stay_unavailable() {
    let (_directory, output, report) = capture("pub fn empty(){}", &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(proposals(&report)["input_complete"], true);
    assert_eq!(proposals(&report)["observed_graphs"], 1);
    assert_eq!(
        proposals(&report)["evaluation"]["candidates"],
        serde_json::json!([])
    );

    let source = "macro_rules! make {()=>{pub fn generated(){}}} make!(); pub fn original(){}";
    let (_directory, output, report) = capture(source, &["--compiler-patterns"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(proposals(&report)["input_complete"], false);
    assert_eq!(proposals(&report)["unavailable_functions"], 1);
    assert_eq!(proposals(&report)["observed_graphs"], 1);
}

#[test]
fn compiler_pattern_units_keep_check_and_test_populations_separate() {
    let source = "pub fn original(){} #[cfg(test)] pub fn test_only(){}";
    let (_directory, _output, report) = capture(source, &["--all-targets", "--compiler-patterns"]);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    let mut counts: Vec<_> = units
        .iter()
        .map(|unit| unit["patterns"]["observed_graphs"].as_u64().unwrap())
        .collect();
    counts.sort();
    assert_eq!(counts, [1, 2]);
}

#[test]
fn core_path_exclusions_apply_before_proposal_mining_and_keep_raw_observations() {
    let (directory, _output, original) = capture(ANIMALS, &["--compiler-patterns"]);
    std::fs::write(
        directory.path().join("exclude.toml"),
        "exclude=['src/**']\n",
    )
    .unwrap();
    let (_output, report) = replay(
        directory.path(),
        &original,
        &["--compiler-advisory-config", "exclude.toml"],
    );
    let patterns = proposals(&report);
    assert_eq!(patterns["input_complete"], true);
    assert_eq!(patterns["policy_selected_functions"], 0);
    assert_eq!(patterns["observed_graphs"], 0);
    assert_eq!(patterns["evaluation"]["candidates"], serde_json::json!([]));
    assert_eq!(
        report["compiler_advisory"]["units"][0]["observations"],
        original["compiler_advisory"]["units"][0]["observations"]
    );
}

#[test]
fn member_proposal_locations_use_workspace_source_root_separately_from_package_root() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nmembers=['member']\nresolver='2'\n",
    )
    .unwrap();
    std::fs::create_dir_all(directory.path().join("member/src")).unwrap();
    std::fs::write(
        directory.path().join("member/Cargo.toml"),
        "[package]\nname='member'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("member/src/lib.rs"), FAMILIES).unwrap();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let unit = &report["compiler_advisory"]["units"][0];
    assert_eq!(unit["source_root"], directory.path().to_str().unwrap());
    assert_eq!(
        unit["package_root"],
        directory.path().join("member").to_str().unwrap()
    );
    let helper = candidate_at(&unit["patterns"]["evaluation"], "tag_left").unwrap();
    assert!(
        helper["sites"]
            .as_array()
            .unwrap()
            .iter()
            .all(|site| site["path"] == "member/src/lib.rs")
    );
}

#[test]
fn compiler_helpers_report_intrinsic_effect_mismatch_without_changing_proposals() {
    let (_directory, _output, clean) = capture(FAMILIES, &["--compiler-patterns"]);
    let source = FAMILIES.replace(
        "std::hint::black_box(value.wrapping_add(7)).wrapping_mul(3)",
        "unsafe {std::hint::black_box(value.wrapping_add(7)).wrapping_mul(3)}",
    );
    let (directory, output, changed) = capture(&source, &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let before = candidate_at(&proposals(&clean)["evaluation"], "tag_left").unwrap();
    let after = candidate_at(&proposals(&changed)["evaluation"], "tag_left").unwrap();
    for field in [
        "id",
        "kind",
        "score_milli",
        "breakdown",
        "sites",
        "definitions",
    ] {
        assert_eq!(before[field], after[field], "{field}");
    }
    assert_eq!(before["counter_evidence"], serde_json::json!([]));
    let warning = after["counter_evidence"][0].as_str().unwrap();
    assert!(warning.contains("recorded effect mismatch"));
    assert!(warning.contains("unsafe"));
    assert!(warning.contains("no recorded intrinsic or escaping-mutation effects"));
    let (_output, replayed) = replay(directory.path(), &changed, &[]);
    assert_eq!(proposals(&changed), proposals(&replayed));
}
