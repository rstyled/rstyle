//! A unit's evidence, derived from the facts read for all of its functions: the core
//! libraries its operations select, and its inherent method owners.

use crate::extraction_profile::ExtractionProfile;
use crate::{HirFnFacts, HirNode, data_library, inherent_calls};
use std::collections::BTreeMap;

/// Installs the unit's evidence on each of its functions' facts.
pub fn capture(profile: ExtractionProfile, facts: &mut [HirFnFacts]) -> Result<(), String> {
    if profile.is_data() {
        libraries(facts)?;
    }
    if profile.supports("rust.verified-inherent-helper-calls") {
        inherent_units(facts)?;
    }

    Ok(())
}

/// The core libraries the unit's library operations and product constants select.
fn libraries(facts: &mut [HirFnFacts]) -> Result<(), String> {
    let mut libraries = BTreeMap::new();
    for function in facts.iter() {
        collect(&function.body, &mut libraries)?;
    }
    let resolution = data_library::Libraries {
        identity: data_library::IDENTITY.into(),
        libraries: libraries.into_values().collect(),
    };
    let value = serde_json::to_value(resolution).map_err(|error| error.to_string())?;
    for function in facts {
        let options = &mut function.build.frontend_options;
        options.insert(data_library::KEY.into(), value.clone());
    }

    Ok(())
}

fn collect(
    node: &HirNode,
    libraries: &mut BTreeMap<String, data_library::Library>,
) -> Result<(), String> {
    if data_library::operation(node).is_some() {
        let callee = data_library::callee(node)?;
        if callee.crate_name == "core" {
            libraries.insert(
                callee.crate_hash.clone(),
                data_library::Library {
                    crate_name: callee.crate_name,
                    crate_hash: callee.crate_hash,
                },
            );
        }
    }
    product_dependency(node, libraries)?;
    node.children
        .iter()
        .try_for_each(|child| collect(child, libraries))
}

/// Const initializers are dependencies, not runtime calls in the captured body.
pub fn product_dependency(
    node: &HirNode,
    libraries: &mut BTreeMap<String, data_library::Library>,
) -> Result<(), String> {
    let Some(resolution) = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == crate::nominal_constants::RESOLUTION)
    else {
        return Ok(());
    };
    let record = crate::nominal_constants::decode(resolution)?;
    let dependency = record.initializer.core_constant.definition;
    if dependency.crate_name != "core" || dependency.crate_hash.is_empty() {
        return Err("Product constant lacks its compiler-selected core dependency".into());
    }
    libraries.insert(
        dependency.crate_hash.clone(),
        data_library::Library {
            crate_name: dependency.crate_name,
            crate_hash: dependency.crate_hash,
        },
    );

    Ok(())
}

/// The unit's inherent method definitions, shared by every function.
fn inherent_units(facts: &mut [HirFnFacts]) -> Result<(), String> {
    let mut definitions = facts
        .iter()
        .filter_map(|function| function.build.frontend_options.get(inherent_calls::KEY))
        .map(|value| {
            serde_json::from_value::<inherent_calls::Definition>(value.clone())
                .map(inherent_calls::Association::from)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    definitions.sort_by(|left, right| left.definition_identity.cmp(&right.definition_identity));
    let inventory = inherent_calls::UnitDefinitions {
        identity: inherent_calls::UNIT_IDENTITY.into(),
        definitions,
    };
    inventory.validate()?;
    let mut shared = crate::FrontendOptions::default();
    shared.insert_data(std::sync::Arc::new(inventory));

    facts.iter_mut().try_for_each(|function| {
        function
            .build
            .frontend_options
            .share_inherent_inventory(&shared)
    })
}
