use super::*;
use serde_json::{Value, json};
use std::fs;

fn example() -> Value {
    serde_json::from_str(include_str!("../testdata/example.cpg.json")).unwrap()
}

#[test]
fn both_encodings_preserve_the_real_document_and_seal_both_representations() {
    let directory = tempfile::tempdir().unwrap();
    for encoding in [Encoding::Json, Encoding::Zstd] {
        let path = directory
            .path()
            .join(format!("fixture{}", encoding.suffix()));
        let seal = write_json(&path, encoding, &example()).unwrap();
        let stored = fs::read(&path).unwrap();
        seal.verify(&stored).unwrap();
        let value: Value = read_sealed(directory.path(), &seal).unwrap();
        assert_eq!(value, example());
        let typed: crate::cpg::model::Document = read_sealed(directory.path(), &seal).unwrap();
        crate::cpg::layout::identity(&typed).unwrap();
        let manifest = DocumentManifest::of(vec![seal]);
        assert_eq!(
            DocumentManifest::read(&serde_json::to_vec(&manifest).unwrap()).unwrap(),
            manifest
        );
    }
}

#[test]
fn a_legacy_plain_capture_remains_readable() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = serde_json::to_vec(&example()).unwrap();
    fs::write(directory.path().join("document-000000.cpg.json"), &bytes).unwrap();
    let manifest = json!({"schema":"rstyle.cpg-documents.manifest/1", "documents":[{
        "file":"document-000000.cpg.json", "bytes":bytes.len(),
        "checksum":format!("xxh3-64:{:016x}", xxhash_rust::xxh3::xxh3_64(&bytes))}]});
    let manifest = DocumentManifest::read(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert_eq!(
        read_sealed::<Value>(directory.path(), &manifest.documents[0]).unwrap(),
        example()
    );
}

#[test]
fn streamed_input_is_not_reformatted_and_failed_writes_leave_no_document() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("stream.cpg.json.zst");
    let bytes =
        b"{\n \"unicode\": \"lambda\", \"values\": [null, -1, true, 18446744073709551615]\n}\n";
    let seal = publish(&path, Encoding::Zstd, |writer| {
        for chunk in bytes.chunks(3) {
            writer.write_all(chunk)?;
        }
        Ok(())
    })
    .unwrap();
    let reconstructed = zstd::stream::decode_all(fs::read(&path).unwrap().as_slice()).unwrap();
    assert_eq!(reconstructed, bytes);
    assert_eq!(seal.decoded_bytes, Some(bytes.len() as u64));
    for input in [b"{\"unfinished\":".as_slice(), b"{} {}", b"[]", b"null"] {
        let failed = directory.path().join("failed.cpg.json.zst");
        assert!(publish(&failed, Encoding::Zstd, |writer| writer.write_all(input)).is_err());
        assert!(!failed.exists());
    }
    let failed = directory.path().join("interrupted.cpg.json.zst");
    assert!(
        publish(&failed, Encoding::Zstd, |writer| {
            writer.write_all(b"{")?;
            Err(std::io::Error::other("frontend failed"))
        })
        .is_err()
    );
    assert!(!failed.exists());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn stored_corruption_truncation_and_incorrect_decoded_seals_fail() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("checked.cpg.json.zst");
    let seal = write_json(&path, Encoding::Zstd, &example()).unwrap();
    let original = fs::read(&path).unwrap();
    let mut wrong = seal.clone();
    wrong.decoded_bytes = Some(1);
    assert!(read_sealed::<Value>(directory.path(), &wrong).is_err());
    wrong = seal.clone();
    wrong.decoded_checksum = Some("sha256:wrong".into());
    assert!(read_sealed::<Value>(directory.path(), &wrong).is_err());
    wrong = seal.clone();
    wrong.checksum = "sha256:wrong".into();
    assert!(read_sealed::<Value>(directory.path(), &wrong).is_err());
    for length in [original.len() - 1, original.len() / 2] {
        fs::write(&path, &original[..length]).unwrap();
        assert!(read_json::<Value>(&path).is_err());
    }
    let mut corrupt = original;
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    fs::write(&path, corrupt).unwrap();
    assert!(read_json::<Value>(&path).is_err());
}

#[test]
fn publication_never_overwrites_a_completed_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.cpg.json.zst");
    write_json(&path, Encoding::Zstd, &json!({"first":true})).unwrap();
    let original = fs::read(&path).unwrap();
    assert!(write_json(&path, Encoding::Zstd, &json!({"second":true})).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn manifests_reject_ambiguous_paths_and_inconsistent_encodings() {
    let seal = SealedDocument::new(0, b"{}");
    for file in [
        "..",
        ".",
        "/tmp/outside.cpg.json",
        "../outside.cpg.json",
        "nested\\outside.cpg.json",
    ] {
        let mut bad = seal.clone();
        bad.file = file.into();
        assert!(
            DocumentManifest::read(&serde_json::to_vec(&DocumentManifest::of(vec![bad])).unwrap())
                .is_err()
        );
    }
    let duplicate = DocumentManifest::of(vec![seal.clone(), seal.clone()]);
    assert!(DocumentManifest::read(&serde_json::to_vec(&duplicate).unwrap()).is_err());
    let mut bad = seal;
    bad.encoding = Encoding::Zstd;
    assert!(
        DocumentManifest::read(&serde_json::to_vec(&DocumentManifest::of(vec![bad])).unwrap())
            .is_err()
    );
}
