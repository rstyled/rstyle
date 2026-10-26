//! Bounded source-derived VALUE components; no memory or graph authority.
//! Enum payloads remain nominal tag routes, never memory field projections.
use super::*;
use crate::{
    HirRef,
    concrete_types::{Nominal, VariantShape},
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum Component {
    ProductField {
        nominal: Nominal,
        position: usize,
        name: String,
    },
    EnumPayload {
        nominal: Nominal,
        variant: String,
        position: usize,
    },
    ResultOk,
    ResultErr,
    /// Identity of the carried reference; this grants no referent-content read.
    SharedReferent,
}

pub(crate) type Route = Vec<Component>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum VariantCase {
    Unit,
    SingleTuple,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum CaseKind {
    Whole,
    EnumVariant {
        nominal: Nominal,
        name: String,
        position: usize,
        shape: VariantCase,
    },
    ResultOk,
    ResultErr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Case {
    pub kind: CaseKind,
    /// Complete reference-bearing components. Empty is an explicit empty case.
    pub components: Vec<Route>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Guard {
    pub scrutinee: HirRef,
    pub pattern: HirRef,
    pub nominal: Nominal,
    pub variant: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum Condition {
    Always,
    /// Exact callee-scoped boolean source condition; no nominal pattern authority.
    Boolean {
        condition: HirRef,
        outcome: bool,
    },
    /// Exact saved Result scrutinee in this function's source/graph scope.
    ResultTag {
        scrutinee: HirRef,
        is_ok: bool,
    },
    Selected(Guard),
    Complement(Guard),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FormalRoot {
    pub parameter: usize,
    pub binding: HirRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Leaf {
    pub root: FormalRoot,
    pub input: Route,
    pub output: Route,
    pub guard: Option<Guard>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Alternative {
    pub input: CaseKind,
    pub output: CaseKind,
    pub condition: Condition,
    pub leaves: Vec<Leaf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Contract<'a> {
    pub input: &'a TypeTree,
    pub output: &'a TypeTree,
    pub input_cases: Vec<Case>,
    pub output_cases: Vec<Case>,
    pub alternatives: Vec<Alternative>,
}

/// Shared wire vocabulary; endpoint types remain in the typed graph/interface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WireContract {
    pub input_cases: Vec<Case>,
    pub output_cases: Vec<Case>,
    pub alternatives: Vec<Alternative>,
}

impl WireContract {
    pub(crate) fn from_contract(contract: &Contract<'_>) -> Self {
        Self {
            input_cases: contract.input_cases.clone(),
            output_cases: contract.output_cases.clone(),
            alternatives: contract.alternatives.clone(),
        }
    }

    /// Compare semantic wire data to an independently supplied source contract.
    /// This does not grant receipt, graph, or caller-summary authority.
    pub(crate) fn validate_against_contract(&self, expected: &Contract<'_>) -> Result<(), String> {
        same_source_contract(
            &Contract {
                input: expected.input,
                output: expected.output,
                input_cases: self.input_cases.clone(),
                output_cases: self.output_cases.clone(),
                alternatives: self.alternatives.clone(),
            },
            expected,
        )
    }
}

pub(crate) fn validate_wire_source_contract(
    wire: &WireContract,
    facts: &dyn EvidenceFacts,
    checked: &CheckedCopyConversion,
) -> Result<(), String> {
    wire.validate_against_contract(&derive_source_contract(facts, checked)?)
}

struct Shape<'a> {
    input: &'a TypeTree,
    output: &'a TypeTree,
    product: &'a TypeTree,
    nominal: &'a Nominal,
    variants: &'a [crate::concrete_types::MixedVariant],
    inverse: bool,
}

struct Source {
    root: FormalRoot,
    guard: Option<Guard>,
}

/// The checked receipt supplies endpoint authority; this derives source semantics only.
pub(crate) fn derive_source_contract<'a>(
    facts: &dyn EvidenceFacts,
    checked: &'a CheckedCopyConversion,
) -> Result<Contract<'a>, String> {
    let shape = Shape::new(checked.input(), checked.output())?;
    let node = super::expression(facts)?;
    let root = formal(facts)?;

    let guard = if shape.inverse {
        let matched = super::matched(node, facts, checked)?;
        Some(Guard {
            scrutinee: matched.scrutinee.hir.clone(),
            pattern: matched.pattern.hir.clone(),
            nominal: shape.nominal.clone(),
            variant: matched.tag,
        })
    } else {
        super::constructor(node, facts, checked)?;
        None
    };
    derive(&shape, &Source { root, guard })
}

/// Exact source-contract comparison. Graph guards/operands require separate replay.
pub(crate) fn validate_source_contract(
    actual: &Contract<'_>,
    facts: &dyn EvidenceFacts,
    checked: &CheckedCopyConversion,
) -> Result<(), String> {
    bounded(actual)?;
    same_source_contract(actual, &derive_source_contract(facts, checked)?)
}

fn same_source_contract(actual: &Contract<'_>, expected: &Contract<'_>) -> Result<(), String> {
    bounded(actual)?;
    if actual != expected {
        return Err("Copy conversion component cases, routes or source guards disagree".into());
    }
    Ok(())
}

fn formal(facts: &dyn EvidenceFacts) -> Result<FormalRoot, String> {
    let [formal] = facts.signature.params.as_slice() else {
        return Err("Copy component origins require one exact formal root".into());
    };
    Ok(FormalRoot {
        parameter: 0,
        binding: formal.pat.hir.clone(),
    })
}

impl<'a> Shape<'a> {
    fn new(input: &'a TypeTree, output: &'a TypeTree) -> Result<Self, String> {
        let (product, tagged, inverse) = match output {
            TypeTree::MixedEnum { .. } => (input, output, false),
            TypeTree::Result { ok, err } if matches!(err.as_ref(), TypeTree::Struct { fields, .. } if fields.is_empty()) => {
                (ok.as_ref(), input, true)
            }
            _ => {
                return Err(
                    "Copy component endpoints require the exact tagged conversion family".into(),
                );
            }
        };
        let TypeTree::MixedEnum {
            nominal, variants, ..
        } = tagged
        else {
            return Err("Copy component input requires its complete nominal tagged value".into());
        };
        let shape = Self {
            input,
            output,
            product,
            nominal,
            variants,
            inverse,
        };

        shape.verify()?;
        Ok(shape)
    }

    fn verify(&self) -> Result<(), String> {
        let [unit, payload] = self.variants else {
            return Err("Copy components require the complete two-variant input roster".into());
        };
        if unit.position != 0
            || payload.position != 1
            || unit.name == payload.name
            || !matches!(unit.shape, VariantShape::Unit {})
            || !matches!(&payload.shape, VariantShape::SingleTuple { field }
                if field.position == 0 && field.name == "0" && &field.ty == self.product)
        {
            return Err("Copy component variant roster or exact payload type disagrees".into());
        }

        let TypeTree::Struct { fields, .. } = self.product else {
            return Err("Copy components require their nominal product".into());
        };
        if !matches!(fields.as_slice(), [name, flag]
            if name.position == 0 && name.name == "name"
                && name.ty == TypeTree::Shared(Box::new(TypeTree::Str))
                && flag.position == 1 && flag.name == "is_conditional"
                && flag.ty == TypeTree::Scalar("bool".into()))
        {
            return Err("Copy component product leaves disagree with the bounded contract".into());
        }
        Ok(())
    }

    fn product_route(&self) -> Result<Route, String> {
        let TypeTree::Struct {
            nominal, fields, ..
        } = self.product
        else {
            return Err("Missing nominal Copy component product".into());
        };
        Ok(vec![
            Component::ProductField {
                nominal: nominal.clone(),
                position: fields[0].position,
                name: fields[0].name.clone(),
            },
            Component::SharedReferent,
        ])
    }

    fn tagged_route(&self) -> Result<Route, String> {
        Ok(std::iter::once(Component::EnumPayload {
            nominal: self.nominal.clone(),
            variant: self.variants[1].name.clone(),
            position: 0,
        })
        .chain(self.product_route()?)
        .collect())
    }

    fn case(&self, index: usize) -> CaseKind {
        let variant = &self.variants[index];
        CaseKind::EnumVariant {
            nominal: self.nominal.clone(),
            name: variant.name.clone(),
            position: variant.position,
            shape: if index == 0 {
                VariantCase::Unit
            } else {
                VariantCase::SingleTuple
            },
        }
    }
}

fn derive<'a>(shape: &Shape<'a>, source: &Source) -> Result<Contract<'a>, String> {
    let contract = if shape.inverse {
        inverse(shape, source)?
    } else {
        wrapping(shape, source)?
    };
    bounded(&contract)?;
    Ok(contract)
}

fn wrapping<'a>(shape: &Shape<'a>, source: &Source) -> Result<Contract<'a>, String> {
    if source.guard.is_some() {
        return Err("Wrapping Copy origins must not invent a scrutinee guard".into());
    }
    let input = shape.product_route()?;
    let output = shape.tagged_route()?;
    let payload = shape.case(1);

    Ok(Contract {
        input: shape.input,
        output: shape.output,
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![input.clone()],
        }],
        output_cases: vec![Case {
            kind: payload.clone(),
            components: vec![output.clone()],
        }],
        alternatives: vec![Alternative {
            input: CaseKind::Whole,
            output: payload,
            condition: Condition::Always,
            leaves: vec![Leaf {
                root: source.root.clone(),
                input,
                output,
                guard: None,
            }],
        }],
    })
}

fn inverse<'a>(shape: &Shape<'a>, source: &Source) -> Result<Contract<'a>, String> {
    let guard = source
        .guard
        .as_ref()
        .ok_or("Inverse Copy origins require their exact source selector")?;
    if guard.nominal != *shape.nominal || guard.variant != shape.variants[1].name {
        return Err("Inverse Copy origin guard disagrees with its nominal payload variant".into());
    }

    let input = shape.tagged_route()?;
    let output: Route = std::iter::once(Component::ResultOk)
        .chain(shape.product_route()?)
        .collect();
    let leaf = Leaf {
        root: source.root.clone(),
        input: input.clone(),
        output: output.clone(),
        guard: Some(guard.clone()),
    };
    Ok(Contract {
        input: shape.input,
        output: shape.output,
        input_cases: vec![
            Case {
                kind: shape.case(0),
                components: vec![],
            },
            Case {
                kind: shape.case(1),
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
        alternatives: inverse_alternatives(shape, guard, leaf),
    })
}

fn inverse_alternatives(shape: &Shape<'_>, guard: &Guard, leaf: Leaf) -> Vec<Alternative> {
    vec![
        Alternative {
            input: shape.case(0),
            output: CaseKind::ResultErr,
            condition: Condition::Complement(guard.clone()),
            leaves: vec![],
        },
        Alternative {
            input: shape.case(1),
            output: CaseKind::ResultOk,
            condition: Condition::Selected(guard.clone()),
            leaves: vec![leaf],
        },
    ]
}

fn bounded(contract: &Contract<'_>) -> Result<(), String> {
    let max_cases = crate::concrete_types::MAX_VARIANTS;
    if contract.input_cases.len() > max_cases
        || contract.output_cases.len() > max_cases
        || contract.alternatives.len() > max_cases
    {
        return Err("Copy component contract exceeds its case bound".into());
    }

    bounded_cases(contract.input, &contract.input_cases)?;
    bounded_cases(contract.output, &contract.output_cases)?;
    bounded_alternatives(contract)
}

fn bounded_cases(tree: &TypeTree, cases: &[Case]) -> Result<(), String> {
    for case in cases {
        routes(&case.components)?;
        for route in &case.components {
            typed_route(tree, route)?;
        }
    }
    Ok(())
}

fn bounded_alternatives(contract: &Contract<'_>) -> Result<(), String> {
    for alternative in &contract.alternatives {
        if alternative.leaves.len() > crate::concrete_types::MAX_FIELDS {
            return Err("Copy component contract exceeds its leaf bound".into());
        }
        for leaf in &alternative.leaves {
            typed_route(contract.input, &leaf.input)?;
            typed_route(contract.output, &leaf.output)?;
        }
    }
    Ok(())
}

fn typed_route(tree: &TypeTree, route: &Route) -> Result<(), String> {
    if route.is_empty()
        || route.len() > crate::concrete_types::MAX_DEPTH
        || route.last() != Some(&Component::SharedReferent)
    {
        return Err(
            "Copy origin requires a bounded route to an explicit reference identity".into(),
        );
    }

    let mut current = tree;
    for component in route {
        current = child(current, component)?;
    }
    if current != &TypeTree::Str {
        return Err(
            "Copy origin route must retain its exact shared string referent identity".into(),
        );
    }
    Ok(())
}

fn child<'a>(tree: &'a TypeTree, component: &Component) -> Result<&'a TypeTree, String> {
    let selected = match (tree, component) {
        (
            TypeTree::Struct {
                nominal, fields, ..
            },
            Component::ProductField {
                nominal: expected,
                position,
                name,
            },
        ) if nominal == expected => fields
            .iter()
            .find(|field| field.position == *position && field.name == *name)
            .map(|field| &field.ty),
        (
            TypeTree::MixedEnum {
                nominal, variants, ..
            },
            Component::EnumPayload {
                nominal: expected,
                variant,
                position,
            },
        ) if nominal == expected => variants
            .iter()
            .find(|candidate| candidate.name == *variant)
            .and_then(|candidate| match &candidate.shape {
                VariantShape::SingleTuple { field } if field.position == *position => {
                    Some(&field.ty)
                }
                _ => None,
            }),
        (TypeTree::Result { ok, .. }, Component::ResultOk) => Some(ok.as_ref()),
        (TypeTree::Result { err, .. }, Component::ResultErr) => Some(err.as_ref()),
        (TypeTree::Shared(inner), Component::SharedReferent) => Some(inner.as_ref()),
        _ => None,
    };
    selected.ok_or_else(|| "Copy origin component changes its exact nominal value route".into())
}

fn routes(routes: &[Route]) -> Result<(), String> {
    if routes.len() > crate::concrete_types::MAX_FIELDS
        || routes
            .iter()
            .any(|route| route.is_empty() || route.len() > crate::concrete_types::MAX_DEPTH)
    {
        return Err("Copy component routes exceed their bounded nonempty vocabulary".into());
    }
    Ok(())
}
