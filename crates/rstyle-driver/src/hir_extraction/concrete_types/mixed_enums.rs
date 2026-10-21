//! Observational mixed enum child inventory, including exact constructor shapes.
use super::*;
use rustc_hir::def::CtorKind;

#[derive(Clone, Copy)]
struct VariantSite<'tcx> {
    instance: Instance<'tcx>,
    depth: usize,
}

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn mixed_enum(
        &mut self,
        definition: ty::AdtDef<'tcx>,
        arguments: ty::GenericArgsRef<'tcx>,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        if !definition.did().is_local()
            || !arguments.is_empty()
            || definition.has_dtor(self.ctx.tcx)
            || self.visiting.contains(&definition.did())
        {
            return Err(unsupported(
                "Mixed enum requires finite local nongeneric destructor-free source",
            ));
        }
        let source = self.declaration(definition.did())?;
        let declarations = mixed_variant_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != definition.variants().len() {
            return Err(unsupported(
                "Mixed enum source/resolved variant inventories disagree",
            ));
        }
        let captured = self.mixed_variants(
            Instance {
                definition,
                arguments,
            },
            declarations,
            depth,
        );
        let variants = captured?;
        let nominal = nominal(
            self.ctx,
            self.ctx.tcx.def_path_str(definition.did()),
            &source,
        )?;
        self.origin(definition.did(), &nominal)?;

        let tree = TypeTree::MixedEnum {
            nominal,
            source,
            variants,
        };
        validate(&tree).map_err(Failure::Unsupported)?;
        Ok(tree)
    }

    fn mixed_variants(
        &mut self,
        instance: Instance<'tcx>,
        declarations: Vec<(String, VariantShape<String>)>,
        depth: usize,
    ) -> Result<Vec<MixedVariant>, Failure> {
        self.visiting.push(instance.definition.did());
        let captured = instance
            .definition
            .variants()
            .iter()
            .zip(declarations)
            .enumerate()
            .map(|(position, (variant, (name, declaration)))| {
                if variant.name.to_string() != name {
                    return Err(unsupported("Mixed enum variant identity/order disagrees"));
                }
                let shape =
                    self.mixed_shape(VariantSite { instance, depth }, variant, declaration)?;
                Ok(MixedVariant {
                    position,
                    name,
                    shape,
                })
            })
            .collect();
        self.visiting.pop();
        captured
    }

    fn mixed_shape(
        &mut self,
        site: VariantSite<'tcx>,
        variant: &ty::VariantDef,
        declaration: VariantShape<String>,
    ) -> Result<VariantShape<Field>, Failure> {
        match declaration {
            VariantShape::Unit {}
                if variant.fields.is_empty() && variant.ctor_kind() == Some(CtorKind::Const) =>
            {
                Ok(VariantShape::Unit {})
            }
            VariantShape::SingleTuple { field: declaration }
                if variant.fields.len() == 1 && variant.ctor_kind() == Some(CtorKind::Fn) =>
            {
                self.mixed_payload(site, variant, declaration)
                    .map(|field| VariantShape::SingleTuple { field })
            }
            _ => Err(unsupported(
                "Mixed enum loses its compiler unit/single tuple constructor shape",
            )),
        }
    }

    fn mixed_payload(
        &mut self,
        site: VariantSite<'tcx>,
        variant: &ty::VariantDef,
        declaration: String,
    ) -> Result<Field, Failure> {
        let field = variant
            .fields
            .iter()
            .next()
            .ok_or_else(|| unsupported("Missing mixed enum payload"))?;
        let tcx = self.ctx.tcx;
        let payload = tcx
            .type_of(field.did)
            .instantiate(tcx, site.instance.arguments);
        let environment = ty::TypingEnv::post_analysis(tcx, self.ctx.typeck.hir_owner.def_id);
        let payload = tcx
            .try_normalize_erasing_regions(environment, payload)
            .map_err(|_| unsupported("Mixed enum payload cannot be resolved"))?;

        Ok(Field {
            position: 0,
            name: "0".into(),
            declaration: declaration.into(),
            ty: self.tree(payload, site.depth + 1)?,
        })
    }
}
