//! Independent topology fixture plus wire/role tampering adversaries.
use super::*;
use serde_json::{Value, json};

use super::super::tests::source;

fn fixture() -> Pair {
    let mut pair = super::super::tests::fixture();
    origin_metadata(&mut pair);
    pair
}

struct Trees {
    input: crate::concrete_types::TypeTree,
    product: crate::concrete_types::TypeTree,
    output: crate::concrete_types::TypeTree,
    error: crate::concrete_types::TypeTree,
}

impl Trees {
    fn new() -> Self {
        let original: Value = serde_json::from_str(include_str!(
            "../../../copy_conversion_members/original.json"
        ))
        .unwrap();
        let product: crate::concrete_types::TypeTree =
            serde_json::from_value(original["payload"].clone()).unwrap();
        let input: crate::concrete_types::TypeTree =
            serde_json::from_value(original["tagged"].clone()).unwrap();
        let error: crate::concrete_types::TypeTree =
            serde_json::from_value(original["error"].clone()).unwrap();
        let output = crate::concrete_types::TypeTree::Result {
            ok: Box::new(product.clone()),
            err: Box::new(error.clone()),
        };
        Self {
            input,
            product,
            output,
            error,
        }
    }

    fn view(&self) -> Types<'_> {
        Types {
            input: &self.input,
            product: &self.product,
            output: &self.output,
        }
    }
}

fn origin_metadata(pair: &mut Pair) {
    let trees = Trees::new();
    let mut formal = pair.pdg.nodes[0].clone();
    formal.id = "formal".into();
    formal.category = NodeCategory::FormalInput;
    formal.attributes =
        serde_json::from_value(json!({"position":0,"symbol_ids":["input"],"defines":["d:input"]}))
            .unwrap();
    pair.pdg.nodes.push(formal);
    for node in &mut pair.pdg.nodes {
        let token = match node.id.as_str() {
            "formal" | "saved" => crate::data_types::TypeToken::from_tree(&trees.input)
                .text()
                .unwrap(),
            "payload" | "read" => crate::data_types::TypeToken::from_tree(&trees.product)
                .text()
                .unwrap(),
            "ok" | "err" | "merge" | "returned" => {
                crate::data_types::TypeToken::from_tree(&trees.output)
                    .text()
                    .unwrap()
            }
            "error" => crate::data_types::TypeToken::from_tree(&trees.error)
                .text()
                .unwrap(),
            "binding" => "unit".into(),
            _ => "bool".into(),
        };
        node.annotations
            .insert("compat.rust.hir-r0:type".into(), json!({"value":token}));
        node.attributes.construct = match node.id.as_str() {
            "selector" => Some("match.enum-variant"),
            "payload" => Some("data.enum-payload"),
            "wildcard" => Some("match.wildcard"),
            "ok" => Some("data.result-ok"),
            "err" => Some("data.result-err"),
            "error" => Some("data.struct"),
            _ => None,
        }
        .map(str::to_owned);
    }
    lexical_metadata(pair);
}

fn lexical_metadata(pair: &mut Pair) {
    for (id, symbol) in [("saved", "input"), ("read", "payload")] {
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.id == id)
            .unwrap();
        node.attributes.symbol_ids = vec![symbol.into()];
        node.attributes.reads =
            serde_json::from_value(json!([{"symbol_id":symbol,"operand_position":0}])).unwrap();
    }
    let binding = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.id == "binding")
        .unwrap();
    binding.attributes.symbol_ids = vec!["payload".into()];
    binding.attributes.defines = vec!["d:payload".into()];
    binding.attributes.write_form = Some("initialization".into());
    let mut edge = pair.pdg.edges[0].clone();
    edge.id = "formal:read".into();
    edge.source = "formal".into();
    edge.target = "saved".into();
    edge.subkind = "reaching_definition".into();
    pair.pdg.edges.push(edge);
    for edge in &mut pair.pdg.edges {
        if edge.subkind == "reaching_definition" {
            let symbol = if edge.source == "formal" {
                "input"
            } else {
                "payload"
            };
            edge.symbol_id = Some(symbol.into());
            edge.definition_id = Some(format!("d:{symbol}"));
        }
    }
}

fn expected_roles() -> Roles {
    Roles {
        values: [
            ("formal", Role::Input),
            ("saved", Role::Input),
            ("payload", Role::SelectedPayload),
            ("read", Role::SelectedPayload),
            ("ok", Role::Ok),
            ("err", Role::Err),
            ("merge", Role::Output),
            ("returned", Role::Output),
        ]
        .into_iter()
        .map(|(id, role)| (id.into(), role))
        .collect(),
        selector: Some(("saved".into(), "selector".into())),
    }
}

fn replay(pair: &Pair) -> Result<(), String> {
    let formal = unique(pair.as_view(), |node| node.id == "formal")?;
    let saved = dependent(pair.as_view(), formal, "reaching_definition")?;
    verify_definition(pair.as_view(), formal, saved)?;
    super::super::graph::consumers(pair.as_view(), formal, &[saved], "reaching_definition")?;
    let mut roles = Roles {
        values: BTreeMap::from([
            ("formal".into(), Role::Input),
            ("saved".into(), Role::Input),
        ]),
        selector: None,
    };
    let merge = graph_inverse(pair.as_view(), saved, &mut roles)?;
    super::super::graph::returned(pair.as_view(), &source(), merge)?;
    roles.values.extend([
        ("merge".into(), Role::Output),
        ("returned".into(), Role::Output),
    ]);
    if roles != expected_roles() {
        return Err("Independent source roles disagree".into());
    }
    inventory(pair.as_view(), &roles, &Trees::new().view())
}

fn annotate(pair: &mut Pair) {
    for node in &mut pair.pdg.nodes {
        if let Some(role) = expected_roles().values.get(&node.id) {
            node.annotations
                .insert(ROLE_KEY.into(), serde_json::to_value(role).unwrap());
        }
    }
}

#[test]
fn complete_origin_roles_follow_saved_input_selected_payload_ok_empty_err_and_output() {
    let mut pair = fixture();
    replay(&pair).unwrap();
    annotate(&mut pair);
    annotations(pair.as_view(), &expected_roles()).unwrap();
    assert!(
        !pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.id == "binding")
            .unwrap()
            .annotations
            .contains_key(ROLE_KEY)
    );
}

#[test]
fn missing_changed_and_scalar_origin_roles_fail() {
    for id in [
        "formal", "saved", "payload", "read", "ok", "err", "merge", "returned",
    ] {
        let mut pair = fixture();
        annotate(&mut pair);
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.id == id)
            .unwrap();
        node.annotations.remove(ROLE_KEY);
        assert!(
            annotations(pair.as_view(), &expected_roles()).is_err(),
            "missing {id}"
        );
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.id == id)
            .unwrap();
        node.annotations
            .insert(ROLE_KEY.into(), json!("wrong_role"));
        assert!(
            annotations(pair.as_view(), &expected_roles()).is_err(),
            "changed {id}"
        );
    }
    let mut pair = fixture();
    annotate(&mut pair);
    pair.pdg
        .nodes
        .iter_mut()
        .find(|node| node.id == "binding")
        .unwrap()
        .annotations
        .insert(ROLE_KEY.into(), json!("selected_payload"));
    assert!(annotations(pair.as_view(), &expected_roles()).is_err());
}

#[test]
fn unknown_reference_nodes_and_legacy_flattened_annotations_fail() {
    let mut pair = fixture();
    let mut unknown = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.id == "payload")
        .unwrap()
        .clone();
    unknown.id = "unknown-reference".into();
    pair.pdg.nodes.push(unknown);
    assert!(inventory(pair.as_view(), &expected_roles(), &Trees::new().view()).is_err());
    for id in [
        "formal", "saved", "payload", "read", "ok", "err", "merge", "returned",
    ] {
        let mut pair = fixture();
        pair.pdg
            .nodes
            .iter_mut()
            .find(|node| node.id == id)
            .unwrap()
            .annotations
            .insert(crate::data_origins::KEY.into(), json!([]));
        assert!(replay(&pair).is_err(), "legacy {id}");
    }
}

#[test]
fn reaching_definition_symbol_definition_and_extra_consumers_fail() {
    for target in ["saved", "read"] {
        for field in ["symbol", "definition", "source"] {
            let mut pair = fixture();
            let edge = pair
                .pdg
                .edges
                .iter_mut()
                .find(|edge| edge.target == target && edge.subkind == "reaching_definition")
                .unwrap();
            match field {
                "symbol" => edge.symbol_id = Some("other".into()),
                "definition" => edge.definition_id = Some("other".into()),
                _ => edge.source = "other".into(),
            }
            assert!(replay(&pair).is_err(), "{target} {field}");
        }
    }
}

#[test]
fn same_typed_unreachable_binding_read_ok_and_empty_err_cannot_claim_origins() {
    for id in [
        "payload", "binding", "read", "ok", "wildcard", "error", "err",
    ] {
        let mut pair = fixture();
        for edge in &mut pair.cfg.edges {
            if edge.target == id {
                edge.target = "exit".into();
            }
        }
        assert!(replay(&pair).is_err(), "{id}");
    }
}

fn contract_routes(
    trees: &Trees,
) -> (
    crate::copy_tagged_values::origins::Route,
    crate::copy_tagged_values::origins::Route,
) {
    use crate::concrete_types::TypeTree;
    use crate::copy_tagged_values::origins::Component;
    let TypeTree::Struct {
        nominal: product, ..
    } = &trees.product
    else {
        unreachable!()
    };
    let TypeTree::MixedEnum {
        nominal: tagged, ..
    } = &trees.input
    else {
        unreachable!()
    };
    let product = vec![
        Component::ProductField {
            nominal: product.clone(),
            position: 0,
            name: "name".into(),
        },
        Component::SharedReferent,
    ];
    (
        std::iter::once(Component::EnumPayload {
            nominal: tagged.clone(),
            variant: "ComponentRange".into(),
            position: 0,
        })
        .chain(product.clone())
        .collect(),
        std::iter::once(Component::ResultOk)
            .chain(product)
            .collect(),
    )
}

fn contract(trees: &Trees) -> crate::copy_tagged_values::origins::Contract<'_> {
    use crate::copy_tagged_values::origins::{Alternative, Case, CaseKind, Condition, Contract};
    let crate::concrete_types::TypeTree::MixedEnum { nominal, .. } = &trees.input else {
        unreachable!()
    };
    let guard = contract_guard(nominal);
    let [unit, payload] = contract_variants(nominal);
    let (input, output) = contract_routes(trees);
    Contract {
        input: &trees.input,
        output: &trees.output,
        input_cases: vec![
            Case {
                kind: unit.clone(),
                components: vec![],
            },
            Case {
                kind: payload.clone(),
                components: vec![input.clone()],
            },
        ],
        output_cases: vec![
            Case {
                kind: CaseKind::ResultOk,
                components: vec![output.clone()],
            },
            Case {
                kind: CaseKind::ResultErr,
                components: vec![],
            },
        ],
        alternatives: vec![
            Alternative {
                input: unit,
                output: CaseKind::ResultErr,
                condition: Condition::Complement(guard.clone()),
                leaves: vec![],
            },
            Alternative {
                input: payload,
                output: CaseKind::ResultOk,
                condition: Condition::Selected(guard.clone()),
                leaves: vec![contract_leaf(trees, guard)],
            },
        ],
    }
}

fn contract_guard(
    nominal: &crate::concrete_types::Nominal,
) -> crate::copy_tagged_values::origins::Guard {
    let node = source();
    crate::copy_tagged_values::origins::Guard {
        scrutinee: node.children[0].hir.clone(),
        pattern: node.children[1].children[0].hir.clone(),
        nominal: nominal.clone(),
        variant: "ComponentRange".into(),
    }
}

fn contract_variants(
    nominal: &crate::concrete_types::Nominal,
) -> [crate::copy_tagged_values::origins::CaseKind; 2] {
    use crate::copy_tagged_values::origins::{CaseKind, VariantCase};
    [
        CaseKind::EnumVariant {
            nominal: nominal.clone(),
            name: "InsufficientInformation".into(),
            position: 0,
            shape: VariantCase::Unit,
        },
        CaseKind::EnumVariant {
            nominal: nominal.clone(),
            name: "ComponentRange".into(),
            position: 1,
            shape: VariantCase::SingleTuple,
        },
    ]
}

fn contract_leaf(
    trees: &Trees,
    guard: crate::copy_tagged_values::origins::Guard,
) -> crate::copy_tagged_values::origins::Leaf {
    use crate::copy_tagged_values::origins::{FormalRoot, Leaf};
    let (input, output) = contract_routes(trees);
    Leaf {
        root: FormalRoot {
            parameter: 0,
            binding: source().children[0]
                .res
                .as_ref()
                .unwrap()
                .target
                .as_ref()
                .unwrap()
                .clone(),
        },
        input,
        output,
        guard: Some(guard),
    }
}

#[test]
fn wire_contract_roundtrips_complete_guarded_cases_without_repeating_endpoint_trees() {
    let trees = Trees::new();
    let expected = contract(&trees);
    let wire = WireContract::from_contract(&expected);
    let serialized = serde_json::to_value(&wire).unwrap();
    assert!(serialized.get("input").is_none());
    assert!(serialized.get("output").is_none());
    let restored: WireContract = serde_json::from_value(serialized).unwrap();
    restored.validate_against_contract(&expected).unwrap();
    assert_eq!(restored, wire);
}

#[test]
fn wire_case_rosters_guard_identity_and_typed_routes_cannot_be_tampered() {
    use crate::copy_tagged_values::origins::Component;
    let trees = Trees::new();
    let expected = contract(&trees);
    for index in 0..7 {
        let mut wire = WireContract::from_contract(&expected);
        match index {
            0 => {
                wire.input_cases.pop();
            }
            1 => {
                wire.output_cases.pop();
            }
            2 => wire.alternatives[1].leaves[0].guard = None,
            3 => {
                wire.alternatives[1].leaves[0]
                    .guard
                    .as_mut()
                    .unwrap()
                    .pattern
                    .local += 1
            }
            4 => wire.alternatives[1].leaves[0].output[0] = Component::ResultErr,
            5 => {
                wire.alternatives[1].leaves[0].input.remove(0);
            }
            _ => {
                let extra = wire.alternatives[1].leaves[0].clone();
                wire.alternatives[0].leaves.push(extra);
            }
        }
        assert!(
            wire.validate_against_contract(&expected).is_err(),
            "{index}"
        );
    }
    let mut unknown = serde_json::to_value(WireContract::from_contract(&expected)).unwrap();
    unknown["silent_unknown"] = json!(true);
    assert!(serde_json::from_value::<WireContract>(unknown).is_err());
}
