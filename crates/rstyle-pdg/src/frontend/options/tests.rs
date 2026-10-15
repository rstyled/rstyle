use super::*;
use crate::{concrete_types::*, field_locations::*};
use serde_json::json;

fn interface() -> InterfaceObservations {
    InterfaceObservations {
        identity: crate::concrete_types::IDENTITY.into(),
        parameters: vec![Observation::Captured {
            ty: TypeTree::Shared(Box::new(TypeTree::Scalar("i32".into()))),
        }],
        result: Observation::Captured {
            ty: TypeTree::Scalar("i32".into()),
        },
    }
}

fn fields() -> FieldObservations {
    FieldObservations {
        identity: crate::field_locations::IDENTITY.into(),
        operations: vec![FieldOperation {
            hir: crate::HirRef {
                krate: 0,
                owner: 1,
                local: 2,
            },
            span: crate::ByteSpan { start: 0, end: 6 },
            snippet: "data.0".into(),
            location: Location::Captured {
                parameter: 0,
                path: vec![Projection::TupleField { position: 0 }],
                value: TypeTree::Scalar("i32".into()),
            },
        }],
        possibly_aliasing_formals: vec![[0, 1], [0, 1]],
    }
}

fn unrendered(options: &FrontendOptions) {
    for entry in options.0.values() {
        match entry {
            Entry::Fields(record) => assert!(record.json.get().is_none()),
            Entry::Interface(record) => assert!(record.json.get().is_none()),
            Entry::Census(record) => assert!(record.json.get().is_none()),
            Entry::Origins(record) => assert!(record.json.get().is_none()),
            _ => {}
        }
    }
}

#[test]
fn moved_records_are_borrowed_without_rendering_or_redecoding() {
    let fields = fields();
    let snippet = fields.operations[0].snippet.as_ptr();
    let interface = interface();
    let identity = interface.identity.as_ptr();
    let mut options = FrontendOptions::default();
    options.insert_data(fields);
    options.insert_data(interface);
    for _ in 0..100 {
        let Cow::Borrowed(fields) = options
            .data::<crate::field_locations::FieldObservations>()
            .unwrap()
        else {
            panic!("owned field copy")
        };
        let Cow::Borrowed(interface) = options
            .data::<crate::concrete_types::InterfaceObservations>()
            .unwrap()
        else {
            panic!("owned interface copy")
        };
        assert_eq!(fields.operations[0].snippet.as_ptr(), snippet);
        assert_eq!(interface.identity.as_ptr(), identity);
        assert_eq!(fields.possibly_aliasing_formals, [[0, 1], [0, 1]]);
    }
    let bytes = serde_json::to_vec(&options).unwrap();
    let loaded: FrontendOptions = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(options, loaded);
    unrendered(&options);
    unrendered(&loaded);
    let first = loaded
        .data::<crate::field_locations::FieldObservations>()
        .unwrap();
    assert!(std::ptr::eq(
        first.as_ref(),
        loaded
            .data::<crate::field_locations::FieldObservations>()
            .unwrap()
            .as_ref()
    ));
}

fn nominal_interface() -> Value {
    json!({
        "identity": crate::concrete_types::IDENTITY,
        "parameters": [],
        "result": {"status":"Captured", "ty": {"kind":"Struct", "evidence": {
            "nominal": {"source_identity":null, "package_name":"pkg", "package_version":"1", "manifest_digest":"sha256:manifest", "definition_path":"pkg::Data", "declaration_digest":"sha256:declaration"},
            "source":"struct Data { value: i32 }", "fields":[]
        }}}
    })
}

#[test]
fn wire_bytes_preserve_explicit_nulls_unknown_evidence_and_sorted_keys() {
    let original = BTreeMap::from([
        (crate::concrete_types::KEY.to_owned(), nominal_interface()),
        (
            crate::field_locations::KEY.to_owned(),
            serde_json::to_value(fields()).unwrap(),
        ),
        (
            "future/evidence".into(),
            json!({"unknown": [null, 1, 1, {"source_identity":null}]}),
        ),
    ]);
    let expected = serde_json::to_vec(&original).unwrap();
    let options: FrontendOptions = serde_json::from_slice(&expected).unwrap();
    assert!(matches!(
        options
            .data::<crate::concrete_types::InterfaceObservations>()
            .unwrap(),
        Cow::Borrowed(_)
    ));
    assert_eq!(serde_json::to_vec(&options).unwrap(), expected);
    unrendered(&options);
    assert_eq!(
        options[crate::concrete_types::KEY],
        original[crate::concrete_types::KEY]
    );
    assert_eq!(options["future/evidence"], original["future/evidence"]);
}

#[test]
fn malformed_and_unregistered_evidence_remains_observational() {
    let malformed = json!({"identity":"future", "operations":false});
    let error = serde_json::from_value::<FieldObservations>(malformed.clone())
        .unwrap_err()
        .to_string();
    let mut options = FrontendOptions::default();
    options.insert(crate::field_locations::KEY.into(), malformed.clone());
    for _ in 0..10 {
        assert_eq!(
            options
                .data::<crate::field_locations::FieldObservations>()
                .unwrap_err(),
            error
        );
    }
    assert_eq!(options[crate::field_locations::KEY], malformed);
    let mut interface = interface();
    interface.identity = "future-version".into();
    options.insert_data(interface);
    assert_eq!(
        options
            .data::<crate::concrete_types::InterfaceObservations>()
            .unwrap()
            .validate()
            .unwrap_err(),
        "Unregistered concrete type observation identity"
    );
    assert_eq!(
        FrontendOptions::default()
            .data::<crate::field_locations::FieldObservations>()
            .unwrap_err(),
        format!("Missing data evidence {}", crate::field_locations::KEY)
    );
}

#[test]
fn compatibility_mutation_invalidates_typed_record_until_unit_freeze() {
    let mut options = FrontendOptions::default();
    options.insert_data(fields());
    let original = options.clone();
    options.get_mut(crate::field_locations::KEY).unwrap()["operations"][0]["snippet"] =
        json!("edited");
    assert_eq!(
        options
            .data::<crate::field_locations::FieldObservations>()
            .unwrap()
            .operations[0]
            .snippet,
        "edited"
    );
    options.normalize_data();
    assert!(matches!(
        options
            .data::<crate::field_locations::FieldObservations>()
            .unwrap(),
        Cow::Borrowed(_)
    ));
    assert_eq!(
        original
            .data::<crate::field_locations::FieldObservations>()
            .unwrap()
            .operations[0]
            .snippet,
        "data.0"
    );
    unrendered(&options);
    options.get_mut(crate::field_locations::KEY).unwrap()["operations"][0]["hir"]["owner"] =
        json!(u64::MAX);
    options.normalize_data();
    assert!(
        options
            .data::<crate::field_locations::FieldObservations>()
            .is_err()
    );
    assert_eq!(
        options[crate::field_locations::KEY]["operations"][0]["hir"]["owner"],
        json!(u64::MAX)
    );
}

#[test]
fn source_census_is_shared_lossless_and_rejects_conflicting_snapshots() {
    use crate::nominal_boundary::census::{Census, IDENTITY};
    let census = Arc::new(Census {
        identity: IDENTITY.into(),
        context: crate::nominal_boundary::tests::context(),
        files: Vec::new(),
    });
    let mut first = FrontendOptions::default();
    first.insert_data(Arc::clone(&census));
    let expected = serde_json::to_vec(&BTreeMap::from([(
        crate::nominal_boundary::census::KEY,
        serde_json::to_value(&census).unwrap(),
    )]))
    .unwrap();
    assert_eq!(serde_json::to_vec(&first).unwrap(), expected);
    let mut second: FrontendOptions = serde_json::from_slice(&expected).unwrap();
    let mut owners = Vec::new();
    first.intern_census(&mut owners).unwrap();
    second.intern_census(&mut owners).unwrap();
    let first = first.data::<Arc<Census>>().unwrap();
    let second = second.data::<Arc<Census>>().unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(owners.len(), 1);
    let mut changed = (**first).clone();
    changed.identity.push_str("-changed");
    let mut options = FrontendOptions::default();
    options.insert_data(Arc::new(changed));
    assert!(options.intern_census(&mut owners).is_err());
    let mut changed = (**first).clone();
    changed.context.package.package_id.push_str("-other");
    options.insert_data(Arc::new(changed));
    options.intern_census(&mut owners).unwrap();
    assert_eq!(owners.len(), 2);
    assert!(!Arc::ptr_eq(&owners[0], &owners[1]));
    unrendered(&options);
}

#[test]
fn declaration_origins_borrow_owned_bytes_and_preserve_wire_mutations() {
    use crate::declaration_origins::{IDENTITY, KEY, Origin, Origins};
    let nominal = nominal_interface()["result"]["ty"]["evidence"]["nominal"].clone();
    let origins = Origins {
        identity: IDENTITY.into(),
        declarations: vec![Origin {
            nominal: serde_json::from_value(nominal).unwrap(),
            source_file: "original.rs".into(),
            file_bytes: "\u{feff}struct Data { value: i32 }\r\n".into(),
            file_digest: "unverified-claim".into(),
            declaration_span: crate::ByteSpan { start: 3, end: 29 },
        }],
    };
    let bytes = origins.declarations[0].file_bytes.as_ptr();
    let expected = serde_json::to_vec(&BTreeMap::from([(
        KEY,
        serde_json::to_value(&origins).unwrap(),
    )]))
    .unwrap();
    let mut options = FrontendOptions::default();
    options.insert_data(origins);
    for _ in 0..100 {
        let Cow::Borrowed(record) = options.data::<Origins>().unwrap() else {
            panic!("copied declaration origins");
        };
        assert_eq!(record.declarations[0].file_bytes.as_ptr(), bytes);
    }
    unrendered(&options);
    assert_eq!(serde_json::to_vec(&options).unwrap(), expected);
    let loaded: FrontendOptions = serde_json::from_slice(&expected).unwrap();
    assert_eq!(loaded, options);
    unrendered(&loaded);
    options.get_mut(KEY).unwrap()["declarations"][0]["file_bytes"] = json!("edited");
    options.normalize_data();
    let Cow::Borrowed(record) = options.data::<Origins>().unwrap() else {
        panic!("origins not normalized");
    };
    assert_eq!(record.declarations[0].file_bytes.as_ref(), "edited");
    assert_ne!(loaded, options);
    options.get_mut(KEY).unwrap()["declarations"][0]["declaration_span"]["start"] = json!(-1);
    options.normalize_data();
    assert!(options.data::<Origins>().is_err());
    assert_eq!(
        options[KEY]["declarations"][0]["declaration_span"]["start"],
        json!(-1)
    );
}

#[test]
fn origins_share_only_exact_census_snapshots_and_edits_detach() {
    use crate::declaration_origins::{Origin, Origins};
    let census = crate::nominal_boundary::resolution::environment::tests::census();
    let file = &census.files[0];
    let nominal = nominal_interface()["result"]["ty"]["evidence"]["nominal"].clone();
    let origin = Origin {
        nominal: serde_json::from_value(nominal).unwrap(),
        source_file: file.path.to_string_lossy().into_owned(),
        file_bytes: file.bytes.to_string().into(),
        file_digest: file.digest.clone(),
        declaration_span: crate::ByteSpan { start: 0, end: 3 },
    };
    assert!(!Arc::ptr_eq(&file.bytes, &origin.file_bytes));
    let mut options = FrontendOptions::default();
    options.insert_data(Arc::clone(&census));
    options.insert_data(Origins {
        identity: crate::declaration_origins::IDENTITY.into(),
        declarations: vec![origin.clone()],
    });
    let before = serde_json::to_vec(&options).unwrap();
    options.share_origin_sources().unwrap();
    let record = options.data::<Origins>().unwrap();
    assert!(Arc::ptr_eq(&file.bytes, &record.declarations[0].file_bytes));
    assert_eq!(serde_json::to_vec(&options).unwrap(), before);
    unrendered(&options);
    for property in ["source_file", "file_digest", "file_bytes"] {
        let mut edited = options.clone();
        edited.get_mut(crate::declaration_origins::KEY).unwrap()["declarations"][0][property] =
            json!("edited");
        edited.normalize_data();
        assert!(edited.share_origin_sources().is_err());
        assert!(Arc::ptr_eq(
            &file.bytes,
            &options.data::<Origins>().unwrap().declarations[0].file_bytes
        ));
        assert_eq!(origin.file_bytes.as_ref(), file.bytes.as_ref());
    }
}
