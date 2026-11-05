use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Inputs(PathBuf);

impl Inputs {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "cpg-storage-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn add(&self, name: &str, value: &serde_json::Value) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
}

impl Drop for Inputs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn archive(values: &[serde_json::Value]) -> (Archive, normalize::Stats) {
    let inputs = Inputs::new();
    let mut normalizer = normalize::Normalizer::new();
    for (index, value) in values.iter().enumerate() {
        let path = inputs.add(&format!("{index}.json"), value);
        normalizer.add(&path).unwrap();
    }
    normalizer.finish().unwrap()
}

#[test]
fn numeric_extremes_unicode_and_empty_values_survive_every_codec() {
    let value = json!({"null":null,"false":false,"true":true,"negative":i64::MIN,
        "large":u64::MAX,"zero":0,"float":1.25,"negative_zero":-0.0,
        "empty_object":{},"empty_array":[],"unicode":"λ🦀\u{0}",
        "ordered":[3,1,2],"nested":[{"name":"shared"},{"name":"shared"}]});
    let (expected, stats) = archive(std::slice::from_ref(&value));
    assert_eq!(expected.expand(expected.documents[0].root), value);
    assert!(stats.unique_values < stats.input_values as usize);
    assert!(stats.unique_strings < stats.input_strings as usize);
    for codec in [
        Codec::Json,
        Codec::Flatbuffers,
        Codec::Flexbuffers,
        Codec::Protobuf,
    ] {
        let bytes = codecs::encode(codec, &expected).unwrap();
        let decoded = codecs::decode(codec, &bytes).unwrap();
        assert_eq!(decoded, expected, "{codec:?}");
        assert_eq!(decoded.digest(), expected.digest());
        let compressed = zstd::stream::encode_all(bytes.as_slice(), 3).unwrap();
        let expanded = zstd::stream::decode_all(compressed.as_slice()).unwrap();
        assert_eq!(expanded.as_slice(), bytes.as_slice());
    }
}

#[test]
fn identical_display_ids_with_different_contexts_are_not_merged() {
    let first = json!({"unit":{"id":"a","target":"32bit","types":[{"id":"type:usize","bits":32}],
        "functions":[{"name":"same","span":{"start":1,"end":2}},{"name":"same","span":{"start":3,"end":4}}]}});
    let second = json!({"unit":{"id":"b","target":"64bit","types":[{"id":"type:usize","bits":64}],
        "functions":[]}});
    let (shared, _) = archive(&[first.clone(), second.clone()]);
    assert_eq!(shared.expand(shared.documents[0].root), first);
    assert_eq!(shared.expand(shared.documents[1].root), second);
    assert_ne!(shared.documents[0].root, shared.documents[1].root);
}

#[test]
fn actual_cpg_fixture_round_trips_without_omitting_extensions_or_sources() {
    let value = serde_json::from_str(include_str!(
        "../../../crates/rstyle-core/src/cpg/testdata/example.cpg.json"
    ))
    .unwrap();
    let (shared, _) = archive(std::slice::from_ref(&value));
    for codec in [
        Codec::Json,
        Codec::Flatbuffers,
        Codec::Flexbuffers,
        Codec::Protobuf,
    ] {
        let bytes = codecs::encode(codec, &shared).unwrap();
        let decoded = codecs::decode(codec, &bytes).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.expand(decoded.documents[0].root), value);
    }
}

#[test]
fn archive_validation_rejects_cycles_and_out_of_range_references() {
    let (mut shared, _) = archive(&[json!([1, 2])]);
    let root = shared.documents[0].root as usize;
    let start = shared.values[root].start as usize;
    shared.edges[start] = root as u32;
    assert!(shared.validate().unwrap_err().contains("Non-topological"));
    shared.documents[0].path = u32::MAX;
    assert!(shared.validate().is_err());
}

#[test]
fn truncated_binary_inputs_are_rejected() {
    let (shared, _) = archive(&[json!({"key":[1,2,3]})]);
    for codec in [Codec::Flatbuffers, Codec::Flexbuffers, Codec::Protobuf] {
        let bytes = codecs::encode(codec, &shared).unwrap();
        assert!(
            codecs::decode(codec, &bytes[..bytes.len() / 2]).is_err(),
            "{codec:?}"
        );
    }
}

#[test]
fn original_json_trials_preserve_pretty_inputs_and_document_boundaries() {
    let inputs = Inputs::new();
    let first = inputs.0.join("pretty.json");
    std::fs::write(
        &first,
        "{\n  \"unicode\": \"λ🦀\", \"nested\": [null, -1, true]\n}\n",
    )
    .unwrap();
    let second = inputs.add(
        "second.json",
        &json!({"max":u64::MAX,"float":-0.0,"unknown":{"x":42}}),
    );
    let paths = vec![first, second];
    let inventory = original::inventory(&paths).unwrap();
    for level in [None, Some(3)] {
        let wire = inputs.0.join(if level.is_some() {
            "compressed"
        } else {
            "plain"
        });
        let encoded = original::trial(
            Operation::Encode,
            &paths,
            (&wire, original::ReadModel::Json),
            level,
        )
        .unwrap();
        let decoded = original::trial(
            Operation::Decode,
            &paths,
            (&wire, original::ReadModel::Json),
            level,
        )
        .unwrap();
        assert_eq!(encoded.archive_digest, inventory.original_digest);
        assert_eq!(decoded.archive_digest, inventory.original_digest);
        assert_eq!(encoded.wire_sha256, decoded.wire_sha256);
        assert_eq!(encoded.wire_bytes, decoded.wire_bytes);
        assert_eq!(encoded.uncompressed_bytes, decoded.uncompressed_bytes);
        assert_eq!(decoded.representation, "original-json-dom");
        assert!(wire.join("document-000001.bin").is_file());
    }
}

#[test]
fn original_document_digest_detects_changes_in_later_documents() {
    let inputs = Inputs::new();
    let first = inputs.add("first.json", &json!({"same":1}));
    let second = inputs.add("second.json", &json!({"changed":1}));
    let paths = vec![first, second.clone()];
    let before = original::inventory(&paths).unwrap().original_digest;
    std::fs::write(second, br#"{"changed":2}"#).unwrap();
    let after = original::inventory(&paths).unwrap().original_digest;
    assert_ne!(before, after);
}

#[test]
fn original_cpg_trials_use_the_real_reader_model() {
    let inputs = Inputs::new();
    let value = serde_json::from_str(include_str!(
        "../../../crates/rstyle-core/src/cpg/testdata/example.cpg.json"
    ))
    .unwrap();
    let paths = vec![inputs.add("cpg.json", &value)];
    let expected = original::inventory(&paths).unwrap().original_digest;
    for level in [None, Some(3)] {
        let wire = inputs.0.join(if level.is_some() { "zstd" } else { "json" });
        original::trial(
            Operation::Encode,
            &paths,
            (&wire, original::ReadModel::Cpg),
            level,
        )
        .unwrap();
        let decoded = original::trial(
            Operation::Decode,
            &paths,
            (&wire, original::ReadModel::Cpg),
            level,
        )
        .unwrap();
        assert_eq!(decoded.archive_digest, expected);
        assert_eq!(decoded.representation, "original-cpg-document");
    }
}
