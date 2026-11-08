use super::*;
use rstyle_core::cpg::transport::{self, DocumentManifest, Encoding};

#[test]
fn cpg_routes_accept_both_storage_encodings_and_verify_published_seals() {
    for encoding in [Encoding::Json, Encoding::Zstd] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join(format!("fixture{}", encoding.suffix()));
        let seal = transport::write_json(&path, encoding, &example()).unwrap();
        let manifest = DocumentManifest::of(vec![seal]);
        std::fs::write(
            directory.path().join(transport::DOCUMENTS_MANIFEST),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        for route in ["check", "patterns"] {
            let output = Command::new(cargo_bin("cargo-rstyle"))
                .args([
                    "rstyle",
                    "cpg",
                    route,
                    "--format",
                    "json",
                    path.to_str().unwrap(),
                ])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(0), "{output:?}");
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            if route == "check" {
                assert_eq!(report["summary"]["functions"], 3);
            }
        }
        let mut changed = std::fs::read(&path).unwrap();
        changed[0] ^= 1;
        std::fs::write(&path, changed).unwrap();
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args(["rstyle", "cpg", "check", path.to_str().unwrap()])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("disagrees with its manifest"),
            "{output:?}"
        );
    }
}
