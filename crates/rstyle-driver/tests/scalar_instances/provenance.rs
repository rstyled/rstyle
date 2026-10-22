//! Independent source anchors and complete compiler bound/assertion census.
use super::*;

fn source_span<'a>(entry: &'a Value, span: &Value) -> &'a str {
    let source = entry["source"]["bytes"].as_str().unwrap();
    let start = span["start"].as_u64().unwrap() as usize;
    let end = span["end"].as_u64().unwrap() as usize;
    &source[start..end]
}

fn check_bounds(entry: &Value, scalar: &str) {
    let provenance = &entry["provenance"];
    assert_eq!(provenance["complete"], true);
    let parameters = provenance["parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), 1);
    let parameter = &parameters[0];
    assert_eq!(parameter["name"], "T");
    assert_eq!(parameter["index"], 0);
    assert_eq!(source_span(entry, &parameter["span"]), "T");
    assert_eq!(source_span(entry, &parameter["declaration_span"]), "T");
    assert_eq!(parameter["definition"]["local"], true);
    let bounds = provenance["bounds"].as_array().unwrap();
    assert_eq!(bounds.len(), 1);
    let bound = &bounds[0];
    assert_eq!(bound["status"], "resolved");
    assert_eq!(bound["parameter_hir"], parameter["hir"]);
    assert_eq!(bound["origin"], "generic_parameter");
    assert_eq!(source_span(entry, &bound["predicate_span"]), ": PartialOrd");
    assert_eq!(source_span(entry, &bound["bound_span"]), "PartialOrd");
    assert_eq!(source_span(entry, &bound["path_span"]), "PartialOrd");
    assert_eq!(bound["trait_definition"]["crate"], "core");
    assert_eq!(
        bound["original_arguments"],
        json!([
            {"kind":"parameter","index":0,"name":"T"},
            {"kind":"parameter","index":0,"name":"T"}
        ])
    );
    let substitutions = bound["substituted_arguments"].as_array().unwrap();
    assert_eq!(substitutions.len(), 2);
    for argument in substitutions {
        assert_eq!(argument["identity"]["name"], scalar);
    }
}

fn check_compiler_census(entry: &Value) {
    let provenance = &entry["provenance"];
    let clauses = provenance["compiler_bounds"].as_array().unwrap();
    assert_eq!(clauses.len(), 2, "explicit PartialOrd and implicit Sized");
    let explicit = &provenance["bounds"][0];
    let matching: Vec<_> = clauses
        .iter()
        .filter(|clause| clause["trait_definition"] == explicit["trait_definition"])
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(
        matching[0]["original_arguments"],
        explicit["original_arguments"]
    );
    assert_eq!(
        matching[0]["substituted_arguments"],
        explicit["substituted_arguments"]
    );
    let implicit: Vec<_> = clauses
        .iter()
        .filter(|clause| clause["trait_definition"] != explicit["trait_definition"])
        .collect();
    assert_eq!(implicit.len(), 1);
    assert_eq!(implicit[0]["status"], "resolved");
    assert_eq!(implicit[0]["trait_definition"]["crate"], "core");
    assert!(
        implicit[0]["trait_definition"]["path"]
            .as_str()
            .unwrap()
            .ends_with("::Sized")
    );
    assert_eq!(
        implicit[0]["original_arguments"].as_array().unwrap().len(),
        1
    );
}

fn check_assertion(entry: &Value) {
    let assertions = entry["provenance"]["assertions"].as_array().unwrap();
    assert_eq!(assertions.len(), 1);
    let assertion = &assertions[0];
    assert_eq!(assertion["status"], "resolved");
    assert_eq!(assertion["definition"]["crate"], "core");
    assert_eq!(assertion["definition"]["local"], false);
    assert!(
        assertion["definition"]["path"]
            .as_str()
            .unwrap()
            .ends_with("::debug_assert")
    );
    assert_eq!(
        source_span(entry, &assertion["span"]),
        "debug_assert!(low <= high, \"ordered bounds\")"
    );
    assert_eq!(assertion["callsite"], assertion["span"]);
    let mut body = Vec::new();
    nodes(&entry["body"], &mut body);
    let actual: Vec<_> = body
        .into_iter()
        .filter(|node| node["kind"] == "expr_assert")
        .collect();
    assert_eq!(actual.len(), 1);
    assert_eq!(assertion["hir"], actual[0]["hir"]);
    assert_eq!(assertion["span"], actual[0]["span"]);
}

#[test]
fn projected_bound_and_assertion_provenance_comes_from_original_compiler_identities() {
    let capture = capture(requests(json!([
        {"function":"clamp","arguments":["i32"]},
        {"function":"clamp","arguments":["u32"]}
    ])));
    assert!(
        capture.output.status.success(),
        "{}",
        String::from_utf8_lossy(&capture.output.stderr)
    );
    let snapshot = capture.snapshot("-scalar-instances.json");
    for (entry, scalar) in snapshot["requests"]
        .as_array()
        .unwrap()
        .iter()
        .zip(["i32", "u32"])
    {
        assert_eq!(entry["status"], "projected", "{entry}");
        assert_eq!(
            entry["provenance"]["identity"],
            "rstyle.rust.scalar-instance-provenance/1"
        );
        check_bounds(entry, scalar);
        check_compiler_census(entry);
        check_assertion(entry);
    }
}
