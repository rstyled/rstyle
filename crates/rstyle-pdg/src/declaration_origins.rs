//! Original compiler-file origins for observational nominal types.
//! These checks bind bytes and locations, not effects or package authority.

mod body;
pub(crate) mod copy;
mod files;
pub use body::context;
pub(crate) use files::Sources;

#[cfg(test)]
use crate::concrete_types::digest;
use crate::{
    ByteSpan,
    concrete_types::{InterfaceObservations, Nominal, Observation, TypeTree},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const KEY: &str = "concrete_declaration_origins";
pub const IDENTITY: &str = "rstyle.rust.declaration-origin/1";
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub nominal: Nominal,
    pub source_file: String,
    pub file_bytes: crate::SourceSnapshot,
    pub file_digest: String,
    pub declaration_span: ByteSpan,
    /// Positive resolved observation only; absence does not prove any effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin_copy: Option<BuiltinCopy>,
}

/// Existing declaration ownership binds this observation to its exact struct.
/// It is not an admission capability or a general derive/Drop inventory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuiltinCopy {
    pub trait_definition: crate::data_library::Callee,
    pub derive_definition: crate::data_library::Callee,
    pub derive_span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origins {
    pub identity: String,
    pub declarations: Vec<Origin>,
}

impl Origins {
    pub(crate) fn share_sources(
        &mut self,
        census: &crate::nominal_boundary::census::Census,
    ) -> Result<(), String> {
        for origin in &mut self.declarations {
            let file = census
                .files
                .iter()
                .find(|file| {
                    file.path == std::path::Path::new(&origin.source_file)
                        && file.bytes == origin.file_bytes
                        && file.digest == origin.file_digest
                })
                .ok_or("Declaration origin disagrees with owned source snapshot")?;
            origin.file_bytes = file.bytes.clone();
        }
        Ok(())
    }
}

/// Independently checks a complete, unique declaration inventory against the
/// original file bytes. It does not re-run or compare producer serialization.
pub fn verify(interface: &InterfaceObservations, origins: &Origins) -> Result<(), String> {
    Sources::default().verify(interface, origins)
}

pub(crate) fn verify_in(
    interface: &InterfaceObservations,
    origins: &Origins,
    environment: &crate::nominal_boundary::resolution::Environment,
) -> Result<(), String> {
    verify_inventory(interface, origins, |origin, declaration| {
        environment.check_origin(origin, declaration)
    })
}

fn verify_inventory(
    interface: &InterfaceObservations,
    origins: &Origins,
    mut check: impl FnMut(&Origin, &str) -> Result<(), String>,
) -> Result<(), String> {
    interface.validate()?;
    if origins.identity != IDENTITY
        || crate::evidence_budget::serialized_len(origins).map_err(|e| e.to_string())? > MAX_BYTES
    {
        return Err("Unregistered or over-budget declaration origins".into());
    }
    let expected = declarations(interface)?;
    if expected.len() != origins.declarations.len() {
        return Err("Concrete declaration origin inventory disagrees".into());
    }
    let mut seen = BTreeMap::new();
    for origin in &origins.declarations {
        let key = &origin.nominal.definition_path;
        let (nominal, source) = expected.get(key).ok_or("Unexpected declaration origin")?;
        if seen.insert(key, ()).is_some() || *nominal != &origin.nominal {
            return Err("Duplicate or inconsistent nominal declaration origin".into());
        }
        check(origin, source)?;
    }

    Ok(())
}

pub fn declarations(
    interface: &InterfaceObservations,
) -> Result<BTreeMap<String, (&Nominal, &str)>, String> {
    let mut entries = BTreeMap::new();
    for observation in interface.parameters.iter().chain([&interface.result]) {
        if let Observation::Captured { ty } = observation {
            collect(ty, &mut entries)?;
        }
    }

    Ok(entries)
}

fn collect<'a>(
    tree: &'a TypeTree,
    entries: &mut BTreeMap<String, (&'a Nominal, &'a str)>,
) -> Result<(), String> {
    match tree {
        TypeTree::Struct {
            nominal,
            source,
            fields,
        } => {
            collect_struct(nominal, source, fields, entries)?;
        }
        TypeTree::Tuple(elements) => {
            for element in elements {
                collect(element, entries)?;
            }
        }
        TypeTree::Shared(element) | TypeTree::Slice(element) | TypeTree::Array { element, .. } => {
            collect(element, entries)?
        }
        _ => {}
    }

    Ok(())
}

fn collect_struct<'a>(
    nominal: &'a Nominal,
    source: &'a str,
    fields: &'a [crate::concrete_types::Field],
    entries: &mut BTreeMap<String, (&'a Nominal, &'a str)>,
) -> Result<(), String> {
    if entries
        .insert(nominal.definition_path.clone(), (nominal, source))
        .is_some_and(|previous| previous != (nominal, source))
    {
        return Err("Conflicting observations for one nominal declaration".into());
    }
    for field in fields {
        collect(&field.ty, entries)?;
    }

    Ok(())
}
