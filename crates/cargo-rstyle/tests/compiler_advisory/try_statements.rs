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
    fixture_with_source(SOURCE)
}

fn fixture_with_source(source: &str) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
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
        "rstyle.compiler-style-metrics/7"
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
fn authored_statement_counts_and_direct_io_survive_source_free_replay() {
    let (directory, report) = fixture();
    for (name, statements, writes) in [
        ("one", 2, 1),
        ("multiple", 3, 2),
        ("matched", 4, 1),
        ("blocked", 4, 1),
        ("expanded", 2, 1),
    ] {
        check_sample(&report, name, statements, writes);
    }
    assert_eq!(
        report["compiler_advisory"]["units"][0]["evaluation"]["summary"]["effect_counts"]["io"],
        6
    );
    replay_statement_metrics(directory.path(), &report);
}

#[test]
fn let_else_macro_tails_count_once_without_counting_generated_match_arms() {
    let source = r#"
pub fn macro_tail(input: Option<u8>) -> std::io::Result<()> {
    let Some(_value) = input else { unreachable!() };
    std::fs::write("/unused", b"tail")?;
    Ok(())
}
pub fn macro_statement(input: Option<u8>) -> std::io::Result<()> {
    let Some(_value) = input else { unreachable!(); };
    std::fs::write("/unused", b"statement")?;
    Ok(())
}
pub fn macro_condition(input: Option<u8>) -> std::io::Result<()> {
    if !matches!(input, Some(_)) {}
    let Some(_value) = input else { unreachable!() };
    std::fs::write("/unused", b"condition")?;
    Ok(())
}
"#;
    let (_directory, report) = fixture_with_source(source);
    for (name, statements) in [
        ("macro_tail", 4),
        ("macro_statement", 4),
        ("macro_condition", 5),
    ] {
        check_sample(&report, name, statements, 1);
    }
}

fn replay_statement_metrics(directory: &Path, original: &Value) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    let (output, replayed) = clones(
        directory,
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
