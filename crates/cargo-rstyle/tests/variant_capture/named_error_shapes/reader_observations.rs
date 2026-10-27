//! Concrete interface observations never replace generic, cleanup or flow proof.
use super::*;

const READERS: &str = r#"
impl Error {
    pub fn buffer_too_small(what: &'static str) -> Error {
        Error(ErrorKind::BufferTooSmall { what })
    }
}
pub fn check_slice_len<T>(slice: &[T], minimum: usize, what: &'static str) -> Result<(), Error> {
    if slice.len() < minimum { return Err(Error::buffer_too_small(what)); }
    Ok(())
}
pub fn reader(slice: &[u8], what: &'static str) -> Result<(u32, usize), Error> {
    check_slice_len(slice, 4, what)?;
    Ok((0, 4))
}
pub fn wrong_width(what: &'static str) -> Result<(u16, usize), Error> {
    Err(Error::buffer_too_small(what))
}
pub fn reversed(what: &'static str) -> Result<(usize, u32), Error> {
    Err(Error::buffer_too_small(what))
}
pub fn other_type(slice: &[u16], what: &'static str) -> Result<(), Error> {
    check_slice_len(slice, 4, what)
}
pub fn changed_comparison<T>(slice: &[T], minimum: usize, what: &'static str) -> Result<(), Error> {
    if slice.len() <= minimum { return Err(Error::buffer_too_small(what)); }
    Ok(())
}
pub fn changed_caller(slice: &[u8], what: &'static str) -> Result<(), Error> {
    changed_comparison(slice, 4, what)
}
"#;

#[test]
fn fallible_reader_interface_capture_does_not_certify_an_unproved_instance() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\n{READERS}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    check_captured_interfaces(&report);
    check_projection(function(&report, "check_slice_len"));
    check_verified_instance(function(&report, "check_slice_len"));
    check_rejected_interfaces(&report);
}

fn check_verified_instance(entry: &Value) {
    let (original, index) = shared_evidence::load(entry);
    let original = std::sync::Arc::new(original);
    let instance = rstyle_pdg::unit_store::UnitStore::capture_slice_length_instance(
        std::sync::Arc::clone(&original),
        index,
    )
    .unwrap();
    assert_eq!(instance.helper_count(), 1);
    let pair = instance.pair(0).unwrap();
    let analysis = instance.analysis();
    let facts = analysis.function(0).unwrap();
    rstyle_pdg::producer::verify(pair, &facts).unwrap();
    assert!(
        instance.functions()[0].signature.params[0]
            .ty
            .normalized
            .as_deref()
            .unwrap()
            .contains("u8")
    );
    assert!(
        original.functions()[index].signature.params[0]
            .ty
            .normalized
            .is_none()
    );
}

fn check_captured_interfaces(report: &Value) {
    for (name, success) in [
        ("check_slice_len", TypeTree::Scalar("unit".into())),
        (
            "wrong_width",
            TypeTree::Tuple(vec![
                TypeTree::Scalar("u16".into()),
                TypeTree::Scalar("usize".into()),
            ]),
        ),
        (
            "reader",
            TypeTree::Tuple(vec![
                TypeTree::Scalar("u32".into()),
                TypeTree::Scalar("usize".into()),
            ]),
        ),
    ] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
        let interface = interface(entry);
        interface.validate().unwrap();
        let Observation::Captured {
            ty: TypeTree::Result { ok, err },
        } = &interface.result
        else {
            panic!(
                "Missing complete interface for {name}: {:?}",
                interface.result
            );
        };
        assert_eq!(ok.as_ref(), &success);
        check_error(err);
    }
}

fn check_rejected_interfaces(report: &Value) {
    let entry = function(report, "reversed");
    assert_eq!(entry["outcome"], "excluded");
    assert!(matches!(
        interface(entry).result,
        Observation::Unsupported { .. }
    ));

    let entry = function(report, "changed_comparison");
    let (store, index) = shared_evidence::load(entry);
    let record = store.functions()[index]
        .build
        .frontend_options
        .get("rstyle.rust.shared-slice-instance-projections")
        .unwrap();
    assert_eq!(record["outcome"]["status"], "unsupported");
    assert!(record["outcome"].get("projection").is_none());
}

fn check_projection(entry: &Value) {
    let (store, index) = shared_evidence::load(entry);
    let facts = &store.functions()[index];
    let record = facts
        .build
        .frontend_options
        .get("rstyle.rust.shared-slice-instance-projections")
        .unwrap();
    assert_eq!(record["obligations_verified"], false);
    assert_eq!(record["outcome"]["status"], "observed", "{record}");
    let projected = &record["outcome"]["projection"];
    assert_eq!(projected["source_digest"], facts.source.digest);
    assert_eq!(
        projected["original_source_scan_complete"],
        facts.source_scan_complete
    );
    assert_eq!(projected["definition"]["local"], true);
    assert_eq!(
        projected["arguments"][0]["identity"],
        serde_json::json!({"kind":"builtin","name":"u8"})
    );
    let first = &projected["parameters"][0]["concrete"]["arguments"][0]["identity"];
    assert_eq!(first["kind"], "reference");
    assert_eq!(first["mutable"], false);
    assert_eq!(
        first["referent"],
        serde_json::json!({
            "kind":"slice", "element":{"kind":"builtin","name":"u8"}
        })
    );
    assert_eq!(projected["parameters"].as_array().unwrap().len(), 3);
    assert_eq!(projected["bindings"].as_array().unwrap().len(), 3);
    let calls = record["call_receipts"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["call"]["requested"], projected["definition"]);
    assert_eq!(calls[0]["call"]["selected"], projected["definition"]);
    assert_eq!(calls[0]["call"]["expanded"], false);
    check_length_method(projected);
}

fn check_length_method(projected: &Value) {
    let methods = projected["methods"].as_array().unwrap();
    assert_eq!(methods.len(), 1);
    let method = &methods[0]["concrete"];
    let original = &methods[0]["original"];
    assert_eq!(original["receiver_type"]["status"], "Unsupported");
    assert_eq!(original["call"], method["call"]);
    assert_eq!(original["receiver"], method["receiver"]);
    assert_eq!(original["source_span"], method["source_span"]);
    let slice = serde_json::json!({
        "kind":"slice", "element":{"kind":"builtin","name":"u8"}
    });
    let receiver = &method["receiver_type"];
    assert_eq!(receiver["arguments"][0]["identity"]["referent"], slice);
    assert_eq!(method["adjusted_type"], *receiver);
    assert_eq!(method["signature_inputs"], serde_json::json!([receiver]));
    assert_eq!(
        method["signature_result"]["arguments"][0]["identity"],
        serde_json::json!({"kind":"builtin","name":"usize"})
    );
    let adjustments = method["adjustments"].as_array().unwrap();
    assert_eq!(adjustments.len(), 2);
    assert_eq!(adjustments[0]["kind"]["kind"], "builtin_dereference");
    assert_eq!(adjustments[0]["target"]["arguments"][0]["identity"], slice);
    assert_eq!(adjustments[1]["kind"]["kind"], "shared_borrow");
    assert_eq!(adjustments[1]["target"], *receiver);
}

fn interface(entry: &Value) -> InterfaceObservations {
    let (store, index) = shared_evidence::load(entry);
    store.functions()[index]
        .build
        .frontend_options
        .data::<InterfaceObservations>()
        .unwrap()
        .into_owned()
}
