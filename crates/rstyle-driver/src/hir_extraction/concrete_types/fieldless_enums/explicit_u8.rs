//! Compiler observation complements, but never replaces, full lexical repr proof.
use super::*;

pub(super) fn observe(
    context: &crate::hir_extraction::hir::Context<'_>,
    definition: ty::AdtDef<'_>,
    source: &str,
    historical_error: String,
) -> Result<Vec<(String, i64)>, Failure> {
    if !context
        .profile
        .supports(rstyle_pdg::concrete_types::EXPLICIT_U8_CAPABILITY)
    {
        return Err(Failure::Unsupported(historical_error));
    }
    let representation = definition.repr();
    if representation.int != Some(rustc_abi::IntegerType::Fixed(rustc_abi::Integer::I8, false))
        || representation.c()
        || representation.transparent()
        || representation.simd()
        || representation.scalable()
        || representation.align.is_some()
        || representation.pack.is_some()
    {
        return Err(unsupported(
            "Extended fieldless enum requires exact compiler repr(u8)",
        ));
    }
    rstyle_pdg::concrete_types::explicit_u8_fieldless_variants(source).map_err(Failure::Unsupported)
}
