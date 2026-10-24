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
        && (matches!(
            &record.signature_result,
            Observation::Captured {
                ty: TypeTree::Scalar(_)
            }
        ) || crate::range_values::receiver_result(&record.signature_result, facts)?)
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

/// Exact builtin dereference followed by a checked Copy value receiver.
pub(crate) fn shared_copy(call: &HirNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if call.kind != "expr_method_call"
        || !crate::extraction_profile::facts_support(facts, "rust.fieldless-enum-values")
    {
        return Ok(false);
    }

    let shared_nominal = call
        .children
        .first()
        .and_then(|child| child.ty.as_ref())
        .and_then(|ty| ty.normalized.as_deref())
        .is_some_and(|text| {
            text.starts_with("{\"kind\":\"shared\",\"value\":{\"kind\":\"struct\",")
                || text.starts_with("{\"kind\":\"shared\",\"value\":{\"kind\":\"fieldless_enum\",")
        });
    if !shared_nominal {
        return Ok(false);
    }
    if shared_signature(call, facts) {
        return Ok(false);
    }

    let capture = crate::data_receivers::verify(facts)?;
    let Some(record) = capture
        .methods
        .iter()
        .find(|record| record.call == call.hir)
    else {
        return Ok(false);
    };

    shared_copy_record(record, facts)
}

/// Rejection only: ordinary shared methods cannot consume an implicit Copy value.
/// Admission still verifies the complete receiver inventory independently.
fn shared_signature(call: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    let Ok(capture) = facts
        .build
        .frontend_options
        .data::<crate::data_receivers::Capture>()
    else {
        return false;
    };
    capture
        .methods
        .iter()
        .find(|record| record.call == call.hir)
        .is_some_and(|record| {
            matches!(
                record.signature_inputs.first(),
                Some(Observation::Captured {
                    ty: TypeTree::Shared(_)
                })
            )
        })
}

fn shared_copy_record(record: &Method, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Observation::Captured {
        ty: TypeTree::Shared(owner),
    } = &record.receiver_type
    else {
        return Ok(false);
    };
    if !matches!(
        owner.as_ref(),
        TypeTree::Struct { .. } | TypeTree::FieldlessEnum { .. }
    ) {
        return Ok(false);
    }
    let adjusted = Observation::Captured { ty: *owner.clone() };
    Ok(record.adjustments.len() == 1
        && record.adjustments[0].kind == Kind::BuiltinDereference
        && record.adjustments[0].target == adjusted
        && record.adjusted_type == adjusted
        && record.signature_inputs.first() == Some(&adjusted)
        && facts.nominal_resolution()?.checked_owned_copy(owner))
}

pub(crate) fn signature_arguments<'a>(
    call: &HirNode,
    facts: &dyn EvidenceFacts,
    operands: &'a [String],
) -> Result<std::borrow::Cow<'a, [String]>, String> {
    if shared_copy(call, facts)? {
        let receiver = crate::data_types::parse(operands.first().ok_or("Missing Copy receiver")?)?;
        let crate::data_types::TypeToken::Shared(owner) = receiver else {
            return Err("Missing shared Copy receiver".into());
        };
        let mut arguments = operands.to_vec();
        arguments[0] = owner.text()?;
        return Ok(std::borrow::Cow::Owned(arguments));
    }
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
        ty: owner @ (TypeTree::Struct { .. } | TypeTree::FieldlessEnum { .. }),
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
    let copied = crate::extraction_profile::facts_support(facts, "rust.fieldless-enum-values")
        && shared_copy_record(record, facts)?;
    if !(copied || from_record(record, facts)?) {
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
