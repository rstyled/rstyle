//! Join source-proved unit declarations to exact compiler type/value exports.
//! This evidence grants no callable, trait, or cleanup summary.
use super::generated_units::{Projection, TemplateRole};
use super::{Binding, Import, ProofError, Site, identity, same_definition};
use crate::generated_sources::capture::{self, Definition, Emitted, Expansion, Transparency};
use std::collections::BTreeSet;
#[cfg(test)]
pub(super) mod tests;

pub(crate) struct GeneratedExports {
    projection: Projection,
    import: Site,
}

impl GeneratedExports {
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.projection.units().iter().map(|unit| unit.name())
    }

    pub(crate) fn import(&self) -> &Site {
        &self.import
    }
}

pub(super) fn verify(
    projection: Projection,
    generated: &capture::UnitCapture,
    import: &Import,
) -> Result<GeneratedExports, ProofError> {
    inputs(&projection, generated, import)?;
    let expansion = expansion(&projection, generated, &import.site.target)?;
    let declarations = Declarations {
        projection: &projection,
        generated,
        expansion,
    };
    let mut checked = BTreeSet::new();

    for unit in projection.units() {
        let structure = declarations.declaration(unit.name(), "Struct")?;
        let constructor = declarations.declaration(unit.name(), "Ctor(Struct, Const)")?;
        parents(structure, constructor, &import.site.target)?;
        for item in [structure, constructor] {
            export(import, unit.name(), item)?;
            if !checked.insert(item.definition.definition_identity.as_str()) {
                return Err(ProofError::Contradictory(
                    "Duplicate generated export definition",
                ));
            }
        }
    }
    complete(&projection, generated, expansion, checked.len())?;
    Ok(GeneratedExports {
        projection,
        import: import.site.clone(),
    })
}

fn inputs(
    projection: &Projection,
    generated: &capture::UnitCapture,
    import: &Import,
) -> Result<(), ProofError> {
    if generated.identity != capture::UNIT_IDENTITY || !generated.complete {
        return Err(ProofError::Missing(
            "Incomplete generated namespace unit inventory",
        ));
    }

    let module = &import.site.target;
    if !projection
        .source()
        .matches_identity(&projection.full_definition().digest)
    {
        return Err(ProofError::Contradictory(
            "Generated namespace original template checksum differs",
        ));
    }

    identity(module)?;
    if module.local
        || import.site.target_kind != "Mod"
        || module.path
            != format!(
                "{}::{}",
                module.crate_name,
                projection.namespace().join("::")
            )
        || import.exports.len() != projection.units().len() * 2
    {
        return Err(ProofError::Contradictory(
            "Generated namespace module or export count differs",
        ));
    }

    if !matches!(&import.site.binding, Binding::Glob | Binding::Named(_)) {
        return Err(ProofError::Unsupported(
            "Generated namespace requires a named module or glob",
        ));
    }

    Ok(())
}

pub(super) fn expansion<'a>(
    projection: &Projection,
    generated: &'a capture::UnitCapture,
    module: &Definition,
) -> Result<&'a Expansion, ProofError> {
    let mut matching = generated.expansions.iter().filter(|expansion| {
        expansion.definition.as_ref() == Some(projection.definition())
            && expansion.invocation.raw == *projection.invocation()
    });
    let selected = matching
        .next()
        .ok_or(ProofError::Missing("Missing original unit expansion"))?;

    if matching.next().is_some()
        || selected.kind != capture::ExpansionKind::Bang
        || selected.parent.is_some()
        || selected.local_inner_macros
        || !selected.invocation.marks.is_empty()
    {
        return Err(ProofError::Contradictory(
            "Generated unit expansion ownership differs",
        ));
    }

    let definition = selected
        .macro_definition
        .as_ref()
        .ok_or(ProofError::Missing(
            "Missing original unit macro definition",
        ))?;

    identity(definition)?;
    if !definition.local
        || definition.crate_name != module.crate_name
        || definition.stable_crate_id != module.stable_crate_id
        || definition.path
            != format!(
                "{}::{}",
                projection.namespace().join("::"),
                projection.macro_name()
            )
        || selected.name != projection.macro_name()
    {
        return Err(ProofError::Contradictory(
            "Generated unit macro definition identity differs",
        ));
    }

    Ok(selected)
}

struct Declarations<'a> {
    projection: &'a Projection,
    generated: &'a capture::UnitCapture,
    expansion: &'a Expansion,
}

impl<'a> Declarations<'a> {
    fn declaration(&self, name: &str, kind: &str) -> Result<&'a Emitted, ProofError> {
        let path = format!("{}::{name}", self.projection.namespace().join("::"));
        let mut matching = self
            .generated
            .emitted
            .iter()
            .filter(|item| item.kind == kind && item.definition.path == path);
        let item = matching
            .next()
            .ok_or(ProofError::Missing("Missing generated unit declaration"))?;
        let [mark] = item.origin.marks.as_slice() else {
            return Err(ProofError::Contradictory(
                "Generated unit declaration hygiene differs",
            ));
        };

        if matching.next().is_some()
            || item.origin.raw != *self.projection.template(TemplateRole::Struct)
            || mark.expansion != self.expansion.expansion
            || mark.transparency != Transparency::SemiOpaque
        {
            return Err(ProofError::Contradictory(
                "Generated unit declaration source differs",
            ));
        }

        identity(&item.definition)?;

        Ok(item)
    }
}

fn parents(
    structure: &Emitted,
    constructor: &Emitted,
    module: &Definition,
) -> Result<(), ProofError> {
    if !same_definition(&structure.parent, module)
        || constructor.parent != structure.definition
        || !structure.definition.local
        || !constructor.definition.local
        || structure.definition.crate_name != module.crate_name
        || constructor.definition.crate_name != module.crate_name
        || structure.definition.stable_crate_id != module.stable_crate_id
        || constructor.definition.stable_crate_id != module.stable_crate_id
        || structure.definition.definition_identity == constructor.definition.definition_identity
    {
        return Err(ProofError::Contradictory(
            "Generated type/constructor parent or owner differs",
        ));
    }
    Ok(())
}

fn export(import: &Import, name: &str, item: &Emitted) -> Result<(), ProofError> {
    let matching = import
        .exports
        .iter()
        .filter(|export| {
            export.binding == Binding::Named(name.into()) && export.target_kind == item.kind
        })
        .collect::<Vec<_>>();
    let [export] = matching.as_slice() else {
        return Err(ProofError::Contradictory(
            "Missing or duplicate generated compiler export",
        ));
    };
    if !export.public
        || !export.route.is_empty()
        || !same_definition(&item.definition, &export.target)
    {
        return Err(ProofError::Contradictory(
            "Generated compiler export identity or route differs",
        ));
    }
    Ok(())
}

fn complete(
    projection: &Projection,
    generated: &capture::UnitCapture,
    expansion: &Expansion,
    checked: usize,
) -> Result<(), ProofError> {
    let count = generated
        .emitted
        .iter()
        .filter(|item| {
            matches!(item.kind.as_str(), "Struct" | "Ctor(Struct, Const)")
                && item
                    .origin
                    .marks
                    .iter()
                    .any(|mark| mark.expansion == expansion.expansion)
        })
        .count();
    if count != checked || checked != projection.units().len() * 2 {
        return Err(ProofError::Contradictory(
            "Generated namespace declaration roster differs",
        ));
    }
    Ok(())
}
