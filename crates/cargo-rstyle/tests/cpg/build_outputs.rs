//! What a build script writes is generated code, wherever the target directory is and
//! whichever crate it belongs to.

use super::*;

const BUILD: &str = r#"fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    let source = "pub fn included() {\n    std::fs::write(\"/unused\", b\"x\").ok();\n}\n";
    std::fs::write(std::path::Path::new(&out).join("included.rs"), source).unwrap();
}
"#;

fn write(path: PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A workspace whose `leaf` includes its build script's output, called in and across crates.
fn workspace() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"leaf\"]\nresolver = \"2\"\n",
    );
    write(
        root.join("leaf/Cargo.toml"),
        "[package]\nname = \"leaf\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n",
    );
    write(root.join("leaf/build.rs"), BUILD);
    write(
        root.join("leaf/src/lib.rs"),
        "include!(concat!(env!(\"OUT_DIR\"), \"/included.rs\"));\n\n\
         pub fn local_caller() {\n    included();\n}\n",
    );
    write(
        root.join("app/Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [dependencies]\nleaf = { path = \"../leaf\" }\n",
    );
    write(
        root.join("app/src/lib.rs"),
        "pub fn cross_caller() {\n    leaf::included();\n}\n",
    );
    dir
}

fn route(root: &Path, target: Option<&Path>, extra: &[&str]) -> (i32, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle"));
    command
        .args(["rstyle", "--format", "json"])
        .args(extra)
        .current_dir(root)
        .env("RSTYLE_DRIVER", driver())
        .env_remove("CARGO_TARGET_DIR");
    if let Some(target) = target {
        command.env("CARGO_TARGET_DIR", target);
    }
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {output:?}"));
    (output.status.code().unwrap(), report)
}

fn run_documents(report: &Value) -> Vec<String> {
    let mut documents = Vec::new();
    for entry in std::fs::read_dir(report["run_directory"].as_str().unwrap()).unwrap() {
        let path = entry.unwrap().path();
        if path.join("documents.manifest.json").exists() {
            documents.extend(documents_in(&path));
        }
    }
    documents
}

fn assert_effects_reach_both_callers(code: i32, report: &Value) {
    assert_eq!(code, 0, "{}", report["compiler_advisory"]["units"]);
    assert_eq!(report["summary_complete"], true);
    assert_eq!(report["summary"]["effect_counts"]["io"], 2, "{report}");
    for unit in report["compiler_advisory"]["units"].as_array().unwrap() {
        assert_eq!(unit["unresolved_callees"], serde_json::json!([]), "{unit}");
    }
}

#[test]
fn what_a_build_script_writes_reaches_callers_in_and_across_crates() {
    let dir = workspace();

    let (code, report) = route(dir.path(), None, &[]);

    assert_effects_reach_both_callers(code, &report);
    let leaf = report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["package_root"].as_str().unwrap().ends_with("leaf"))
        .unwrap();
    let generated = leaf["generated_effects"].as_array().unwrap();
    assert_eq!(generated.len(), 1, "{leaf}");
    assert_eq!(generated[0]["name"], "included");
    assert_eq!(generated[0]["generated_file"], true);
}

#[test]
fn a_target_directory_outside_the_workspace_changes_nothing() {
    let dir = workspace();
    let target = tempfile::tempdir().unwrap();
    let (_, inside) = route(dir.path(), None, &[]);

    let (code, outside) = route(dir.path(), Some(target.path()), &[]);

    assert_effects_reach_both_callers(code, &outside);
    assert_eq!(outside["summary"], inside["summary"]);
}

#[test]
fn no_rule_reports_what_a_build_script_writes() {
    let dir = workspace();
    write(
        dir.path().join("strict.toml"),
        "[strict_style.fn_length]\nmax_lines = 1\n[side_effects]\nmax_density_milli = 0\nmin_statements = 0\nmin_lines = 0\n",
    );

    let (code, report) = route(dir.path(), None, &["--config", "strict.toml"]);

    assert_eq!(code, 1, "{report}");
    let messages: Vec<&str> = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["message"].as_str().unwrap())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("cross_caller"))
    );
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("`included`")),
        "{messages:?}"
    );
}

#[test]
fn a_saved_run_and_cpg_check_agree_with_the_route() {
    let dir = workspace();
    let (_, report) = route(dir.path(), None, &[]);
    let run = report["run_directory"].as_str().unwrap().to_owned();

    let (code, replayed) = route(dir.path(), None, &["--replay-run", &run]);

    assert_effects_reach_both_callers(code, &replayed);
    assert_eq!(replayed["summary"], report["summary"]);
    let documents = run_documents(&report);
    assert_eq!(check_summary(dir.path(), &documents), report["summary"]);
}

#[test]
fn documents_mark_a_build_scripts_file_generated() {
    let dir = workspace();
    let (_, report) = route(dir.path(), None, &[]);

    let documents = parsed(&run_documents(&report));

    let files: Vec<&Value> = documents
        .iter()
        .flat_map(|document| document["unit"]["files"].as_array().unwrap())
        .collect();
    let generated: Vec<&&Value> = files
        .iter()
        .filter(|file| file["path"].as_str().unwrap().ends_with("out/included.rs"))
        .collect();
    assert!(!generated.is_empty());
    for file in generated {
        assert_eq!(file["origin"], "generated", "{}", file["path"]);
        assert_eq!(file["generated_by"], "build script");
    }
    for file in files {
        if file["path"].as_str().unwrap().ends_with("src/lib.rs") {
            assert_eq!(file["origin"], "source", "{}", file["path"]);
        }
    }
}

/// A workspace calling a dependency outside it, whose build output lands in its `target/`.
fn depending_on_outside() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    write(
        outside.join("Cargo.toml"),
        "[package]\nname = \"outside\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n[workspace]\n",
    );
    write(outside.join("build.rs"), BUILD);
    write(
        outside.join("src/lib.rs"),
        "include!(concat!(env!(\"OUT_DIR\"), \"/included.rs\"));\n",
    );
    let root = dir.path().join("root");
    write(
        root.join("Cargo.toml"),
        "[package]\nname = \"root\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [dependencies]\noutside = { path = \"../outside\" }\n\n[workspace]\n",
    );
    // A generated function makes the route resolve callees across units.
    write(
        root.join("src/lib.rs"),
        "#[derive(Clone)]\npub struct Marker;\n\n\
         pub fn caller() {\n    outside::included();\n}\n",
    );
    (dir, root)
}

#[test]
fn a_dependencys_build_output_under_target_is_still_a_library() {
    let (_dir, root) = depending_on_outside();

    let (code, report) = route(&root, None, &[]);

    assert_eq!(code, 0, "{}", report["compiler_advisory"]["units"]);
    assert_eq!(report["summary_complete"], true);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["unresolved_callees"], serde_json::json!([]));
}
