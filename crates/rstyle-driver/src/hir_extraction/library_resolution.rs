//! Compiler-resolved core identities under the supported local toolchain contract.

use super::Options;
use rstyle_pdg::{HirFnFacts, HirNode, data_library::*};
use rustc_hir::{
    HirId, ItemLocalId, OwnerId,
    def_id::{DefId, DefIndex, LocalDefId},
};
use rustc_middle::ty::TyCtxt;
use std::collections::BTreeMap;

pub fn capture(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if !options.selected().is_data() {
        return Ok(());
    }
    let mut callees = BTreeMap::new();
    for function in facts.iter() {
        collect(tcx, &function.body, &mut callees)?;
    }

    let libraries = callees.into_values().collect();
    let resolution = Libraries {
        identity: IDENTITY.into(),
        libraries,
    };
    let value = serde_json::to_value(resolution).map_err(|error| error.to_string())?;
    for function in facts {
        function
            .build
            .frontend_options
            .insert(KEY.into(), value.clone());
    }
    Ok(())
}

fn collect(
    tcx: TyCtxt<'_>,
    node: &HirNode,
    callees: &mut BTreeMap<String, Library>,
) -> Result<(), String> {
    if operation(node).is_some() {
        let id = HirId {
            owner: OwnerId {
                def_id: LocalDefId {
                    local_def_index: DefIndex::from_u32(node.hir.owner),
                },
            },
            local_id: ItemLocalId::from_u32(node.hir.local),
        };
        let callee = tcx
            .typeck(id.owner.def_id)
            .type_dependent_def_id(id)
            .ok_or("Missing compiler-selected library callee")?;
        if !callee.is_local() && tcx.crate_name(callee.krate).as_str() == "core" {
            let library = library(tcx, callee);
            callees.insert(library.crate_hash.clone(), library);
        }
    }
    collect_product_dependency(node, callees)?;
    for child in &node.children {
        collect(tcx, child, callees)?;
    }
    Ok(())
}

/// Const initializers are dependencies, not runtime calls in the captured body.
fn collect_product_dependency(
    node: &HirNode,
    libraries: &mut BTreeMap<String, Library>,
) -> Result<(), String> {
    let Some(resolution) = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == rstyle_pdg::nominal_constants::RESOLUTION)
    else {
        return Ok(());
    };
    let record = rstyle_pdg::nominal_constants::decode(resolution)?;
    let dependency = record.initializer.core_constant.definition;
    if dependency.crate_name != "core" || dependency.crate_hash.is_empty() {
        return Err("Product constant lacks its compiler-selected core dependency".into());
    }
    libraries.insert(
        dependency.crate_hash.clone(),
        Library {
            crate_name: dependency.crate_name,
            crate_hash: dependency.crate_hash,
        },
    );

    Ok(())
}

fn library(tcx: TyCtxt<'_>, callee: DefId) -> Library {
    Library {
        crate_name: "core".into(),
        crate_hash: tcx.crate_hash(callee.krate).to_string(),
    }
}

/// One exact compiler definition identity, without granting operation support.
pub(super) fn definition(tcx: TyCtxt<'_>, definition: DefId) -> Callee {
    Callee {
        path: tcx.def_path_str(definition),
        crate_name: tcx.crate_name(definition.krate).to_string(),
        crate_hash: tcx.crate_hash(definition.krate).to_string(),
        definition_hash: format!("{:?}", tcx.def_path_hash(definition)),
        implementation: None,
    }
}
