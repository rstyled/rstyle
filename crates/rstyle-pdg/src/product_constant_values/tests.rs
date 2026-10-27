use super::*;
use crate::{ByteSpan, HirRef, data_library::Callee};

fn selected() -> SelectedUse {
    let definition = Definition {
        definition: Callee {
            path: "Index::LIMIT".into(),
            crate_name: "fixture".into(),
            crate_hash: "local-unencoded".into(),
            definition_hash: "actual-selected".into(),
            implementation: None,
        },
        source_file: "/fixture/src/lib.rs".into(),
        source_span: ByteSpan { start: 20, end: 50 },
    };
    SelectedUse {
        hir: HirRef {
            krate: 0,
            owner: 2,
            local: 3,
        },
        span: ByteSpan { start: 5, end: 17 },
        constant: definition.clone(),
        implementation: definition.clone(),
        tuple_constructor: definition,
    }
}

#[test]
fn independent_selection_rejects_duplicate_and_split_use_identity() {
    let first = selected();
    let mut observations = Selections {
        identity: IDENTITY.into(),
        uses: vec![first.clone()],
    };
    observations.validate().unwrap();
    for change_hir in [false, true] {
        let mut second = first.clone();
        if change_hir {
            second.hir.local += 1;
        } else {
            second.span.end += 1;
        }
        observations.uses = vec![first.clone(), second];
        assert!(observations.validate().is_err());
    }
    observations.uses = vec![first; MAX_USES + 1];
    assert!(observations.validate().is_err());
}

#[test]
fn selection_wire_requires_all_actual_definition_receipts() {
    let original = serde_json::to_value(selected()).unwrap();
    for name in [
        "hir",
        "span",
        "constant",
        "implementation",
        "tuple_constructor",
    ] {
        let mut changed = original.clone();
        changed.as_object_mut().unwrap().remove(name);
        assert!(serde_json::from_value::<SelectedUse>(changed).is_err());
    }
    let mut changed = original;
    changed["invented_purity"] = true.into();
    assert!(serde_json::from_value::<SelectedUse>(changed).is_err());
}

#[test]
fn literal_retains_scalar_type_position_and_exact_independent_value() {
    for value in ["2147483646", "2147483645"] {
        let checked = VerifiedValue {
            owner_type: "original-nominal-owner".into(),
            field: crate::nominal_constants::ScalarValue {
                ty: "u32".into(),
                value: value.into(),
            },
        };
        let decoded: Vec<(u8, String, String)> =
            serde_json::from_str(&literal(&checked).unwrap()).unwrap();
        assert_eq!(decoded, vec![(0, "u32".into(), value.into())]);
    }
}
