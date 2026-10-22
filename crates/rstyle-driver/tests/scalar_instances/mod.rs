//! Requested owner-local projections are diagnostic, never concrete admissions.
mod provenance;

use super::*;
use serde_json::{Value, json};

const SOURCE: &str = r#"
pub fn clamp<T: PartialOrd>(value: T, low: T, high: T) -> T {
    debug_assert!(low <= high, "ordered bounds");
    let saved: T = value;
    let chosen: T = if saved < low { return low; } else { saved };
    if chosen > high { high } else { chosen }
}
pub fn tuple<T: Copy>(value: T) -> T { let pair = (value, value); pair.0 }
pub fn nested<T: Copy>(value: T) -> T { let callback = || value; callback() }
pub fn konst<const N: usize>(value: i32) -> i32 { value }
pub fn relations<T: PartialOrd>(a: T, b: T) -> bool {
    let _ = a == b; let _ = a != b; let _ = a < b;
    let _ = a <= b; let _ = a > b; a >= b
}
pub fn minimum<T: PartialOrd>(a: T, b: T) -> T { if a < b { a } else { b } }
"#;

struct Capture {
    directory: PathBuf,
    output: Output,
}

impl Capture {
    fn snapshot(&self, suffix: &str) -> Value {
        let files: Vec<_> = std::fs::read_dir(self.directory.join("snapshots"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(suffix)
            })
            .collect();
        assert_eq!(files.len(), 1, "one complete {suffix} snapshot");
        serde_json::from_slice(&std::fs::read(&files[0]).unwrap()).unwrap()
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn capture(request: Value) -> Capture {
    let directory = directory();
    let input = directory.join("projection.rs");
    let requests = directory.join("requests.json");
    std::fs::write(&input, SOURCE).unwrap();
    std::fs::write(&requests, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = selected_invocation(&input, &directory.join("facts.json"), Some("hir-data-v3"))
        .env("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY", "1")
        .env("RSTYLE_DIAGNOSTIC_FACTS_DIR", directory.join("snapshots"))
        .env("RSTYLE_DIAGNOSTIC_INSTANCE_REQUESTS", requests)
        .output()
        .unwrap();
    Capture { directory, output }
}

fn requests(entries: Value) -> Value {
    json!({"identity":"rstyle.diagnostic.scalar-instance-requests/1", "crate_name":"extraction_fixture", "requests":entries})
}

fn nodes<'a>(node: &'a Value, result: &mut Vec<&'a Value>) {
    result.push(node);
    for child in node["children"].as_array().unwrap() {
        nodes(child, result);
    }
}

fn check_projection(entry: &Value, ty: &str) {
    assert_eq!(
        entry["status"],
        "projected",
        "projection: {}",
        entry.get("reason").unwrap_or(&Value::Null)
    );
    assert_eq!(entry["signature"]["return_ty"]["normalized"], ty);
    assert_eq!(entry["signature"]["generics"].as_array().unwrap().len(), 1);
    assert_eq!(entry["arguments"][0]["identity"]["name"], ty);
    for parameter in entry["signature"]["params"].as_array().unwrap() {
        assert_eq!(parameter["ty"]["normalized"], ty);
        assert_eq!(parameter["ty"]["spelling"], "T");
        assert_eq!(parameter["pat"]["ty"]["normalized"], ty);
    }
    let mut body = Vec::new();
    nodes(&entry["body"], &mut body);
    let saved = body
        .iter()
        .find(|node| node["kind"] == "pat_binding" && node["snippet"] == "saved")
        .unwrap();
    assert_eq!(saved["ty"]["normalized"], ty);
    let returned = body.iter().find(|node| node["kind"] == "expr_ret").unwrap();
    assert_eq!(returned["ty"]["normalized"], ty);
    let assertion = body
        .iter()
        .find(|node| node["kind"] == "expr_assert")
        .unwrap();
    let mut assertion_nodes = Vec::new();
    nodes(assertion, &mut assertion_nodes);
    assert!(
        assertion_nodes
            .iter()
            .any(|node| node["kind"] == "expr_path" && node["ty"]["normalized"] == ty)
    );
    check_comparisons(entry, &body, ty);
    check_never_adjustment(entry, ty);
}

fn check_comparisons(entry: &Value, body: &[&Value], ty: &str) {
    let comparisons = entry["comparisons"].as_array().unwrap();
    assert_eq!(
        comparisons.len(),
        3,
        "assertion and both branch comparisons retained"
    );
    for observation in comparisons {
        assert_eq!(observation["status"], "resolved", "comparison status");
        assert_eq!(observation["selected"]["crate"], "core");
        check_operand_adjustments(observation);
        assert_eq!(observation["operand_types"][0]["identity"]["name"], ty);
        let node = body
            .iter()
            .find(|node| node["hir"] == observation["hir"])
            .unwrap();
        assert_eq!(
            node["dispatch"]["kind"], "overloaded",
            "projection must retain overload authority requirement"
        );
        assert_eq!(node["dispatch"]["operand_tys"], json!([ty, ty]));
        assert_eq!(node["dispatch"]["result_ty"], "bool");
    }
}

#[test]
fn requested_scalar_body_projection_preserves_generic_roster_and_complete_source() {
    let capture = capture(requests(json!([
        {"function":"clamp", "arguments":["i32"]},
        {"function":"clamp", "arguments":["u32"]}
    ])));
    assert!(
        capture.output.status.success(),
        "{}",
        String::from_utf8_lossy(&capture.output.stderr)
    );
    let snapshot = capture.snapshot("-scalar-instances.json");
    assert_eq!(
        snapshot["identity"],
        "rstyle.diagnostic.scalar-instance-bodies/1"
    );
    assert_eq!(snapshot["obligations_verified"], false);
    let entries = snapshot["requests"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    check_projection(&entries[0], "i32");
    check_projection(&entries[1], "u32");
    for field in ["definition", "source", "fn_span"] {
        assert_eq!(entries[0][field], entries[1][field]);
    }
    assert_ne!(entries[0]["arguments"], entries[1]["arguments"]);
    let raw = capture.snapshot("-pre-helper-certification.json");
    let functions: Vec<HirFnFacts> = serde_json::from_value(raw["facts"].clone()).unwrap();
    assert_eq!(
        functions.len(),
        6,
        "ordinary inventory stays definition-only"
    );
    let clamp = functions
        .iter()
        .find(|function| function.name == "clamp")
        .unwrap();
    assert_eq!(clamp.signature.return_ty.observed, "T");
    assert_eq!(clamp.signature.generics.len(), 1);
    assert_ne!(
        classify(clamp).status,
        EligibilityVerdict::Eligible,
        "diagnostic projection grants no admission"
    );
    assert_eq!(entries[0]["source"]["bytes"], &clamp.source.bytes[..]);
    check_bindings(&entries[0], clamp, "i32");
    check_bindings(&entries[1], clamp, "u32");
}

#[test]
fn unsupported_projection_never_publishes_a_supported_prefix() {
    let capture = capture(requests(json!([
        {"function":"clamp", "arguments":["f32"]},
        {"function":"clamp", "arguments":[]},
        {"function":"missing", "arguments":["i32"]},
        {"function":"tuple", "arguments":["i32"]},
        {"function":"nested", "arguments":["u32"]},
        {"function":"konst", "arguments":["i32"]}
    ])));
    assert!(
        capture.output.status.success(),
        "{}",
        String::from_utf8_lossy(&capture.output.stderr)
    );
    let snapshot = capture.snapshot("-scalar-instances.json");
    let entries = snapshot["requests"].as_array().unwrap();
    for (entry, status) in entries.iter().zip([
        "unsupported",
        "unsupported",
        "unavailable",
        "unsupported",
        "unsupported",
        "unsupported",
    ]) {
        assert_eq!(entry["status"], status);
        assert!(
            entry["reason"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
        );
        for field in [
            "body",
            "signature",
            "bindings",
            "comparisons",
            "adjustments",
            "provenance",
            "source",
        ] {
            assert!(entry.get(field).is_none(), "no partial {field}");
        }
    }
}

#[test]
fn malformed_projection_requests_fail_capture_instead_of_defaulting() {
    for request in [
        json!({"identity":"wrong", "crate_name":"extraction_fixture", "requests":[]}),
        requests(json!([{"function":"clamp", "arguments":["i32"], "unknown":true}])),
        json!({"identity":"rstyle.diagnostic.scalar-instance-requests/1", "crate_name":"other", "requests":[]}),
    ] {
        let capture = capture(request);
        assert!(!capture.output.status.success());
        let published = std::fs::read_dir(capture.directory.join("snapshots"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with("-scalar-instances.json")
            });
        assert!(
            !published,
            "malformed input cannot publish an empty successful projection"
        );
    }
}

fn check_operand_adjustments(observation: &Value) {
    assert_eq!(observation["requested_signature"], observation["signature"]);
    let operands = observation["operand_adjustments"].as_array().unwrap();
    assert_eq!(operands.len(), 2);
    for (index, operand) in operands.iter().enumerate() {
        assert_eq!(
            operand["source_type"][0],
            observation["operand_types"][index]
        );
        assert_eq!(operand["steps"].as_array().unwrap().len(), 1);
        assert_eq!(operand["steps"][0]["kind"], "shared_borrow");
        assert_eq!(operand["steps"][0]["target"], operand["adjusted_type"]);
        assert_eq!(
            operand["adjusted_type"][0],
            observation["signature"]["inputs"][index]
        );
    }
}

fn check_never_adjustment(entry: &Value, ty: &str) {
    let never = entry["adjustments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|adjustment| {
            adjustment["steps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|step| step["kind"] == "never_to_any")
        })
        .expect("early return branch has actual compiler NeverToAny coercion");
    assert_eq!(never["source_type"][0]["identity"]["name"], "!");
    assert_eq!(never["adjusted_type"][0]["identity"]["name"], ty);
    assert!(
        never.get("kinds").is_none(),
        "debug strings are not structural adjustment evidence"
    );
}

fn check_bindings(entry: &Value, raw: &HirFnFacts, ty: &str) {
    let projected = entry["bindings"].as_array().unwrap();
    assert_eq!(projected.len(), raw.bindings.len());
    for (projected, original) in projected.iter().zip(&raw.bindings) {
        assert_eq!(projected["ty"]["normalized"], ty);
        let mut preserved = projected.clone();
        let mut original = serde_json::to_value(original).unwrap();
        preserved.as_object_mut().unwrap().remove("ty");
        original.as_object_mut().unwrap().remove("ty");
        assert_eq!(
            preserved, original,
            "binding source/identity/mutability/inventory preserved"
        );
    }
}

fn scalar_pair(arguments: &Value, ty: &str) {
    let arguments = arguments.as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    for argument in arguments {
        assert_eq!(argument["identity_status"], "known");
        assert_eq!(argument["identity"]["kind"], "builtin");
        assert_eq!(argument["identity"]["name"], ty);
    }
}

fn relation_census(entry: &Value, ty: &str) {
    assert_eq!(entry["status"], "projected");
    let comparisons = entry["comparisons"].as_array().unwrap();
    assert_eq!(comparisons.len(), 6);
    let expected = [
        ("==", "PartialEq::eq"),
        ("!=", "PartialEq::ne"),
        ("<", "PartialOrd::lt"),
        ("<=", "PartialOrd::le"),
        (">", "PartialOrd::gt"),
        (">=", "PartialOrd::ge"),
    ];
    for (observation, (operator, method)) in comparisons.iter().zip(expected) {
        assert_eq!(observation["operator"], operator);
        assert_eq!(observation["status"], "resolved");
        assert!(
            observation["requested"]["path"]
                .as_str()
                .unwrap()
                .ends_with(method)
        );
        assert_eq!(
            observation["requested_trait"],
            observation["selected_trait"]
        );
        scalar_pair(&observation["requested_arguments"], ty);
        scalar_pair(&observation["operand_types"], ty);
        check_operand_adjustments(observation);
        assert!(
            observation["selected_arguments"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(observation["result_type"][0]["identity"]["name"], "bool");
        assert_eq!(
            observation["signature"]["output"][0]["identity"]["name"],
            "bool"
        );
        let inputs = observation["signature"]["inputs"].as_array().unwrap();
        assert_eq!(inputs.len(), 2);
        for input in inputs {
            let identity = &input["identity"];
            assert_eq!(identity["kind"], "reference");
            assert_eq!(identity["mutable"], false);
            assert_eq!(identity["referent"]["name"], ty);
        }
    }
}

#[test]
fn all_six_comparisons_preserve_trait_census_and_structural_scalar_signatures() {
    let capture = capture(requests(json!([
        {"function":"relations", "arguments":["i32"]},
        {"function":"relations", "arguments":["u32"]}
    ])));
    assert!(
        capture.output.status.success(),
        "{}",
        String::from_utf8_lossy(&capture.output.stderr)
    );
    let snapshot = capture.snapshot("-scalar-instances.json");
    let entries = snapshot["requests"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    relation_census(&entries[0], "i32");
    relation_census(&entries[1], "u32");
    for (signed, unsigned) in entries[0]["comparisons"]
        .as_array()
        .unwrap()
        .iter()
        .zip(entries[1]["comparisons"].as_array().unwrap())
    {
        assert_ne!(
            signed["selected"]["definition_identity"],
            unsigned["selected"]["definition_identity"]
        );
    }
}

fn checked_entry(original: &HirFnFacts, mut entry: Value) -> Result<(), String> {
    let object = entry.as_object_mut().unwrap();
    object.remove("request");
    object.remove("status");
    let projection = serde_json::from_value(entry).map_err(|error| error.to_string())?;
    let checked = rstyle_pdg::scalar_instances::verify(original, &projection)?;
    assert_ne!(
        classify(checked.facts()).status,
        EligibilityVerdict::Eligible
    );
    assert_eq!(
        checked.facts().signature.generics,
        original.signature.generics
    );
    Ok(())
}

#[test]
fn independently_checked_projection_requires_complete_operation_and_adjustment_censuses() {
    let capture = capture(requests(json!([
        {"function":"minimum", "arguments":["i32"]},
        {"function":"minimum", "arguments":["u32"]}
    ])));
    assert!(capture.output.status.success());
    let raw = capture.snapshot("-pre-helper-certification.json");
    let facts: Vec<HirFnFacts> = serde_json::from_value(raw["facts"].clone()).unwrap();
    let original = facts.iter().find(|facts| facts.name == "minimum").unwrap();
    let snapshot = capture.snapshot("-scalar-instances.json");
    let entries = snapshot["requests"].as_array().unwrap();
    for entry in entries {
        checked_entry(original, entry.clone()).unwrap();
        for field in ["comparisons", "adjustments"] {
            let mut missing = entry.clone();
            missing[field].as_array_mut().unwrap().pop();
            assert!(checked_entry(original, missing).is_err(), "missing {field}");
            let mut duplicate = entry.clone();
            let records = duplicate[field].as_array_mut().unwrap();
            records.push(records[0].clone());
            assert!(
                checked_entry(original, duplicate).is_err(),
                "duplicate {field}"
            );
        }
    }
    let mut swapped = entries[0].clone();
    swapped["arguments"] = entries[1]["arguments"].clone();
    assert!(checked_entry(original, swapped).is_err());
    let mut swapped = entries[0].clone();
    swapped["comparisons"] = entries[1]["comparisons"].clone();
    assert!(checked_entry(original, swapped).is_err());
    let mut changed = original.clone();
    changed
        .build
        .frontend_options
        .insert("definition_identity".into(), json!("other"));
    assert!(checked_entry(&changed, entries[0].clone()).is_err());
}
