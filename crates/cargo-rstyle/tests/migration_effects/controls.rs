use super::*;

fn capture(source: &str) -> (TempDir, Output, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = requested(directory.path(), &[]);
    (directory, output, report)
}

const CONTROL_CHAIN: &str = r#"
fn leaf(target:&mut u32, value:u32) {
    if value == 0 { *target=1; } else { *target=2; }
    while value < 0 { *target=3; }
    loop { *target=4; break; }
    for index in 0u32..0u32 { *target=index; }
}
fn wrapper(target:&mut u32, value:u32) {
    if value == 0 { leaf(target,value); } else { leaf(target,value); }
}
pub fn caller(target:&mut u32, value:u32) { wrapper(target,value); }
"#;

fn complete(report: &Value, name: &str, sites: u64, calls: usize) {
    let evidence = &function(report, name)["evidence"];
    assert_eq!(evidence["coverage"], "complete", "{name}: {evidence}");
    assert_eq!(evidence["caller_visible_mutation"], "writes_formal");
    assert_eq!(evidence["direct_site_count"], sites);
    assert_eq!(evidence["calls"].as_array().unwrap().len(), calls);
    assert_eq!(
        evidence["roots"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|root| root["formal"] == 0)
            .count(),
        1
    );
}

#[test]
fn primitive_control_sites_compose_through_two_edges_without_runtime_multiplication() {
    let (_directory, output, report) = capture(CONTROL_CHAIN);
    assert!(output.status.success(), "{}", stdout(&output));
    complete(&report, "leaf", 5, 0);
    complete(&report, "wrapper", 0, 2);
    complete(&report, "caller", 0, 1);
    let wrapper = function(&report, "wrapper");
    let calls = wrapper["evidence"]["calls"].as_array().unwrap();
    assert_ne!(calls[0]["span"], calls[1]["span"]);
    assert_ne!(calls[0]["call_index"], calls[1]["call_index"]);
    assert_eq!(calls[0]["summary_index"], calls[1]["summary_index"]);
    let leaf = summary(&report, wrapper, &calls[0]);
    assert_eq!(leaf["sites"].as_array().unwrap().len(), 5);
    for call in calls {
        assert_eq!(call["mappings"].as_array().unwrap().len(), 5);
        for mapping in call["mappings"].as_array().unwrap() {
            assert_eq!(mapping["caller_root"]["formal"], 0);
        }
    }
}

#[test]
fn control_calls_preserve_source_occurrences_and_shadowed_local_roots() {
    let source = r#"
fn leaf(target:&mut u32) { *target=1; }
pub fn caller(target:&mut u32, value:u32) {
    if value == 0 { leaf(target); } else { leaf(target); }
    while value < 0 { leaf(target); }
    loop { leaf(target); break; }
    for index in 0u32..0u32 { leaf(target); }
    { let mut target=0u32; target=2; }
    *target=3;
}
"#;
    let (_directory, output, report) = capture(source);
    assert!(output.status.success(), "{}", stdout(&output));
    complete(&report, "caller", 2, 5);
    assert_calls(&report, source);
    let caller = function(&report, "caller");
    let roots = caller["evidence"]["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 2);
    assert!(roots.iter().any(|root| root["provenance"] == "local_owned"));
    let calls = caller["evidence"]["calls"].as_array().unwrap();
    let spans: std::collections::BTreeSet<_> = calls
        .iter()
        .map(|call| slice(original_function(source, caller), &call["span"]))
        .collect();
    // Identical spelling is deliberately repeated at five different source positions.
    assert_eq!(spans.len(), 1);
    let positions: std::collections::BTreeSet<_> = calls
        .iter()
        .map(|call| call["span"]["start"].as_u64().unwrap())
        .collect();
    assert_eq!(positions.len(), 5);
}

#[test]
fn reference_redirection_after_shadow_scope_or_in_initializer_stays_unknown() {
    for body in [
        "{let mut target=0u32;target=2;} target=target; *target=3;",
        "let target={target=target;0u32};",
    ] {
        let source = format!("pub fn caller(mut target:&mut u32){{{body}}}");
        let (_directory, output, report) = capture(&source);
        assert_eq!(output.status.code(), Some(2));
        let evidence = &function(&report, "caller")["evidence"];
        assert_eq!(evidence["coverage"], "incomplete");
        assert_eq!(evidence["caller_visible_mutation"], "unknown");
        assert!(
            evidence["roots"]
                .as_array()
                .unwrap()
                .iter()
                .all(|root| root["provenance"] != "formal_borrowed")
        );
    }
}

#[test]
fn unsupported_control_remainders_keep_direct_witnesses_and_never_become_clean() {
    for remainder in [
        "if value == 0 { std::hint::black_box(value); }",
        "if value == 0 { let alias=&mut *target; leaf(alias); }",
        "for index in 0u32..=1u32 { leaf(target); }",
    ] {
        let source = format!(
            "fn leaf(target:&mut u32) {{ *target=1; }} \
             pub fn caller(target:&mut u32,value:u32) {{ *target=9; {remainder} }}"
        );
        let (_directory, output, report) = capture(&source);
        assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
        let evidence = &function(&report, "caller")["evidence"];
        assert_eq!(
            evidence["coverage"], "incomplete",
            "{remainder}: {evidence}"
        );
        assert_eq!(evidence["direct_site_count"], 1);
        assert_eq!(
            evidence["caller_visible_mutation"],
            "witnessed_writes_formal_with_unknown_remainder"
        );
        assert_eq!(evidence["roots"][0]["formal"], 0);
    }
}

#[test]
fn conditional_third_edge_and_omitted_owners_remain_bounded() {
    let source = format!(
        "{CONTROL_CHAIN} pub fn outer(target:&mut u32,value:u32) \
         {{ *target=9; if value == 0 {{ caller(target,value); }} }}"
    );
    let (directory, output, report) = capture(&source);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let evidence = &function(&report, "outer")["evidence"];
    assert_eq!(evidence["coverage"], "incomplete");
    assert_eq!(evidence["direct_site_count"], 1);
    assert_eq!(
        evidence["caller_visible_mutation"],
        "witnessed_writes_formal_with_unknown_remainder"
    );
    let (output, bounded) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(bounded["mutation_effects"]["execution"], "budget_exhausted");
    for function in bounded["mutation_effects"]["functions"].as_array().unwrap() {
        assert_eq!(function["evidence"]["coverage"], "incomplete");
        assert_eq!(function["evidence"]["caller_visible_mutation"], "unknown");
    }
}

#[test]
fn control_summaries_replay_strictly_without_source_or_tools_in_both_details() {
    let (directory, output, original) = capture(CONTROL_CHAIN);
    assert!(output.status.success(), "{}", stdout(&output));
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let output = replay(directory.path(), &original, detail);
        assert!(output.status.success(), "{}", stdout(&output));
        let replay: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(replay["verification_policy"], "strict-replay");
        assert_eq!(replay["mutation_effects"], original["mutation_effects"]);
        assert_eq!(
            replay["owned_source_inventory"],
            original["owned_source_inventory"]
        );
        assert_eq!(replay["matching"], original["matching"]);
    }
}
