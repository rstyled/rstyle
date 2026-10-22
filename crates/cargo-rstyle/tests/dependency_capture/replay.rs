//! Independently reopen the exact dependency closure, including artifact attacks.
use super::*;

fn replay(directory: &Path, run: &str) -> (Output, Value) {
    let output = rstyle(
        directory,
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--replay-run",
            run,
        ],
    );
    let report = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    (output, report)
}

pub(super) fn check(directory: &Path, captured: &Value) {
    let run = captured["run_directory"].as_str().unwrap();
    let (output, imported) = replay(directory, run);
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(imported["verification_policy"], "strict-replay");
    assert_eq!(imported["assessment_complete"], true);
    for name in ["caller", "local", "selected_value"] {
        assert_eq!(
            named(&imported, name)["outcome"],
            "admitted",
            "{}",
            named(&imported, name)
        );
    }
    let path = Path::new(run).join("dependency-closure.json");
    let original = std::fs::read(&path).unwrap();
    let manifest: Value = serde_json::from_slice(&original).unwrap();
    for field in [
        "extern_alias",
        "local_compiled_crate",
        "artifact",
        "callee",
        "caller",
    ] {
        let mut changed = manifest.clone();
        changed["associations"][0][field] = Value::Null;
        std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        let (output, _) = replay(directory, run);
        assert!(!output.status.success(), "accepted changed {field}");
    }
    std::fs::write(&path, original).unwrap();
    missing_artifact(directory, run, &manifest);
    resealed_replacement(directory, run, &manifest);
    missing_owner(directory, run, &manifest);
}

fn missing_artifact(directory: &Path, run: &str, manifest: &Value) {
    let artifact = manifest["associations"][0]["artifact"]["path"]
        .as_str()
        .unwrap();
    let original = std::fs::read(artifact).unwrap();
    std::fs::write(artifact, b"independent replacement metadata").unwrap();
    let (output, _) = replay(directory, run);
    assert!(
        !output.status.success(),
        "accepted stale dependency artifact"
    );
    std::fs::remove_file(artifact).unwrap();
    let (output, _) = replay(directory, run);
    assert!(
        !output.status.success(),
        "accepted missing dependency artifact"
    );
    std::fs::write(artifact, original).unwrap();
    assert!(replay(directory, run).0.status.success());
}

fn resealed_replacement(directory: &Path, run: &str, manifest: &Value) {
    let artifact = manifest["associations"][0]["artifact"]["path"]
        .as_str()
        .unwrap();
    let cargo = std::fs::read_to_string(Path::new(run).join("cargo.jsonl")).unwrap();
    let replacement = cargo
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|record| record["reason"] == "compiler-artifact")
        .flat_map(|record| record["filenames"].as_array().unwrap().clone())
        .find(|path| {
            path.as_str()
                .is_some_and(|path| path.ends_with(".rmeta") && path != artifact)
        })
        .unwrap();
    let bytes = std::fs::read(replacement.as_str().unwrap()).unwrap();
    let original = std::fs::read(artifact).unwrap();
    std::fs::write(artifact, &bytes).unwrap();
    let mut changed = manifest.clone();
    changed["associations"][0]["artifact"]["bytes"] = bytes.len().into();
    changed["associations"][0]["artifact"]["checksum"] =
        rstyle_pdg::unit_store::transport_checksum(&bytes).into();
    let path = Path::new(run).join("dependency-closure.json");
    std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(
        !replay(directory, run).0.status.success(),
        "resealed metadata from another compiled unit must reject"
    );
    std::fs::write(artifact, original).unwrap();
    std::fs::write(path, serde_json::to_vec(manifest).unwrap()).unwrap();
}

fn missing_owner(directory: &Path, run: &str, manifest: &Value) {
    let path = Path::new(run).join("dependency-closure.json");
    let mut changed = manifest.clone();
    changed["ledgers"].as_array_mut().unwrap().pop();
    changed["namespaces"].as_array_mut().unwrap().pop();
    changed["associations"] = serde_json::json!([]);
    std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(
        !replay(directory, run).0.status.success(),
        "selected dependency omission must reject"
    );
    std::fs::write(path, serde_json::to_vec(manifest).unwrap()).unwrap();
}
