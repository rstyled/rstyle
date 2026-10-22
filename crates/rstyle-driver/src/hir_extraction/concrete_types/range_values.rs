//! Exact observational capture of the pinned, non-Copy legacy Range<usize>.
use super::*;
use rstyle_pdg::range_values::authority;

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn range_value(
        &self,
        value: Ty<'tcx>,
        depth: usize,
    ) -> Result<Option<TypeTree>, Failure> {
        if !self.ctx.profile.supports("rust.fresh-range-results") {
            return Ok(None);
        }

        let ty::Adt(definition, arguments) = value.kind() else {
            return Ok(None);
        };
        let tcx = self.ctx.tcx;
        if tcx.lang_items().range_struct() != Some(definition.did()) {
            return Ok(None);
        }

        if depth != 0
            || arguments.len() != 1
            || !matches!(arguments[0].kind(), ty::GenericArgKind::Type(ty) if ty == tcx.types.usize)
        {
            return Err(unsupported(
                "Range capture requires a root Range<usize> value",
            ));
        }
        Ok(Some(TypeTree::RangeUsize {
            observation: self.range_observation(value, *definition, arguments),
        }))
    }

    fn range_observation(
        &self,
        value: Ty<'tcx>,
        definition: ty::AdtDef<'tcx>,
        arguments: ty::GenericArgsRef<'tcx>,
    ) -> authority::Observation {
        let tcx = self.ctx.tcx;
        let environment = ty::TypingEnv::post_analysis(tcx, self.ctx.typeck.hir_owner.def_id);
        let fields = definition
            .non_enum_variant()
            .fields
            .iter()
            .enumerate()
            .map(|(position, field)| authority::Field {
                position,
                name: field.name.to_string(),
                scalar: tcx
                    .type_of(field.did)
                    .instantiate(tcx, arguments)
                    // Exact lang-item fields contain only the substituted usize.
                    .skip_normalization()
                    .to_string(),
            })
            .collect();
        authority::Observation {
            definition: super::super::library_resolution::definition(tcx, definition.did()),
            lang_item: "Range".into(),
            arguments: vec![arguments.type_at(0).to_string()],
            fields,
            is_copy: tcx.type_is_copy_modulo_regions(environment, value),
            needs_drop: value.needs_drop(tcx, environment),
            has_drop_impl: definition.has_dtor(tcx),
        }
    }
}
