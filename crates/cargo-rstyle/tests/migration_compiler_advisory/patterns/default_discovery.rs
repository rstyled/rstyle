use super::mining_options::{checked, remove_sources};
use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)}\npub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";

pub(super) fn raw(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["patterns", "--format", "json"];
    args.extend_from_slice(extra);
    rstyle(directory, &args)
}

pub(super) fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(output),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn fixture(extra: &[&str]) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), HELPERS).unwrap();
    let output = raw(directory.path(), extra);
    checked(&output, 0);
    (directory, json(&output))
}

pub(super) fn saved(directory: &Path, original: &Value, command: &str, extra: &[&str]) -> Output {
    let mut process = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    process
        .current_dir(directory)
        .args([
            command,
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut process).unwrap()
}

fn candidate(report: &Value) -> &Value {
    &proposals(report)["evaluation"]["candidates"][0]
}

pub(super) fn dismiss(directory: &Path, original: &Value) -> Vec<u8> {
    let id = candidate(original)["id"].as_str().unwrap();
    let output = saved(
        directory,
        original,
        "ignore",
        &[id, "--reason", "Reviewed repository helper"],
    );
    checked(&output, 0);
    assert!(stdout(&output).contains(id));
    let bytes = std::fs::read(directory.join("rstyle-ignore.toml")).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("Reviewed repository helper"));
    bytes
}

#[test]
fn default_patterns_uses_authored_compiler_discovery_and_matches_explicit_engine() {
    let (directory, original) = fixture(&[]);
    assert_eq!(original["matching_execution"], "not_requested");
    assert_eq!(proposals(&original)["source_scope"]["kind"], "authored");
    remove_sources(directory.path());
    let output = saved(
        directory.path(),
        &original,
        "patterns",
        &["--engine", "compiler", "--format", "json"],
    );
    checked(&output, 0);
    let explicit = json(&output);
    assert_eq!(explicit["compiler_advisory"], original["compiler_advisory"]);
    assert_eq!(explicit["capture_failures"], original["capture_failures"]);
    assert_eq!(explicit["handoff_failures"], original["handoff_failures"]);
}

#[test]
fn default_ignore_dismisses_saved_default_candidates_without_source_or_tools() {
    let (directory, original) = fixture(&[]);
    remove_sources(directory.path());
    let bytes = dismiss(directory.path(), &original);
    let status = saved(directory.path(), &original, "ignore", &["--status"]);
    checked(&status, 0);
    for expected in ["active", "first", "second", "src/lib.rs:1", "src/lib.rs:2"] {
        assert!(stdout(&status).contains(expected), "{}", stdout(&status));
    }
    let output = saved(
        directory.path(),
        &original,
        "patterns",
        &["--format", "json", "--fail-on-candidates"],
    );
    checked(&output, 0);
    let reviewed = json(&output);
    assert_eq!(
        proposals(&reviewed)["evaluation"]["candidates"],
        proposals(&original)["evaluation"]["candidates"]
    );
    assert_eq!(proposals(&reviewed)["evaluation"]["ignored_count"], 1);
    assert_eq!(
        proposals(&reviewed)["evaluation"]["ignored"][0]["reason"],
        "Reviewed repository helper"
    );
    assert_eq!(
        std::fs::read(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

#[test]
fn default_ignore_status_retains_live_locations_when_members_stop_forming_a_candidate() {
    let (directory, original) = fixture(&[]);
    let bytes = dismiss(directory.path(), &original);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "\n\npub fn first(a:u128)->u128 {a}\npub fn second(a:u128)->u128 {a}\n",
    )
    .unwrap();
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let changed = json(&output);
    assert_eq!(
        proposals(&changed)["evaluation"]["candidates"],
        serde_json::json!([])
    );
    remove_sources(directory.path());
    let status = saved(directory.path(), &changed, "ignore", &["--status"]);
    checked(&status, 0);
    let text = stdout(&status);
    for expected in [
        "stale",
        "first",
        "second",
        "src/lib.rs:3",
        "src/lib.rs:4",
        "body changed",
    ] {
        assert!(text.contains(expected), "{text}");
    }
    assert!(!text.contains("gone (renamed or deleted)"), "{text}");
    assert_eq!(
        std::fs::read(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

fn stale_entry(directory: &Path, original: &Value) -> Vec<u8> {
    let mut bytes = dismiss(directory, original);
    bytes.extend_from_slice(b"\n# Preserve this stale review until pruning\n[ignore.deadbeef0000]\nkind=\"generic_fn\"\nfns=[\"missing::gone\"]\nreason=\"Deleted helper\"\n");
    std::fs::write(directory.join("rstyle-ignore.toml"), &bytes).unwrap();
    bytes
}

#[test]
fn default_ignore_prune_requires_complete_workspace_and_preserves_live_reviews() {
    let (directory, original) = fixture(&["--all-targets"]);
    let bytes = stale_entry(directory.path(), &original);
    remove_sources(directory.path());
    for flags in [
        vec!["--prune", "--all-targets"],
        vec!["--prune", "--all-targets", "-p", "clone_fixture"],
        vec!["--prune", "--compiler-pattern-scope", "captured"],
    ] {
        let output = saved(directory.path(), &original, "ignore", &flags);
        checked(&output, 2);
        assert_eq!(
            std::fs::read(directory.path().join("rstyle-ignore.toml")).unwrap(),
            bytes
        );
    }
    let dry = saved(
        directory.path(),
        &original,
        "ignore",
        &["--prune", "--dry-run"],
    );
    checked(&dry, 0);
    assert!(stdout(&dry).contains("would remove deadbeef0000"));
    assert_eq!(
        std::fs::read(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
    let pruned = saved(directory.path(), &original, "ignore", &["--prune"]);
    checked(&pruned, 0);
    assert!(stdout(&pruned).contains("removed deadbeef0000"));
    let kept = std::fs::read_to_string(directory.path().join("rstyle-ignore.toml")).unwrap();
    assert!(!kept.contains("[ignore.deadbeef0000]"));
    assert!(kept.contains(candidate(&original)["id"].as_str().unwrap()));
    assert!(kept.contains("Reviewed repository helper"));
}

#[test]
fn default_workspace_ignore_reuses_cross_package_members_on_saved_replay() {
    let directory = propagation::workspace(
        "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)}",
    );
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}",
    )
    .unwrap();
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = json(&output);
    let sites = candidate(&original)["sites"].as_array().unwrap();
    assert_eq!(sites.len(), 2);
    assert!(sites.iter().any(|site| site["path"] == "leaf/src/lib.rs"));
    assert!(sites.iter().any(|site| site["path"] == "caller/src/lib.rs"));
    std::fs::remove_dir_all(directory.path().join("leaf")).unwrap();
    std::fs::remove_dir_all(directory.path().join("caller")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let bytes = dismiss(directory.path(), &original);
    let status = saved(directory.path(), &original, "ignore", &["--status"]);
    checked(&status, 0);
    assert!(stdout(&status).contains("leaf/src/lib.rs:1"));
    assert!(stdout(&status).contains("caller/src/lib.rs:1"));
    assert!(stdout(&status).contains("active"));
    let hidden = saved(
        directory.path(),
        &original,
        "patterns",
        &["--format", "json", "--fail-on-candidates"],
    );
    checked(&hidden, 0);
    assert_eq!(proposals(&json(&hidden))["evaluation"]["ignored_count"], 1);
    assert_eq!(
        std::fs::read(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

#[test]
fn saved_partial_selection_cannot_prune_or_infer_deletion_from_unobserved_members() {
    for selection in [vec![], vec!["-p", "clone_fixture", "--all-targets"]] {
        let (directory, original) = fixture(&selection);
        let bytes = stale_entry(directory.path(), &original);
        remove_sources(directory.path());
        let status = saved(directory.path(), &original, "ignore", &["--status"]);
        checked(&status, 0);
        let text = stdout(&status);
        assert!(text.contains("unassessed"), "{text}");
        assert!(text.contains("missing::gone"), "{text}");
        assert!(
            text.contains("unobserved in this compilation population"),
            "{text}"
        );
        assert!(!text.contains("gone (renamed or deleted)"), "{text}");
        for mode in [vec!["--prune"], vec!["--prune", "--dry-run"]] {
            let output = saved(directory.path(), &original, "ignore", &mode);
            checked(&output, 2);
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("complete workspace capture made with --all-targets")
            );
            assert_eq!(
                std::fs::read(directory.path().join("rstyle-ignore.toml")).unwrap(),
                bytes
            );
        }
    }
}

#[test]
fn default_ignore_preserves_alternate_ledger_path_across_shared_configuration() {
    let (directory, original) = fixture(&[]);
    let bytes = dismiss(directory.path(), &original);
    std::fs::rename(
        directory.path().join("rstyle-ignore.toml"),
        directory.path().join("reviewed.toml"),
    )
    .unwrap();
    remove_sources(directory.path());
    let status = saved(
        directory.path(),
        &original,
        "ignore",
        &["--ignore-file", "reviewed.toml", "--status"],
    );
    checked(&status, 0);
    assert!(stdout(&status).contains("active"));
    let dismissed = saved(
        directory.path(),
        &original,
        "ignore",
        &[
            "--ignore-file",
            "reviewed.toml",
            candidate(&original)["id"].as_str().unwrap(),
            "--reason",
            "Reviewed repository helper",
        ],
    );
    checked(&dismissed, 0);
    assert!(!directory.path().join("rstyle-ignore.toml").exists());
    assert_eq!(
        std::fs::read(directory.path().join("reviewed.toml")).unwrap(),
        bytes
    );
}
