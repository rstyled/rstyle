//! A checked implicit shared borrow of an existing owned Copy value snapshot.
use crate::{
    HirNode,
    concrete_types::{Observation, TypeTree},
    data_receivers::{Kind, Method},
    unit_store::EvidenceFacts,
};

pub(crate) fn from_record(record: &Method, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Observation::Captured {
        ty: tree @ TypeTree::Struct { .. },
    } = &record.receiver_type
    else {
        return Ok(false);
    };
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.owned-copy-receiver")
    {
        return Ok(false);
    }
    let Observation::Captured {
        ty: TypeTree::Shared(adjusted),
    } = &record.adjusted_type
    else {
        return Ok(false);
    };
    Ok(adjusted.as_ref() == tree
        && record.adjustments.len() == 1
        && record.adjustments[0].kind == Kind::SharedBorrow
        && record.adjustments[0].target == record.adjusted_type
        && record.signature_inputs.first() == Some(&record.adjusted_type)
        && matches!(
            &record.signature_result,
            Observation::Captured {
                ty: TypeTree::Scalar(_)
            }
        )
        && facts.nominal_resolution()?.checked_owned_copy(tree))
}

pub(crate) fn checked(call: &HirNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if call.kind != "expr_method_call"
        || !call
            .children
            .first()
            .and_then(|child| child.ty.as_ref())
            .and_then(|ty| ty.normalized.as_deref())
            .is_some_and(|ty| {
                crate::data_types::parse(ty)
                    .is_ok_and(|ty| matches!(ty, crate::data_types::TypeToken::Struct { .. }))
            })
    {
        return Ok(false);
    }
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|record| record.call == call.hir)
        .ok_or("Missing owned receiver adjustment")?;
    from_record(record, facts)
}

pub(crate) fn signature_arguments<'a>(
    call: &HirNode,
    facts: &dyn EvidenceFacts,
    operands: &'a [String],
) -> Result<std::borrow::Cow<'a, [String]>, String> {
    if !checked(call, facts)? {
        return Ok(std::borrow::Cow::Borrowed(operands));
    }

    let receiver =
        crate::data_types::parse(operands.first().ok_or("Missing owned receiver type")?)?;
    let mut arguments = operands.to_vec();
    arguments[0] = crate::data_types::TypeToken::Shared(Box::new(receiver)).text()?;
    Ok(std::borrow::Cow::Owned(arguments))
}

/// A by-value receiver reuses the independently checked nominal Copy boundary.
pub(crate) fn value_self(facts: &dyn EvidenceFacts, owner: &TypeTree) -> Result<bool, String> {
    Ok(
        crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.owned-copy-self")
            && facts.nominal_resolution()?.checked_owned_copy(owner),
    )
}

pub(crate) fn value_call(record: &Method, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Observation::Captured {
        ty: owner @ TypeTree::Struct { .. },
    } = &record.receiver_type
    else {
        return Ok(false);
    };
    Ok(record.adjustments.is_empty()
        && record.adjusted_type == record.receiver_type
        && record.signature_inputs.first() == Some(&record.receiver_type)
        && value_self(facts, owner)?)
}

/// Convert an already checked instantiated signature to the source operand types.
pub(crate) fn call_signature(
    record: &Method,
    facts: &dyn EvidenceFacts,
    signature: &mut [String],
) -> Result<bool, String> {
    if value_call(record, facts)? {
        return Ok(true);
    }
    if !from_record(record, facts)? {
        return Ok(false);
    }

    let Observation::Captured { ty } = &record.receiver_type else {
        return Err("Missing concrete owned receiver".into());
    };
    *signature
        .first_mut()
        .ok_or("Missing instantiated receiver input")? = crate::data_types::token(ty)?;
    Ok(true)
}
