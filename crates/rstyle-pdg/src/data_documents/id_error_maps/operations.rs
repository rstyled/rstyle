//! Exact source spans and distinct constructs identify every callback operation.
use super::*;

pub(crate) struct Operations<'a> {
    pub receiver: &'a PdgNode,
    pub environment: &'a PdgNode,
    pub selector: &'a PdgNode,
    pub payload: &'a PdgNode,
    pub declaration: &'a PdgNode,
    pub error_read: &'a PdgNode,
    pub message_read: &'a PdgNode,
    pub constructor: &'a PdgNode,
    pub error: &'a PdgNode,
    pub success_payload: &'a PdgNode,
    pub success: &'a PdgNode,
    pub merge: &'a PdgNode,
}

pub(crate) fn operations<'a>(
    pair: PairView<'a>,
    checked: &contract::Checked<'_>,
) -> Result<Operations<'a>, String> {
    let output = crate::data_types::token(&checked.output)?;

    Ok(Operations {
        receiver: super::super::value_operation(pair, checked.receiver)?,
        environment: super::super::typed_operation(
            pair,
            checked.closure,
            contract::ENVIRONMENT,
            &crate::data_types::token(&checked.storage)?,
        )?,
        selector: super::super::typed_operation(pair, checked.source, contract::SELECTOR, "bool")?,
        payload: super::super::typed_operation(
            pair,
            checked.binding,
            contract::PAYLOAD,
            &crate::data_types::token(checked.error())?,
        )?,
        declaration: super::super::typed_operation(
            pair,
            checked.binding,
            contract::BINDING,
            "unit",
        )?,
        error_read: super::super::value_operation(pair, checked.error_read)?,
        message_read: super::super::typed_operation(
            pair,
            checked.message_read,
            contract::MESSAGE,
            &crate::data_types::token(&crate::concrete_types::TypeTree::Shared(Box::new(
                crate::concrete_types::TypeTree::Str,
            )))?,
        )?,
        constructor: super::super::value_operation(pair, checked.constructor)?,
        error: super::super::typed_operation(pair, checked.source, contract::ERROR, &output)?,
        success_payload: super::super::typed_operation(
            pair,
            checked.source,
            contract::SUCCESS,
            &crate::data_types::token(checked.success())?,
        )?,
        success: super::super::typed_operation(pair, checked.source, contract::OK, &output)?,
        merge: merge(pair, checked)?,
    })
}

fn merge<'a>(pair: PairView<'a>, checked: &contract::Checked<'_>) -> Result<&'a PdgNode, String> {
    let merge = super::super::value_operation(pair, checked.source)?;

    if merge.category != NodeCategory::Merge
        || merge.attributes.construct.as_deref() != Some(contract::RESULT)
        || super::super::wire_type(merge)? != crate::data_types::token(&checked.output)?
        || merge.original_kind.known_value() != Some("compat.rust.hir-r0.expression_merge")
    {
        return Err("ID error map loses its complete Result merge".into());
    }

    Ok(merge)
}
