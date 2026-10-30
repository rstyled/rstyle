use super::*;

fn case(id: &str) -> Value {
    let labels: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/capability-cases.json"
    ))
    .unwrap();
    labels["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

fn workspace(id: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        case(id)["source"].as_str().unwrap(),
    )
    .unwrap();
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--capability-proposals",
    ];
    arguments.extend_from_slice(extra);
    clones(directory, &arguments)
}

fn function<'a>(report: &'a Value, suffix: &str) -> &'a Value {
    let qualified = format!("::{suffix}");
    let mut found = report["capability_proposals"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            let name = function["name"].as_str().unwrap();
            name == suffix || name.ends_with(&qualified)
        });
    let function = found
        .next()
        .unwrap_or_else(|| panic!("missing {suffix}: {report}"));
    assert!(found.next().is_none(), "ambiguous {suffix}");
    function
}

fn wrapper(function: &Value) -> &Value {
    let availability = &function["evidence"]["availability"];
    assert_eq!(availability["status"], "known");
    assert!(availability["value"].is_object(), "{function}");
    &availability["value"]
}

fn complete(report: &Value, findings: usize) {
    let consumer = &report["capability_proposals"];
    assert_eq!(consumer["schema"], "rstyle.capability-proposals/1");
    assert_eq!(consumer["execution"], "completed");
    assert_eq!(consumer["coverage"], "complete");
    assert_eq!(consumer["findings"], findings);
    assert_eq!(consumer["proposals"].as_array().unwrap().len(), findings);
    assert_eq!(
        consumer["assessed_functions"],
        consumer["expected_functions"]
    );
}

fn span<'a>(source: &'a str, range: &Value) -> &'a str {
    source
        .get(range["start"].as_u64().unwrap() as usize..range["end"].as_u64().unwrap() as usize)
        .unwrap()
}

fn assert_wrapper_source(function: &Value, source: &str) {
    let witness = wrapper(function);
    assert!(span(source, &witness["caller_header"]).contains("fn "));
    let methods = witness["methods"].as_array().unwrap();
    assert_eq!(methods.len(), 2);
    assert_ne!(methods[0]["callee_header"], methods[1]["callee_header"]);
    assert!(
        methods[0]["call_span"]["start"].as_u64().unwrap()
            < methods[1]["call_span"]["start"].as_u64().unwrap()
    );
    for method in methods {
        let name = method["name"].as_str().unwrap();
        assert!(span(source, &method["callee_header"]).contains(&format!("fn {name}")));
        assert!(span(source, &method["call_span"]).contains(&format!(".{name}()")));
    }
}

#[test]
fn ordered_local_wrapper_calls_propose_capabilities_without_body_admission() {
    for id in [
        "ordered-local-method-pair-opaque-body",
        "unrelated-trait-declaration",
        "inline-module-wrappers",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert!(output.status.success(), "{id}: {}", stdout(&output));
        complete(&report, 1);
        assert_pair_source(&report, case(id)["source"].as_str().unwrap());
        let opaque = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| {
                function["facts"]["name"]
                    .as_str()
                    .unwrap()
                    .ends_with("::size")
            })
            .unwrap();
        assert_eq!(opaque["outcome"], "excluded");
    }
}

fn assert_pair_source(report: &Value, source: &str) {
    let alpha = function(report, "alpha");
    let beta = function(report, "beta");
    assert_eq!(wrapper(alpha)["owner_name"], "Alpha");
    assert_eq!(wrapper(beta)["owner_name"], "Beta");
    assert_ne!(wrapper(alpha)["owner"], wrapper(beta)["owner"]);
    assert_wrapper_source(alpha, source);
    assert_wrapper_source(beta, source);
    let proposal = &report["capability_proposals"]["proposals"][0];
    let endpoints = [&proposal["left"], &proposal["right"]];
    assert!(endpoints.contains(&&alpha["key"]));
    assert!(endpoints.contains(&&beta["key"]));
    assert_eq!(
        proposal["roles"],
        serde_json::json!([
            {"left_method":0,"right_method":0}, {"left_method":1,"right_method":1}
        ])
    );
}

#[test]
fn declarations_single_owners_and_repeated_members_do_not_invent_capabilities() {
    for id in [
        "declarations-without-wrappers",
        "same-owner",
        "duplicate-selected-member",
        "different-ordinary-arity",
        "same-free-functions",
        "one-formal-wrong-receiver",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert!(output.status.success(), "{id}: {}", stdout(&output));
        complete(&report, 0);
    }
}

#[test]
fn exact_existing_trait_members_suppress_the_ordered_capability_pair() {
    let directory = workspace("exact-existing-trait");
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    complete(&report, 0);
    let consumer = &report["capability_proposals"];
    assert_eq!(consumer["suppressed"].as_array().unwrap().len(), 1);
    assert!(
        !consumer["suppressed"][0]["reason"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    for name in ["alpha", "beta"] {
        let methods = wrapper(function(&report, name))["methods"]
            .as_array()
            .unwrap();
        for (method, member) in methods.iter().zip(["used", "limit"]) {
            assert_eq!(method["origin"]["kind"], "local_trait");
            assert_eq!(method["origin"]["member"], member);
        }
    }
}

#[test]
fn occurrence_order_preserves_the_changed_member_correspondence() {
    let id = "reversed-member-order";
    let directory = workspace(id);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    complete(&report, 1);
    assert_pair_source(&report, case(id)["source"].as_str().unwrap());
    let beta = wrapper(function(&report, "beta"));
    assert_eq!(beta["methods"][0]["name"], "capacity");
    assert_eq!(beta["methods"][1]["name"], "size");
}

#[test]
fn namesake_trait_members_in_distinct_modules_do_not_suppress_capabilities() {
    let directory = workspace("different-traits-with-same-name");
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["capability_proposals"]["coverage"], "incomplete");
    assert_eq!(report["capability_proposals"]["findings"], 0);
    assert_eq!(
        report["capability_proposals"]["unavailable_pairs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        report["capability_proposals"]["suppressed"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let alpha = wrapper(function(&report, "alpha"));
    let beta = wrapper(function(&report, "beta"));
    assert_ne!(alpha["owner"]["module"], beta["owner"]["module"]);
    assert_ne!(
        alpha["methods"][0]["origin"]["trait_header"],
        beta["methods"][0]["origin"]["trait_header"]
    );
    assert_eq!(alpha["methods"][0]["origin"]["trait_name"], "Stats");
    assert_eq!(beta["methods"][0]["origin"]["trait_name"], "Stats");
}

#[test]
fn unsupported_namespaces_receivers_and_wrapper_work_remain_incomplete() {
    for id in [
        "wrong-second-receiver",
        "additional-wrapper-work",
        "generic-method",
        "conditional-import",
        "inherited-default-method",
        "coerced-receiver",
        "competing-receiver-origins",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{id}: {}", stdout(&output));
        assert_eq!(report["capability_proposals"]["coverage"], "incomplete");
        assert_eq!(report["capability_proposals"]["findings"], 0);
        assert!(
            ["alpha", "beta"].iter().any(|name| {
                function(&report, name)["evidence"]["availability"]["status"] != "known"
            }),
            "{id}: {report}"
        );
    }
}

#[test]
fn capabilities_are_opt_in_and_full_compact_reports_and_failure_flags_agree() {
    let directory = workspace("ordered-local-method-pair-opaque-body");
    let (output, full) = requested(directory.path(), &[]);
    assert!(output.status.success());
    let (output, compact) = requested(directory.path(), &["--report-detail", "compact"]);
    assert!(output.status.success());
    assert_eq!(
        full["capability_proposals"],
        compact["capability_proposals"]
    );
    let (output, default) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    assert_eq!(
        default["capability_proposals"]["execution"],
        "not_requested"
    );
    assert_eq!(default["matching"], full["matching"]);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    complete(&report, 1);
}

#[test]
fn omitted_owner_and_pair_work_never_become_complete_empty_scope() {
    let directory = workspace("ordered-local-method-pair-opaque-body");
    for arguments in [
        ["--max-analysis-functions", "0"],
        ["--max-capability-pairs", "0"],
    ] {
        let (output, report) = requested(directory.path(), &arguments);
        assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
        let consumer = &report["capability_proposals"];
        assert_eq!(consumer["execution"], "budget_exhausted");
        assert_eq!(consumer["coverage"], "incomplete");
        assert_eq!(consumer["findings"], 0);
        assert!(consumer["expected_functions"].as_u64().unwrap() > 0);
    }
}

#[test]
fn r0_missing_source_is_unknown_instead_of_complete_no_proposal() {
    let directory = workspace("ordered-local-method-pair-opaque-body");
    let (output, report) = clones(directory.path(), &["--capability-proposals"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["capability_proposals"]["coverage"], "incomplete");
    assert_eq!(report["capability_proposals"]["findings"], 0);
    assert_eq!(
        function(&report, "alpha")["evidence"]["availability"]["status"],
        "unknown"
    );
}

#[test]
fn witnessed_capabilities_survive_an_unsupported_wrapper_without_a_clean_exit() {
    let id = "ordered-local-method-pair-opaque-body";
    let directory = workspace(id);
    let mut source = case(id)["source"].as_str().unwrap().to_owned();
    source
        .push_str(" pub fn opaque(y:&Beta)->(u32,u32) {({let saved=y.size();saved},y.capacity())}");
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["capability_proposals"]["coverage"], "incomplete");
    assert_eq!(report["capability_proposals"]["findings"], 1);
    assert_eq!(
        function(&report, "opaque")["evidence"]["availability"]["status"],
        "unsupported"
    );
}

#[test]
fn strict_replay_preserves_original_capabilities_without_tree_or_tools() {
    let directory = workspace("ordered-local-method-pair-opaque-body");
    let (output, original) = requested(directory.path(), &[]);
    assert!(output.status.success());
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let output = replay(directory.path(), &original, detail);
        assert!(output.status.success(), "{}", stdout(&output));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(
            report["capability_proposals"],
            original["capability_proposals"]
        );
        assert_eq!(report["matching"], original["matching"]);
    }
}

fn replay(directory: &Path, original: &Value, detail: &str) -> Output {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    diagnostic_process::output(
        Command::new(executable)
            .current_dir(directory)
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--capability-proposals",
                "--report-detail",
                detail,
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}
