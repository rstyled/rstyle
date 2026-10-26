use super::*;
use rstyle_pdg::generated_sources::capture::{Outcome, Transparency, UnitCapture};
use serde_json::json;

pub(super) fn member(original: &HirFnFacts) {
    let changes = [
        ("/status", json!("unsupported")),
        ("/observation/identity", json!("unreviewed")),
        ("/observation/admission_authority", json!(true)),
        ("/observation/trait_definition/crate", json!("lookalike")),
        (
            "/observation/trait_method/definition_identity",
            json!("different"),
        ),
        (
            "/observation/implementation/stable_crate_id",
            json!("foreign"),
        ),
        ("/observation/selected_arguments", json!([{}])),
        ("/observation/method_header/is_async", json!(true)),
        ("/observation/method_source/span/start", json!(0)),
        ("/observation/implementation_source/span/end", json!(0)),
        ("/observation/members", json!([])),
        (
            "/observation/associated_error/normalized_projection",
            json!([]),
        ),
        ("/observation/associated_error/source/span/start", json!(0)),
        ("/observation/requested_signature/variadic", json!(true)),
        ("/observation/selected_signature/safety", json!("Unsafe")),
        (
            "/observation/trait_arguments/0/identity/definition/stable_crate_id",
            json!("foreign"),
        ),
    ];
    for (path, value) in changes {
        let mut changed = original.clone();
        let observation = changed
            .build
            .frontend_options
            .get_mut("rstyle.rust.generated-conversion-member")
            .unwrap();
        *observation
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("missing {path}")) = value;
        assert!(probe(&changed).is_err(), "accepted changed {path}");
    }
    let mut changed = original.clone();
    changed.signature.return_ty.normalized = Some("u64".into());
    assert!(probe(&changed).is_err());
}

pub(super) fn source(original: &HirFnFacts) {
    for mutation in 0..6 {
        let mut changed = original.clone();
        let capture = original.generated_source_capture().unwrap().unwrap();
        let mut capture: UnitCapture = (*capture).clone();
        let observation = capture
            .outcomes
            .iter_mut()
            .find_map(|outcome| match outcome {
                Outcome::Observed { observation } if observation.owner.path == original.name => {
                    Some(observation)
                }
                _ => None,
            })
            .unwrap();
        match mutation {
            0 => {
                observation.nodes.pop();
            }
            1 => observation.nodes[0].hir.owner += 1,
            2 => observation.nodes[0].origin.marks[0].transparency = Transparency::Opaque,
            3 => observation.nodes[0].origin.raw.span.start += 1,
            4 => observation
                .implementation
                .definition_identity
                .push_str("foreign"),
            5 => {
                capture.emitted.retain(|entry| entry.kind != "AssocTy");
            }
            _ => unreachable!(),
        }
        changed
            .build
            .frontend_options
            .insert_data(Arc::new(capture));
        assert!(
            probe(&changed).is_err(),
            "accepted source mutation {mutation}"
        );
    }
}

pub(super) fn calls(original: &HirFnFacts) {
    for (path, value) in [
        ("/status", json!("unsupported")),
        ("/expanded", json!(false)),
        ("/span/start", json!(999)),
        ("/requested/definition_identity", json!("foreign")),
        ("/selected/stable_crate_id", json!("foreign")),
        ("/requested_arguments/1/identity/name", json!("u8")),
        (
            "/requested_arguments/0/identity/definition/path",
            json!("Other"),
        ),
        ("/selected_arguments", json!([{}])),
        ("/signature/inputs", json!(["u8"])),
        ("/signature/output", json!("()")),
    ] {
        let mut changed = original.clone();
        let inventory = changed
            .build
            .frontend_options
            .get_mut("rstyle.rust.call-instances")
            .unwrap();
        let call = inventory["calls"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|call| {
                call["requested"]["path"]
                    .as_str()
                    .unwrap()
                    .ends_with("TryFrom::try_from")
            })
            .unwrap();
        *call.pointer_mut(path).unwrap() = value;
        assert!(probe(&changed).is_err(), "accepted changed leaf {path}");
    }
    roster(original);
}

fn roster(original: &HirFnFacts) {
    for mutation in 0..3 {
        let mut changed = original.clone();
        let inventory = changed
            .build
            .frontend_options
            .get_mut("rstyle.rust.call-instances")
            .unwrap();
        match mutation {
            0 => inventory["complete"] = false.into(),
            1 => {
                inventory["calls"].as_array_mut().unwrap().pop();
            }
            2 => {
                let call = inventory["calls"][2].clone();
                inventory["calls"].as_array_mut().unwrap().push(call);
            }
            _ => unreachable!(),
        }
        assert!(
            probe(&changed).is_err(),
            "accepted changed call roster {mutation}"
        );
    }
}

pub(super) fn selected_body(entry: &Value) {
    crate::shared_evidence::reject_edit(entry, |bundle, index| {
        let calls = bundle["functions"][index]["build"]["frontend_options"]
            ["rstyle.rust.call-instances"]["calls"].as_array_mut().unwrap();
        let leaf = calls
            .iter_mut()
            .find(|call| {
                call["requested"]["path"]
                    .as_str()
                    .unwrap()
                    .ends_with("TryFrom::try_from")
            })
            .unwrap();
        leaf["selected"]["definition_identity"] = "another-selected-body".into();
    });
}
