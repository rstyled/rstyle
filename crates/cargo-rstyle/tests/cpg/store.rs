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

#[test]
fn an_unpacked_run_packs_again_to_the_same_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let emitted = emitted_crate(dir.path());
    let store = dir.path().join("store").display().to_string();
    let pack = |capture: &Path| -> String {
        let capture = capture.display().to_string();
        let output = succeeded(
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
        let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
        summary["manifest"].as_str().unwrap().to_owned()
    };
    let first = pack(&emitted);
    let unpacked = dir.path().join("unpacked");
    let out = unpacked.display().to_string();
    succeeded(
        dir.path(),
        &[
            "rstyle",
            "cpg",
            "unpack",
            "--store",
            &store,
            "--manifest",
            &first,
            "--out",
            &out,
        ],
    );
    // The unpacked run is read through its documents manifest, and holds what was packed.
    assert_eq!(pack(&unpacked), first);
}

/// `rstyle` in a new directory's store, which must fail; returns its error output.
fn failed(dir: &Path, arguments: &[&str]) -> String {
    let output = rstyle(dir, arguments);
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn pack_names_a_run_directory_it_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store").display().to_string();
    let missing = dir.path().join("missing").display().to_string();
    let error = failed(
        dir.path(),
        &[
            "rstyle",
            "cpg",
            "pack",
            "--capture",
            &missing,
            "--store",
            &store,
        ],
    );
    assert!(error.contains("missing"), "{error}");
}

#[test]
fn pack_names_a_document_without_its_unit() {
    let dir = tempfile::tempdir().unwrap();
    let documents = dir.path().join("documents");
    std::fs::create_dir_all(&documents).unwrap();
    std::fs::write(
        documents.join("document-0.cpg.json"),
        b"{\"layout\": \"bundled\"}",
    )
    .unwrap();
    let store = dir.path().join("store").display().to_string();
    let capture = documents.display().to_string();
    let error = failed(
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
    assert!(error.contains("document-0.cpg.json"), "{error}");
}

#[test]
fn unpack_refuses_an_object_that_is_not_a_run_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let objects = rstyle_core::cpg::store::Objects::new(&dir.path().join("store/objects"));
    let name = objects.put(b"{\"schema\": \"something else\"}").unwrap();
    let store = dir.path().join("store").display().to_string();
    let out = dir.path().join("out").display().to_string();
    let arguments = [
        "rstyle",
        "cpg",
        "unpack",
        "--store",
        &store,
        "--manifest",
        &name,
        "--out",
        &out,
    ];
    assert!(failed(dir.path(), &arguments).contains("is not a run manifest"));
}

#[test]
fn pack_names_a_sealed_document_without_its_unit() {
    use rstyle_core::cpg::transport::{self, DOCUMENTS_MANIFEST, DocumentManifest, Encoding};
    let dir = tempfile::tempdir().unwrap();
    let documents = dir.path().join("documents");
    std::fs::create_dir_all(&documents).unwrap();
    let seal = transport::write_json(
        &documents.join("document-000000.cpg.json.zst"),
        Encoding::Zstd,
        &json!({"layout": "bundled"}),
    )
    .unwrap();
    let manifest = serde_json::to_vec(&DocumentManifest::of(vec![seal])).unwrap();
    std::fs::write(documents.join(DOCUMENTS_MANIFEST), manifest).unwrap();
    let store = dir.path().join("store").display().to_string();
    let capture = documents.display().to_string();
    let arguments = [
        "rstyle",
        "cpg",
        "pack",
        "--capture",
        &capture,
        "--store",
        &store,
    ];
    assert!(failed(dir.path(), &arguments).contains("document-000000.cpg.json.zst"));
}

#[test]
fn a_unit_whose_documents_are_unchanged_reuses_its_pack() {
    let dir = tempfile::tempdir().unwrap();
    let emitted = emitted_crate(dir.path());
    let store = dir.path().join("store");
    let (capture, kept) = (emitted.display().to_string(), store.display().to_string());
    let arguments = [
        "rstyle",
        "cpg",
        "pack",
        "--capture",
        &capture,
        "--store",
        &kept,
    ];
    let first: Value = serde_json::from_slice(&succeeded(dir.path(), &arguments).stdout).unwrap();
    let objects = || -> usize { walk(&store.join("objects")).len() };
    let before = objects();
    let second: Value = serde_json::from_slice(&succeeded(dir.path(), &arguments).stdout).unwrap();
    assert_eq!(second["manifest"], first["manifest"]);
    assert_eq!(objects(), before, "the second pack wrote nothing new");

    // A unit whose pack is gone from the store is packed again.
    for object in walk(&store.join("objects")) {
        std::fs::remove_file(object).unwrap();
    }
    let third: Value = serde_json::from_slice(&succeeded(dir.path(), &arguments).stdout).unwrap();
    assert_eq!(third["manifest"], first["manifest"]);
    assert_eq!(objects(), before);
}

/// Every file under `directory`.
fn walk(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    entries
        .map(|entry| entry.unwrap().path())
        .flat_map(|path| match path.is_dir() {
            true => walk(&path),
            false => vec![path],
        })
        .collect()
}
