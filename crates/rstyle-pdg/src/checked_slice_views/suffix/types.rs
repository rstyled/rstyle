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
    let expected_range = TypeToken::RangeFromUsize.text()?;
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
        .ok_or("Missing checked suffix get dispatch")?;
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
        || crate::data_aggregates::token(checked.start)? != "usize"
    {
        return Err("Checked suffix get loses its concrete slice/range/result signature".into());
    }

    receiver_signature(node, &result, facts)
}

fn receiver_signature(
    node: &HirNode,
    result: &TypeTree,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|method| method.call == node.hir)
        .ok_or("Missing complete checked suffix get receiver capture")?;

    super::super::shared::signature(
        record,
        result,
        facts,
        super::super::shared::RangeKind::Suffix,
    )
}
