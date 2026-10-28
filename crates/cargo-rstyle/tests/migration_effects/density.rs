use super::*;

const POLICY: &str = "[side_effects]\nmax_density_milli=500\nmin_statements=0\nmin_lines=0\n";
const CHAIN: &str = "fn leaf(target:&mut u32){*target=1;*target=2;}\npub fn caller(target:&mut u32){*target=0;leaf(target);leaf(target);}\n";

fn workspace(source: &str, policy: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("effects.toml"), policy).unwrap();
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--effect-density",
        "--effect-config",
        "effects.toml",
    ];
    arguments.extend_from_slice(extra);
    clones(directory, &arguments)
}

fn density<'a>(report: &'a Value, name: &str) -> &'a Value {
    &super::function(report, name)["density"]
}

fn measured(report: &Value, name: &str, actual: u64, outcome: &str) {
    let density = density(report, name);
    assert_eq!(density["complete"], true, "{name}: {density}");
    assert_eq!(density["event_inventory"]["coverage"], "complete");
    assert_eq!(density["measurement"]["actual"], actual);
    assert_eq!(density["measurement"]["outcome"], outcome);
    assert_eq!(density["measurement"]["exempt"], false);
    assert!(
        density["event_inventory"]["unresolved_boundaries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn density_deduplicates_direct_roots_and_counts_each_delegated_call_occurrence() {
    let directory = workspace(CHAIN, POLICY);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let effects = &report["mutation_effects"];
    assert_eq!(effects["schema"], "rstyle.mutation-effects/4");
    assert_eq!(effects["scalar_mutations_requested"], false);
    assert_eq!(effects["density_requested"], true);
    assert_eq!(effects["findings"], 1);
    assert_eq!(effects["visible_findings"], 1);
    assert_eq!(effects["units"][0]["density_summaries_classified"], 1);
    measured(&report, "leaf", 500, "evaluated_no_finding");
    measured(&report, "caller", 1000, "finding");
    for (name, events, statements) in [("leaf", 1, 2), ("caller", 3, 3)] {
        let function = super::function(&report, name);
        let inventory = &function["density"]["event_inventory"];
        assert_eq!(inventory["counts"]["mutation"], events);
        assert_eq!(
            inventory["events"].as_array().unwrap().len(),
            events as usize
        );
        assert_eq!(
            function["statement_inventory"]["statements"]["value"],
            statements
        );
        assert!(inventory["events"].as_array().unwrap().iter().all(|event| {
            event["kind"] == "mutation"
                && event["span"]["start"].as_u64().unwrap() < event["span"]["end"].as_u64().unwrap()
        }));
    }
}

#[test]
fn classified_safe_external_events_complete_density_independently_of_scalar_effects() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    let directory = workspace("pub fn caller(){std::thread::yield_now();}\n", &policy);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    measured(&report, "caller", 1000, "finding");
    assert_eq!(
        density(&report, "caller")["event_inventory"]["counts"]["time"],
        1
    );
    assert_eq!(
        super::function(&report, "caller")["evidence"]["coverage"],
        "incomplete"
    );
    let abort = workspace("pub fn caller(){std::process::abort();}\n", POLICY);
    let (output, report) = requested(abort.path(), &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    measured(&report, "caller", 3000, "finding");
    assert_eq!(
        density(&report, "caller")["event_inventory"]["counts"]["io"],
        1
    );
}

#[test]
fn unknown_external_events_cannot_gain_ratios_or_minimum_exemptions_from_zero_weights() {
    let policy =
        "[side_effects]\nmin_statements=100\nmin_lines=100\n[side_effects.weights]\nunknown=0\n";
    let directory = workspace("pub fn caller(){std::thread::yield_now();}\n", policy);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let density = density(&report, "caller");
    assert_eq!(density["complete"], false);
    assert_eq!(density["measurement"]["outcome"], "not_evaluated");
    assert!(density["measurement"]["actual"].is_null());
    assert_eq!(density["measurement"]["exempt"], false);
    assert_eq!(density["visible_findings"], 0);
}

#[test]
fn combined_requests_require_both_axes_and_text_keeps_visible_density_findings() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    let directory = workspace("pub fn caller(){std::thread::yield_now();}", &policy);
    let (output, report) = requested(directory.path(), &["--mutation-effects"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["mutation_effects"]["coverage"], "incomplete");
    measured(&report, "caller", 1000, "finding");
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--effect-density",
            "--effect-config",
            "effects.toml",
            "--fail-on-findings",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("side_effect_density caller at"), "{text}");
    assert!(text.contains("1000 > 500 milli"), "{text}");
}

#[test]
fn nested_arguments_macros_unresolved_helpers_and_mutable_external_arguments_remain_incomplete() {
    let policy = format!(
        "{POLICY}\n[side_effects.weights]\nunknown=0\n[side_effects.paths]\ntime=['std::thread::sleep','std::mem::swap']\n"
    );
    for source in [
        "pub fn caller(){std::thread::sleep(std::time::Duration::from_secs(0));}",
        "macro_rules! opaque {()=>{()}} pub fn caller(){opaque!();}",
        "pub fn caller(callback:fn()){callback();}",
        "pub fn caller(a:&mut u32,b:&mut u32){std::mem::swap(a,b);}",
        "fn leaf(t:&mut u32){*t=1;} fn first(t:&mut u32){leaf(t);} fn second(t:&mut u32){first(t);} pub fn caller(t:&mut u32){second(t);}",
    ] {
        let directory = workspace(source, &policy);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{source}: {}",
            stdout(&output)
        );
        let density = density(&report, "caller");
        assert_eq!(density["complete"], false, "{source}: {density}");
        assert!(density["measurement"]["actual"].is_null(), "{source}");
        assert_eq!(density["measurement"]["outcome"], "not_evaluated");
        assert_eq!(density["measurement"]["exempt"], false);
    }
}

#[test]
fn named_density_suppression_preserves_the_raw_measurement() {
    let source = "// rstyle-allow(side_effect_density): reviewed state update\npub fn caller(target:&mut u32){*target=1;}\n";
    let directory = workspace(source, POLICY);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    measured(&report, "caller", 1000, "finding");
    let density = density(&report, "caller");
    assert_eq!(density["suppressed"], true);
    assert_eq!(density["source_suppression"]["status"], "allowed");
    assert_eq!(density["visible_findings"], 0);
    assert_eq!(report["mutation_effects"]["findings"], 1);
    assert_eq!(report["mutation_effects"]["visible_findings"], 0);
}

fn replay(directory: &Path, original: &Value, detail: &str) -> Output {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    diagnostic_process::output(
        Command::new(executable)
            .current_dir(directory)
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--effect-density",
                "--effect-config",
                "effects.toml",
                "--fail-on-findings",
                "--report-detail",
                detail,
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}

#[test]
fn density_replays_without_source_or_tools_and_recomputes_changed_policy() {
    let directory = workspace(CHAIN, POLICY);
    let (output, original) = requested(directory.path(), &[]);
    assert!(output.status.success());
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let output = replay(directory.path(), &original, detail);
        assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
        let current: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(current["verification_policy"], "strict-replay");
        assert_eq!(current["mutation_effects"], original["mutation_effects"]);
    }
    std::fs::write(
        directory.path().join("effects.toml"),
        POLICY.replace("max_density_milli=500", "max_density_milli=1000"),
    )
    .unwrap();
    let output = replay(directory.path(), &original, "compact");
    assert!(output.status.success(), "{}", stdout(&output));
    let current: Value = serde_json::from_slice(&output.stdout).unwrap();
    measured(&current, "caller", 1000, "evaluated_no_finding");
    assert_eq!(current["mutation_effects"]["visible_findings"], 0);
}

#[test]
fn density_budget_exhaustion_and_unsupported_configuration_cannot_become_clean() {
    let directory = workspace(CHAIN, POLICY);
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["mutation_effects"]["execution"], "budget_exhausted");
    for function in report["mutation_effects"]["functions"].as_array().unwrap() {
        assert_eq!(function["density"]["complete"], false);
        assert!(function["density"]["measurement"]["actual"].is_null());
    }
    for invalid in [
        "[side_effects]\ntransitive_depth=3\n",
        "[side_effects]\nmax_density_milis=500\n",
        "[side_effects.weights]\nunknown=-1\n",
        "exclude=['[']\n",
    ] {
        std::fs::write(directory.path().join("effects.toml"), invalid).unwrap();
        let output = rstyle(
            directory.path(),
            &[
                "clones",
                "--format",
                "json",
                "--effect-density",
                "--effect-config",
                "effects.toml",
            ],
        );
        assert!(
            !output.status.success(),
            "accepted invalid config: {invalid}"
        );
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn density_propagates_configured_local_call_kinds_without_duplicating_inner_occurrences() {
    let policy = format!("{POLICY}\n[side_effects.paths]\nio=['wrapper']\ntime=['leaf']\n");
    let source = "fn leaf(){} fn wrapper(){leaf();leaf();} pub fn caller(){wrapper();}";
    let directory = workspace(source, &policy);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    measured(&report, "leaf", 0, "evaluated_no_finding");
    measured(&report, "wrapper", 1000, "finding");
    measured(&report, "caller", 4000, "finding");
    assert_eq!(
        density(&report, "wrapper")["event_inventory"]["counts"]["time"],
        2
    );
    let kinds = &density(&report, "caller")["event_inventory"]["counts"];
    assert_eq!(kinds["time"], 1);
    assert_eq!(kinds["io"], 1);
}

#[test]
fn density_exclusions_preserve_unavailable_raw_evidence_and_suppression_does_not_complete_it() {
    let source = "// rstyle-allow(side_effect_density): reviewed call\npub fn caller(){std::thread::yield_now();}";
    let directory = workspace(source, POLICY);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(density(&report, "caller")["suppressed"], true);
    assert_eq!(density(&report, "caller")["complete"], false);
    std::fs::write(
        directory.path().join("effects.toml"),
        format!("exclude=['src/**']\n{POLICY}"),
    )
    .unwrap();
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let density = density(&report, "caller");
    assert_eq!(density["source_exclusion"]["status"], "excluded");
    assert_eq!(density["complete"], true);
    assert_eq!(density["measurement"]["outcome"], "not_evaluated");
    assert!(density["measurement"]["actual"].is_null());
}
