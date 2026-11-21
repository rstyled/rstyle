//! `rstyle cpg pack` keeps a run's documents as one pack per unit in a local store, and
//! `rstyle cpg unpack` writes them back as documents any command reads.

use super::*;

/// A crate's documents, as `cpg emit` writes them into `dir/cpg`.
fn emitted_crate(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"packed\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/lib.rs"),
        "pub fn total(items: &[u32]) -> u32 {\n    items.iter().sum()\n}\n\n// A note above.\npub fn first(items: &[u32]) -> Option<u32> {\n    items.first().copied()\n}\n",
    )
    .unwrap();
    let emitted = dir.join("cpg");
    let emit = rstyle(
        dir,
        &["rstyle", "cpg", "emit", "--out", emitted.to_str().unwrap()],
    );
    assert_eq!(emit.status.code(), Some(0), "{emit:?}");
    emitted
}

/// `rstyle` in `dir` with `arguments`, which must succeed.
fn succeeded(dir: &Path, arguments: &[&str]) -> Output {
    let output = rstyle(dir, arguments);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    output
}

#[test]
fn a_packed_run_unpacks_to_the_documents_it_was_packed_from() {
    let dir = tempfile::tempdir().unwrap();
    let emitted = emitted_crate(dir.path());
    let store = dir.path().join("store").display().to_string();
    let capture = emitted.display().to_string();
    let pack = succeeded(
        dir.path(),
        &[
            "rstyle",
            "cpg",
            "pack",
            "--capture",
            &capture,
            "--store",
            &store,
        ],
    );
    let summary: Value = serde_json::from_slice(&pack.stdout).unwrap();
    assert_eq!(summary["units"], 1);

    let unpacked = dir.path().join("unpacked");
    let out = unpacked.display().to_string();
    let manifest = summary["manifest"].as_str().unwrap();
    succeeded(
        dir.path(),
        &[
            "rstyle",
            "cpg",
            "unpack",
            "--store",
            &store,
            "--manifest",
            manifest,
            "--out",
            &out,
        ],
    );
    assert_eq!(
        parsed(&documents_in(&unpacked)),
        parsed(&documents_in(&emitted))
    );

    // A command reads the unpacked run through its manifest, as it reads any saved run.
    let check = rstyle(dir.path(), &["rstyle", "cpg", "check", "--capture", &out]);
    assert!(matches!(check.status.code(), Some(0 | 1)), "{check:?}");
}

#[test]
fn a_run_whose_documents_disagree_is_not_packed() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = example();
    first["unit"]["types"] = json!([{"id": "type:u8", "kind": "integer"}]);
    let mut second = example();
    second["unit"]["functions"] = json!([]);
    second["unit"]["types"] =
        json!([{"id": "type:u8", "kind": "integer", "modifiers": ["rust:copy"]}]);
    for (position, document) in [first, second].iter().enumerate() {
        std::fs::write(
            dir.path().join(format!("document-{position}.cpg.json")),
            serde_json::to_vec(document).unwrap(),
        )
        .unwrap();
    }
    let store = dir.path().join("store");
    let pack = rstyle(
        dir.path(),
        &[
            "rstyle",
            "cpg",
            "pack",
            "--capture",
            dir.path().to_str().unwrap(),
            "--store",
            store.to_str().unwrap(),
        ],
    );
    assert_ne!(pack.status.code(), Some(0), "{pack:?}");
    assert!(
        String::from_utf8_lossy(&pack.stderr).contains("disagree on types entry type:u8"),
        "{pack:?}"
    );
}
