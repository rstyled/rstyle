use super::*;

#[test]
fn named_mutation_suppressions_hide_only_the_chosen_rules_and_preserve_raw_evidence() {
    let directory = workspace(WRITES);
    let (_, baseline) = requested(directory.path(), &[]);
    for (annotation, hidden, visible) in [
        ("mutation_per_target", 1, 1),
        ("mutated_targets", 1, 1),
        ("mutation_per_target, mutated_targets", 2, 0),
        ("fn_params, side_effect_density, abstraction", 0, 2),
    ] {
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("// rstyle-allow({annotation}): reviewed scalar state\n{WRITES}"),
        )
        .unwrap();
        let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
        assert_eq!(
            output.status.code(),
            Some(if visible == 0 { 0 } else { 1 }),
            "{}",
            report["mutation_analysis"]
        );
        counts(&report, 2, visible, hidden);
        assert_raw_same(function(&report), function(&baseline));
        assert_eq!(function(&report)["policy"]["coverage"], "complete");
        assert_eq!(
            function(&report)["policy"]["source_suppression"]["status"],
            "allowed"
        );
        assert_eq!(report["mutation_analysis"]["policy_evaluated_functions"], 1);
        assert_eq!(report["mutation_analysis"]["excluded_functions"], 0);
        for id in ["mutation_per_target", "mutated_targets"] {
            assert_eq!(
                decision(function(&report), id)["disposition"],
                if annotation.split(',').any(|name| name.trim() == id) {
                    "suppressed"
                } else {
                    "evaluated"
                }
            );
        }
    }
}

#[test]
fn invalid_captured_annotations_hide_nothing_and_add_a_visible_diagnostic() {
    for annotation in [
        "// rstyle-allow(mutation_per_target)",
        "// rstyle-allow(mutation_per_target): ",
        "// rstyle-allow(mutation_per_target, unknown_rule): reason",
    ] {
        let directory = workspace(&format!("{annotation}\n{WRITES}"));
        let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1));
        counts(&report, 3, 3, 0);
        let function = function(&report);
        assert_eq!(
            function["policy"]["source_suppression"]["status"],
            "invalid"
        );
        assert!(!function["policy"]["invalid_suppression"].is_null());
        assert_eq!(function["evidence"]["write_sites"]["value"], 6);
        for id in ["mutation_per_target", "mutated_targets"] {
            assert_eq!(decision(function, id)["disposition"], "evaluated");
        }
    }
}

#[test]
fn suppressing_a_witnessed_breach_cannot_clear_an_unknown_mutation_remainder() {
    let body = "pub fn many(callback:fn()) { let mut value=0u32; value=1; value=2; value=3; callback(); }\n";
    let directory = workspace(body);
    let (_, baseline) = requested(directory.path(), &[]);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("// rstyle-allow(mutation_per_target): retained local state\n{body}"),
    )
    .unwrap();
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2));
    counts(&report, 1, 0, 1);
    let function = function(&report);
    assert_raw_same(function, &baseline["mutation_analysis"]["functions"][0]);
    assert_eq!(function["coverage"], "incomplete");
    assert_eq!(function["evidence"]["write_sites"]["kind"], "lower_bound");
    assert_eq!(function["evidence"]["write_sites"]["value"], 3);
    assert_eq!(function["policy"]["coverage"], "incomplete");
    assert_eq!(report["mutation_analysis"]["unavailable_functions"], 1);
    assert_eq!(
        report["mutation_analysis"]["policy_unavailable_functions"],
        1
    );
}

#[test]
fn a_blank_line_stops_mutation_annotation_association() {
    let directory = workspace(&format!(
        "// rstyle-allow(mutation_per_target, mutated_targets): separate item\n\n{WRITES}"
    ));
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&report, 2, 2, 0);
    assert_eq!(
        function(&report)["policy"]["source_suppression"]["status"],
        "none"
    );
}
