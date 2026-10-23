//! Original impl binders and exact standard conversions remain diagnostic only.
use super::*;
use serde_json::json;

const SOURCE: &str = r#"
pub struct Input<'h> { pub haystack: &'h [u8], pub end: usize }
impl<'h> Input<'h> {
    pub fn new<H: ?Sized + AsRef<[u8]>>(haystack: &'h H) -> Input<'h> {
        let haystack = haystack.as_ref();
        Input { haystack, end: haystack.len() }
    }
}
impl<'h, H: ?Sized + AsRef<[u8]>> From<&'h H> for Input<'h> {
    fn from(haystack: &'h H) -> Input<'h> { Input::new(haystack) }
}
pub fn ordinary(x: u8) -> u8 { x }
"#;

fn inventory() -> Value {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let snapshots = tempfile::tempdir().unwrap();
    let requests = snapshots.path().join("requests.json");
    let names = [
        "Input::<'h>::new",
        "<Input<'h> as std::convert::From<&'h H>>::from",
    ];
    let entries: Vec<_> = names
        .into_iter()
        .flat_map(|name| {
            ["[u8]", "str", "u8"].map(|haystack| {
                json!({
                    "function": name, "haystack": haystack,
                })
            })
        })
        .chain([json!({"function":"absent", "haystack":"str"})])
        .collect();
    std::fs::write(
        &requests,
        serde_json::to_vec(&json!({
            "identity":"rstyle.diagnostic.reference-instance-requests/1",
            "crate_name":"clone_fixture", "requests": entries,
        }))
        .unwrap(),
    )
    .unwrap();
    let output = run(directory.path(), snapshots.path(), &requests);
    let files: Vec<_> = std::fs::read_dir(snapshots.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.to_string_lossy()
                .ends_with("-reference-instances.json")
        })
        .collect();
    assert_eq!(
        files.len(),
        1,
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&std::fs::read(&files[0]).unwrap()).unwrap()
}

fn run(directory: &Path, snapshots: &Path, requests: &Path) -> Output {
    let output = diagnostic_process::output(
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
            .env("RSTYLE_DIAGNOSTIC_REFERENCE_INSTANCE_REQUESTS", requests)
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], false);
    assert!(
        report["functions"].as_array().unwrap().is_empty(),
        "{report}"
    );
    output
}

#[test]
fn original_reference_impl_instances_are_observations_without_admission() {
    let inventory = inventory();
    assert_eq!(inventory["admission_authority"], false);
    let entries = inventory["requests"].as_array().unwrap();
    assert_eq!(entries.len(), 7);
    for entry in entries {
        let haystack = entry["request"]["haystack"].as_str().unwrap();
        let name = entry["request"]["function"].as_str().unwrap();
        let expected = if name == "absent" {
            "unavailable"
        } else if haystack == "u8" {
            "unsupported"
        } else {
            "observed"
        };
        assert_eq!(entry["status"], expected, "{entry}");
        if expected == "observed" {
            assert!(
                entry["original"]["source"]["bytes"]
                    .as_str()
                    .unwrap()
                    .contains("'h H")
            );
            assert!(!entry["nodes"].as_array().unwrap().is_empty());
            provenance(entry);
            if name.ends_with("::new") {
                assert_eq!(entry["conversions"].as_array().unwrap().len(), 1);
                conversion(&entry["conversions"][0]);
            } else {
                assert_eq!(entry["calls"].as_array().unwrap().len(), 1);
            }
        }
    }
}

fn provenance(entry: &Value) {
    let provenance = &entry["provenance"];
    assert_eq!(provenance["complete"], true);
    let owners = provenance["owners"].as_array().unwrap();
    assert_eq!(owners.len(), 2);
    let indices: Vec<_> = owners
        .iter()
        .flat_map(|owner| {
            owner["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|parameter| parameter["index"].as_u64().unwrap())
        })
        .collect();
    assert_eq!(indices, [0, 1]);
    assert!(
        owners[0]["declaration"]["span"]["start"].as_u64().unwrap()
            < entry["original"]["fn_span"]["start"].as_u64().unwrap()
    );
    assert!(
        owners
            .iter()
            .any(|owner| !owner["clauses"].as_array().unwrap().is_empty())
    );
    let input = &provenance["signature"]["inputs"][0];
    assert_eq!(input["kind"], "reference");
    assert_eq!(input["lifetime"]["index"], 0);
    assert_eq!(input["referent"]["index"], 1);
    let output = &provenance["signature"]["output"];
    assert_eq!(output["kind"], "adt");
    assert_eq!(output["arguments"][0]["value"]["index"], 0);
}

fn conversion(conversion: &Value) {
    for key in [
        "requested",
        "requested_trait",
        "selected",
        "selected_impl",
        "selected_trait",
    ] {
        assert_eq!(conversion[key]["crate"], "core", "{conversion}");
        assert_eq!(conversion[key]["local"], false);
    }
    for key in ["requested_signature", "signature"] {
        assert_eq!(
            conversion[key]["receiver_region"],
            conversion[key]["result_region"]
        );
        assert_eq!(conversion[key]["bound_variables"], 1);
    }
}
