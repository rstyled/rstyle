use super::*;
use crate::dependency_namespaces::capture::{IDENTITY, KEY, UnitCapture, Unsupported};

fn capture() -> Arc<UnitCapture> {
    Arc::new(UnitCapture {
        identity: IDENTITY.into(),
        imports: Vec::new(),
        declarations: Vec::new(),
        unsupported: vec![Unsupported {
            definition: crate::generated_sources::capture::Definition {
                path: "dep::unresolved".into(),
                definition_identity: "observed-definition".into(),
                crate_name: "dep".into(),
                stable_crate_id: "observed-crate".into(),
                local: true,
            },
            reason: "Unknown export footprint".into(),
        }],
    })
}

#[test]
fn namespace_observations_share_without_json_or_checksums() {
    let capture = capture();
    let mut first = FrontendOptions::default();
    first.insert_data(Arc::clone(&capture));
    let second = first.clone();
    assert_eq!(first, second);
    for options in [&first, &second] {
        let borrowed = options.data::<Arc<UnitCapture>>().unwrap();
        assert!(matches!(borrowed, Cow::Borrowed(_)));
        assert!(Arc::ptr_eq(&capture, borrowed.as_ref()));
        let Some(Entry::DependencyNamespaces(record)) = options.0.get(KEY) else {
            panic!("expected typed namespace observations");
        };
        assert!(record.json.get().is_none());
    }
}

#[test]
fn namespace_wire_preserves_unknown_and_unsupported_evidence() {
    let mut options = FrontendOptions::default();
    options.insert_data(capture());
    let original = serde_json::to_value(&options).unwrap();
    let restored: FrontendOptions = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(restored, options);
    options.get_mut(KEY).unwrap()["unknown"] = true.into();
    options.normalize_data();
    assert!(options.data::<Arc<UnitCapture>>().is_err());
    assert_eq!(options.get(KEY).unwrap()["unknown"], true);

    let mut missing = original;
    missing[KEY].as_object_mut().unwrap().remove("unsupported");
    let missing: FrontendOptions = serde_json::from_value(missing).unwrap();
    assert!(missing.data::<Arc<UnitCapture>>().is_err());
    // Retained unsupported observations differ from absence; neither grants proof.
    assert!(restored.contains_key(KEY));
    assert!(!FrontendOptions::default().contains_key(KEY));
}
