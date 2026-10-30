use super::*;
use std::os::unix::fs::PermissionsExt;

const SHIM: &str = r#"#!/bin/sh
if [ "$1" = metadata ]; then
    printf '%s\n' "$RUSTUP_TOOLCHAIN" "$@" >> "$METADATA_TEST_LOG"
fi
exec "$METADATA_TEST_CARGO" "$@"
"#;

struct Fixture {
    directory: TempDir,
    path: std::ffi::OsString,
}

impl Fixture {
    fn new() -> Self {
        let directory = super::propagation::workspace("pub fn leaf() {}\n");
        let bin = directory.path().join("tools");
        std::fs::create_dir(&bin).unwrap();
        std::fs::write(bin.join("cargo"), SHIM).unwrap();
        std::fs::set_permissions(bin.join("cargo"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        let inherited = std::env::var_os("PATH").unwrap();
        let path =
            std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&inherited)))
                .unwrap();
        Self { directory, path }
    }

    fn run(&self, extra: &[&str]) -> Output {
        std::fs::write(self.directory.path().join("metadata.log"), "").unwrap();
        diagnostic_process::output(
            Command::new(cargo_bin("cargo-rstyle"))
                .args([
                    "clones",
                    "--format",
                    "json",
                    "--extraction-profile",
                    "hir-data-v3",
                    "--compiler-advisory-only",
                    "--compiler-advisory",
                    "--compiler-diagnostic-scope",
                    "authored",
                    "--compiler-diagnostic-rules",
                    "fn_params",
                    "--manifest-path",
                ])
                .arg(self.directory.path().join("Cargo.toml"))
                .args(extra)
                .current_dir(self.directory.path())
                .env("PATH", &self.path)
                .env("METADATA_TEST_CARGO", env!("CARGO"))
                .env(
                    "METADATA_TEST_LOG",
                    self.directory.path().join("metadata.log"),
                )
                .env("RSTYLE_DRIVER", driver())
                .env_remove("CARGO_TARGET_DIR"),
        )
        .unwrap()
    }

    fn metadata_invocation(&self) {
        let log = std::fs::read_to_string(self.directory.path().join("metadata.log")).unwrap();
        let lines: Vec<_> = log.lines().collect();
        assert_eq!(
            lines.iter().filter(|line| **line == "metadata").count(),
            1,
            "Capture must acquire metadata once: {log}"
        );
        let pinned: toml::Value =
            toml::from_str(include_str!("../../../../rust-toolchain.toml")).unwrap();
        assert_eq!(lines[0], pinned["toolchain"]["channel"].as_str().unwrap());
        assert_eq!(
            &lines[1..6],
            [
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--manifest-path"
            ]
        );
        assert_eq!(
            Path::new(lines[6]),
            self.directory.path().join("Cargo.toml")
        );
    }
}

fn plan(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["capture_complete"], true);
    let directory = Path::new(report["run_directory"].as_str().unwrap());
    serde_json::from_slice(&std::fs::read(directory.join("plan.json")).unwrap()).unwrap()
}

#[test]
fn metadata_is_acquired_once_for_selected_package_and_all_target_contexts() {
    let fixture = Fixture::new();
    let root = fixture.directory.path().join("leaf");
    std::fs::create_dir(root.join("tests")).unwrap();
    std::fs::write(root.join("tests/probe.rs"), "pub fn sample() {}\n").unwrap();
    let ordinary = plan(&fixture.run(&["-p", "leaf"]));
    fixture.metadata_invocation();
    assert_eq!(ordinary["targets"].as_array().unwrap().len(), 1);
    assert_eq!(ordinary["targets"][0]["name"], "leaf");
    assert_eq!(ordinary["targets"][0]["package_context"]["name"], "leaf");
    let all = plan(&fixture.run(&["-p", "leaf", "--all-targets"]));
    fixture.metadata_invocation();
    let names: std::collections::BTreeSet<_> = all["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|target| target["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, std::collections::BTreeSet::from(["leaf", "probe"]));
}

#[test]
fn metadata_reuse_preserves_unknown_package_and_cargo_failure_outcomes() {
    let fixture = Fixture::new();
    let unknown = fixture.run(&["-p", "absent"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown package absent"));
    fixture.metadata_invocation();
    std::fs::write(
        fixture.directory.path().join("Cargo.toml"),
        "invalid manifest",
    )
    .unwrap();
    let invalid = fixture.run(&[]);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("Cargo.toml"));
    fixture.metadata_invocation();
}
