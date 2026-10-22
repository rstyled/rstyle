//! Full captured receiver signatures; unsupported observations grant no evidence.
use super::*;
use crate::{
    concrete_types::{Observation, TypeTree},
    data_receivers::Kind,
    data_types::TypeToken,
};

pub(super) fn verify(
    node: &HirNode,
    checked: &Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let expected_receiver = crate::data_types::token(
        &TypeTree::Scalar("u8".into())
            .wrapped(TypeTree::Slice)
            .wrapped(TypeTree::Shared),
    )?;
    let expected_range = TypeToken::RangeToUsize.text()?;
    let result = TypeTree::Option(Box::new(
        TypeTree::Scalar("u8".into())
            .wrapped(TypeTree::Slice)
            .wrapped(TypeTree::Shared),
    ));
    let expected_result = crate::data_types::token(&result)?;
    let expected = [expected_receiver.as_str(), expected_range.as_str()];
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing checked prefix get dispatch")?;
    if dispatch.kind != "resolved_method"
        || dispatch.op != GET
        || dispatch
            .operand_tys
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected
        || dispatch.result_ty != expected_result
        || crate::data_aggregates::token(node)? != expected_result
        || crate::data_aggregates::token(checked.receiver)? != expected_receiver
        || crate::data_aggregates::token(checked.range)? != expected_range
        || crate::data_aggregates::token(checked.end)? != "usize"
    {
        return Err("Checked prefix get loses its concrete slice/range/result signature".into());
    }
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|method| method.call == node.hir)
        .ok_or("Missing complete checked prefix get receiver capture")?;

    signature(record, &result, facts)
}

fn signature(
    record: &crate::data_receivers::Method,
    result: &TypeTree,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let captured = |ty| Observation::Captured { ty };
    let [
        input,
        Observation::Captured {
            ty: TypeTree::RangeToUsize { observation },
        },
    ] = record.signature_inputs.as_slice()
    else {
        return Err("Checked prefix get requires complete captured input observations".into());
    };
    crate::range_values::authority::verify_range_to(observation, facts)?;
    let kinds: Vec<_> = record.adjustments.iter().map(|step| &step.kind).collect();
    if record.receiver_type
        != captured(
            TypeTree::Scalar("u8".into())
                .wrapped(TypeTree::Slice)
                .wrapped(TypeTree::Shared),
        )
        || record.adjusted_type
            != captured(
                TypeTree::Scalar("u8".into())
                    .wrapped(TypeTree::Slice)
                    .wrapped(TypeTree::Shared),
            )
        || *input
            != captured(
                TypeTree::Scalar("u8".into())
                    .wrapped(TypeTree::Slice)
                    .wrapped(TypeTree::Shared),
            )
        || record.signature_result != captured(result.clone())
        || !(kinds.is_empty() || kinds == [&Kind::BuiltinDereference, &Kind::SharedBorrow])
    {
        return Err(
            "Checked prefix get requires exact immutable instantiated receiver evidence".into(),
        );
    }

    Ok(())
}
