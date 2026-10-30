//! Concrete product observations retain original owners without granting admission.
use super::*;
use serde_json::json;
mod request_loading;

const SOURCE: &str = r#"
use std::ops::Range;
#[derive(Clone, Copy)]
pub struct Span { pub start: usize, pub end: usize }
pub struct Match { pub pattern: usize, pub span: Span }
impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Span { Span { start: range.start, end: range.end } }
}
impl Match {
    pub fn new<S: Into<Span>>(pattern: usize, span: S) -> Match {
        let span: Span = span.into();
        assert!(span.start <= span.end, "invalid match span");
        Match { pattern, span }
    }
    pub fn other<S: Into<Span>>(pattern: usize, span: S) -> Match {
        Match::new(pattern, span)
    }
}
pub struct Bad<T>(T);
impl<T> Bad<T> {
    pub fn new<S: Into<Span>>(pattern: usize, span: S) -> Match {
        Match::new(pattern, span)
    }
}
"#;

fn run(directory: &Path, snapshots: &Path, request: &Value) -> Output {
    let requests = snapshots.join("requests.json");
    std::fs::write(&requests, serde_json::to_vec(request).unwrap()).unwrap();
    diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
            ])
            .current_dir(directory)
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY", "1")
            .env("RSTYLE_DIAGNOSTIC_FACTS_DIR", snapshots)
            .env("RSTYLE_DIAGNOSTIC_PRODUCT_INSTANCE_REQUESTS", requests)
            .env_remove("RSTYLE_DIAGNOSTIC_REFERENCE_INSTANCE_REQUESTS")
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap()
}

fn files(directory: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with("-product-instances.json"))
        .collect()
}

#[test]
fn product_instances_preserve_original_binders_and_selected_conversions() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let snapshots = tempfile::tempdir().unwrap();
    let output = run(
        directory.path(),
        snapshots.path(),
        &json!({
            "identity":"rstyle.diagnostic.product-instance-requests/1",
            "crate_name":"clone_fixture",
            "requests":[
                {"function":"Match::new","arguments":["Span","Range<usize>"]},
                {"function":"Bad::<T>::new","arguments":["Span"]},
                {"function":"Match::other","arguments":["Span"]},
                {"function":"absent","arguments":["Span"]}
            ]
        }),
    );
    let inventory = inventory(&output, snapshots.path());
    assert_eq!(
        inventory["identity"],
        "rstyle.diagnostic.product-instance-bodies/1"
    );
    assert_eq!(inventory["admission_authority"], false);
    assert_eq!(
        inventory["scope"],
        "original associated Match constructor, Span/Range substitutions and selected Into/From observations only"
    );
    let entries = inventory["requests"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0]["status"], "observed", "{}", entries[0]);
    assert_eq!(entries[1]["status"], "observed", "{}", entries[1]);
    assert_eq!(entries[1]["instances"][0]["status"], "unsupported");
    assert_eq!(entries[2]["status"], "unsupported");
    assert_eq!(entries[3]["status"], "unavailable");
    let instances = entries[0]["instances"].as_array().unwrap();
    assert_eq!(instances.len(), 2);
    for (index, instance) in instances.iter().enumerate() {
        assert_eq!(instance["status"], "observed", "{instance}");
        original_owner(instance);
        conversion(instance, index == 1);
    }
}

fn inventory(output: &Output, snapshots: &Path) -> Value {
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success());
    assert_eq!(report["capture_complete"], false);
    assert!(report["functions"].as_array().unwrap().is_empty());
    let files = files(snapshots);
    assert_eq!(
        files.len(),
        1,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let inventory: Value = serde_json::from_slice(&std::fs::read(&files[0]).unwrap()).unwrap();
    assert!(std::fs::read_dir(snapshots).unwrap().all(|entry| {
        !entry
            .unwrap()
            .path()
            .to_string_lossy()
            .ends_with("-reference-instances.json")
    }));
    inventory
}

fn original_owner(instance: &Value) {
    let owners = instance["owners"].as_array().unwrap();
    assert_eq!(owners.len(), 2);
    assert!(owners[0]["parameters"].as_array().unwrap().is_empty());
    let parameters = owners[1]["parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0]["name"], "S");
    assert_eq!(parameters[0]["index"], 0);
    assert_eq!(owners[1]["parent"], owners[0]["definition"]);
    assert_eq!(
        instance["original_signature"]["inputs"][1],
        json!({
            "kind":"parameter", "index":0, "name":"S"
        })
    );
    let nodes = instance["nodes"].as_array().unwrap();
    assert!(
        nodes
            .iter()
            .any(|node| node["original"]["kind"] == "parameter" && node["original"]["name"] == "S")
    );
    assert_eq!(instance["projected_inputs"][1], instance["arguments"]);
}

fn conversion(instance: &Value, range: bool) {
    let conversions = instance["conversions"].as_array().unwrap();
    assert_eq!(conversions.len(), 1);
    let conversion = &conversions[0];
    assert_eq!(
        conversion["identity"],
        "rstyle.rust.product-into-from-observation/1"
    );
    assert_eq!(conversion["receiver_type"], instance["arguments"]);
    assert!(conversion["adjustments"].as_array().unwrap().is_empty());
    for (key, trait_name) in [("into", "Into"), ("from", "From")] {
        let selected = &conversion[key];
        assert_eq!(selected["requested_trait"]["crate"], "core");
        assert!(
            selected["requested_trait"]["path"]
                .as_str()
                .unwrap()
                .ends_with(trait_name)
        );
        assert_eq!(selected["requested_trait"], selected["selected_trait"]);
        assert_eq!(
            selected["requested_arguments"],
            selected["substituted_impl_trait_arguments"]
        );
        assert_eq!(
            selected["requested_signature"],
            selected["selected_signature"]
        );
        assert_eq!(selected["requested_signature"]["bound_variables"], 0);
    }
    assert_eq!(conversion["into"]["selected"]["crate"], "core");
    assert_eq!(conversion["from"]["selected"]["local"], range);
    let body = &instance["selected_local_from"];
    if range {
        assert_eq!(body["definition"], conversion["from"]["selected"]);
        assert_eq!(body["source_scan_complete"], true);
        assert!(body["source_diagnostics"].as_array().unwrap().is_empty());
        assert_eq!(
            body["source"]["bytes"],
            "fn from(range: Range<usize>) -> Span { Span { start: range.start, end: range.end } }"
        );
    } else {
        assert!(body.is_null());
        assert_eq!(conversion["from"]["selected"]["crate"], "core");
    }
}

#[test]
fn product_diagnostic_wrong_crate_cannot_publish_an_inventory() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let snapshots = tempfile::tempdir().unwrap();
    let output = run(
        directory.path(),
        snapshots.path(),
        &json!({
            "identity":"rstyle.diagnostic.product-instance-requests/1",
            "crate_name":"another_crate",
            "requests":[{"function":"Match::new","arguments":["Span"]}]
        }),
    );
    assert!(!output.status.success());
    assert!(files(snapshots.path()).is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["capture_complete"], false);
    assert!(report["functions"].as_array().unwrap().is_empty());
    assert!(
        report["capture_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| {
                failure
                    .as_str()
                    .unwrap()
                    .contains("Unregistered product diagnostic context or oversized requests")
            }),
        "{}",
        report["capture_failures"]
    );
}
