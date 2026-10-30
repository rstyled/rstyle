use super::*;

const PRODUCT: &str = "RSTYLE_DIAGNOSTIC_PRODUCT_INSTANCE_REQUESTS";
const REFERENCE: &str = "RSTYLE_DIAGNOSTIC_REFERENCE_INSTANCE_REQUESTS";

fn failure(variable: &str, contents: Option<&str>, profile: &str, snapshots: bool) -> String {
    let directory = clone_workspace();
    let retained = tempfile::tempdir().unwrap();
    let path = retained.path().join("requests.json");
    if let Some(contents) = contents {
        std::fs::write(&path, contents).unwrap();
    }
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                profile,
            ])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY", "1")
            .env_remove(PRODUCT)
            .env_remove(REFERENCE)
            .env(variable, &path)
            .env_remove("RSTYLE_DIAGNOSTIC_FACTS_DIR")
            .envs(snapshots.then_some(("RSTYLE_DIAGNOSTIC_FACTS_DIR", retained.path())))
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["capture_complete"], false);
    assert!(report["functions"].as_array().unwrap().is_empty());
    assert!(std::fs::read_dir(retained.path()).unwrap().all(|entry| {
        let path = entry.unwrap().path();
        let name = path.to_string_lossy();
        !name.ends_with("-product-instances.json") && !name.ends_with("-reference-instances.json")
    }));
    report["capture_failures"].to_string()
}

#[test]
fn request_loading_errors_precede_context_validation_and_snapshot_publication() {
    for variable in [PRODUCT, REFERENCE] {
        let missing = failure(variable, None, "hir-helpers-v1", true);
        assert!(
            missing.contains(&std::io::Error::from_raw_os_error(2).to_string()),
            "{missing}"
        );
        assert!(!missing.contains("Unregistered"), "{missing}");
        let malformed = failure(variable, Some("{"), "hir-data-v3", false);
        assert!(malformed.contains("EOF while parsing"), "{malformed}");
        assert!(!malformed.contains("snapshot directory"), "{malformed}");
    }
}

fn request(product: bool, crate_name: &str) -> Value {
    let (lens, entry) = if product {
        (
            "product",
            json!({"function":"absent", "arguments":["Span"]}),
        )
    } else {
        ("reference", json!({"function":"absent", "haystack":"str"}))
    };
    json!({
        "identity":format!("rstyle.diagnostic.{lens}-instance-requests/1"),
        "crate_name":crate_name, "requests":[entry]
    })
}

#[test]
fn request_loading_retains_each_closed_schema_and_reference_context_validation() {
    for (variable, product) in [(PRODUCT, true), (REFERENCE, false)] {
        let mut unknown = request(product, "clone_fixture");
        unknown["unexpected"] = json!(true);
        let reason = failure(variable, Some(&unknown.to_string()), "hir-data-v3", true);
        assert!(reason.contains("unknown field"), "{reason}");
    }
    let wrong = request(false, "another_crate").to_string();
    let reason = failure(REFERENCE, Some(&wrong), "hir-data-v3", true);
    assert!(
        reason.contains("Unregistered reference-instance diagnostic context or requests"),
        "{reason}"
    );
}
