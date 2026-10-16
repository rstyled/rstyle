//! Explicit selected-unit evidence for the local nominal data contract.

pub mod census;
pub mod resolution;

use crate::nominal_binding::PackageContext;
use serde::{Deserialize, Serialize};
use std::path::{Component, PathBuf};

pub const CONTEXT_FLAG: &str = "--rstyle-selected-unit";
pub const CONTEXT_KEY: &str = "rstyle.nominal_selected_unit";
pub const MAX_CONTEXT_BYTES: usize = 64 * 1024;

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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{concrete_types::digest, nominal_binding::PACKAGE_IDENTITY};

    pub fn context() -> UnitContext {
        UnitContext {
            package: PackageContext {
                identity: PACKAGE_IDENTITY.into(),
                package_id: "path+file:///sample#sample@1.0.0".into(),
                name: "sample".into(),
                version: "1.0.0".into(),
                manifest_path: "/sample/Cargo.toml".into(),
                manifest_bytes: "[package]".into(),
                manifest_digest: digest(b"[package]"),
            },
            root_source: "/sample/src/lib.rs".into(),
            crate_name: "sample".into(),
        }
    }

    #[test]
    fn context_transport_preserves_namespace_and_rejects_unknown_or_corrupt_fields() {
        let original = context();
        assert_eq!(
            UnitContext::parse(&original.transport().unwrap()).unwrap(),
            original
        );
        let mut value = serde_json::to_value(&original).unwrap();
        value["trusted"] = true.into();
        assert!(UnitContext::parse(&value.to_string()).is_err());
        let mut changed = original.clone();
        changed.package.manifest_bytes = format!("{}x", changed.package.manifest_bytes).into();
        assert!(changed.transport().is_err());
        changed = original;
        changed.package.package_id.push('x');
        assert_ne!(changed, context());
    }

    #[test]
    fn invalid_navigation_and_transport_overflow_fail_explicitly() {
        for root in ["src/lib.rs", "/sample/../other/lib.rs", "/sample/./lib.rs"] {
            let mut value = context();
            value.root_source = root.into();
            assert!(value.transport().is_err());
        }
        let mut value = context();
        value.crate_name = "sample-invalid".into();
        assert!(value.transport().is_err());
        value = context();
        value.package.manifest_bytes = "x".repeat(MAX_CONTEXT_BYTES).into();
        value.package.manifest_digest = digest(value.package.manifest_bytes.as_bytes());
        assert!(value.transport().is_err());
        assert!(UnitContext::parse(&" ".repeat(MAX_CONTEXT_BYTES + 1)).is_err());
    }
}
