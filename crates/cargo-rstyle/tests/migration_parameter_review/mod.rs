use super::*;
mod live;
mod status;

const ORIGINAL: &str = "pub fn many(a: u32, b: u32) -> u32 { a + b }\n";
const REASON: &str = "Keep this public parameter boundary.";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    for (path, bytes) in [
        ("src/lib.rs", source),
        (
            "analysis.toml",
            "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\ncount_self=false\n",
        ),
        (
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"clone_fixture\"\nversion = \"0.1.0\"\n",
        ),
    ] {
        std::fs::write(directory.path().join(path), bytes).unwrap();
    }
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec![
        "--locked",
        "--extraction-profile",
        "hir-data-v3",
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
        "--review-file",
        "reviews.json",
    ];
    arguments.extend_from_slice(extra);
    clones(directory, &arguments)
}

fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn key<'a>(report: &'a Value, name: &str) -> &'a str {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap()["key"]
        .as_str()
        .unwrap()
}

fn counts(report: &Value, hidden: usize, visible: usize) {
    let review = &report["parameter_review"];
    assert_eq!(review["schema"], "rstyle.parameter-review/1");
    assert_eq!(review["hidden_findings"], hidden);
    assert_eq!(review["visible_findings"], visible);
    assert_eq!(review["findings"], hidden + visible);
}

struct Replay<'a> {
    directory: &'a Path,
    original: &'a Value,
}

impl Replay<'_> {
    fn command(&self, format: &str) -> Command {
        let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
        command
            .current_dir(self.directory)
            .args([
                "clones",
                "--format",
                format,
                "--extraction-profile",
                "hir-data-v3",
                "--review-file",
                "reviews.json",
                "--replay-run",
                self.original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", self.directory.join("no-tools"))
            .env("CARGO", self.directory.join("no-cargo"))
            .env("RUSTC", self.directory.join("no-rustc"))
            .env("RSTYLE_DRIVER", self.directory.join("no-driver"));
        command
    }

    fn run(&self, extra: &[&str]) -> Output {
        diagnostic_process::output(
            self.command("json")
                .args(["--source-rules", "--analysis-config", "analysis.toml"])
                .args(extra),
        )
        .unwrap()
    }

    fn dismiss(&self, name: &str) {
        let output = self.run(&[
            "--dismiss-parameter",
            key(self.original, name),
            "--review-reason",
            REASON,
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        counts(&parsed(&output), 1, 0);
    }
}
