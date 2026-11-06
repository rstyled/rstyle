//! What the CLI and the Rust driver agree on about one captured unit.
pub mod diagnostics;

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use xxhash_rust::xxh3::xxh3_64;

pub const PACKAGE_IDENTITY: &str = "rstyle.rust.cargo-package-capture/1";
pub const CONTEXT_FLAG: &str = "--rstyle-selected-unit";
pub const CONTEXT_KEY: &str = "rstyle.nominal_selected_unit";
pub const MAX_CONTEXT_BYTES: usize = 64 * 1024;

/// Nonadversarial byte-consistency checksum, never evidence authorization.
pub fn checksum(bytes: &[u8]) -> String {
    format!("xxh3-64:{:016x}", xxh3_64(bytes))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageContext {
    pub identity: String,
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub manifest_digest: String,
}

impl PackageContext {
    pub fn validate(&self) -> Result<(), String> {
        if self.identity != PACKAGE_IDENTITY
            || self.package_id.is_empty()
            || self.name.is_empty()
            || self.version.is_empty()
            || !self.manifest_path.is_absolute()
            || self
                .manifest_path
                .file_name()
                .is_none_or(|name| name != "Cargo.toml")
        {
            return Err("Invalid or inconsistent Cargo package context".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitContext {
    pub package: PackageContext,
    pub root_source: PathBuf,
    pub crate_name: String,
}

impl UnitContext {
    pub fn validate(&self) -> Result<(), String> {
        self.package.validate()?;
        if self
            .root_source
            .to_string_lossy()
            .split('/')
            .any(|part| part == ".")
            || !self.root_source.is_absolute()
            || self
                .root_source
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || self.crate_name.is_empty()
            || !self
                .crate_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("Invalid selected-unit root or crate identity".into());
        }
        Ok(())
    }

    pub fn transport(&self) -> Result<String, String> {
        self.validate()?;
        let text = serde_json::to_string(self).map_err(|e| e.to_string())?;
        check_transport_budget(&text)?;
        Ok(text)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        check_transport_budget(text)?;
        let context: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        context.validate()?;
        Ok(context)
    }
}

fn check_transport_budget(text: &str) -> Result<(), String> {
    if text.len() > MAX_CONTEXT_BYTES {
        return Err("Selected-unit context exceeds transport budget".into());
    }
    Ok(())
}

/// Normalize an absolute local path lexically, without reopening its source file.
pub fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Nominal binding requires an absolute source path".into());
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir if !result.pop() => {
                return Err("Source path escapes its root".into());
            }
            Component::ParentDir | Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }

    Ok(result)
}
