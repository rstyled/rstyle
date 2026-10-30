use super::*;

const NAME: &str = "external::Worker::many";

fn method_source(source: &str) -> String {
    format!(
        "pub struct Worker;\nimpl Worker {{\n{}\n}}\n",
        source.replace("fn many(", "fn many(&self, ")
    )
}

#[test]
fn external_method_length_review_replays_and_changed_measurements_are_stale() {
    let directory = workspace("pub mod external;\n");
    let source = directory.path().join("src/external.rs");
    std::fs::write(&source, method_source(ORIGINAL)).unwrap();
    let (output, original) = capture(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&original, 0, 1);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss(NAME);
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    let store: Value = serde_json::from_slice(&bytes).unwrap();
    let locator = &store["length_entries"][0]["finding"]["locator"];
    assert_eq!(locator["source"], "src/external.rs");
    assert_eq!(locator["qualified_name"], "crate::external::Worker::many");
    assert_eq!(locator["declaration_kind"], "method");
    assert_stale_length(directory.path(), &source);
    for path in ["src/external.rs", "src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let output = replay.run(&["--fail-on-findings"]);
    assert!(output.status.success());
    let report = parsed(&output);
    counts(&report, 1, 0);
    assert_eq!(report["verification_policy"], "strict-replay");
    assert_eq!(
        report["length_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(report["analysis"]["findings"], 1);
    assert_eq!(report["analysis"]["visible_findings"], 1);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

fn assert_stale_length(directory: &Path, source: &Path) {
    let changed = ORIGINAL.replace("    sum + 1", "    let next = sum + 2;\n    next + 1");
    std::fs::write(source, method_source(&changed)).unwrap();
    let (output, report) = capture(directory, &["--review-status", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&report, 0, 1);
    assert_eq!(report["length_review"]["decisions"][0]["state"], "stale");
    assert_eq!(report["review_status"]["records"][0]["state"], "stale");
}
