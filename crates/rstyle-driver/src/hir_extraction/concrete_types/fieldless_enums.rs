//! Observe complete fieldless enums; source validation independently checks values.
use super::*;

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn fieldless_enum(
        &mut self,
        definition: ty::AdtDef<'tcx>,
        arguments: ty::GenericArgsRef<'tcx>,
    ) -> Result<TypeTree, Failure> {
        if !definition.did().is_local()
            || !arguments.is_empty()
            || definition.has_dtor(self.ctx.tcx)
            || self.ctx.tcx.data_layout.pointer_size().bits() != 64
        {
            return Err(unsupported(
                "Fieldless enum requires local concrete destructor-free 64-bit context",
            ));
        }

        let source = self.declaration(definition.did())?;
        let variants = rstyle_pdg::concrete_types::fieldless_variants(&source)
            .map_err(Failure::Unsupported)?;
        if variants.len() != definition.variants().len() {
            return Err(unsupported("Fieldless enum inventory disagrees"));
        }

        for ((variant, (_, value)), (name, expected)) in definition
            .variants()
            .iter()
            .zip(definition.discriminants(self.ctx.tcx))
            .zip(&variants)
        {
            if !variant.fields.is_empty()
                || variant.name.to_string() != *name
                || value.val as i64 != *expected
            {
                return Err(unsupported(
                    "Resolved fieldless discriminant disagrees with source",
                ));
            }
        }

        let nominal = nominal(
            self.ctx,
            self.ctx.tcx.def_path_str(definition.did()),
            &source,
        )?;
        self.origin(definition.did(), &nominal)?;
        Ok(TypeTree::FieldlessEnum {
            nominal,
            source,
            variants,
        })
    }
}
