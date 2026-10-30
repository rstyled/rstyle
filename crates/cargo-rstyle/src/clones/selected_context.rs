//! Exact per-function association with one checked, immutable selected unit.
use super::capture;
use rstyle_pdg::nominal_boundary::UnitContext;
use serde::Deserialize;

pub(super) struct SelectedContext {
    expected: Option<UnitContext>,
    json: Option<serde_json::Value>,
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
        let json = expected
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| error.to_string())?;
        Ok(Self { expected, json })
    }

    pub fn check(&self, value: Option<&serde_json::Value>) -> Result<(), String> {
        let Some(expected) = &self.expected else {
            return if value.is_none() {
                Ok(())
            } else {
                Err("Unexpected selected-unit data context".into())
            };
        };

        let value = value.ok_or("Missing selected-unit data context")?;
        if self.json.as_ref() == Some(value) {
            return Ok(());
        }
        // Preserve typed path equality for historical equivalent spellings.
        let actual: UnitContext =
            Deserialize::deserialize(value).map_err(|error| error.to_string())?;
        if &actual != expected {
            return Err(
                "Selected-unit context disagrees with frozen Cargo plan or invocation".into(),
            );
        }
        Ok(())
    }
}

/// Compare invocation syntax in place; no argument strings need ownership.
pub(super) fn invocation(value: &serde_json::Value, expected: &[String]) -> Result<(), String> {
    let actual = value
        .as_array()
        .ok_or("Captured invocation is not an array")?;
    if actual.len() < expected.len()
        || actual
            .iter()
            .zip(expected)
            .any(|(actual, expected)| actual.as_str() != Some(expected.as_str()))
    {
        return Err("frontend evidence belongs to another compiler invocation".into());
    }
    match &actual[expected.len()..] {
        [] => Ok(()),
        [flag, path]
            if flag.as_str() == Some("--sysroot")
                && path.as_str().is_some_and(|path| !path.is_empty()) =>
        {
            Ok(())
        }
        _ => Err("frontend invocation disagrees with run".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstyle_pdg::{
        concrete_types::digest,
        nominal_binding::{PACKAGE_IDENTITY, PackageContext},
    };
    use serde_json::json;

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
    fn exact_json_fast_path_preserves_typed_path_equivalence_and_malformed_rejection() {
        let original = context();
        let checked = SelectedContext::checked(Some(original.clone())).unwrap();
        let exact = serde_json::to_value(&original).unwrap();
        let mut equivalent = exact.clone();
        equivalent["root_source"] = "/sample//src/lib.rs".into();
        equivalent["package"]["manifest_path"] = "/sample//Cargo.toml".into();
        assert_ne!(equivalent, exact);
        let typed: UnitContext = serde_json::from_value(equivalent.clone()).unwrap();
        assert_eq!(typed, original);
        assert!(checked.check(Some(&equivalent)).is_ok());
        for field in ["root_source", "crate_name", "package"] {
            let mut changed = exact.clone();
            changed[field] = serde_json::Value::Null;
            assert!(checked.check(Some(&changed)).is_err());
            changed.as_object_mut().unwrap().remove(field);
            assert!(checked.check(Some(&changed)).is_err());
        }
        let mut unknown = exact;
        unknown["package"]["unchecked"] = true.into();
        assert!(checked.check(Some(&unknown)).is_err());
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

    #[test]
    fn invocation_keeps_exact_order_types_and_only_the_existing_sysroot_suffix() {
        let expected = vec!["--crate-name".into(), "sample".into()];
        for (actual, accepted) in [
            (json!(["--crate-name", "sample"]), true),
            (
                json!(["--crate-name", "sample", "--sysroot", "/rust"]),
                true,
            ),
            (json!(["sample", "--crate-name"]), false),
            (json!(["--crate-name"]), false),
            (json!(["--crate-name", "other"]), false),
            (json!(["--crate-name", "sample", "--sysroot", ""]), false),
            (json!(["--crate-name", "sample", "--sysroot", 1]), false),
            (json!(["--crate-name", "sample", "--sysroot"]), false),
            (
                json!(["--crate-name", "sample", "--target", "/rust"]),
                false,
            ),
            (
                json!(["--crate-name", "sample", "--sysroot", "/rust", "x"]),
                false,
            ),
            (json!(["--crate-name", 1]), false),
            (json!([null, "sample"]), false),
            (json!([]), false),
            (json!(null), false),
            (json!({"0":"--crate-name", "1":"sample"}), false),
        ] {
            assert_eq!(invocation(&actual, &expected).is_ok(), accepted, "{actual}");
        }
        assert!(invocation(&json!([]), &[]).is_ok());
        assert!(invocation(&json!(["--sysroot", "/rust"]), &[]).is_ok());
        assert!(invocation(&json!([1]), &[]).is_err());
    }
}
