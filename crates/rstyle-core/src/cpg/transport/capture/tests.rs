use super::*;
use crate::cpg::transport::{Encoding, write_json};
use serde_json::{Value, json};
use std::fs;

fn capture(directory: &Path) -> Vec<SealedDocument> {
    let seals = [Encoding::Zstd, Encoding::Json]
        .into_iter()
        .enumerate()
        .map(|(i, encoding)| {
            write_json(
                &directory.join(format!("document-{i}{}", encoding.suffix())),
                encoding,
                &json!({"position":i}),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let manifest = serde_json::to_vec(&DocumentManifest::of(seals.clone())).unwrap();
    fs::write(directory.join(DOCUMENTS_MANIFEST), manifest).unwrap();
    seals
}

#[test]
fn one_manifest_snapshot_controls_order_seals_and_verified_totals() {
    let dir = tempfile::tempdir().unwrap();
    let seals = capture(dir.path());
    let mut reader = CaptureReader::open(dir.path()).unwrap();
    let manifest = fs::read(dir.path().join(DOCUMENTS_MANIFEST)).unwrap();
    assert_eq!(
        reader.progress().manifest_checksum,
        checksum(&manifest).unwrap()
    );
    assert!(reader.complete().is_err());
    fs::remove_file(dir.path().join(DOCUMENTS_MANIFEST)).unwrap();

    assert_eq!(
        reader.next_document::<Value>().unwrap(),
        Some(json!({"position":0}))
    );
    assert_eq!(reader.progress().verified_documents, 1);
    assert!(reader.complete().is_err());
    assert_eq!(reader.next().unwrap().unwrap(), json!({"position":1}));
    assert!(reader.next().is_none());
    reader.complete().unwrap();
    assert_eq!(reader.progress().verified_documents, 2);
    assert_eq!(
        reader.progress().stored_bytes,
        seals.iter().map(|s| s.bytes).sum::<u64>()
    );
    assert_eq!(
        reader.progress().decoded_bytes,
        seals.iter().map(|s| s.decoded_bytes.unwrap()).sum::<u64>()
    );
}

#[test]
fn missing_or_corrupt_documents_stop_the_capture_without_counting_failed_bytes() {
    for missing in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let seals = capture(dir.path());
        let mut reader = CaptureReader::open(dir.path()).unwrap();
        reader.next().unwrap().unwrap();
        let progress = reader.progress().clone();
        let path = dir.path().join(&seals[1].file);
        if missing {
            fs::remove_file(path).unwrap();
        } else {
            fs::write(path, b"changed").unwrap();
        }

        let error = reader.next().unwrap().unwrap_err();
        assert!(error.contains(&seals[1].file), "{error}");
        assert_eq!(reader.progress(), &progress);
        assert!(reader.next().is_none());
        assert!(reader.complete().is_err());
    }
}

#[test]
fn legacy_plain_seals_have_exact_decoded_totals() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"{\"legacy\":true}\n";
    fs::write(dir.path().join("legacy.cpg.json"), bytes).unwrap();
    let manifest = json!({"schema":"rstyle.cpg-documents.manifest/1", "documents":[{
        "file":"legacy.cpg.json", "bytes":bytes.len(),
        "checksum":format!("xxh3-64:{:016x}",xxhash_rust::xxh3::xxh3_64(bytes))
    }]});
    fs::write(
        dir.path().join(DOCUMENTS_MANIFEST),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let mut reader = CaptureReader::open(dir.path()).unwrap();
    assert_eq!(reader.next().unwrap().unwrap(), json!({"legacy":true}));
    reader.complete().unwrap();
    assert_eq!(reader.progress().stored_bytes, bytes.len() as u64);
    assert_eq!(reader.progress().decoded_bytes, bytes.len() as u64);
}

#[test]
fn absence_of_a_nonempty_manifest_cannot_certify_a_capture() {
    let dir = tempfile::tempdir().unwrap();
    assert!(CaptureReader::open(dir.path()).is_err());
    for bytes in [
        b"{".to_vec(),
        serde_json::to_vec(&DocumentManifest::of(Vec::new())).unwrap(),
    ] {
        fs::write(dir.path().join(DOCUMENTS_MANIFEST), bytes).unwrap();
        assert!(CaptureReader::open(dir.path()).is_err());
    }
}
