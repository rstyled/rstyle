use super::*;

fn documents() -> [(String, &'static [u8]); 2] {
    [("cfg".into(), b"abc"), ("pdg".into(), b"")]
}

fn manifest() -> Manifest {
    serde_json::from_value(serde_json::json!({"documents": [
        {"key":"pdg", "checksum":"xxh3-64:2d06800538d394c2"},
        {"key":"cfg", "checksum":"xxh3-64:78af5f94892f3950"}
    ]}))
    .unwrap()
}

#[test]
fn new_publications_use_the_existing_transport_checksum() {
    let identity = identity("cfg".into(), b"abc");
    assert_eq!(identity.checksum, "xxh3-64:78af5f94892f3950");
    let wire = serde_json::to_value(identity).unwrap();
    assert_eq!(wire["checksum"], "xxh3-64:78af5f94892f3950");
    assert!(wire.get("sha256").is_none());
    verify_manifest(&manifest(), documents()).unwrap();
}

#[test]
fn historical_sha_manifests_remain_readable() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({"documents": [
        {"key":"cfg", "sha256":"sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"},
        {"key":"pdg", "sha256":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}
    ]})).unwrap();
    verify_manifest(&manifest, documents()).unwrap();
    assert!(verify_manifest(&manifest, [("cfg".into(), b"abd"), ("pdg".into(), b"")]).is_err());
}

#[test]
fn checksums_reject_changed_bytes_keys_cardinality_and_unknown_algorithms() {
    let original = manifest();
    assert!(verify_manifest(&original, [("cfg".into(), b"abd"), ("pdg".into(), b"")]).is_err());
    for checksum in [
        "sha256:wrong",
        "xxh3-64:wrong",
        "unknown:78af5f94892f3950",
        "",
    ] {
        let mut changed = original.clone();
        changed.documents[0].checksum = checksum.into();
        assert!(verify_manifest(&changed, documents()).is_err());
    }
    let mut changed = original.clone();
    changed.documents[0].key = "cfg".into();
    assert!(verify_manifest(&changed, documents()).is_err());
    let mut changed = original.clone();
    changed.documents.pop();
    assert!(verify_manifest(&changed, documents()).is_err());
    let mut changed = original.clone();
    changed.documents.push(original.documents[0].clone());
    assert!(verify_manifest(&changed, documents()).is_err());
}

#[test]
fn conflicting_historical_and_new_checksum_fields_are_rejected() {
    let result = serde_json::from_value::<DocumentIdentity>(serde_json::json!({
        "key":"cfg", "checksum":"xxh3-64:78af5f94892f3950", "sha256":"sha256:wrong"
    }));
    assert!(result.is_err());
}
