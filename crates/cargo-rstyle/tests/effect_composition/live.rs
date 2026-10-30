use super::*;

#[test]
fn nested_scalar_writes_map_through_reordered_formals_to_the_exact_top_owner() {
    let directory = workspace(REORDERED);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_effects"]);
    complete(&report, "caller", "writes_formal");
    let caller = function(&report, "caller");
    let events = caller["evidence"]["calls"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        slice(function_source(REORDERED, caller), &events[0]["span"]),
        "wrapper(delta, second)"
    );
    let mapped = events[0]["mappings"].as_array().unwrap();
    assert_eq!(mapped.len(), 1);
    assert!(mapped[0].get("callee_site_index").is_none());
    assert_eq!(mapped[0]["caller_root"]["formal"], 1);
    let summary = summary(&report, caller, &events[0]);
    let index = mapped[0]["summary_site_index"].as_u64().unwrap() as usize;
    let witness = &summary["sites"][index];
    assert_eq!(witness["mapped_root"]["formal"], 1);
    assert_eq!(witness["root"]["formal"], 0);
    assert_eq!(witness["site_index"], 0);
    assert_original_witness(
        &report,
        REORDERED,
        Witness {
            summary,
            site: witness,
        },
        ExpectedWitness {
            owner: "leaf",
            assignment: "*target += delta",
            path_length: 1,
        },
    );
}

struct Witness<'a> {
    summary: &'a Value,
    site: &'a Value,
}

struct ExpectedWitness {
    owner: &'static str,
    assignment: &'static str,
    path_length: usize,
}

const LEAF_INCREMENT: ExpectedWitness = ExpectedWitness {
    owner: "leaf",
    assignment: "*target += 1",
    path_length: 1,
};

fn assert_original_witness(
    report: &Value,
    source: &str,
    witness: Witness<'_>,
    expected: ExpectedWitness,
) {
    let Witness { summary, site } = witness;
    let original = owner(report, &summary["unit"], &site["function_index"]);
    assert_eq!(original["key"], function(report, expected.owner)["key"]);
    assert!(
        slice(function_source(source, original), &site["span"]).starts_with(expected.assignment)
    );
    assert_eq!(site["caller_visible"], true);
    assert_eq!(site["root"]["provenance"], "formal_borrowed");
    let path = site["call_path"].as_array().unwrap();
    assert_eq!(path.len(), expected.path_length);
    for step in path {
        let original = owner(report, &summary["unit"], &step["function_index"]);
        assert!(slice(function_source(source, original), &step["span"]).starts_with("leaf("));
        assert!(step["call_index"].as_u64().is_some());
        assert!(!step["native"].is_null());
        assert_eq!(step["function_index"], summary["function_index"]);
    }
}

#[test]
fn wrapper_direct_sites_and_nested_sites_keep_distinct_original_owners() {
    let source = "fn leaf(target: &mut u32) { *target += 1; }\nfn wrapper(target: &mut u32) { *target += 2; leaf(target); }\npub fn caller(target: &mut u32) { wrapper(target); }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_effects"]);
    complete(&report, "caller", "writes_formal");
    let caller = function(&report, "caller");
    assert_eq!(caller["evidence"]["direct_site_count"], 0);
    let event = &caller["evidence"]["calls"][0];
    let summary = summary(&report, caller, event);
    let sites = summary["sites"].as_array().unwrap();
    assert_eq!(sites.len(), 2);
    assert_eq!(event["mappings"].as_array().unwrap().len(), 2);
    for witness in sites {
        if witness["function_index"] == summary["function_index"] {
            assert_original_witness(
                &report,
                source,
                Witness {
                    summary,
                    site: witness,
                },
                ExpectedWitness {
                    owner: "wrapper",
                    assignment: "*target += 2",
                    path_length: 0,
                },
            );
        } else {
            assert_original_witness(
                &report,
                source,
                Witness {
                    summary,
                    site: witness,
                },
                LEAF_INCREMENT,
            );
        }
        assert_eq!(witness["mapped_root"]["formal"], 0);
    }
}

#[test]
fn repeated_wrapper_and_leaf_calls_keep_four_paths_and_reuse_completed_summaries() {
    let directory = workspace(REPEATED);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_effects"]);
    complete(&report, "caller", "writes_formal");
    let caller = function(&report, "caller");
    let events = caller["evidence"]["calls"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_ne!(events[0]["span"], events[1]["span"]);
    assert_eq!(events[0]["summary_index"], events[1]["summary_index"]);
    let summary = summary(&report, caller, &events[0]);
    let sites = summary["sites"].as_array().unwrap();
    assert_eq!(sites.len(), 2);
    assert_ne!(
        sites[0]["call_path"][0]["call_index"],
        sites[1]["call_path"][0]["call_index"]
    );
    assert_ne!(
        sites[0]["call_path"][0]["span"],
        sites[1]["call_path"][0]["span"]
    );
    assert_eq!(sites[0]["site_index"], sites[1]["site_index"]);
    for site in sites {
        assert_original_witness(&report, REPEATED, Witness { summary, site }, LEAF_INCREMENT);
    }
    for event in events {
        assert_eq!(event["mappings"].as_array().unwrap().len(), 2);
    }
    let summaries = report["mutation_effects"]["summaries"].as_array().unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(
        report["mutation_effects"]["units"][0]["statistics"]["leaf_checks"],
        1
    );
    assert_eq!(
        report["mutation_effects"]["units"][0]["statistics"]["wrapper_checks"],
        1
    );
}

#[test]
fn transitive_owned_local_rebinding_is_not_a_caller_visible_write() {
    let source = "fn leaf(mut value: u32) -> u32 { value += 1; value }\nfn wrapper(mut value: u32) -> u32 { value += 2; leaf(value) }\npub fn caller(value: u32) -> u32 { wrapper(value) }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_effects"]);
    complete(&report, "caller", "none_proven");
    let caller = function(&report, "caller");
    assert!(caller["evidence"]["roots"].as_array().unwrap().is_empty());
    assert!(
        caller["evidence"]["calls"][0]["mappings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn full_and_compact_compositions_replay_without_sources_or_compiler_tools() {
    let directory = workspace(REPEATED);
    let (output, original) = requested(directory.path(), &[]);
    assert!(output.status.success());
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let output = replay(directory.path(), &original, detail);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let replay: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(replay["verification_policy"], "strict-replay");
        assert_eq!(replay["mutation_effects"], original["mutation_effects"]);
        assert_eq!(replay["matching"], original["matching"]);
    }
}

#[test]
fn transitive_effects_do_not_reclassify_callee_assignments_as_direct_caller_mutations() {
    let directory = workspace(REPEATED);
    let (direct_output, direct) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--mutation-rules"],
    );
    let (combined_output, combined) = requested(directory.path(), &["--mutation-rules"]);
    assert_eq!(combined_output.status.code(), direct_output.status.code());
    assert_eq!(combined["mutation_analysis"], direct["mutation_analysis"]);
    complete(&combined, "caller", "writes_formal");
    assert_eq!(
        function(&combined, "caller")["evidence"]["direct_site_count"],
        0
    );
}
