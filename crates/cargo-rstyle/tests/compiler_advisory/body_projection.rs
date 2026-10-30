use super::propagation::checked;
use super::*;

const SOURCE: &str = "pub fn wide(a:u32,b:u32,c:u32,d:u32,e:u32)->u32 { let mut total=a+b; if c>0 { total+=d; } total+e }\npub fn caller(value:u32)->u32 { wide(value,2,3,4,5) }\n";
const NUMERIC: &[&str] = &[
    "--compiler-advisory-only",
    "--compiler-plans",
    "--compiler-complexity-top",
    "10",
];
const LOCAL: &[&str] = &[
    "--source-rules",
    "--guard-clauses",
    "--guard-plans",
    "--mutation-rules",
    "--mutation-effects",
    "--effect-density",
    "--compiler-patterns",
];

fn fixture(extra: &[&str], code: i32) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) =
        super::propagation::capture(directory.path(), &[NUMERIC, extra].concat());
    checked(&output, code);
    assert_eq!(report["capture_complete"], true);
    (directory, report)
}

fn replay(directory: &Path, original: &Value, detail: &str, extra: &[&str]) -> (Output, Value) {
    super::reviews::replay(
        directory,
        original,
        &[NUMERIC, &["--report-detail", detail], extra].concat(),
    )
}

fn bundle(report: &Value) -> &Path {
    Path::new(
        report["functions"][0]["unit_evidence"]["bundle"]
            .as_str()
            .unwrap(),
    )
}

fn same_numeric(full: &Value, compact: &Value) {
    for field in [
        "compiler_advisory",
        "capture_complete",
        "handoff_failures",
        "strict_assessment_execution",
        "matching_execution",
    ] {
        assert_eq!(full[field], compact[field], "{field}");
    }
    let full_functions = full["functions"].as_array().unwrap();
    let compact_functions = compact["functions"].as_array().unwrap();
    assert_eq!(full_functions.len(), compact_functions.len());
    for (full, compact) in full_functions.iter().zip(compact_functions) {
        for field in [
            "key",
            "compilation_unit",
            "unit_evidence",
            "outcome",
            "reasons",
        ] {
            assert_eq!(full[field], compact[field], "{field}");
        }
        assert_eq!(full["facts"]["name"], compact["name"]);
        assert_eq!(full["facts"]["fn_span"], compact["fn_span"]);
    }
}

fn complete_original_bodies(full: &Value, wire: &Value) {
    assert_eq!(wire["schema"], "rstyle.diagnostic.raw-facts/4");
    assert_eq!(wire["phase"], "compiler-advisory-only");
    let originals = wire["facts"].as_array().unwrap();
    assert!(
        originals
            .iter()
            .any(|function| !function["bindings"].as_array().unwrap().is_empty())
    );
    for function in full["functions"].as_array().unwrap() {
        let facts = &function["facts"];
        let original = originals
            .iter()
            .find(|original| original["name"] == facts["name"])
            .unwrap();
        assert!(!original["body"]["children"].as_array().unwrap().is_empty());
        assert_eq!(facts["body"], original["body"]);
        assert_eq!(facts["bindings"], original["bindings"]);
    }
}

#[test]
fn compact_numeric_projection_preserves_full_snapshot_findings_plans_ranking_and_roots() {
    let (directory, original) = fixture(&[], 0);
    let bytes = std::fs::read(bundle(&original)).unwrap();
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, full) = replay(directory.path(), &original, "full", &[]);
    checked(&output, 0);
    let (output, compact) = replay(directory.path(), &original, "compact", &[]);
    checked(&output, 0);
    same_numeric(&full, &compact);
    assert_eq!(full["compiler_advisory"], original["compiler_advisory"]);
    let advisory = &compact["compiler_advisory"];
    assert_eq!(advisory["complete"], true);
    assert!(advisory["findings"].as_u64().unwrap() > 0);
    assert_eq!(advisory["planning"]["input_complete"], true);
    assert!(
        !advisory["planning"]["groups"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        advisory["units"][0]["complexity_ranking"]["input_complete"],
        true
    );
    assert_eq!(
        advisory["units"][0]["complexity_ranking"]["known_functions"],
        2
    );
    assert_eq!(
        advisory["units"][0]["complexity_ranking"]["owners"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        !advisory["units"][0]["complexity_ranking"]["functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    complete_original_bodies(&full, &wire);
    assert_eq!(std::fs::read(bundle(&original)).unwrap(), bytes);
}

#[test]
fn compact_local_consumers_and_patterns_keep_full_body_results() {
    let (directory, original) = fixture(LOCAL, 2);
    let (output, full) = replay(directory.path(), &original, "full", LOCAL);
    checked(&output, 2);
    let (output, compact) = replay(directory.path(), &original, "compact", LOCAL);
    checked(&output, 2);
    same_numeric(&full, &compact);
    for field in [
        "analysis",
        "mutation_analysis",
        "mutation_effects",
        "guard_planning",
        "edit_planning",
    ] {
        assert_eq!(full[field], compact[field], "{field}");
        assert!(!full[field].is_null(), "missing {field}");
    }
    assert_eq!(compact["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(compact["mutation_analysis"]["coverage"], "incomplete");
    assert_eq!(compact["mutation_effects"]["coverage"], "complete");
    let caller = compact["mutation_analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "caller")
        .unwrap();
    assert_eq!(caller["coverage"], "incomplete");
    assert!(
        caller["evidence"]["boundaries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["availability"] == "unknown")
    );
    let caller_effects = compact["mutation_effects"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "caller")
        .unwrap();
    assert_eq!(caller_effects["evidence"]["direct_coverage"], "incomplete");
    assert_eq!(caller_effects["evidence"]["coverage"], "complete");
    assert!(
        compact["compiler_advisory"]["units"][0]
            .get("patterns")
            .is_some()
    );
    let wire = serde_json::from_slice(&std::fs::read(bundle(&original)).unwrap()).unwrap();
    complete_original_bodies(&full, &wire);
}

#[test]
fn compact_and_full_reject_typed_body_unknown_fields_and_invalid_flat_child_rosters() {
    let (directory, original) = fixture(&[], 0);
    let wire: Value = serde_json::from_slice(&std::fs::read(bundle(&original)).unwrap()).unwrap();
    let ledger = only::ledger_path(&original);
    for attack in ["unknown", "flat"] {
        let mut changed = wire.clone();
        let body = &mut changed["facts"][0]["body"];
        if attack == "unknown" {
            body["children"][0]["unrecognized_typed_body_field"] = true.into();
        } else {
            let mut root = body.clone();
            root["children"] = serde_json::json!([]);
            *body = serde_json::json!([{"node":root,"children":1}]);
        }
        only::reseal_raw(&original, &ledger, &changed);
        for detail in ["compact", "full"] {
            let (output, report) = replay(directory.path(), &original, detail, &[]);
            checked(&output, 2);
            assert_eq!(report["capture_complete"], true, "{attack}/{detail}");
            assert!(
                !report["handoff_failures"].as_array().unwrap().is_empty(),
                "{attack}/{detail}"
            );
        }
    }
    only::reseal_raw(&original, &ledger, &wire);
    let (output, restored) = replay(directory.path(), &original, "compact", &[]);
    checked(&output, 0);
    assert_eq!(restored["compiler_advisory"], original["compiler_advisory"]);
}
