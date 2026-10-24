use super::*;
use crate::generated_sources::capture::{KEY, UNIT_IDENTITY, UnitCapture};

#[test]
fn generated_observations_share_one_unit_without_materializing_json() {
    let capture = Arc::new(UnitCapture {
        identity: UNIT_IDENTITY.into(),
        outcomes: Vec::new(),
        expansions: Vec::new(),
        emitted: Vec::new(),
        complete: false,
        scope: "diagnostic-only".into(),
    });
    let mut first = FrontendOptions::default();
    first.insert_data(Arc::clone(&capture));
    let second = first.clone();
    for options in [&first, &second] {
        let borrowed = options.data::<Arc<UnitCapture>>().unwrap();
        assert!(matches!(borrowed, Cow::Borrowed(_)));
        assert!(Arc::ptr_eq(&capture, borrowed.as_ref()));
        let Some(Entry::GeneratedSources(record)) = options.0.get(KEY) else {
            panic!("expected retained typed diagnostic observation");
        };
        assert!(record.json.get().is_none());
    }
}

#[test]
fn generated_observations_roundtrip_without_inventing_completeness() {
    let json = serde_json::json!({
        "identity": UNIT_IDENTITY, "outcomes": [], "expansions": [],
        "emitted": [], "complete": false, "scope": "diagnostic-only"
    });
    let mut options = FrontendOptions::default();
    options.insert(KEY.into(), json.clone());
    assert!(!options.data::<Arc<UnitCapture>>().unwrap().complete);
    let wire = serde_json::to_value(&options).unwrap();
    let restored: FrontendOptions = serde_json::from_value(wire).unwrap();
    assert_eq!(restored.get(KEY), Some(&json));
    options
        .get_mut(KEY)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("complete");
    options.normalize_data();
    assert!(options.data::<Arc<UnitCapture>>().is_err());
    assert!(options.get(KEY).unwrap().get("complete").is_none());
}
