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
    for input in [
        b"{\"unfinished\":".as_slice(),
        b"{} {}",
        b"[]",
        b"null",
        b"{\"x\":\"\xff\"}",
    ] {
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

/// A text read checks the seal of the stored bytes before it decodes them, then that of the
/// decoded ones, and a parsed read fails as the text read under it does.
#[test]
fn text_reads_check_both_seals_and_say_which_one_failed() {
    let directory = tempfile::tempdir().unwrap();
    let decoded = serde_json::to_string(&example()).unwrap();
    for encoding in [Encoding::Json, Encoding::Zstd] {
        let path = directory
            .path()
            .join(format!("checked{}", encoding.suffix()));
        let seal = write_json(&path, encoding, &example()).unwrap();
        assert_eq!(read_sealed_text(directory.path(), &seal).unwrap(), decoded);
        assert_eq!(read_json_text(&path).unwrap(), decoded);

        let failure = |seal: &SealedDocument| {
            let error = read_sealed_text(directory.path(), seal).unwrap_err();
            let parsed = read_sealed::<Value>(directory.path(), seal).unwrap_err();
            assert_eq!(parsed, error);
            error
        };
        let disagrees = format!(
            "Captured document {} disagrees with its manifest",
            seal.file
        );
        let mut wrong = seal.clone();
        wrong.checksum = "sha256:wrong".into();
        assert_eq!(failure(&wrong), disagrees);
        wrong = seal.clone();
        wrong.bytes += 1;
        assert_eq!(failure(&wrong), disagrees);
        wrong.bytes = seal.bytes - 1;
        assert_eq!(failure(&wrong), "CPG transport exceeds its declared size");
        wrong = seal.clone();
        wrong.decoded_checksum = Some("sha256:wrong".into());
        assert_eq!(
            failure(&wrong),
            format!(
                "{}: Decoded document {} disagrees with its manifest",
                path.display(),
                seal.file
            )
        );
        wrong = seal.clone();
        wrong.file = "missing.cpg.json".into();
        assert!(
            failure(&wrong).starts_with("missing captured artifact missing.cpg.json: "),
            "{}",
            failure(&wrong)
        );
    }
}

#[test]
fn decoding_stops_past_the_sealed_size_and_names_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("oversized.cpg.json.zst");
    let mut seal = write_json(&path, Encoding::Zstd, &example()).unwrap();
    seal.decoded_bytes = Some(1);
    assert_eq!(
        read_sealed_text(directory.path(), &seal).unwrap_err(),
        format!(
            "{}: CPG transport exceeds its declared size",
            path.display()
        )
    );
    let stored = fs::read(&path).unwrap();
    fs::write(&path, &stored[..stored.len() / 2]).unwrap();
    let error = read_json_text(&path).unwrap_err();
    assert!(
        error.starts_with(&format!("{}: ", path.display())),
        "{error}"
    );
}

/// Text that isn't UTF-8 fails the read, as it did when a document was parsed as it was read,
/// whether it is sealed or not.
#[test]
fn text_that_is_not_utf8_fails_its_read() {
    let directory = tempfile::tempdir().unwrap();
    for encoding in [Encoding::Json, Encoding::Zstd] {
        let path = directory
            .path()
            .join(format!("latin1{}", encoding.suffix()));
        let seal = publish(&path, encoding, |writer| writer.write_all(b"{\"x\":\"x\"}")).unwrap();
        let bytes = match encoding {
            Encoding::Json => b"{\"x\":\"\xff\"}".to_vec(),
            Encoding::Zstd => zstd::encode_all(b"{\"x\":\"\xff\"}".as_slice(), 3).unwrap(),
        };
        fs::write(&path, &bytes).unwrap();
        let expected = format!(
            "{}: invalid utf-8 sequence of 1 bytes from index 6",
            path.display()
        );
        assert_eq!(read_json_text(&path).unwrap_err(), expected);
        assert_eq!(read_json::<Value>(&path).unwrap_err(), expected);
        let resealed = SealedDocument {
            checksum: Observed::of(&bytes).unwrap().checksum(),
            bytes: bytes.len() as u64,
            decoded_checksum: Some(Observed::of(b"{\"x\":\"\xff\"}").unwrap().checksum()),
            ..seal
        };
        assert_eq!(
            read_sealed_text(directory.path(), &resealed).unwrap_err(),
            expected
        );
    }
}

/// A file that opens but can't be read, as a directory on Unix, fails its read and names it.
#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_read_names_itself() {
    let directory = tempfile::tempdir().unwrap();
    let unreadable = directory.path().join("document.cpg.json");
    fs::create_dir(&unreadable).unwrap();
    let expected = format!("{}: ", unreadable.display());
    let error = read_json_text(&unreadable).unwrap_err();
    assert!(error.starts_with(&expected), "{error}");
    let seal = SealedDocument {
        file: "document.cpg.json".into(),
        bytes: u64::MAX >> 1,
        ..SealedDocument::new(0, b"{}")
    };
    let error = read_sealed_text(directory.path(), &seal).unwrap_err();
    assert!(error.starts_with(&expected), "{error}");
}
