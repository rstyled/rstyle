//! Read-only normal flow with explicit panic and checked Copy or no-drop results.
use crate::{data_types::TypeToken, producer::PairView, unit_store::EvidenceFacts};

pub(crate) const SUMMARY: &str = "rstyle.helper.checked-normal-body/1";

pub(crate) fn supported(facts: &dyn EvidenceFacts, pair: PairView<'_>) -> bool {
    pair.cfg.extraction_evidence.status == crate::ExtractionStatus::Partial
        && (copy_context(facts)
            || crate::helper_calls::guarded_effects::supported(facts).unwrap_or(false)
            || crate::helper_calls::id_readers::supported(facts).unwrap_or(false)
            || crate::helper_calls::widening_readers::supported(facts).unwrap_or(false))
}

pub(crate) fn copy_context(facts: &dyn EvidenceFacts) -> bool {
    copy_result(facts)
        && facts
            .signature
            .params
            .iter()
            .all(|parameter| copy_type(facts, &parameter.ty))
        && facts
            .bindings
            .iter()
            .all(|binding| copy_type(facts, &binding.ty))
}

pub(crate) fn copy_result(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, "rust.end-of-input")
        && facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .and_then(|text| crate::data_types::parse(text).ok())
            .is_some_and(|ty| crate::data_aggregates::reference_free(&ty) && copy_token(facts, &ty))
}

fn copy_type(facts: &dyn EvidenceFacts, ty: &crate::ResolvedTy) -> bool {
    ty.normalized
        .as_deref()
        .and_then(|text| crate::data_types::parse(text).ok())
        .is_some_and(|ty| copy_token(facts, &ty))
}

fn copy_token(facts: &dyn EvidenceFacts, ty: &TypeToken) -> bool {
    match ty {
        TypeToken::Scalar(_) | TypeToken::Shared(_) | TypeToken::IntConversionResult(_) => true,
        TypeToken::SliceArrayResult {
            borrowed: false, ..
        } => crate::extraction_profile::facts_support(facts, "rust.accelerator-packing"),
        TypeToken::Tuple(elements) => elements.iter().all(|ty| copy_token(facts, ty)),
        TypeToken::Array { element, .. } | TypeToken::Option(element) => copy_token(facts, element),
        TypeToken::Struct { .. } | TypeToken::Enum { .. } | TypeToken::FieldlessEnum { .. } => {
            facts
                .nominal_resolution()
                .is_ok_and(|resolution| resolution.checked_copy_token(ty))
        }
        _ => false,
    }
}
