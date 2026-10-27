//! Compiler identities for bounded core nonzero values; no source-call permission.
use super::*;
use rstyle_pdg::nonzero_values::{Definition, Observation, Scalar};

impl Capture<'_, '_> {
    pub(super) fn nonzero_value(&self, value: Ty<'_>) -> Result<Option<TypeTree>, Failure> {
        let ty::Adt(definition, arguments) = value.kind() else {
            return Ok(None);
        };
        let tcx = self.ctx.tcx;
        if self.ctx.profile != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
            || !tcx.is_diagnostic_item(rustc_span::sym::NonZero, definition.did())
        {
            return Ok(None);
        }
        let [argument] = &arguments[..] else {
            return Err(unsupported(
                "NonZero requires one exact concrete integer argument",
            ));
        };
        let scalar = match argument.kind() {
            ty::GenericArgKind::Type(value) => match value.kind() {
                ty::Uint(ty::UintTy::U8) => Scalar::U8,
                ty::Uint(ty::UintTy::U16) => Scalar::U16,
                _ => return Err(unsupported("NonZero supports only concrete u8/u16 values")),
            },
            _ => return Err(unsupported("NonZero requires a concrete type argument")),
        };

        let id = definition.did();
        let observation = Observation {
            definition: Definition {
                path: tcx.def_path_str(id),
                definition_identity: format!("{:?}", tcx.def_path_hash(id)),
                crate_name: tcx.crate_name(id.krate).to_string(),
                stable_crate_id: format!("{:?}", tcx.stable_crate_id(id.krate)),
                local: id.is_local(),
            },
            scalar,
        };
        let observation = observation.canonicalized().map_err(Failure::Unsupported)?;
        Ok(Some(TypeTree::NonZero(observation)))
    }
}
