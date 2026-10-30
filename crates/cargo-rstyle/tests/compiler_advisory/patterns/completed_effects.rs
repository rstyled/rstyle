use super::*;

const DIRECT: &str = r#"
use std::path::Path;
pub fn file(p: &Path) -> bool { p.is_file() && p.extension().is_some() }
pub fn absolute(p: &Path) -> bool { p.is_absolute() && p.extension().is_some() }
"#;

const TRANSITIVE: &str = r#"
use std::path::Path;
fn disk(p: &Path) -> bool { p.is_file() }
fn lexical(p: &Path) -> bool { p.is_absolute() }
pub fn file(p: &Path) -> bool { disk(p) && p.extension().is_some() }
pub fn absolute(p: &Path) -> bool { lexical(p) && p.extension().is_some() }
"#;

fn candidate(report: &Value) -> &Value {
    candidate_at(&proposals(report)["evaluation"], "file").expect("shared Path predicate proposal")
}

fn notes(report: &Value) -> Vec<&str> {
    candidate(report)["counter_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .filter(|note| note.starts_with("core-policy effect mismatch"))
        .collect()
}

fn metadata(patterns: &Value, complete: bool) {
    assert_eq!(
        patterns["effect_evidence"],
        serde_json::json!({
            "schema": "rstyle.compiler-pattern-effects/1",
            "basis": "completed_core_policy_effect_kinds",
            "policy": "compiler_advisory.effective_config",
            "metrics": "compiler_advisory.metrics",
            "context": "compiler_advisory units listed by member_units; per-function owners retained",
            "input_complete": complete,
            "review_identity": "structural; effect annotations recomputed under current policy"
        })
    );
}

fn same_proposal(before: &Value, after: &Value) {
    let mut expected = candidate(before).clone();
    let mut observed = candidate(after).clone();
    expected.as_object_mut().unwrap().remove("counter_evidence");
    observed.as_object_mut().unwrap().remove("counter_evidence");
    assert_eq!(expected, observed);
    assert_eq!(before["matching"], after["matching"]);
    assert_eq!(before["accepted_pairs"], after["accepted_pairs"]);
}

fn policy_replay(directory: &Path, original: &Value, policy: &str) -> Value {
    std::fs::write(directory.join("policy.toml"), policy).unwrap();
    let (output, report) = replay(
        directory,
        original,
        &["--compiler-advisory-config", "policy.toml"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    report
}

#[test]
fn configured_io_annotations_change_without_changing_the_existing_proposal() {
    let (directory, output, original) = capture(DIRECT, &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(candidate(&original)["kind"], "parameterize");
    metadata(proposals(&original), true);
    assert_eq!(notes(&original).len(), 1);
    assert!(notes(&original)[0].contains("io"));
    assert!(notes(&original)[0].contains("no recorded effects under this policy"));
    let unclassified = policy_replay(directory.path(), &original, "[side_effects.paths]\nio=[]\n");
    assert!(notes(&unclassified).is_empty());
    same_proposal(&original, &unclassified);
    let equal = DIRECT.replace("p.is_absolute()", "p.is_dir()");
    let (_directory, output, equal) = capture(&equal, &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(candidate(&equal)["kind"], "parameterize");
    assert!(notes(&equal).is_empty());
}

#[test]
fn inherited_effect_annotations_follow_depth_while_reviews_remain_structural() {
    let (directory, output, original) = capture(TRANSITIVE, &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(notes(&original).len(), 1);
    reviews::dismiss_candidate(directory.path(), &original, candidate(&original), 0);
    let zero = policy_replay(
        directory.path(),
        &original,
        "[side_effects]\ntransitive_depth=0\n",
    );
    let one = policy_replay(
        directory.path(),
        &original,
        "[side_effects]\ntransitive_depth=1\n",
    );
    assert!(notes(&zero).is_empty());
    assert_eq!(notes(&one).len(), 1);
    same_proposal(&zero, &one);
    std::fs::write(
        directory.path().join("policy.toml"),
        "[side_effects]\ntransitive_depth=0\n",
    )
    .unwrap();
    let (output, reviewed) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(
        reviewed["compiler_advisory"]["pattern_review"]["decisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["candidate_id"] == candidate(&one)["id"]
                && row["state"] == "hidden_exact")
    );
    assert!(notes(&reviewed).is_empty());
    same_proposal(&one, &reviewed);
}

#[test]
fn completed_effect_notes_replay_without_sources_and_stay_with_their_units() {
    let (directory, output, original) = capture(DIRECT, &["--compiler-patterns", "--all-targets"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(original["capture_failures"], serde_json::json!([]));
    assert_eq!(original["handoff_failures"], serde_json::json!([]));
    let units = original["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    for unit in units {
        let id = unit["unit"].as_u64().unwrap() as usize;
        let observed = original["unit_contexts"][id]["observed_unit"]
            .as_u64()
            .unwrap() as usize;
        let test = original["observed_units"][observed]["test"]
            .as_bool()
            .unwrap();
        metadata(&unit["patterns"], !test);
        assert_eq!(unit["patterns"]["input_complete"], !test);
        let unavailable = unit["unavailable"].as_array().unwrap();
        assert_eq!(unavailable.len(), usize::from(test));
        if test {
            // The compiler-generated test harness has no original function source.
            assert_eq!(
                unavailable[0]["reason"],
                "Existing compiler collector has no original function source"
            );
        }
    }
    assert_eq!(
        units[0]["patterns"]["evaluation"]["candidates"],
        units[1]["patterns"]["evaluation"]["candidates"]
    );
    assert_eq!(original["compiler_advisory"]["complete"], false);
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let (output, saved) = replay(directory.path(), &original, &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(original["compiler_advisory"], saved["compiler_advisory"]);
    assert_eq!(original["matching"], saved["matching"]);
}

#[test]
fn generated_workspace_callees_resolve_with_incomplete_effect_evidence() {
    let generated = TRANSITIVE.replace(
        "fn disk(p: &Path) -> bool { p.is_file() }",
        "macro_rules! define_disk { () => { fn disk(p: &Path) -> bool { p.is_file() } }; } define_disk!();",
    );
    let (_directory, output, report) = capture(&generated, &["--compiler-patterns"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    metadata(proposals(&report), false);
    assert!(
        report["compiler_advisory"]["units"][0]["unresolved_callees"]
            .as_array()
            .unwrap()
            .iter()
            .all(|callee| !callee.as_str().unwrap().ends_with("::disk"))
    );
    assert_eq!(report["compiler_advisory"]["complete"], false);
}
