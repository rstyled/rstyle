use super::*;
mod live;
mod status;

const ORIGINAL: &str = "pub fn many(target:&mut u32){*target=1;}\n";
const CLEAN: &str = "pub fn many(target:&mut u32){}\n";
const UNKNOWN: &str = "pub fn many(target:&mut u32){std::thread::yield_now();}\n";
const POLICY: &str = "[side_effects]\nmax_density_milli=100\nmin_statements=0\nmin_lines=0\n";
const REASON: &str = "Keep this explicit caller-visible write.";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    policy(directory.path(), POLICY);
    directory
}

fn policy(directory: &Path, policy: &str) {
    std::fs::write(directory.join("effects.toml"), policy).unwrap();
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--effect-density",
        "--effect-config",
        "effects.toml",
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

fn function(report: &Value) -> &Value {
    report["mutation_effects"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == "many")
        .unwrap()
}

fn key(report: &Value) -> &str {
    function(report)["key"].as_str().unwrap()
}

fn counts(report: &Value, hidden: usize, visible: usize) {
    assert_eq!(
        report["density_review"]["schema"],
        "rstyle.density-review/1"
    );
    assert_eq!(report["density_review"]["hidden_findings"], hidden);
    assert_eq!(report["density_review"]["visible_findings"], visible);
}

fn raw_same(left: &Value, right: &Value) {
    for field in ["density", "evidence", "statement_inventory"] {
        assert_eq!(function(left)[field], function(right)[field], "raw {field}");
    }
    assert_eq!(
        left["mutation_effects"]["findings"],
        right["mutation_effects"]["findings"]
    );
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
                "--effect-density",
                "--effect-config",
                "effects.toml",
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
        diagnostic_process::output(self.command("json").args(extra)).unwrap()
    }

    fn refused(&self, extra: &[&str]) {
        assert_eq!(self.run(extra).status.code(), Some(2));
        assert!(!self.directory.join("reviews.json").exists());
    }

    fn dismiss(&self) -> Value {
        let output = self.run(&[
            "--dismiss-density",
            key(self.original),
            "--review-reason",
            REASON,
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        parsed(&output)
    }

    fn omitted(&self) -> Value {
        let output = diagnostic_process::output(
            Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
                .current_dir(self.directory)
                .args([
                    "clones",
                    "--format",
                    "json",
                    "--extraction-profile",
                    "hir-data-v3",
                    "--source-rules",
                    "--review-file",
                    "reviews.json",
                    "--review-prune",
                    "--replay-run",
                    self.original["run_directory"].as_str().unwrap(),
                ]),
        )
        .unwrap();
        parsed(&output)
    }
}

fn retained(directory: &Path) -> Vec<u8> {
    std::fs::read(directory.join("reviews.json")).unwrap()
}
