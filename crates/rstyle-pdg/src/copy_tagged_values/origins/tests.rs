//! Fixed original endpoint data supplies semantics, never checked source authority.
use super::*;
use crate::HirFnFacts;
use serde_json::Value;

struct Fixture {
    product: TypeTree,
    tagged: TypeTree,
    result: TypeTree,
    root: FormalRoot,
    wrap_root: FormalRoot,
    guard: Guard,
}

impl Fixture {
    fn new() -> Self {
        let original: Value =
            serde_json::from_str(include_str!("../../copy_conversion_members/original.json"))
                .unwrap();
        let product: TypeTree = serde_json::from_value(original["payload"].clone()).unwrap();
        let tagged: TypeTree = serde_json::from_value(original["tagged"].clone()).unwrap();
        let error: TypeTree = serde_json::from_value(original["error"].clone()).unwrap();
        let facts: HirFnFacts = serde_json::from_str(include_str!("../tests/1.json")).unwrap();
        let wrap: HirFnFacts = serde_json::from_str(include_str!("../tests/0.json")).unwrap();
        let TypeTree::MixedEnum { nominal, .. } = &tagged else {
            unreachable!()
        };
        let matched = &facts.body.children[0];
        let guard = Guard {
            scrutinee: matched.children[0].hir.clone(),
            pattern: matched.children[1].children[0].hir.clone(),
            nominal: nominal.clone(),
            variant: "ComponentRange".into(),
        };
        Self {
            result: TypeTree::Result {
                ok: Box::new(product.clone()),
                err: Box::new(error),
            },
            product,
            tagged,
            root: FormalRoot {
                parameter: 0,
                binding: facts.signature.params[0].pat.hir.clone(),
            },
            wrap_root: FormalRoot {
                parameter: 0,
                binding: wrap.signature.params[0].pat.hir.clone(),
            },
            guard,
        }
    }

    fn source(&self, inverse: bool) -> Source {
        Source {
            root: if inverse {
                self.root.clone()
            } else {
                self.wrap_root.clone()
            },
            guard: inverse.then(|| self.guard.clone()),
        }
    }

    fn shape(&self, inverse: bool) -> Shape<'_> {
        if inverse {
            Shape::new(&self.tagged, &self.result).unwrap()
        } else {
            Shape::new(&self.product, &self.tagged).unwrap()
        }
    }

    fn product_route(&self) -> Route {
        let TypeTree::Struct { nominal, .. } = &self.product else {
            unreachable!()
        };
        vec![
            Component::ProductField {
                nominal: nominal.clone(),
                position: 0,
                name: "name".into(),
            },
            Component::SharedReferent,
        ]
    }

    fn tagged_route(&self) -> Route {
        let mut route = vec![Component::EnumPayload {
            nominal: self.guard.nominal.clone(),
            variant: "ComponentRange".into(),
            position: 0,
        }];
        route.extend(self.product_route());
        route
    }

    fn variant(&self, payload: bool) -> CaseKind {
        CaseKind::EnumVariant {
            nominal: self.guard.nominal.clone(),
            name: if payload {
                "ComponentRange"
            } else {
                "InsufficientInformation"
            }
            .into(),
            position: usize::from(payload),
            shape: if payload {
                VariantCase::SingleTuple
            } else {
                VariantCase::Unit
            },
        }
    }

    fn wrapping(&self) -> Contract<'_> {
        Contract {
            input: &self.product,
            output: &self.tagged,
            input_cases: vec![Case {
                kind: CaseKind::Whole,
                components: vec![self.product_route()],
            }],
            output_cases: vec![Case {
                kind: self.variant(true),
                components: vec![self.tagged_route()],
            }],
            alternatives: vec![Alternative {
                input: CaseKind::Whole,
                output: self.variant(true),
                condition: Condition::Always,
                leaves: vec![Leaf {
                    root: self.wrap_root.clone(),
                    input: self.product_route(),
                    output: self.tagged_route(),
                    guard: None,
                }],
            }],
        }
    }

    fn inverse(&self) -> Contract<'_> {
        let output: Route = std::iter::once(Component::ResultOk)
            .chain(self.product_route())
            .collect();
        Contract {
            input: &self.tagged,
            output: &self.result,
            input_cases: vec![
                Case {
                    kind: self.variant(false),
                    components: vec![],
                },
                Case {
                    kind: self.variant(true),
                    components: vec![self.tagged_route()],
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
                    input: self.variant(false),
                    output: CaseKind::ResultErr,
                    condition: Condition::Complement(self.guard.clone()),
                    leaves: vec![],
                },
                Alternative {
                    input: self.variant(true),
                    output: CaseKind::ResultOk,
                    condition: Condition::Selected(self.guard.clone()),
                    leaves: vec![Leaf {
                        root: self.root.clone(),
                        input: self.tagged_route(),
                        output,
                        guard: Some(self.guard.clone()),
                    }],
                },
            ],
        }
    }
}

#[test]
fn wrapping_preserves_exact_reference_in_its_constructed_nominal_tag() {
    let fixture = Fixture::new();
    let derived = derive(&fixture.shape(false), &fixture.source(false)).unwrap();
    assert_eq!(derived, fixture.wrapping());
    same_source_contract(&derived, &fixture.wrapping()).unwrap();
    assert_eq!(derived.input_cases.len(), 1);
    assert_eq!(derived.output_cases.len(), 1);
    assert_eq!(derived.alternatives[0].condition, Condition::Always);
}

#[test]
fn inverse_keeps_both_input_cases_and_both_result_cases_including_empty_error() {
    let fixture = Fixture::new();
    let derived = derive(&fixture.shape(true), &fixture.source(true)).unwrap();
    assert_eq!(derived, fixture.inverse());
    same_source_contract(&derived, &fixture.inverse()).unwrap();
    assert!(derived.input_cases[0].components.is_empty());
    assert!(derived.output_cases[1].components.is_empty());
    assert!(derived.alternatives[0].leaves.is_empty());
    assert!(matches!(
        derived.alternatives[0].condition,
        Condition::Complement(_)
    ));
    assert!(matches!(
        derived.alternatives[1].condition,
        Condition::Selected(_)
    ));
}

#[test]
fn source_direction_requires_the_exact_selector_and_nominal_unit_error() {
    let fixture = Fixture::new();
    assert!(derive(&fixture.shape(true), &fixture.source(false)).is_err());
    assert!(derive(&fixture.shape(false), &fixture.source(true)).is_err());
    let mut source = fixture.source(true);
    source.guard.as_mut().unwrap().variant = "InsufficientInformation".into();
    assert!(derive(&fixture.shape(true), &source).is_err());
    let wrong_error = TypeTree::Result {
        ok: Box::new(fixture.product.clone()),
        err: Box::new(TypeTree::Tuple(vec![])),
    };
    assert!(Shape::new(&fixture.tagged, &wrong_error).is_err());
    let expected = fixture.inverse();
    let mut duplicate = expected.clone();
    duplicate.input_cases[1]
        .components
        .push(expected.input_cases[1].components[0].clone());
    assert!(same_source_contract(&duplicate, &expected).is_err());
}

#[test]
fn missing_or_duplicate_input_output_and_alternative_cases_are_rejected() {
    let fixture = Fixture::new();
    let expected = fixture.inverse();
    for index in 0..3 {
        let mut missing = expected.clone();
        match index {
            0 => {
                missing.input_cases.pop();
            }
            1 => {
                missing.output_cases.pop();
            }
            _ => {
                missing.alternatives.pop();
            }
        }
        assert!(same_source_contract(&missing, &expected).is_err());
        let mut duplicate = expected.clone();
        match index {
            0 => duplicate.input_cases.push(duplicate.input_cases[0].clone()),
            1 => duplicate
                .output_cases
                .push(duplicate.output_cases[0].clone()),
            _ => duplicate
                .alternatives
                .push(duplicate.alternatives[0].clone()),
        }
        assert!(same_source_contract(&duplicate, &expected).is_err());
    }
}

#[test]
fn missing_duplicate_and_unrelated_leaf_origins_are_rejected() {
    let fixture = Fixture::new();
    let expected = fixture.inverse();
    let mut missing = expected.clone();
    missing.alternatives[1].leaves.clear();
    assert!(same_source_contract(&missing, &expected).is_err());
    let mut duplicate = expected.clone();
    duplicate.alternatives[1]
        .leaves
        .push(expected.alternatives[1].leaves[0].clone());
    assert!(same_source_contract(&duplicate, &expected).is_err());
    let mut unrelated = expected.clone();
    unrelated.alternatives[1].leaves[0].root.binding.local += 1;
    assert!(same_source_contract(&unrelated, &expected).is_err());
    unrelated.alternatives[1].leaves[0].root.parameter = 1;
    assert!(same_source_contract(&unrelated, &expected).is_err());
}

#[test]
fn wrong_scrutinee_pattern_nominal_variant_and_missing_leaf_guard_are_rejected() {
    let fixture = Fixture::new();
    let expected = fixture.inverse();
    for index in 0..5 {
        let mut actual = expected.clone();
        let guard = actual.alternatives[1].leaves[0].guard.as_mut().unwrap();
        match index {
            0 => guard.scrutinee.local += 1,
            1 => guard.pattern.local += 1,
            2 => guard.nominal.definition_path = "other::TryFromParsed".into(),
            3 => guard.variant = "InsufficientInformation".into(),
            _ => actual.alternatives[1].leaves[0].guard = None,
        }
        assert!(same_source_contract(&actual, &expected).is_err(), "{index}");
    }
    let mut inverted = expected.clone();
    inverted.alternatives[1].condition = Condition::Complement(fixture.guard.clone());
    assert!(same_source_contract(&inverted, &expected).is_err());
}

#[test]
fn nominal_enum_product_and_result_routes_cannot_be_flattened_or_swapped() {
    let fixture = Fixture::new();
    let expected = fixture.inverse();
    for index in 0..5 {
        let mut actual = expected.clone();
        let leaf = &mut actual.alternatives[1].leaves[0];
        match index {
            0 => {
                leaf.input.remove(0);
            }
            1 => leaf.output[0] = Component::ResultErr,
            2 => leaf.input[0] = leaf.input[1].clone(),
            3 => {
                if let Component::ProductField { nominal, .. } = &mut leaf.input[1] {
                    nominal.definition_path = "other::ComponentRange".into();
                }
            }
            _ => {
                leaf.output.pop();
            }
        }
        assert!(same_source_contract(&actual, &expected).is_err(), "{index}");
    }
}

#[test]
fn unrelated_endpoint_shapes_incomplete_tag_rosters_and_payload_identity_fail() {
    let fixture = Fixture::new();
    assert!(Shape::new(&fixture.product, &fixture.result).is_err());
    assert!(Shape::new(&fixture.tagged, &TypeTree::Scalar("unit".into())).is_err());
    for index in 0..4 {
        let mut tagged = fixture.tagged.clone();
        let TypeTree::MixedEnum { variants, .. } = &mut tagged else {
            unreachable!()
        };
        match index {
            0 => {
                variants.pop();
            }
            1 => variants.push(variants[0].clone()),
            2 => variants[1].shape = VariantShape::Unit {},
            _ => {
                if let VariantShape::SingleTuple { field } = &mut variants[1].shape {
                    field.ty = TypeTree::Scalar("bool".into());
                }
            }
        }
        assert!(Shape::new(&tagged, &fixture.result).is_err(), "{index}");
    }
}

#[test]
fn bounded_routes_and_explicit_empty_cases_reject_unbounded_or_invented_leaves() {
    let fixture = Fixture::new();
    let expected = fixture.inverse();
    let mut actual = expected.clone();
    actual.output_cases[1]
        .components
        .push(vec![Component::ResultErr, Component::SharedReferent]);
    assert!(same_source_contract(&actual, &expected).is_err());
    let mut actual = expected.clone();
    actual.alternatives[1].leaves[0].input =
        vec![Component::ResultOk; crate::concrete_types::MAX_DEPTH + 1];
    assert!(same_source_contract(&actual, &expected).is_err());
    let mut actual = expected.clone();
    actual.input_cases =
        vec![expected.input_cases[0].clone(); crate::concrete_types::MAX_VARIANTS + 1];
    assert!(same_source_contract(&actual, &expected).is_err());
}
