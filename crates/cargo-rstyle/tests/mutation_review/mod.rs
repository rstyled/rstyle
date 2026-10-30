use super::*;
mod live;
mod status;

const ORIGINAL: &str = "pub fn many() -> u32 { let mut first=0u32; let mut second=0u32; first=1; first=2; first=3; second=1; second=2; second=3; first + second }\n";
const CLEAN: &str = "pub fn many() -> u32 { let mut first=0u32; first=1; first }\n";
const OPAQUE: &str =
    "pub fn many(callback:fn()) { let mut first=0u32; first=1; first=2; first=3; callback(); }\n";
const REASON: &str = "Keep these explicit scalar writes.";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    for (path, bytes) in [
        ("src/lib.rs", source),
        (
            "analysis.toml",
            "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=100\n",
        ),
    ] {
        std::fs::write(directory.path().join(path), bytes).unwrap();
    }
    policy(directory.path(), 2);
    directory
}

fn policy(directory: &Path, maximum: usize) {
    std::fs::write(
        directory.join("mutation.toml"),
        format!("[mutation_budget]\nmax_mutations_per_target={maximum}\nmax_mutated_targets=1\n"),
    )
    .unwrap();
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--mutation-rules",
        "--mutation-config",
        "mutation.toml",
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
    report["mutation_analysis"]["functions"]
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
    let review = &report["mutation_review"];
    assert_eq!(review["schema"], "rstyle.mutation-rule-review/1");
    assert_eq!(review["hidden_findings"], hidden);
    assert_eq!(review["visible_findings"], visible);
}

fn assert_raw_same(left: &Value, right: &Value) {
    for field in [
        "evidence",
        "rules",
        "coverage",
        "execution",
        "availability",
        "outcome",
        "density",
        "policy",
    ] {
        assert_eq!(function(left)[field], function(right)[field], "raw {field}");
    }
    assert_eq!(
        left["mutation_analysis"]["findings"],
        right["mutation_analysis"]["findings"]
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
                "--mutation-rules",
                "--mutation-config",
                "mutation.toml",
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

    fn dismiss(&self, flag: &str) -> Value {
        let output = self.run(&[flag, key(self.original), "--review-reason", REASON]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        parsed(&output)
    }
}

fn retained(directory: &Path) -> Vec<u8> {
    std::fs::read(directory.join("reviews.json")).unwrap()
}
