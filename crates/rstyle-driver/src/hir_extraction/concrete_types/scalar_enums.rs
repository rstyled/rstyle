//! Observe complete finite local scalar enum declarations using existing origins.
use super::*;

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn scalar_enum(
        &mut self,
        definition: ty::AdtDef<'tcx>,
        arguments: ty::GenericArgsRef<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        if !self.ctx.profile.supports("rust.scalar-enum-values")
            || !definition.did().is_local()
            || !arguments.is_empty()
            || definition.has_dtor(self.ctx.tcx)
        {
            return Err(unsupported(
                "Scalar enum requires local nongeneric destructor-free declarations",
            ));
        }

        let source = self.declaration(definition.did())?;
        let declarations = variant_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != definition.variants().len() {
            return Err(unsupported(
                "Enum source/resolved variant inventories disagree",
            ));
        }
        let variants = self.enum_variants(
            Instance {
                definition,
                arguments,
            },
            declarations,
            depth,
        )?;

        let nominal = nominal(
            self.ctx,
            self.ctx.tcx.def_path_str(definition.did()),
            &source,
        )?;
        self.origin(definition.did(), &nominal)?;
        Ok(TypeTree::Enum {
            nominal,
            source,
            variants,
        })
    }
    fn enum_variants(
        &mut self,
        instance: Instance<'tcx>,
        declarations: Vec<(String, String)>,
        depth: usize,
    ) -> Result<Vec<Variant>, Failure> {
        let variants = instance
            .definition
            .variants()
            .iter()
            .zip(declarations)
            .enumerate()
            .map(|(position, (variant, (name, declaration)))| {
                if variant.name.to_string() != name || variant.fields.len() != 1 {
                    return Err(unsupported(
                        "Enum source/resolved tuple payload inventories disagree",
                    ));
                }
                let field = variant
                    .fields
                    .iter()
                    .next()
                    .ok_or_else(|| unsupported("Missing enum payload"))?;
                let payload = self.enum_payload(field, instance.arguments, depth)?;
                Ok(Variant {
                    position,
                    name,
                    payload: Field {
                        position: 0,
                        name: "0".into(),
                        declaration: declaration.into(),
                        ty: payload,
                    },
                })
            })
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(variants)
    }

    fn enum_payload(
        &mut self,
        field: &ty::FieldDef,
        arguments: ty::GenericArgsRef<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let tcx = self.ctx.tcx;
        let environment = ty::TypingEnv::post_analysis(tcx, self.ctx.typeck.hir_owner.def_id);
        let payload = tcx
            .try_normalize_erasing_regions(
                environment,
                tcx.type_of(field.did).instantiate(tcx, arguments),
            )
            .map_err(|_| unsupported("Concrete enum payload cannot be resolved"))?;
        if !matches!(payload.kind(), ty::Bool | ty::Int(_) | ty::Uint(_)) {
            return Err(unsupported("Enum requires scalar bool/integer payloads"));
        }

        self.tree(payload, depth + 1)
    }
}
