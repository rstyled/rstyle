//! Bounded original generic product observations, without value/drop authority.
use super::*;
use rstyle_pdg::scalar_product_instances::{Argument, Observation};

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn scalar_product_instance(
        &mut self,
        instance: Instance<'tcx>,
        depth: usize,
    ) -> Result<Option<TypeTree>, Failure> {
        if !self
            .ctx
            .profile
            .supports("rust.slice-prefix-product-results")
        {
            return Ok(None);
        }
        let Some(argument) = scalar_argument(instance.arguments) else {
            return Ok(None);
        };
        let definition = instance.definition;
        if !definition.is_struct()
            || !definition.did().is_local()
            || definition.has_dtor(self.ctx.tcx)
            || self.ctx.tcx.lang_items().unsafe_cell_type() == Some(definition.did())
            || self.visiting.contains(&definition.did())
            || definition.non_enum_variant().fields.len() != 2
        {
            return Err(unsupported(
                "Scalar product instance lacks a bounded local declaration",
            ));
        }

        let source = self.declaration(definition.did())?;
        let declarations = field_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != 2 {
            return Err(unsupported(
                "Scalar product instance field roster disagrees",
            ));
        }
        let fields = self.struct_fields(instance, declarations, depth)?;
        let nominal = nominal(
            self.ctx,
            self.ctx.tcx.def_path_str(definition.did()),
            &source,
        )?;
        let observation = Observation {
            nominal,
            source,
            argument,
            fields,
        };
        observation.check_shape().map_err(Failure::Unsupported)?;
        self.origin(definition.did(), &observation.nominal)?;
        Ok(Some(TypeTree::ScalarProductInstance(Box::new(observation))))
    }
}

fn scalar_argument(arguments: ty::GenericArgsRef<'_>) -> Option<Argument> {
    let [region, value] = &arguments[..] else {
        return None;
    };
    if !matches!(region.kind(), ty::GenericArgKind::Lifetime(_)) {
        return None;
    }
    let ty::GenericArgKind::Type(value) = value.kind() else {
        return None;
    };
    match value.kind() {
        ty::Uint(ty::UintTy::U8) => Some(Argument::U8),
        ty::Uint(ty::UintTy::U16) => Some(Argument::U16),
        _ => None,
    }
}
