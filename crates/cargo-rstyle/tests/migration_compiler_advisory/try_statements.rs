use super::*;

const SOURCE: &str = r#"
pub fn one() -> Result<(), String> {
    std::fs::write("/unused", b"one").map_err(|error| error.to_string())?;
    Ok(())
}
pub fn multiple() -> std::io::Result<()> {
    std::fs::write("/unused", b"first")?;
    std::fs::write("/unused", b"second")?;
    Ok(())
}
pub fn matched(flag: bool) -> std::io::Result<()> {
    match flag {
        true => std::fs::write("/unused", b"matched")?,
        false => (),
    }
    Ok(())
}
pub fn blocked() -> std::io::Result<()> {
    {
        let bytes = b"blocked";
        std::fs::write("/unused", bytes)
    }?;
    Ok(())
}
macro_rules! attempt { ($value:expr) => { $value? }; }
pub fn expanded() -> std::io::Result<()> {
    attempt!(std::fs::write("/unused", b"expanded"));
    Ok(())
}
"#;

fn fixture() -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    std::fs::write(
        directory.path().join("policy.toml"),
        "[side_effects]\nmin_lines=0\nmin_statements=0\nmax_density_milli=500\ntransitive_depth=0\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(
        report["compiler_advisory"]["metrics"],
        "rstyle.compiler-style-metrics/5"
    );
    assert_eq!(report["compiler_advisory"]["complete"], true);
    (directory, report)
}

fn check_sample(report: &Value, name: &str, statements: u64, writes: u64) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap();
    let facts =
        &function["facts"]["build"]["frontend_options"][rstyle_core::advisory::KEY]["facts"];
    assert_eq!(facts["statements"], statements, "{name}");
    let io = facts["calls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|call| call["callee"].as_str().unwrap().starts_with("std::fs::"))
        .count();
    assert_eq!(io as u64, writes, "{name}");
    let diagnostics = report["compiler_advisory"]["units"][0]["evaluation"]["diagnostics"]
        .as_array()
        .unwrap();
    let diagnostic = diagnostics
        .iter()
        .find(|row| row["rule_id"] == "side_effect_density" && row["line"] == facts["line"])
        .unwrap_or_else(|| panic!("Missing direct IO density finding for {name}"));
    assert_eq!(diagnostic["actual"], 3000 * writes / statements, "{name}");
    assert_eq!(diagnostic["limit"], 500);
}

#[test]
fn question_marks_do_not_add_statements_or_erase_direct_io_calls() {
    let (_directory, report) = fixture();
    check_sample(&report, "one", 2, 1);
    check_sample(&report, "multiple", 3, 2);
}

#[test]
fn authored_match_arms_operand_blocks_and_macro_io_remain_counted() {
    let (_directory, report) = fixture();
    check_sample(&report, "matched", 4, 1);
    check_sample(&report, "blocked", 4, 1);
    check_sample(&report, "expanded", 2, 1);
    assert_eq!(
        report["compiler_advisory"]["units"][0]["evaluation"]["summary"]["effect_counts"]["io"],
        6
    );
}

#[test]
fn corrected_statement_metrics_replay_without_source_or_manifest() {
    let (directory, original) = fixture();
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let (output, replayed) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-config",
            "policy.toml",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--max-pairs",
            "0",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(original["compiler_advisory"], replayed["compiler_advisory"]);
    assert_eq!(original["matching"], replayed["matching"]);
}

#[test]
fn legacy_counts_stay_frozen_while_advisory_preserves_the_same_events() {
    let (directory, report) = fixture();
    let output = rstyle(directory.path(), &["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let legacy = facts_for(directory.path(), "clone_fixture-lib.json");
    for (name, statements) in [("one", 4), ("multiple", 7), ("matched", 6), ("blocked", 6)] {
        let old = fact(&legacy, name);
        assert_eq!(old["statements"], statements, "{name}");
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["facts"]["name"] == name)
            .unwrap();
        let new =
            &function["facts"]["build"]["frontend_options"][rstyle_core::advisory::KEY]["facts"];
        for field in [
            "calls",
            "effects",
            "edges",
            "mutations",
            "mutated_types",
            "param_list",
            "sig",
        ] {
            assert_eq!(old[field], new[field], "{name}: {field}");
        }
    }
}
