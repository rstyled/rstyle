//! Full captured receiver signatures; unsupported observations grant no evidence.
use super::*;
use crate::{concrete_types::TypeTree, data_types::TypeToken};

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

    super::shared::signature(record, &result, facts, super::shared::RangeKind::Prefix)
}
