use super::*;
use crate::declaration_origins::{IDENTITY, KEY, Origins};

fn origins() -> Origins {
    Origins {
        identity: IDENTITY.into(),
        declarations: Vec::new(),
        generated: Vec::new(),
    }
}

fn measured(value: &Origins) -> usize {
    serde_json::to_vec(value).unwrap().len()
}

#[test]
fn origin_size_is_retained_only_for_the_exact_immutable_record() {
    let mut options = FrontendOptions::default();
    options.insert_data(origins());
    let original = options.data::<Origins>().unwrap();
    let expected = measured(&original);
    for _ in 0..3 {
        assert_eq!(options.origin_bytes(&original).unwrap(), expected);
    }
    let Some(Entry::Origins(record)) = options.0.get(KEY) else {
        panic!("missing typed origins");
    };
    assert_eq!(record.encoded_len.get(), Some(&expected));

    let mut changed = original.into_owned();
    changed.identity.push_str("\r\n\"\\π");
    assert_ne!(measured(&changed), expected);
    assert_eq!(options.origin_bytes(&changed).unwrap(), measured(&changed));
    assert_eq!(record.encoded_len.get(), Some(&expected));
    assert!(record.json.get().is_none());
}

#[test]
fn json_edits_and_replacement_discard_retained_origin_size() {
    let mut options = FrontendOptions::default();
    options.insert_data(origins());
    let original = options.data::<Origins>().unwrap();
    let before = options.origin_bytes(&original).unwrap();
    let untouched = options.clone();
    options.get_mut(KEY).unwrap()["identity"] = serde_json::json!("longer-identity-\r\n\"\\π");
    options.normalize_data();
    let Some(Entry::Origins(record)) = options.0.get(KEY) else {
        panic!("missing edited typed origins");
    };
    assert!(record.encoded_len.get().is_none());
    let edited = options.data::<Origins>().unwrap();
    assert_eq!(options.origin_bytes(&edited).unwrap(), measured(&edited));
    assert_ne!(measured(&edited), before);
    assert_eq!(
        untouched
            .origin_bytes(&untouched.data::<Origins>().unwrap())
            .unwrap(),
        before
    );

    options.insert_data(origins());
    let replacement = options.data::<Origins>().unwrap();
    assert_eq!(options.origin_bytes(&replacement).unwrap(), before);
}
