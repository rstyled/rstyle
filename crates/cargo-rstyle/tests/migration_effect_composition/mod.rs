use super::*;
mod limits;
mod live;

const REORDERED: &str = "fn leaf(target: &mut u32, delta: u32) { *target += delta; }\nfn wrapper(delta: u32, target: &mut u32) { leaf(target, delta); }\npub fn caller(first: &mut u32, second: &mut u32, delta: u32) { wrapper(delta, second); }\n";
const REPEATED: &str = "fn leaf(target: &mut u32) { *target += 1; }\nfn wrapper(target: &mut u32) { leaf(target); leaf(target); }\npub fn caller(target: &mut u32) { wrapper(target); wrapper(target); }\n";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec!["--extraction-profile", "hir-data-v3", "--mutation-effects"];
    arguments.extend_from_slice(extra);
    clones(directory, &arguments)
}

fn function<'a>(report: &'a Value, suffix: &str) -> &'a Value {
    let qualified = format!("::{suffix}");
    let matches = report["mutation_effects"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            let name = function["name"].as_str().unwrap();
            name == suffix || name.ends_with(&qualified)
        })
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "missing or ambiguous {suffix}");
    matches[0]
}

fn owner<'a>(report: &'a Value, unit: &Value, index: &Value) -> &'a Value {
    report["mutation_effects"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["unit"] == *unit && function["evidence"]["function_index"] == *index
        })
        .unwrap_or_else(|| panic!("missing original owner {unit}:{index}"))
}

fn summary<'a>(report: &'a Value, caller: &Value, event: &Value) -> &'a Value {
    report["mutation_effects"]["summaries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|summary| {
            summary["unit"] == caller["unit"] && summary["summary_index"] == event["summary_index"]
        })
        .unwrap_or_else(|| panic!("missing summary for {event}"))
}

fn slice<'a>(source: &'a str, span: &Value) -> &'a str {
    let start = span["start"].as_u64().unwrap() as usize;
    let end = span["end"].as_u64().unwrap() as usize;
    source
        .get(start..end)
        .unwrap_or_else(|| panic!("invalid span {span} in {source}"))
}

fn function_source<'a>(source: &'a str, function: &Value) -> &'a str {
    slice(source, &function["fn_span"])
}

fn complete(report: &Value, name: &str, visible: &str) {
    assert_eq!(
        report["mutation_effects"]["schema"],
        "rstyle.mutation-effects/2"
    );
    let evidence = &function(report, name)["evidence"];
    assert_eq!(evidence["coverage"], "complete", "{evidence}");
    assert_eq!(evidence["caller_visible_mutation"], visible, "{evidence}");
    assert!(
        evidence["unresolved_boundaries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn replay(directory: &Path, original: &Value, detail: &str) -> Output {
    diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory)
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--mutation-effects",
                "--report-detail",
                detail,
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RUSTC", directory.join("no-rustc"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}
