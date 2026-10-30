//! Unit-owned build settings. Function observations are not build identities.
use crate::BuildFacts;
use serde::Serialize;

const OPTIONS: [&str; 5] = [
    "edition",
    "target",
    "invocation",
    crate::extraction_profile::SELECTION_KEY,
    crate::nominal_boundary::CONTEXT_KEY,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BuildId(u32);

#[derive(Serialize)]
pub struct BuildContexts {
    identity: &'static str,
    contexts: Vec<BuildFacts>,
}

impl Default for BuildContexts {
    fn default() -> Self {
        Self {
            identity: "rstyle.rust.unit-build-contexts/1",
            contexts: Vec::new(),
        }
    }
}

impl BuildContexts {
    /// Retain the existing exact compiler settings, excluding per-function observations.
    pub fn intern(&mut self, build: &BuildFacts) -> Result<BuildId, String> {
        if let Some(index) = self.contexts.iter().position(|value| same(value, build)) {
            return Ok(BuildId(index as u32));
        }
        let id =
            BuildId(u32::try_from(self.contexts.len()).map_err(|_| "Too many build contexts")?);
        self.contexts.push(settings(build));
        Ok(id)
    }
}

fn same(first: &BuildFacts, second: &BuildFacts) -> bool {
    first.producer_revision == second.producer_revision
        && first.rustc_version == second.rustc_version
        && first.rustc_commit == second.rustc_commit
        && first.edition == second.edition
        && first.target_triple == second.target_triple
        && first.pointer_width == second.pointer_width
        && first.data_layout == second.data_layout
        && first.cfgs == second.cfgs
        && first.features == second.features
        && first.overflow_checks == second.overflow_checks
        && first.debug_assertions == second.debug_assertions
        && first.panic_strategy == second.panic_strategy
        && first.opt_level == second.opt_level
        && first
            .frontend_options
            .same_selected(&second.frontend_options, &OPTIONS)
}

fn settings(build: &BuildFacts) -> BuildFacts {
    BuildFacts {
        producer_revision: build.producer_revision.clone(),
        frontend_options: build.frontend_options.selected(&OPTIONS),
        rustc_version: build.rustc_version.clone(),
        rustc_commit: build.rustc_commit.clone(),
        edition: build.edition.clone(),
        target_triple: build.target_triple.clone(),
        pointer_width: build.pointer_width.clone(),
        data_layout: build.data_layout.clone(),
        cfgs: build.cfgs.clone(),
        features: build.features.clone(),
        overflow_checks: build.overflow_checks.clone(),
        debug_assertions: build.debug_assertions.clone(),
        panic_strategy: build.panic_strategy.clone(),
        opt_level: build.opt_level.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{known_bool, known_count, known_str};
    use serde_json::json;

    fn build() -> BuildFacts {
        BuildFacts {
            producer_revision: known_str("revision"),
            frontend_options: std::collections::BTreeMap::from([
                ("invocation".into(), json!(["rustc", "--crate-type=lib"])),
                (
                    crate::nominal_boundary::CONTEXT_KEY.into(),
                    json!({"package":"one", "target":"lib"}),
                ),
            ])
            .into(),
            rustc_version: known_str("rustc"),
            rustc_commit: known_str("commit"),
            edition: known_str("2021"),
            target_triple: known_str("aarch64-unknown-linux-gnu"),
            pointer_width: known_count(64),
            data_layout: known_str("layout"),
            cfgs: vec!["debug_assertions".into()],
            features: vec!["feature".into()],
            overflow_checks: known_bool(true),
            debug_assertions: known_bool(true),
            panic_strategy: known_str("unwind"),
            opt_level: known_str("No"),
        }
    }

    #[test]
    fn exact_build_settings_share_one_owned_context() {
        let first = build();
        let mut table = BuildContexts::default();
        let id = table.intern(&first).unwrap();
        let mut second = first.clone();
        second
            .frontend_options
            .insert("source_identifier".into(), json!("other"));
        second.frontend_options.insert(
            crate::concrete_types::KEY.into(),
            json!({"invalid": "observation"}),
        );
        assert_eq!(table.intern(&second).unwrap(), id);
        assert_eq!(table.contexts.len(), 1);
        assert!(
            !table.contexts[0]
                .frontend_options
                .contains_key(crate::concrete_types::KEY)
        );
        assert_eq!(
            table.contexts[0].frontend_options.get("invocation"),
            first.frontend_options.get("invocation")
        );
        let serialized = serde_json::to_value(&table).unwrap();
        assert_eq!(serialized["identity"], "rstyle.rust.unit-build-contexts/1");
        assert!(serialized.to_string().find("sha256").is_none());
    }

    #[test]
    fn independently_owned_equal_settings_merge_and_mutations_remain_distinct() {
        let original = build();
        let wire = serde_json::to_value(&original).unwrap();
        let mut independent: BuildFacts = serde_json::from_value(wire).unwrap();
        let mut table = BuildContexts::default();
        let id = table.intern(&original).unwrap();
        assert_eq!(table.intern(&independent).unwrap(), id);
        assert!(std::ptr::eq(
            original.frontend_options.get("invocation").unwrap(),
            table.contexts[0]
                .frontend_options
                .get("invocation")
                .unwrap()
        ));

        independent.frontend_options.get_mut("invocation").unwrap()[1] = json!("--test");
        assert_ne!(table.intern(&independent).unwrap(), id);
        assert_eq!(table.contexts.len(), 2);
        assert_eq!(table.intern(&original).unwrap(), id);
        let wire = serde_json::to_value(&table).unwrap();
        assert_eq!(
            wire["contexts"][0]["frontend_options"]["invocation"],
            json!(["rustc", "--crate-type=lib"])
        );
        assert_eq!(
            wire["contexts"][1]["frontend_options"]["invocation"],
            json!(["rustc", "--test"])
        );
    }

    #[test]
    fn every_build_field_and_option_preserves_exact_distinctions() {
        let first = build();
        let original = serde_json::to_value(&first).unwrap();
        for (key, value) in original.as_object().unwrap() {
            if key == "frontend_options" {
                continue;
            }
            let mut changed = original.clone();
            changed[key] = if value.is_array() {
                json!(["other"])
            } else {
                let mut fact = value.clone();
                fact["value"] = match &value["value"] {
                    serde_json::Value::Bool(value) => json!(!value),
                    serde_json::Value::Number(_) => json!(32),
                    _ => json!("other"),
                };
                fact
            };
            let second: BuildFacts = serde_json::from_value(changed).unwrap();
            let mut table = BuildContexts::default();
            assert_ne!(
                table.intern(&first).unwrap(),
                table.intern(&second).unwrap(),
                "{key}"
            );
        }
        for key in OPTIONS {
            let mut second = first.clone();
            second.frontend_options.insert(key.into(), json!(null));
            let mut table = BuildContexts::default();
            assert_ne!(
                table.intern(&first).unwrap(),
                table.intern(&second).unwrap(),
                "{key}"
            );
        }
    }
}
