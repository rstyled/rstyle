use super::*;
use std::{
    io::{PipeWriter, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, Stdio},
};

struct Invocation {
    child: Child,
    input: PipeWriter,
    number: i32,
    directory: TempDir,
}

impl Invocation {
    fn new(route: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let (reader, input) = std::io::pipe().unwrap();
        let number = reader.as_raw_fd();
        let mut command = Command::new(cargo_bin("cargo-rstyle"));
        command
            .args([
                "rstyle",
                "cpg",
                route,
                "--input-fd",
                &number.to_string(),
                "--format",
                "json",
            ])
            .current_dir(directory.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        inherit(&mut command, number);
        let child = command.spawn().unwrap();
        drop(reader);
        Self {
            child,
            input,
            number,
            directory,
        }
    }

    fn run(mut self, bytes: Vec<u8>) -> (Output, i32) {
        let writer = std::thread::spawn(move || self.input.write_all(&bytes));
        let output = self.child.wait_with_output().unwrap();
        if let Err(error) = writer.join().unwrap() {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
        assert_eq!(std::fs::read_dir(self.directory.path()).unwrap().count(), 0);
        (output, self.number)
    }
}

fn inherit(command: &mut Command, number: i32) {
    // SAFETY: the reader stays owned until spawn returns. Only the child changes
    // its descriptor flags, and fcntl is async-signal-safe in the pre-exec hook.
    unsafe {
        command.pre_exec(move || match libc::fcntl(number, libc::F_SETFD, 0) {
            -1 => Err(std::io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
}

fn sequence() -> Vec<u8> {
    let mut bytes = vec![0x1e];
    serde_json::to_writer(&mut bytes, &example()).unwrap();
    bytes.extend_from_slice(b"\n\x1enull\n");
    bytes
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {output:?}"))
}

#[test]
fn inherited_pipe_routes_share_stdin_analysis_and_identify_the_selected_descriptor() {
    let bytes = sequence();
    for route in ["receive", "check", "patterns"] {
        let (output, number) = Invocation::new(route).run(bytes.clone());
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        let mut received = report(&output);
        let receipt = if route == "receive" {
            &mut received
        } else {
            received.get_mut("capture_receipt").unwrap()
        };
        assert_eq!(receipt["schema"], "rstyle.cpg-stream-receipt/1");
        assert_eq!(receipt["status"], "accepted");
        assert_eq!(receipt["source"], "fd");
        assert_eq!(receipt["input_fd"], number);
        assert_eq!(receipt["transport"]["received_bytes"], bytes.len());
        assert_eq!(receipt["transport"]["complete"], true);
        receipt.as_object_mut().unwrap().remove("input_fd");
        receipt["source"] = json!("stdin");
        let expected = super::stdin::command(route, &bytes, &["--format", "json"]);
        assert_eq!(received, report(&expected));
    }
}

#[test]
fn a_closed_descriptor_returns_only_a_rejected_receipt_on_stdout() {
    for route in ["receive", "check", "patterns"] {
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "rstyle",
                "cpg",
                route,
                "--input-fd",
                "2147483647",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let receipt = report(&output);
        assert_eq!(receipt["status"], "rejected");
        assert_eq!(receipt["source"], "fd");
        assert_eq!(receipt["input_fd"], i32::MAX);
        assert!(receipt["transport"].is_null());
        assert!(
            receipt["failures"][0]["message"]
                .as_str()
                .unwrap()
                .contains("Cannot duplicate input fd")
        );
    }
}

#[test]
fn a_write_only_descriptor_rejects_input_and_preserves_the_report_channel() {
    for route in ["receive", "check", "patterns"] {
        for number in ["1", "2"] {
            let output = Command::new(cargo_bin("cargo-rstyle"))
                .args([
                    "rstyle",
                    "cpg",
                    route,
                    "--input-fd",
                    number,
                    "--format",
                    "json",
                ])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            let receipt = report(&output);
            assert_eq!(receipt["status"], "rejected");
            assert_eq!(receipt["source"], "fd");
            assert_eq!(receipt["transport"]["received_documents"], 0);
            assert_eq!(receipt["transport"]["complete"], false);
            assert!(receipt.get("summary").is_none());
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
fn incomplete_descriptor_streams_do_not_accept_a_valid_document_prefix() {
    let mut bytes = sequence();
    bytes.truncate(bytes.len() - b"\x1enull\n".len());
    for route in ["receive", "check", "patterns"] {
        let (output, number) = Invocation::new(route).run(bytes.clone());
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let receipt = report(&output);
        assert_eq!(receipt["status"], "rejected");
        assert_eq!(receipt["source"], "fd");
        assert_eq!(receipt["input_fd"], number);
        assert_eq!(receipt["transport"]["received_documents"], 1);
        assert_eq!(receipt["transport"]["complete"], false);
        assert!(receipt.get("summary").is_none());
    }
}

#[test]
fn descriptor_zero_can_be_selected_explicitly() {
    let output = assert_cmd::Command::new(cargo_bin("cargo-rstyle"))
        .args([
            "rstyle",
            "cpg",
            "receive",
            "--input-fd",
            "0",
            "--format",
            "json",
        ])
        .write_stdin(sequence())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let receipt = report(&output);
    assert_eq!(receipt["status"], "accepted");
    assert_eq!(receipt["source"], "fd");
    assert_eq!(receipt["input_fd"], 0);
}
