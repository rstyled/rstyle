use super::*;

#[test]
fn accepted_pairs_keep_source_navigation_in_full_compact_and_text_reports() {
    let directory = clone_workspace();
    for profile in ["r0", "hir-data-v3"] {
        let (output, full) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(output.status.success());
        let (output, compact) = clones(
            directory.path(),
            &[
                "--extraction-profile",
                profile,
                "--report-detail",
                "compact",
            ],
        );
        assert!(output.status.success());
        assert_eq!(full["matching"], compact["matching"]);
        assert_eq!(full["accepted_pairs"], compact["accepted_pairs"]);
        assert_locations(&compact, profile);

        let output = rstyle(
            directory.path(),
            &["clones", "--extraction-profile", profile],
        );
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("Accepted pair: function:"), "{text}");
        assert!(text.contains("first at "), "{text}");
        assert!(text.contains("second at "), "{text}");
        assert!(text.contains("src/lib.rs"), "{text}");
        assert!(text.contains("not equivalence or safe refactoring"));
    }
}

fn assert_locations(report: &Value, profile: &str) {
    let pairs = report["accepted_pairs"].as_array().unwrap();
    assert!(!pairs.is_empty());
    for pair in pairs {
        for endpoint in ["first", "second"] {
            let location = &pair[endpoint];
            let function = report["functions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|function| function["key"] == location["key"])
                .unwrap();
            assert_eq!(location["name"], function["name"]);
            assert_eq!(location["span"], function["fn_span"]);
            assert!(location["path"].as_str().unwrap().ends_with("src/lib.rs"));
            if profile == "hir-data-v3" {
                assert!(location["line"].as_u64().unwrap() > 0);
                assert!(location["column"].as_u64().unwrap() > 0);
            }
        }
    }
}

#[test]
fn text_pair_locations_do_not_hide_budget_incompleteness() {
    let directory = clone_workspace();
    let output = rstyle(directory.path(), &["clones", "--max-pairs", "0"]);
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("enumeration=false, evaluation=false"),
        "{text}"
    );
    assert!(!text.contains("Accepted pair:"), "{text}");
}
