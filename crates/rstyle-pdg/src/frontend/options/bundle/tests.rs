use super::*;
use crate::{
    ByteSpan,
    concrete_types::Nominal,
    declaration_origins::{Origin, Origins},
    nominal_boundary::census::{Census, UnitFile},
};
use serde_json::json;

fn options() -> FrontendOptions {
    let bytes: crate::SourceSnapshot = "struct Data { value: i32 }\r\n".into();
    let digest = crate::concrete_types::digest(bytes.as_bytes());
    let context = crate::nominal_boundary::tests::context();
    let census = Census {
        identity: crate::nominal_boundary::census::IDENTITY.into(),
        context,
        files: vec![UnitFile {
            path: "/sample/src/lib.rs".into(),
            bytes: bytes.clone(),
            digest: digest.clone(),
        }],
    };
    let origin = Origin {
        nominal: Nominal {
            source_identity: None,
            package_name: "sample".into(),
            package_version: "1.0.0".into(),
            manifest_digest: census.context.package.manifest_digest.clone(),
            definition_path: "sample::Data".into(),
            declaration_digest: "sha256:claim".into(),
        },
        source_file: "/sample/src/lib.rs".into(),
        file_bytes: bytes,
        file_digest: digest,
        declaration_span: ByteSpan { start: 0, end: 26 },
    };
    let mut options = FrontendOptions::default();
    options.insert_data(Arc::new(census));
    options.0.insert(
        crate::declaration_origins::KEY.into(),
        Entry::Origins(Record::new(
            Origins {
                identity: crate::declaration_origins::IDENTITY.into(),
                declarations: vec![origin],
            },
            vec!["/declarations/0/nominal".into()],
        )),
    );
    options.insert(
        "unknown".into(),
        json!({"snapshot": 999, "source_identity": null, "kind": "origins"}),
    );
    options
}

fn encoded(options: &FrontendOptions) -> (Value, Vec<crate::SourceSnapshot>) {
    let mut sources = source_table::Owners::default();
    let wire = serde_json::to_value(Export::new(options, &mut sources).unwrap()).unwrap();
    let snapshots = sources
        .snapshots
        .into_iter()
        .map(crate::SourceSnapshot::from)
        .collect();
    (wire, snapshots)
}

fn decode(wire: Value, snapshots: &[crate::SourceSnapshot]) -> Result<FrontendOptions, String> {
    let import: Import = serde_json::from_value(wire).map_err(|error| error.to_string())?;
    let mut sources = source_table::Imported::new(snapshots);
    let options = import.decode(&mut sources)?;
    sources.finish()?;
    Ok(options)
}

#[test]
fn census_and_origins_share_one_snapshot_and_preserve_readable_json() {
    let original = options();
    let (wire, snapshots) = encoded(&original);
    assert_eq!(snapshots.len(), 1);
    assert!(
        wire[crate::nominal_boundary::census::KEY]["value"]["files"][0]
            .get("bytes")
            .is_none()
    );
    assert!(
        wire[crate::declaration_origins::KEY]["value"]["declarations"][0]
            .get("file_bytes")
            .is_none()
    );
    let imported = decode(wire, &snapshots).unwrap();
    assert_eq!(
        serde_json::to_vec(&original).unwrap(),
        serde_json::to_vec(&imported).unwrap()
    );
    assert_eq!(original, imported);
    let census = imported.data::<Arc<Census>>().unwrap();
    let origins = imported.data::<Origins>().unwrap();
    assert!(crate::SourceSnapshot::ptr_eq(
        &census.files[0].bytes,
        &origins.declarations[0].file_bytes
    ));
    assert!(crate::SourceSnapshot::ptr_eq(
        &snapshots[0],
        &census.files[0].bytes
    ));
    crate::nominal_boundary::census::verify(&census).unwrap();
}

#[test]
fn malformed_and_tag_like_original_evidence_stays_lossless() {
    let mut original = FrontendOptions::default();
    original.insert(
        crate::declaration_origins::KEY.into(),
        json!({"kind": "origins", "value": {"identity": "forged"}}),
    );
    original.insert(
        crate::nominal_boundary::census::KEY.into(),
        json!({"files": "unavailable"}),
    );
    let (wire, snapshots) = encoded(&original);
    assert!(snapshots.is_empty());
    assert_eq!(wire[crate::declaration_origins::KEY]["kind"], "inline");
    let imported = decode(wire, &snapshots).unwrap();
    assert_eq!(original, imported);
    assert!(imported.data::<Origins>().is_err());
    assert!(imported.data::<Arc<Census>>().is_err());
}

#[test]
fn source_option_references_preserve_boundary_checks() {
    let (wire, snapshots) = encoded(&options());
    let census = crate::nominal_boundary::census::KEY;
    let origins = crate::declaration_origins::KEY;
    let mut dangling = wire.clone();
    dangling[census]["value"]["files"][0]["snapshot"] = u32::MAX.into();
    assert!(decode(dangling, &snapshots).is_err());
    let mut digest = wire.clone();
    digest[census]["value"]["files"][0]["digest"] = "sha256:changed".into();
    let imported = decode(digest, &snapshots).unwrap();
    assert!(
        crate::nominal_boundary::census::verify(&imported.data::<Arc<Census>>().unwrap()).is_err()
    );
    for path in [
        "/declarations/00/nominal",
        "/declarations/9/nominal",
        "/absent",
        "",
    ] {
        let mut invalid = wire.clone();
        invalid[origins]["value"]["nulls"] = json!([path]);
        assert!(decode(invalid, &snapshots).is_err());
    }
    let mut duplicate = wire.clone();
    duplicate[origins]["value"]["nulls"] =
        json!(["/declarations/0/nominal", "/declarations/0/nominal"]);
    assert!(decode(duplicate, &snapshots).is_err());
    let mut wrong = wire.clone();
    wrong[census] = wire[origins].clone();
    assert!(decode(wrong, &snapshots).is_err());
    let mut unknown = wire;
    unknown[census]["value"]["trusted"] = true.into();
    assert!(decode(unknown, &snapshots).is_err());
}
