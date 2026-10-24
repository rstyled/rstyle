//! Receipt transport checks do not certify the observed program's semantics.
use super::*;
use crate::frontend::*;

fn node() -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local: 2,
        },
        kind: "pat_slice_rest".into(),
        span: ByteSpan { start: 3, end: 5 },
        snippet: "..".into(),
        ty: None,
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: Vec::new(),
    }
}

fn observation(node: &HirNode, status: CaptureStatus) -> Observation {
    Observation {
        identity: IDENTITY.into(),
        hir: node.hir.clone(),
        span: node.span,
        status,
        ty: TypeShape::Slice(Box::new(TypeShape::Byte)),
        adjustments: Vec::new(),
        details: Details::Rest,
    }
}

#[test]
fn transport_preserves_unsupported_and_unknown_outcomes() {
    for status in [
        CaptureStatus::Captured,
        CaptureStatus::Unsupported("Overloaded dereference".into()),
        CaptureStatus::Unknown("Missing binding mode".into()),
    ] {
        let mut node = node();
        let expected = observation(&node, status);
        node.res = Some(expected.resolution().unwrap());
        assert_eq!(Observation::decode(&node).unwrap(), expected);
    }
}

#[test]
fn decoding_rejects_receipt_rebound_to_another_node_or_source_span() {
    let mut node = node();
    node.res = Some(
        observation(&node, CaptureStatus::Captured)
            .resolution()
            .unwrap(),
    );
    node.hir.owner += 1;
    assert!(Observation::decode(&node).is_err());
    node.hir.owner -= 1;
    node.span.end += 1;
    assert!(Observation::decode(&node).is_err());
}

#[test]
fn decoding_rejects_unknown_fields_and_path_resolution_targets() {
    let mut node = node();
    let original = observation(&node, CaptureStatus::Captured);
    let mut value = serde_json::to_value(&original).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("permission".into(), true.into());
    node.res = Some(PathRes {
        kind: RESOLUTION.into(),
        target: None,
        description: serde_json::to_string(&value).unwrap(),
    });
    assert!(Observation::decode(&node).is_err());
    node.res = Some(original.resolution().unwrap());
    node.res.as_mut().unwrap().target = Some(node.hir.clone());
    assert!(Observation::decode(&node).is_err());
}
