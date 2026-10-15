//! Exact per-function association with one checked, immutable selected unit.
use super::capture;
use rstyle_pdg::nominal_boundary::UnitContext;
use serde::Deserialize;

pub(super) struct SelectedContext {
    expected: Option<UnitContext>,
}

impl SelectedContext {
    pub fn new(unit: &capture::Unit) -> Result<Self, String> {
        let expected = unit
            .extraction_profile
            .is_data()
            .then(|| capture::selected_unit_context(&unit.target, &unit.invocation))
            .transpose()?;
        Self::checked(expected)
    }

    fn checked(expected: Option<UnitContext>) -> Result<Self, String> {
        if let Some(context) = &expected {
            context.transport()?;
        }
        Ok(Self { expected })
    }

    pub fn check(&self, value: Option<&serde_json::Value>) -> Result<(), String> {
        let Some(expected) = &self.expected else {
            return if value.is_none() {
                Ok(())
            } else {
                Err("Unexpected selected-unit data context".into())
            };
        };

        let actual: UnitContext =
            Deserialize::deserialize(value.ok_or("Missing selected-unit data context")?)
                .map_err(|error| error.to_string())?;
        if &actual != expected {
            return Err(
                "Selected-unit context disagrees with frozen Cargo plan or invocation".into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstyle_pdg::{
        concrete_types::digest,
        nominal_binding::{PACKAGE_IDENTITY, PackageContext},
    };

    fn context() -> UnitContext {
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
    fn checked_owner_requires_exact_context_for_every_observation() {
        let mut original = context();
        let checked = SelectedContext::checked(Some(original.clone())).unwrap();
        let value = serde_json::to_value(&original).unwrap();
        assert!(checked.check(Some(&value)).is_ok());
        assert!(checked.check(Some(&value)).is_ok());
        assert!(checked.check(None).is_err());

        original.package.manifest_bytes = format!("{}x", original.package.manifest_bytes).into();
        assert!(SelectedContext::checked(Some(original.clone())).is_err());
        assert!(
            checked
                .check(Some(&serde_json::to_value(&original).unwrap()))
                .is_err()
        );
        original.package.manifest_digest = digest(original.package.manifest_bytes.as_bytes());
        assert!(SelectedContext::checked(Some(original.clone())).is_ok());
        assert!(
            checked
                .check(Some(&serde_json::to_value(original).unwrap()))
                .is_err()
        );
    }

    #[test]
    fn context_presence_and_all_selected_fields_remain_checked() {
        let value = serde_json::to_value(context()).unwrap();
        let absent = SelectedContext::checked(None).unwrap();
        assert!(absent.check(None).is_ok());
        assert!(absent.check(Some(&value)).is_err());
        let checked = SelectedContext::checked(Some(context())).unwrap();

        for field in ["root_source", "crate_name"] {
            let mut changed = value.clone();
            changed[field] = "other".into();
            assert!(checked.check(Some(&changed)).is_err());
        }
        for field in [
            "identity",
            "package_id",
            "name",
            "version",
            "manifest_path",
            "manifest_digest",
        ] {
            let mut changed = value.clone();
            changed["package"][field] = "other".into();
            assert!(checked.check(Some(&changed)).is_err());
        }
        let mut unknown = value;
        unknown["unchecked"] = true.into();
        assert!(checked.check(Some(&unknown)).is_err());
    }
}
