//! A unit's evidence, derived from its documents and the facts read for all of its
//! functions: its source census, the core libraries its operations select, and its
//! inherent method owners.

use crate::extraction_profile::ExtractionProfile;
use crate::nominal_boundary::census::{self, Census, UnitFile};
use crate::{HirFnFacts, HirNode, data_library, inherent_calls};
use rstyle_core::cpg::model::Document;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

/// What a unit's documents record of the unit itself, gathered as each is read: its
/// files, their absolute paths and the selected unit, as the capture session records
/// them, and the census's compiler observations.
#[derive(Default)]
pub struct Documents {
    files: BTreeMap<String, String>,
    paths: BTreeMap<String, String>,
    selected: Option<Value>,
    observations: BTreeMap<&'static str, Value>,
    generated: BTreeMap<u32, Value>,
}

/// The unit extensions recording the unit's own compiler observations.
const OBSERVATIONS: [&str; 3] = [
    "rust:source-census",
    "rust:dependency-namespaces",
    super::generated_sources::UNIT_EXTENSION,
];

impl Documents {
    pub fn add(&mut self, document: &Document) {
        let Some(unit) = &document.unit else {
            return;
        };
        let files = unit
            .files
            .iter()
            .filter_map(|file| Some((file.id.clone(), file.content.clone()?)));
        self.files.extend(files);
        self.extensions(unit);
        let generated = unit.functions.iter().flatten().filter_map(|function| {
            let extensions = function.extensions.as_ref()?;
            let observed = extensions.get(super::generated_sources::FUNCTION_EXTENSION)?;
            Some((super::hir_ref(&function.root).owner, observed.clone()))
        });

        self.generated.extend(generated);
    }

    /// The capture session and the unit's own observations its extensions record.
    fn extensions(&mut self, unit: &rstyle_core::cpg::model::Unit) {
        let extension = |name: &str| unit.extensions.as_ref()?.get(name);
        if let Some(session) = extension("rust:compiler-session") {
            self.session(session);
        }
        let observed = OBSERVATIONS
            .into_iter()
            .filter_map(|name| Some((name, extension(name)?.clone())));

        self.observations.extend(observed);
    }

    /// A unit observation the documents record, decoded, or the error that prevented
    /// observing it.
    pub(super) fn observation<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        what: &str,
    ) -> Result<T, String> {
        observed(self.observations.get(name), what)
    }

    /// The generated functions among `facts`, by their position, with the sites their
    /// documents record.
    pub(super) fn generated_functions(
        &self,
        facts: &[HirFnFacts],
    ) -> Result<Vec<(usize, super::generated_sources::Function)>, String> {
        let recorded = facts.iter().enumerate().filter_map(|(index, function)| {
            Some((index, self.generated.get(&function.body.hir.owner)?))
        });
        recorded
            .map(|(index, observed)| {
                let function =
                    serde_json::from_value(observed.clone()).map_err(|e| e.to_string())?;
                Ok((index, function))
            })
            .collect()
    }

    /// Each file's absolute path and the selected unit, as the capture session records them.
    fn session(&mut self, session: &Value) {
        let paths = session
            .get("files")
            .and_then(Value::as_object)
            .into_iter()
            .flatten();
        let paths = paths.filter_map(|(id, path)| Some((id.clone(), path.as_str()?.to_owned())));
        self.paths.extend(paths);
        self.selected = session.get("selected_unit").cloned();
    }

    /// The unit's census of its files, without the compiler observations; none outside
    /// a selected unit.
    fn file_census(&self) -> Result<Option<Census>, String> {
        let Some(context) = self.selected.as_ref().filter(|context| !context.is_null()) else {
            return Ok(None);
        };
        let context = serde_json::from_value(context.clone()).map_err(|error| error.to_string())?;
        let mut files = self
            .files
            .iter()
            .filter_map(|(id, content)| {
                let path = self.paths.get(id)?;
                let bytes = crate::SourceSnapshot::from(content.as_str());
                Some(UnitFile {
                    path: path.into(),
                    digest: bytes.checksum().to_owned(),
                    bytes,
                })
            })
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(Some(Census {
            identity: census::IDENTITY.into(),
            context,
            files,
            external_imports: Vec::new(),
            documentation: Vec::new(),
            expansions: Vec::new(),
        }))
    }

    /// The census's compiler observations, as the unit records them.
    fn observations(&self) -> Result<census::Observations, String> {
        observed(
            self.observations.get(OBSERVATIONS[0]),
            "source census observations",
        )
    }
}

/// A unit observation the documents record, or the error that prevented observing it.
fn observed<T: serde::de::DeserializeOwned>(
    observation: Option<&Value>,
    what: &str,
) -> Result<T, String> {
    let observation = observation.ok_or_else(|| format!("The unit records no {what}"))?;
    if let Some(error) = observation.get("error").and_then(Value::as_str) {
        return Err(error.to_owned());
    }
    serde_json::from_value(observation.clone()).map_err(|error| error.to_string())
}

/// Installs the unit's dependency namespaces on its first function's facts, for the
/// data profile that reads them.
fn dependency_namespaces(
    documents: &Documents,
    profile: ExtractionProfile,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    let Some(first) = facts.first_mut() else {
        return Ok(());
    };
    if profile != ExtractionProfile::DataV3 {
        return Ok(());
    }
    let namespaces: crate::dependency_namespaces::capture::UnitCapture = observed(
        documents.observations.get(OBSERVATIONS[1]),
        "dependency namespaces",
    )?;
    first
        .build
        .frontend_options
        .insert_data(Arc::new(namespaces));

    Ok(())
}

/// Installs all of the unit's evidence on its functions' facts: its sources, its
/// generated-source capture, its core libraries and its inherent owners.
pub fn evidence(
    documents: &Documents,
    profile: ExtractionProfile,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    sources(documents, profile, facts)?;
    super::generated_sources::capture(documents, profile, facts)?;
    capture(profile, facts)
}

/// Installs the unit's source evidence on its functions' facts: its census and its
/// dependency namespaces.
fn sources(
    documents: &Documents,
    profile: ExtractionProfile,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    source_census(documents, facts)?;
    dependency_namespaces(documents, profile, facts)
}

/// Installs the unit's census on each of its functions' facts.
fn source_census(documents: &Documents, facts: &mut [HirFnFacts]) -> Result<(), String> {
    let Some(mut census) = documents.file_census()? else {
        return Ok(());
    };
    let observed = documents.observations()?;
    census.external_imports = observed.external_imports;
    census.documentation = observed.documentation;
    census.expansions = observed.expansions;

    retain(census, facts)
}

/// Installs the unit's census of its files alone, for local consumers.
pub fn file_census(documents: &Documents, facts: &mut [HirFnFacts]) -> Result<(), String> {
    match documents.file_census()? {
        Some(census) => retain(census, facts),
        None => Ok(()),
    }
}

/// Installs `census` on each function's facts, within its budget.
fn retain(census: Census, facts: &mut [HirFnFacts]) -> Result<(), String> {
    census::check_budget(&census)?;
    let census = Arc::new(census);
    for function in facts {
        let options = &mut function.build.frontend_options;
        options.insert_data(Arc::clone(&census));
    }

    Ok(())
}

/// Installs the unit's core libraries and inherent owners on its functions' facts.
fn capture(profile: ExtractionProfile, facts: &mut [HirFnFacts]) -> Result<(), String> {
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
fn product_dependency(
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
