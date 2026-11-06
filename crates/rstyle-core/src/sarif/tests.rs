use super::*;
use crate::candidates::{Breakdown, CandidateKind, Site};
use serde_json::json;

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
    let log = value(check(&report(vec![violation("src/a.rs", 3)]), Path::new("/w")).unwrap());
    assert_eq!(log["version"], "2.1.0");
    assert_eq!(log["$schema"], SCHEMA);
    let run = &log["runs"][0];
    assert_eq!(run["tool"]["driver"]["name"], "rstyle");
    assert_eq!(run["automationDetails"]["id"], "rstyle/check/");
    assert_eq!(run["originalUriBaseIds"]["%SRCROOT%"]["uri"], "file:///w/");
    assert_eq!(
        run["results"],
        json!([{
            "ruleId": "fn_length",
            "level": "warning",
            "message": {"text": "too long"},
            "locations": [{"physicalLocation": {
                "artifactLocation": {"uri": "src/a.rs", "uriBaseId": "%SRCROOT%"},
                "region": {"startLine": 3}
            }}]
        }])
    );
}

#[test]
fn a_candidate_has_one_primary_site_and_keeps_its_id() {
    let sites = vec![site("a.go", 1), site("b.go", 2), site("c.go", 3)];
    let log = value(patterns(&candidates(sites), Path::new("/w")).unwrap());
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
    let log = value(check(&report(Vec::new()), Path::new("/w/")).unwrap());
    assert_eq!(log["runs"][0]["results"], json!([]));
    assert_eq!(
        log["runs"][0]["originalUriBaseIds"]["%SRCROOT%"]["uri"],
        "file:///w/"
    );
}

#[test]
fn an_unknown_line_or_path_is_left_out_not_invented() {
    let log = value(check(&report(vec![violation("a.rs", 0)]), Path::new("/w")).unwrap());
    let located = &log["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
    assert!(located.get("region").is_none());

    let log = value(check(&report(vec![violation("", 4)]), Path::new("/w")).unwrap());
    assert!(log["runs"][0]["results"][0].get("locations").is_none());
}

#[test]
fn paths_are_escaped_and_an_absolute_one_needs_no_base() {
    let log = value(
        check(
            &report(vec![violation("a b/#é:c.rs", 1)]),
            Path::new("/w x"),
        )
        .unwrap(),
    );
    let run = &log["runs"][0];
    assert_eq!(
        run["originalUriBaseIds"]["%SRCROOT%"]["uri"],
        "file:///w%20x/"
    );
    assert_eq!(
        run["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "a%20b/%23%C3%A9%3Ac.rs"
    );

    let log = value(check(&report(vec![violation("/x/a.rs", 1)]), Path::new("/w")).unwrap());
    assert_eq!(
        log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"],
        json!({"uri": "file:///x/a.rs"})
    );
}

#[test]
fn a_relative_source_root_is_refused() {
    let error = check(&report(Vec::new()), Path::new("relative")).unwrap_err();
    assert!(error.contains("must be absolute"), "{error}");
}

#[test]
fn what_this_module_writes_passes_its_own_validation() {
    let log = value(patterns(&candidates(vec![site("a.go", 1)]), Path::new("/w")).unwrap());
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
