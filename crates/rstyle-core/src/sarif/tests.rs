use super::*;
use crate::candidates::{Breakdown, CandidateKind, Site};
use serde_json::json;

/// A source root, which must be absolute wherever the tests run, and its URI.
const ROOT: &str = if cfg!(windows) { r"C:\w" } else { "/w" };
const ROOT_URI: &str = if cfg!(windows) {
    "file:///C%3A/w/"
} else {
    "file:///w/"
};

fn report(diagnostics: Vec<Diagnostic>) -> Report {
    Report {
        diagnostics,
        summary: Default::default(),
        fix_groups: Vec::new(),
        functions: Vec::new(),
    }
}

fn violation(path: &str, line: usize) -> Diagnostic {
    Diagnostic {
        path: path.into(),
        line,
        column: 7,
        rule_id: "fn_length".into(),
        message: "too long".into(),
        actual: 60,
        limit: 50,
        details: Vec::new(),
    }
}

fn site(path: &str, line: usize) -> Site {
    Site {
        path: path.into(),
        line,
        id: "id".into(),
        name: "name".into(),
        locator: "locator".into(),
        shape: None,
    }
}

fn candidates(sites: Vec<Site>) -> PatternsReport {
    PatternsReport {
        signature_groups: Vec::new(),
        candidates: vec![Candidate {
            id: "abcd1234".into(),
            kind: CandidateKind::TraitMethod,
            score_milli: 900,
            breakdown: Breakdown {
                support: 2,
                lift_milli: 1,
                holes: 0,
                coverage_milli: 1000,
            },
            observation: "seen".into(),
            inference: "means".into(),
            possible_refactor: "do".into(),
            sites,
            definitions: Vec::new(),
            counter_evidence: Vec::new(),
        }],
        suppressed: Vec::new(),
        ignored: Vec::new(),
        ignored_count: 0,
    }
}

fn value(log: Log) -> Value {
    serde_json::to_value(log).unwrap()
}

#[test]
fn a_diagnostic_is_one_warning_located_by_its_line() {
    let log = value(check(&report(vec![violation("src/a.rs", 3)]), Path::new(ROOT)).unwrap());
    assert_eq!(log["version"], "2.1.0");
    assert_eq!(log["$schema"], SCHEMA);
    let run = &log["runs"][0];
    assert_eq!(run["tool"]["driver"]["name"], "rstyle");
    assert_eq!(run["automationDetails"]["id"], "rstyle/check/");
    assert_eq!(run["originalUriBaseIds"]["%SRCROOT%"]["uri"], ROOT_URI);
    assert_eq!(
        run["results"],
        json!([{
            "ruleId": "fn_length",
            "level": "warning",
            "message": {"text": "too long"},
            "locations": [{"physicalLocation": {
                "artifactLocation": {"uri": "src/a.rs", "uriBaseId": "%SRCROOT%"},
                "region": {"startLine": 3}
            }}],
            "properties": {"actual": 60, "limit": 50}
        }])
    );
}

#[test]
fn a_candidate_has_one_primary_site_and_keeps_its_id() {
    let sites = vec![site("a.go", 1), site("b.go", 2), site("c.go", 3)];
    let log = value(patterns(&candidates(sites), Path::new(ROOT)).unwrap());
    let run = &log["runs"][0];
    assert_eq!(run["automationDetails"]["id"], "rstyle/patterns/");
    let result = &run["results"][0];
    assert_eq!(result["ruleId"], "trait_method");
    assert_eq!(result["message"]["text"], "seen\nmeans\ndo");
    assert_eq!(result["fingerprints"]["rstyle/candidate/v1"], "abcd1234");
    assert_eq!(result["locations"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "a.go"
    );
    assert_eq!(result["relatedLocations"].as_array().unwrap().len(), 2);
}

#[test]
fn an_empty_report_has_an_empty_results_array() {
    let log = value(
        check(
            &report(Vec::new()),
            Path::new(if cfg!(windows) { r"C:\w\" } else { "/w/" }),
        )
        .unwrap(),
    );
    assert_eq!(log["runs"][0]["results"], json!([]));
    assert_eq!(
        log["runs"][0]["originalUriBaseIds"]["%SRCROOT%"]["uri"],
        ROOT_URI
    );
}

#[test]
fn an_unknown_line_or_path_is_left_out_not_invented() {
    let log = value(check(&report(vec![violation("a.rs", 0)]), Path::new(ROOT)).unwrap());
    let located = &log["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
    assert!(located.get("region").is_none());

    let log = value(check(&report(vec![violation("", 4)]), Path::new(ROOT)).unwrap());
    assert!(log["runs"][0]["results"][0].get("locations").is_none());
}

#[test]
fn paths_are_escaped_and_an_absolute_one_needs_no_base() {
    let log = value(
        check(
            &report(vec![violation("a b/#é:c.rs", 1)]),
            Path::new(if cfg!(windows) { r"C:\w x" } else { "/w x" }),
        )
        .unwrap(),
    );
    let run = &log["runs"][0];
    assert_eq!(
        run["originalUriBaseIds"]["%SRCROOT%"]["uri"],
        if cfg!(windows) {
            "file:///C%3A/w%20x/"
        } else {
            "file:///w%20x/"
        }
    );
    assert_eq!(
        run["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "a%20b/%23%C3%A9%3Ac.rs"
    );

    let (absolute, uri) = if cfg!(windows) {
        (r"C:\x\a.rs", "file:///C%3A/x/a.rs")
    } else {
        ("/x/a.rs", "file:///x/a.rs")
    };
    let log = value(check(&report(vec![violation(absolute, 1)]), Path::new(ROOT)).unwrap());
    assert_eq!(
        log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"],
        json!({"uri": uri})
    );
}

#[test]
fn a_relative_source_root_is_refused() {
    let error = check(&report(Vec::new()), Path::new("relative")).unwrap_err();
    assert!(error.contains("must be absolute"), "{error}");
}

#[test]
fn what_this_module_writes_passes_its_own_validation() {
    let log = value(patterns(&candidates(vec![site("a.go", 1)]), Path::new(ROOT)).unwrap());
    validate(&log).unwrap();
}

#[test]
fn a_frontend_log_needs_the_version_a_run_and_a_named_tool() {
    let driver = json!({"tool": {"driver": {"name": "x"}}});
    validate(&json!({"version": "2.1.0", "runs": [driver]})).unwrap();
    for (log, expected) in [
        (json!({"version": "2.0.0", "runs": [driver]}), "version"),
        (json!({"version": "2.1.0"}), "at least one run"),
        (json!({"version": "2.1.0", "runs": []}), "at least one run"),
        (json!({"version": "2.1.0", "runs": [{}]}), "name its tool"),
        (
            json!({"version": "2.1.0", "runs": [{"tool": driver["tool"], "results": 3}]}),
            "must be an array",
        ),
    ] {
        let error = validate(&log).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[cfg(windows)]
#[test]
fn a_canonical_windows_root_is_a_plain_file_uri() {
    let log = value(check(&report(Vec::new()), Path::new(r"\\?\C:\w x")).unwrap());
    assert_eq!(
        log["runs"][0]["originalUriBaseIds"]["%SRCROOT%"]["uri"],
        "file:///C%3A/w%20x/"
    );
}

#[test]
fn a_frontend_run_gets_a_category_and_root_only_if_it_has_none() {
    let log = json!({"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "x"}}}]});
    let runs = labeled_runs(&log, "go/calls/", Path::new(ROOT)).unwrap();
    assert_eq!(runs[0]["automationDetails"]["id"], "go/calls/");
    assert_eq!(runs[0]["originalUriBaseIds"]["%SRCROOT%"]["uri"], ROOT_URI);

    let own = json!({"version": "2.1.0", "runs": [{
        "tool": {"driver": {"name": "x"}},
        "automationDetails": {"id": "own/"},
        "originalUriBaseIds": {"BASE": {"uri": "file:///b/"}},
    }]});
    let runs = labeled_runs(&own, "go/calls/", Path::new(ROOT)).unwrap();
    assert_eq!(runs[0]["automationDetails"]["id"], "own/");
    assert!(runs[0]["originalUriBaseIds"].get("%SRCROOT%").is_none());
}

#[test]
fn several_runs_of_one_frontend_analysis_stay_distinguishable() {
    let run = json!({"tool": {"driver": {"name": "x"}}});
    let log = json!({"version": "2.1.0", "runs": [run, run]});
    let runs = labeled_runs(&log, "go/calls/", Path::new(ROOT)).unwrap();
    assert_eq!(runs[0]["automationDetails"]["id"], "go/calls/0/");
    assert_eq!(runs[1]["automationDetails"]["id"], "go/calls/1/");
}

#[test]
fn a_combined_log_is_valid_and_keeps_every_run_in_order() {
    let core = value(check(&report(Vec::new()), Path::new(ROOT)).unwrap());
    let frontend = json!({"tool": {"driver": {"name": "x"}}});
    let log = combined(vec![core["runs"][0].clone(), frontend]);
    validate(&log).unwrap();
    assert_eq!(log["$schema"], SCHEMA);
    let runs = log["runs"].as_array().unwrap();
    assert_eq!(runs[0]["automationDetails"]["id"], "rstyle/check/");
    assert_eq!(runs[1]["tool"]["driver"]["name"], "x");
}

fn detail(label: &str, count: usize, lines: Vec<usize>) -> Detail {
    Detail {
        label: label.into(),
        count,
        lines,
    }
}

fn group(id: usize, path: &str, line: usize, rules: &[&str]) -> FixGroup {
    FixGroup {
        id,
        functions: vec![crate::report::GroupFunction {
            path: path.into(),
            line,
            name: "f".into(),
            rules: rules.iter().map(|rule| (*rule).into()).collect(),
        }],
        hints: Vec::new(),
        plan: None,
    }
}

#[test]
fn a_check_finding_says_what_raised_it_and_by_how_much() {
    let mut raised = violation("src/a.rs", 3);
    raised.details = vec![
        detail("Branch (nesting 1)", 2, vec![5]),
        detail("io: std::fs::read", 2, vec![7, 9]),
    ];
    let log = value(check(&report(vec![raised]), Path::new(ROOT)).unwrap());
    let result = &log["runs"][0]["results"][0];

    let points: Vec<(u64, &str)> = result["relatedLocations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["physicalLocation"]["region"]["startLine"]
                    .as_u64()
                    .unwrap(),
                l["message"]["text"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        points,
        [
            (5, "Branch (nesting 1) +2"),
            (7, "io: std::fs::read"),
            (9, "io: std::fs::read")
        ]
    );
    assert_eq!(result["properties"], json!({"actual": 60, "limit": 50}));
}

#[test]
fn a_check_run_carries_its_fix_groups_and_each_finding_names_its_own() {
    let mut fixed = report(vec![violation("src/a.rs", 3), violation("src/b.rs", 4)]);
    fixed.fix_groups = vec![
        group(1, "src/a.rs", 3, &["fn_length"]),
        group(2, "src/b.rs", 4, &["other_rule"]),
    ];
    let log = value(check(&fixed, Path::new(ROOT)).unwrap());
    let run = &log["runs"][0];

    assert_eq!(run["results"][0]["properties"]["fix_group"], 1);
    // Group 2 names the function for another rule, so this diagnostic is in no group.
    assert!(run["results"][1]["properties"].get("fix_group").is_none());
    assert_eq!(run["properties"]["fix_groups"][1]["id"], 2);
    assert_eq!(
        run["properties"]["fix_groups"][0]["functions"][0]["path"],
        "src/a.rs"
    );
}

#[test]
fn a_check_without_fix_groups_has_no_run_properties() {
    let log = value(check(&report(vec![violation("src/a.rs", 3)]), Path::new(ROOT)).unwrap());
    assert!(log["runs"][0].get("properties").is_none());
}
