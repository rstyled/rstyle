use super::*;
use rstyle_core::cpg::transport::{self, DocumentManifest, Encoding, SealedDocument};

fn capture(values: &[(Value, Encoding)]) -> (TempDir, Vec<SealedDocument>) {
    let dir = tempfile::tempdir().unwrap();
    let seals = values
        .iter()
        .enumerate()
        .map(|(i, (value, encoding))| {
            transport::write_json(
                &dir.path()
                    .join(format!("document-{i}{}", encoding.suffix())),
                *encoding,
                value,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    manifest(dir.path(), seals.clone());
    (dir, seals)
}

fn manifest(directory: &Path, seals: Vec<SealedDocument>) {
    std::fs::write(
        directory.join(transport::DOCUMENTS_MANIFEST),
        serde_json::to_vec(&DocumentManifest::of(seals)).unwrap(),
    )
    .unwrap();
}

fn command(route: &str, directory: &Path, extra: &[&str]) -> Output {
    Command::new(cargo_bin("cargo-rstyle"))
        .args(["rstyle", "cpg", route, "--format", "json", "--capture"])
        .arg(directory)
        .args(extra)
        .output()
        .unwrap()
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {output:?}"))
}

fn other() -> Value {
    let text = example()
        .to_string()
        .replace("example::", "another::")
        .replace("src/lib.rs", "another/lib.rs");
    let mut value: Value = serde_json::from_str(&text).unwrap();
    value["unit"]["id"] = json!("unit:another");
    value
}

#[test]
fn capture_directory_reports_match_explicit_files_and_include_an_acceptance_receipt() {
    let (dir, seals) = capture(&[(example(), Encoding::Zstd), (other(), Encoding::Json)]);
    std::fs::write(
        dir.path().join("unlisted.cpg.json.zst"),
        b"not part of the capture",
    )
    .unwrap();
    for route in ["check", "patterns"] {
        let extra = if route == "patterns" {
            vec!["--complexity"]
        } else {
            vec![]
        };
        let output = command(route, dir.path(), &extra);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let mut received = report(&output);
        let receipt = received
            .as_object_mut()
            .unwrap()
            .remove("capture_receipt")
            .unwrap();
        assert_eq!(receipt["status"], "accepted");
        assert_eq!(receipt["transport"]["verified_documents"], 2);
        assert_eq!(receipt["validation"]["graphs"], route == "patterns");
        assert_eq!(receipt["validation"]["runtime_bounds"], route == "patterns");
        let mut explicit = Command::new(cargo_bin("cargo-rstyle"));
        explicit
            .args(["rstyle", "cpg", route, "--format", "json"])
            .args(extra);
        explicit.args(seals.iter().map(|seal| dir.path().join(&seal.file)));
        let explicit = explicit.output().unwrap();
        assert_eq!(explicit.status.code(), Some(0), "{explicit:?}");
        assert_eq!(received, report(&explicit));
    }
}

#[test]
fn receiving_a_capture_reports_exact_verified_totals_without_running_analysis() {
    let (dir, seals) = capture(&[(example(), Encoding::Json), (other(), Encoding::Zstd)]);
    let output = command("receive", dir.path(), &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let receipt = report(&output);
    assert_eq!(receipt["schema"], "rstyle.cpg-receipt/1");
    assert_eq!(receipt["status"], "accepted");
    assert_eq!(receipt["failures"], json!([]));
    assert_eq!(receipt["transport"]["expected_documents"], 2);
    assert_eq!(receipt["transport"]["verified_documents"], 2);
    assert_eq!(
        receipt["transport"]["stored_bytes"],
        seals.iter().map(|s| s.bytes).sum::<u64>()
    );
    assert_eq!(
        receipt["transport"]["decoded_bytes"],
        seals.iter().map(|s| s.decoded_bytes.unwrap()).sum::<u64>()
    );
    assert!(receipt.get("summary").is_none());
}

#[test]
fn incomplete_captures_return_a_structured_rejection() {
    let dir = tempfile::tempdir().unwrap();
    for empty in [false, true] {
        if empty {
            manifest(dir.path(), vec![]);
        }
        let output = command("receive", dir.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let receipt = report(&output);
        assert_eq!(receipt["status"], "rejected");
        assert!(receipt["transport"].is_null());
        assert_eq!(receipt["failures"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn a_late_transport_failure_reports_only_successfully_verified_documents() {
    for missing in [false, true] {
        let (dir, seals) = capture(&[(example(), Encoding::Json), (other(), Encoding::Zstd)]);
        let path = dir.path().join(&seals[1].file);
        if missing {
            std::fs::remove_file(path).unwrap();
        } else {
            std::fs::write(path, b"truncated").unwrap();
        }
        for route in ["receive", "check", "patterns"] {
            let output = command(route, dir.path(), &[]);
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            let receipt = report(&output);
            assert_eq!(receipt["status"], "rejected");
            assert_eq!(receipt["transport"]["verified_documents"], 1);
            assert_eq!(receipt["transport"]["stored_bytes"], seals[0].bytes);
            assert!(
                receipt["failures"][0]["message"]
                    .as_str()
                    .unwrap()
                    .contains(&seals[1].file)
            );
            assert!(receipt.get("summary").is_none());
        }
    }
}

#[test]
fn verified_bytes_do_not_accept_an_unsupported_profile_or_duplicate_function() {
    let mut future = other();
    future["profile"]["version"] = json!("99.0.0");
    for invalid in [future, example()] {
        let (dir, _) = capture(&[(example(), Encoding::Zstd), (invalid, Encoding::Json)]);
        let output = command("receive", dir.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let receipt = report(&output);
        assert_eq!(receipt["status"], "rejected");
        assert_eq!(receipt["transport"]["verified_documents"], 2);
        let message = receipt["failures"][0]["message"].as_str().unwrap();
        assert!(
            message.contains("this rstyle reads") || message.contains("appears more than once"),
            "{message}"
        );
    }
}

#[test]
fn split_capture_functions_can_precede_their_unit() {
    let mut values = split(&example());
    values.reverse();
    let values = values
        .into_iter()
        .map(|value| (value, Encoding::Zstd))
        .collect::<Vec<_>>();
    let (dir, _) = capture(&values);
    let output = command("check", dir.path(), &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report = report(&output);
    assert_eq!(report["summary"]["functions"], 3);
    assert_eq!(report["capture_receipt"]["status"], "accepted");
}

#[test]
fn rule_findings_keep_a_valid_capture_accepted() {
    let mut value = example();
    let parameters = value["unit"]["functions"][1]["signature"]["parameters"]
        .as_array_mut()
        .unwrap();
    for position in 2..6 {
        parameters.push(json!({"position":position,"symbol":"top.a","type":"type:u32"}));
    }
    let (dir, _) = capture(&[(value, Encoding::Zstd)]);
    let output = command("check", dir.path(), &[]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let report = report(&output);
    assert_eq!(report["capture_receipt"]["status"], "accepted");
    assert!(report["summary"]["violations"].as_u64().unwrap() > 0);
}

#[test]
fn capture_and_explicit_documents_cannot_be_mixed() {
    let (dir, seals) = capture(&[(example(), Encoding::Json)]);
    let path = dir.path().join(&seals[0].file);
    let output = command("check", dir.path(), &[path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot be used with"),
        "{output:?}"
    );
}

#[test]
fn a_verified_document_outside_the_model_is_named_by_every_route() {
    let mut extra = other();
    extra["unit"]["functions"][0]["nodes"][0]["role"] = json!("tail");
    let (dir, seals) = capture(&[(example(), Encoding::Zstd), (extra, Encoding::Json)]);
    let expected = format!(
        "{}: Invalid compat.cpg document: unknown field `role`",
        seals[1].file
    );
    for route in ["receive", "check", "patterns", "flow", "analyze"] {
        let output = command(route, dir.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let receipt = report(&output);
        assert_eq!(receipt["status"], "rejected");
        assert_eq!(receipt["transport"]["verified_documents"], 2);
        let message = receipt["failures"][0]["message"].as_str().unwrap();
        assert!(message.starts_with(&expected), "{message}");
        assert!(message.contains(" at line 1 column "), "{message}");
    }
}

#[test]
fn listed_documents_are_read_against_the_manifest_beside_them() {
    let (dir, seals) = capture(&[(example(), Encoding::Zstd), (other(), Encoding::Json)]);
    let text = example()
        .to_string()
        .replace("example::", "third::")
        .replace("src/lib.rs", "third/lib.rs");
    let mut third: Value = serde_json::from_str(&text).unwrap();
    third["unit"]["id"] = json!("unit:third");
    let plain = tempfile::tempdir().unwrap();
    let unsealed = plain.path().join("third.cpg.json");
    std::fs::write(&unsealed, serde_json::to_vec(&third).unwrap()).unwrap();
    let check = |paths: &[PathBuf]| {
        Command::new(cargo_bin("cargo-rstyle"))
            .args(["rstyle", "cpg", "check", "--format", "json"])
            .args(paths)
            .output()
            .unwrap()
    };
    let sealed: Vec<PathBuf> = seals
        .iter()
        .map(|seal| dir.path().join(&seal.file))
        .collect();
    let mut both = sealed.clone();
    both.push(unsealed);
    let output = check(&both);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(report(&output)["summary"]["functions"], 9);

    let stray = dir.path().join("stray.cpg.json");
    std::fs::write(&stray, serde_json::to_vec(&third).unwrap()).unwrap();
    let output = check(&[sealed[0].clone(), stray.clone()]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let expected = format!("{} is not listed in its document manifest", stray.display());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&expected),
        "{output:?}"
    );

    std::fs::write(&sealed[1], serde_json::to_vec(&third).unwrap()).unwrap();
    let output = check(&sealed);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let expected = format!(
        "Captured document {} disagrees with its manifest",
        seals[1].file
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&expected),
        "{output:?}"
    );
}
