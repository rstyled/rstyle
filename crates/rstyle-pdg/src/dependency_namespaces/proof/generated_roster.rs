//! Exhaust the source-derived emitted graph; no callable or cleanup permission.
mod derives;
mod expected;
mod inventory;
#[cfg(test)]
mod tests;

use super::{
    ProofError,
    generated_units::{Projection, TemplateRole},
};
use crate::generated_sources::capture::{self, Definition, Emitted, Expansion, Mark, Transparency};
use inventory::{Context, Expected, Inventory, TemplateRecord};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn verify(
    projection: &Projection,
    generated: &capture::UnitCapture,
    expansion: &Expansion,
    module: &Definition,
) -> Result<(), ProofError> {
    if generated.identity != capture::UNIT_IDENTITY
        || generated.scope != "selected-local-bang-expansion-closure"
        || expansion.kind != capture::ExpansionKind::Bang
    {
        return Err(ProofError::Contradictory(
            "Generated roster capture scope differs",
        ));
    }
    let graph = derives::Graph::new(projection, generated, expansion)?;
    let module = local_module(projection, module)?;
    let mut inventory = Inventory::new(projection, generated, &graph, &module)?;
    expected::units(projection, &mut inventory, expansion, &module)?;
    expected::ratios(projection, &mut inventory, expansion, &module)?;
    derives::units(projection, &mut inventory, &graph, &module)?;
    inventory.finish()
}

fn local_module(projection: &Projection, module: &Definition) -> Result<Definition, ProofError> {
    if module.local
        || module.crate_name.is_empty()
        || module.stable_crate_id.is_empty()
        || module.definition_identity.is_empty()
        || module.path
            != format!(
                "{}::{}",
                module.crate_name,
                projection.namespace().join("::")
            )
    {
        return Err(ProofError::Contradictory(
            "Generated roster module identity differs",
        ));
    }
    let mut local = module.clone();
    local.local = true;
    local.path = projection.namespace().join("::");
    Ok(local)
}

fn path(projection: &Projection, name: &str) -> String {
    format!("{}::{name}", projection.namespace().join("::"))
}

fn mark(expansion: &Expansion, transparency: Transparency) -> Mark {
    Mark {
        expansion: expansion.expansion.clone(),
        transparency,
    }
}
