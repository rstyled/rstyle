//! Every source repetition contributes its own exact definitions and parent edges.
use super::*;

pub(super) fn units(
    projection: &Projection,
    inventory: &mut Inventory<'_>,
    expansion: &Expansion,
    module: &Definition,
) -> Result<(), ProofError> {
    use TemplateRole::*;
    let mut ctx = Context {
        projection,
        inventory,
        mark: mark(expansion, Transparency::SemiOpaque),
    };
    for unit in projection.units() {
        let name = path(projection, unit.name());
        let structure = ctx.take(TemplateRecord {
            kind: "Struct",
            path: name.clone(),
            parent: module,
            role: Struct,
        })?;
        ctx.take(TemplateRecord {
            kind: "Ctor(Struct, Const)",
            path: name.clone(),
            parent: &structure.definition,
            role: Struct,
        })?;
        let implementation = ctx.take(TemplateRecord {
            kind: "Impl { of_trait: false }",
            path: name.clone(),
            parent: module,
            role: InherentImpl,
        })?;
        inherent_members(&mut ctx, &name, &implementation.definition)?;
    }
    Ok(())
}

fn inherent_members(
    ctx: &mut Context<'_, '_, '_>,
    name: &str,
    implementation: &Definition,
) -> Result<(), ProofError> {
    use TemplateRole::*;
    let per = ctx.take(TemplateRecord {
        kind: "AssocFn",
        path: format!("{name}::per"),
        parent: implementation,
        role: Per,
    })?;
    ctx.take(TemplateRecord {
        kind: "TyParam",
        path: format!("{name}::per::T"),
        parent: &per.definition,
        role: PerTypeParameter,
    })?;
    let per_t = ctx.take(TemplateRecord {
        kind: "AssocFn",
        path: format!("{name}::per_t"),
        parent: implementation,
        role: PerT,
    })?;
    ctx.take(TemplateRecord {
        kind: "TyParam",
        path: format!("{name}::per_t::Output"),
        parent: &per_t.definition,
        role: PerTOutput,
    })?;
    ctx.take(TemplateRecord {
        kind: "TyParam",
        path: format!("{name}::per_t::impl MultipleOf<Self, Output> + Copy"),
        parent: &per_t.definition,
        role: PerTOpaque,
    })?;
    Ok(())
}

pub(super) fn ratios(
    projection: &Projection,
    inventory: &mut Inventory<'_>,
    expansion: &Expansion,
    module: &Definition,
) -> Result<(), ProofError> {
    let mut ctx = Context {
        projection,
        inventory,
        mark: mark(expansion, Transparency::SemiOpaque),
    };
    for unit in projection.units() {
        ctx.ratios(unit, module)?;
    }
    Ok(())
}

impl Context<'_, '_, '_> {
    fn ratios(
        &mut self,
        unit: &super::super::generated_units::Unit,
        module: &Definition,
    ) -> Result<(), ProofError> {
        let name = path(self.projection, unit.name());
        let multiple = path(self.projection, "sealed::MultipleOf");
        let default = path(self.projection, "sealed::DefaultOutput");
        for ratio in unit.ratios() {
            let larger = path(self.projection, ratio.larger());
            for (outputs, implementation_role, value_role) in [
                (
                    ratio.integers(),
                    TemplateRole::IntegerImpl,
                    TemplateRole::IntegerValue,
                ),
                (
                    ratio.floats(),
                    TemplateRole::FloatImpl,
                    TemplateRole::FloatValue,
                ),
            ] {
                for output in outputs {
                    let owner = format!("<{larger} as {multiple}<{name}, {output}>>");
                    let implementation = self.take(TemplateRecord {
                        kind: "Impl { of_trait: true }",
                        path: owner.clone(),
                        parent: module,
                        role: implementation_role,
                    })?;
                    self.take(TemplateRecord {
                        kind: "AssocConst",
                        path: format!("{owner}::VALUE"),
                        parent: &implementation.definition,
                        role: value_role,
                    })?;
                }
            }
            let owner = format!("<{larger} as {default}<{name}>>");
            self.default_output(owner, module)?;
        }
        Ok(())
    }

    fn default_output(&mut self, owner: String, module: &Definition) -> Result<(), ProofError> {
        let implementation = self.take(TemplateRecord {
            kind: "Impl { of_trait: true }",
            path: owner.clone(),
            parent: module,
            role: TemplateRole::DefaultImpl,
        })?;
        self.take(TemplateRecord {
            kind: "AssocTy",
            path: format!("{owner}::Output"),
            parent: &implementation.definition,
            role: TemplateRole::DefaultType,
        })?;
        Ok(())
    }
}
