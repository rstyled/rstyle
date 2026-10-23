//! Common immutable byte-view method shape and receiver evidence.
use crate::{
    HirNode,
    concrete_types::{Observation, TypeTree},
    data_receivers::{Kind, Method},
    data_types::TypeToken,
    unit_store::EvidenceFacts,
};

#[derive(Clone, Copy)]
pub(super) enum RangeKind {
    Prefix,
    Suffix,
}

impl RangeKind {
    fn capability(self) -> &'static str {
        match self {
            Self::Prefix => super::CAPABILITY,
            Self::Suffix => super::suffix::CAPABILITY,
        }
    }

    fn token(self) -> TypeToken {
        match self {
            Self::Prefix => TypeToken::RangeToUsize,
            Self::Suffix => TypeToken::RangeFromUsize,
        }
    }

    fn verify_observation(self, ty: &Observation, facts: &dyn EvidenceFacts) -> Result<(), String> {
        match (self, ty) {
            (
                Self::Prefix,
                Observation::Captured {
                    ty: TypeTree::RangeToUsize { observation },
                },
            ) => crate::range_values::authority::verify_range_to(observation, facts),
            (
                Self::Suffix,
                Observation::Captured {
                    ty: TypeTree::RangeFromUsize { observation },
                },
            ) => crate::range_values::authority::verify_range_from(observation, facts),
            _ => Err("Checked slice get requires its complete captured range observation".into()),
        }
    }
}

pub(super) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts, range: RangeKind) -> bool {
    crate::extraction_profile::facts_support(facts, range.capability())
        && node.kind == "expr_method_call"
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.op == super::GET)
        && node.children.get(1).is_some_and(|input| {
            input.kind == "expr_struct"
                && crate::data_aggregates::parsed_type(input).is_ok_and(|ty| ty == range.token())
        })
}

pub(super) fn signature(
    record: &Method,
    result: &TypeTree,
    facts: &dyn EvidenceFacts,
    range: RangeKind,
) -> Result<(), String> {
    let [input, range_input] = record.signature_inputs.as_slice() else {
        return Err(
            "Checked slice get requires exactly two complete captured input observations".into(),
        );
    };
    range.verify_observation(range_input, facts)?;
    immutable_receiver(record, input, result)
}

fn immutable_receiver(
    record: &Method,
    input: &Observation,
    result: &TypeTree,
) -> Result<(), String> {
    let receiver = Observation::Captured {
        ty: TypeTree::Scalar("u8".into())
            .wrapped(TypeTree::Slice)
            .wrapped(TypeTree::Shared),
    };
    let expected_result = Observation::Captured { ty: result.clone() };
    let kinds: Vec<_> = record.adjustments.iter().map(|step| &step.kind).collect();

    if record.receiver_type != receiver
        || record.adjusted_type != receiver
        || *input != receiver
        || record.signature_result != expected_result
        || !(kinds.is_empty() || kinds == [&Kind::BuiltinDereference, &Kind::SharedBorrow])
    {
        return Err(
            "Checked slice get requires exact immutable instantiated receiver evidence".into(),
        );
    }
    Ok(())
}

pub(super) fn exact_core(
    callee: &crate::data_library::Callee,
    registry: &crate::data_library::Registry,
    paths: &[&str],
    canonical: &str,
) -> bool {
    paths.contains(&callee.path.as_str())
        && registry.exact_core_definition(callee, canonical)
        && callee.implementation.is_none()
}

/// The accepted spelling set and canonical pinned identity are separate claims.
pub(super) struct CoreContract {
    pub paths: &'static [&'static str],
    pub canonical: &'static str,
    pub error: &'static str,
}

pub(super) fn require_exact_core(
    callee: &crate::data_library::Callee,
    registry: &crate::data_library::Registry,
    contract: CoreContract,
) -> Result<(), String> {
    if !exact_core(callee, registry, contract.paths, contract.canonical) {
        return Err(contract.error.into());
    }
    Ok(())
}
