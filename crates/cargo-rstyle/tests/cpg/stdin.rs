use super::*;
use std::{io::Write, process::Stdio};

fn sequence(documents: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for document in documents {
        bytes.push(0x1e);
        serde_json::to_writer(&mut bytes, document).unwrap();
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(b"\x1enull\n");
    bytes
}

fn command(route: &str, bytes: &[u8], extra: &[&str]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(cargo_bin("cargo-rstyle"))
        .args(["rstyle", "cpg", route, "--stdin"])
        .args(extra)
        .current_dir(directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let bytes = bytes.to_vec();
    let writer = std::thread::spawn(move || input.write_all(&bytes));
    let output = child.wait_with_output().unwrap();
    if let Err(error) = writer.join().unwrap() {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    output
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {output:?}"))
}

fn other() -> Value {
    let text = example()
        .to_string()
        .replace("example::", "other::")
        .replace("src/lib.rs", "other/lib.rs");
    let mut value: Value = serde_json::from_str(&text).unwrap();
    value["unit"]["id"] = json!("unit:other");
    value
}

#[test]
fn stdin_analysis_matches_files_without_creating_capture_files() {
    let documents = [example(), other()];
    let bytes = sequence(&documents);
    for route in ["check", "patterns"] {
        let extra = if route == "patterns" {
            vec!["--format", "json", "--complexity"]
        } else {
            vec!["--format", "json"]
        };
        let output = command(route, &bytes, &extra);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        let mut received = report(&output);
        let receipt = received
            .as_object_mut()
            .unwrap()
            .remove("capture_receipt")
            .unwrap();
        assert_eq!(receipt["schema"], "rstyle.cpg-stream-receipt/1");
        assert_eq!(receipt["status"], "accepted");
        assert_eq!(receipt["source"], "stdin");
        assert!(receipt.get("capture").is_none());
        assert!(receipt["transport"].get("manifest_checksum").is_none());
        assert_eq!(
            receipt["transport"],
            json!({
                "framing":"rfc7464", "encoding":"json", "received_documents":2,
                "received_bytes":bytes.len(), "complete":true,
            })
        );
        assert_eq!(receipt["validation"]["graphs"], route == "patterns");
        assert_eq!(receipt["validation"]["runtime_bounds"], route == "patterns");
        let explicit = if route == "check" {
            check(&documents, &extra)
        } else {
            mine(&documents, &extra)
        };
        assert_eq!(received, report(&explicit));
    }
}

#[test]
fn receive_returns_a_stream_receipt_without_style_evaluation() {
    let mut value = example();
    for position in 2..6 {
        value["unit"]["functions"][1]["signature"]["parameters"]
            .as_array_mut()
            .unwrap()
            .push(json!({"position":position,"symbol":"top.a","type":"type:u32"}));
    }
    let bytes = sequence(&[value]);
    let output = command(
        "receive",
        &bytes,
        &["--format", "json", "--cpg-compression", "zstd"],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let receipt = report(&output);
    assert_eq!(receipt["status"], "accepted");
    assert_eq!(receipt["transport"]["received_documents"], 1);
    assert!(receipt.get("summary").is_none());

    let output = command("check", &bytes, &["--format", "json"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let checked = report(&output);
    assert_eq!(checked["capture_receipt"]["status"], "accepted");
    assert!(checked["summary"]["violations"].as_u64().unwrap() > 0);
}

#[test]
fn stdin_rejects_a_valid_prefix_when_the_stream_is_unfinished_or_has_trailing_data() {
    let complete = sequence(&[example()]);
    let mut missing_terminal = complete.clone();
    missing_terminal.truncate(complete.len() - b"\x1enull\n".len());
    let mut trailing = complete.clone();
    trailing.extend_from_slice(b"\x1e{}");
    let mut bad_second = missing_terminal.clone();
    bad_second.extend_from_slice(b"\x1e{\"bad\":\"\xff\"}\n\x1enull\n");
    for bytes in [
        Vec::new(),
        b"\x1enull\n".to_vec(),
        missing_terminal,
        trailing,
        bad_second,
    ] {
        for route in ["receive", "check", "patterns"] {
            let output = command(route, &bytes, &["--format", "json"]);
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            let receipt = report(&output);
            assert_eq!(receipt["status"], "rejected");
            assert_eq!(receipt["transport"]["complete"], false);
            assert_eq!(receipt["failures"].as_array().unwrap().len(), 1);
            assert!(receipt.get("summary").is_none());
            assert!(receipt.get("evaluation").is_none());
        }
    }
}

#[test]
fn valid_json_does_not_bypass_profile_facts_or_duplicate_validation() {
    let mut future = other();
    future["profile"]["version"] = json!("99.0.0");
    let mut invalid = other();
    invalid["unit"]["files"][0]
        .as_object_mut()
        .unwrap()
        .remove("content");
    for second in [example(), future, invalid] {
        for route in ["receive", "check", "patterns"] {
            let output = command(
                route,
                &sequence(&[example(), second.clone()]),
                &["--format", "json"],
            );
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            let receipt = report(&output);
            assert_eq!(receipt["status"], "rejected");
            assert_eq!(receipt["transport"]["received_documents"], 2);
            assert_eq!(receipt["failures"].as_array().unwrap().len(), 1);
        }
    }
}

#[test]
fn split_stdin_documents_can_arrive_before_their_unit() {
    let mut documents = split(&example());
    documents.reverse();
    let output = command("check", &sequence(&documents), &["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(report(&output)["summary"]["functions"], 3);
    assert_eq!(
        report(&output)["capture_receipt"]["transport"]["complete"],
        true
    );
}

#[test]
fn completed_transport_does_not_accept_an_incomplete_split_cpg() {
    let mut documents = split(&example());
    documents.remove(0);
    for route in ["receive", "check", "patterns"] {
        let output = command(route, &sequence(&documents), &["--format", "json"]);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let receipt = report(&output);
        assert_eq!(receipt["status"], "rejected");
        assert_eq!(receipt["transport"]["complete"], true);
        assert!(
            receipt["failures"][0]["message"]
                .as_str()
                .unwrap()
                .contains("exactly one unit document, found 0")
        );
        assert!(receipt.get("summary").is_none());
        assert!(receipt.get("evaluation").is_none());
    }
}

#[test]
fn stdin_is_explicit_and_exclusive_with_disk_inputs() {
    for route in ["receive", "check", "patterns"] {
        for extra in [
            vec![],
            vec!["--stdin", "--capture", "capture"],
            vec!["--stdin", "document.json"],
        ] {
            let output = Command::new(cargo_bin("cargo-rstyle"))
                .args(["rstyle", "cpg", route])
                .args(extra)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            assert!(output.stdout.is_empty());
        }
    }
}

#[test]
fn text_receipts_identify_streams() {
    let bytes = sequence(&[example()]);
    let output = command("receive", &bytes, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(stdout(&output).contains("CPG stream accepted: 1 documents"));
    let output = command("check", &bytes, &[]);
    assert!(stdout(&output).contains("rstyle: 3 functions, 0 violations"));
    assert!(stdout(&output).contains("CPG stream accepted:"));
    let output = command("receive", b"", &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).contains("CPG stream rejected:"));
}
