use super::propagation::{checked, workspace};
use super::*;

const SOURCE: &str =
    "pub fn wide(a:u32,b:u32)->u32 {a+b}\n#[cfg(test)] pub fn test_only(a:u32,b:u32)->u32 {a-b}\n";

fn replay(directory: &Path, original: &Value, format: &str) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args(["--format", format])
        .args(["--replay-run", original["run_directory"].as_str().unwrap()])
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

fn assert_contexts(text: &str, original: &Value) {
    let contexts = original["unit_contexts"].as_array().unwrap();
    let mut populations = std::collections::BTreeSet::new();
    for context in contexts {
        let id = context["id"].as_u64().unwrap();
        let unit = &original["observed_units"][context["observed_unit"].as_u64().unwrap() as usize];
        let target = &unit["target"];
        let test = unit["test"].as_bool().unwrap();
        populations.insert((target["name"].as_str().unwrap(), test));
        let header = format!(
            "compiler unit {id}: package={} target={} kind=lib test={test} source={}",
            target["package"].as_str().unwrap(),
            target["name"].as_str().unwrap(),
            target["source"].as_str().unwrap(),
        );
        assert!(text.lines().any(|line| line == header), "{header}\n{text}");
        let suffix = format!("[compiler unit {id}]");
        let diagnostics: Vec<_> = text
            .lines()
            .filter(|line| line.contains("fn_params:") && line.ends_with(&suffix))
            .collect();
        assert_eq!(diagnostics.len(), if test { 2 } else { 1 });
        assert_eq!(
            diagnostics.iter().any(|line| line.contains("test_only")),
            test
        );
        let scores = text
            .lines()
            .filter(|line| line.contains("src/lib.rs:") && !line.contains("fn_params:"))
            .filter(|line| line.ends_with(&suffix))
            .count();
        assert_eq!(scores, if test { 2 } else { 1 });
    }
    assert_eq!(
        populations,
        std::collections::BTreeSet::from([
            ("caller", false),
            ("caller", true),
            ("leaf", false),
            ("leaf", true),
        ])
    );
}

#[test]
fn root_style_text_labels_same_source_diagnostics_and_scores_by_saved_compilation_unit() {
    let directory = workspace(SOURCE);
    std::fs::write(directory.path().join("caller/src/lib.rs"), SOURCE).unwrap();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let output = rstyle(directory.path(), &["--all-targets", "--format", "json"]);
    checked(&output, 1);
    let original: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(original["diagnostics"].as_array().unwrap().len(), 6);
    assert_eq!(original["functions"].as_array().unwrap().len(), 6);
    let output = replay(directory.path(), &original, "text");
    checked(&output, 1);
    let before = stdout(&output);
    assert_contexts(&before, &original);

    for package in ["caller", "leaf"] {
        std::fs::remove_dir_all(directory.path().join(package).join("src")).unwrap();
        std::fs::remove_file(directory.path().join(package).join("Cargo.toml")).unwrap();
    }
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let output = replay(directory.path(), &original, "text");
    checked(&output, 1);
    assert_eq!(stdout(&output), before);
    let output = replay(directory.path(), &original, "json");
    checked(&output, 1);
    let saved: Value = serde_json::from_slice(&output.stdout).unwrap();
    for field in [
        "diagnostics",
        "diagnostic_units",
        "functions",
        "function_units",
        "compiler_advisory",
    ] {
        assert_eq!(saved[field], original[field], "{field}");
    }
}

#[test]
fn clean_root_style_text_keeps_context_without_inventing_diagnostic_or_score_rows() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn clean() {}\n").unwrap();
    let output = rstyle(directory.path(), &[]);
    checked(&output, 0);
    let text = stdout(&output);
    assert!(text.contains("compiler unit 0: package="));
    assert!(text.contains("kind=lib test=false source="));
    assert!(!text.contains("[compiler unit"));
    assert!(!text.contains("CC  COG"));
    assert!(text.contains("rstyle: 1 functions, 0 violations"));
}
