//! The build settings a document's unit records, as the driver's build facts.

use crate::frontend::BuildFacts;
use crate::model::{BoolFact, CountFact, FactStatus, StringFact};
use rstyle_core::cpg::model::{Definition, Producer, Unit};
use serde_json::json;

pub(super) fn facts(unit: &Unit, producer: &Producer) -> BuildFacts {
    let cfgs = unit.configuration.clone();
    let features = cfgs
        .iter()
        .filter_map(|cfg| {
            cfg.strip_prefix("feature=\"")
                .and_then(|feature| feature.strip_suffix('"'))
                .map(str::to_owned)
        })
        .collect();
    let semantics = &unit.semantics;
    let panic = match semantics.exceptions.as_str() {
        "abort" => "abort",
        _ => "unwind",
    };

    BuildFacts {
        producer_revision: optional(producer.revision.as_deref()),
        frontend_options: Default::default(),
        // A toolchain's version string begins with its name, as `rustc --version` prints it.
        rustc_version: known(&format!(
            "{} {}",
            unit.toolchain.name, unit.toolchain.version
        )),
        rustc_commit: optional(unit.toolchain.commit.as_deref()),
        edition: known(&unit.language.version),
        target_triple: known(
            unit.extensions
                .as_ref()
                .and_then(|extensions| extensions.get("rust:llvm-target")?.as_str())
                .unwrap_or(&unit.target.triple),
        ),
        pointer_width: CountFact {
            status: FactStatus::Known,
            value: Some(u64::from(unit.target.pointer_width)),
            reason: None,
        },
        data_layout: optional(unit.target.data_layout.as_deref()),
        cfgs,
        features,
        overflow_checks: boolean(Some(semantics.integer_overflow == "checked")),
        debug_assertions: boolean(semantics.debug_assertions),
        panic_strategy: known(panic),
        opt_level: optional(semantics.optimization.as_deref()),
    }
}

/// The options the frontend records from its session: the edition, target and
/// compiler invocation, and the profile and unit it selected.
pub(super) fn session_options(
    unit: &Unit,
    session: Option<&serde_json::Value>,
    options: &mut crate::frontend::FrontendOptions,
) {
    options.insert("edition".into(), json!(unit.language.version));
    options.insert("target".into(), json!(unit.target.triple));
    let Some(session) = session else {
        return;
    };
    if let Some(invocation) = session.get("invocation") {
        options.insert("invocation".into(), invocation.clone());
    }
    if let Some(selection) = session
        .get("profile_selection")
        .filter(|selection| !selection.is_null())
    {
        options.insert(
            crate::extraction_profile::SELECTION_KEY.into(),
            selection.clone(),
        );
    }
    if let Some(unit) = session.get("selected_unit").filter(|unit| !unit.is_null()) {
        options.insert(crate::nominal_boundary::CONTEXT_KEY.into(), unit.clone());
    }
}

/// The function's own compiler identity, and its crate's, where the profile links
/// helpers across crates.
pub(super) fn identity_options(
    profile: crate::extraction_profile::ExtractionProfile,
    definition: &Definition,
    options: &mut crate::frontend::FrontendOptions,
) {
    let identity = |key: &str| super::identity(definition, key);
    if profile.supports("rust.verified-helper-calls")
        && let Some(hash) = identity("def_path_hash")
    {
        options.insert("definition_identity".into(), json!(hash));
    }
    let linked = profile == crate::extraction_profile::ExtractionProfile::DataV3
        && profile.supports("rust.verified-helper-calls");
    if linked
        && let (Some(crate_name), Some(stable)) =
            (identity("crate_name"), identity("stable_crate_id"))
    {
        let record = json!({"identity": "rstyle.rust.compiled-crate/1", "crate_name": crate_name,
                            "stable_crate_id": stable});
        options.insert("rstyle.rust.compiled-crate".into(), record);
    }
}

fn known(value: &str) -> StringFact {
    optional(Some(value))
}

fn optional(value: Option<&str>) -> StringFact {
    match value {
        Some(value) => StringFact {
            status: FactStatus::Known,
            value: Some(value.to_owned()),
            reason: None,
        },
        None => StringFact {
            status: FactStatus::Unknown,
            value: None,
            reason: Some("The document does not record it".into()),
        },
    }
}

fn boolean(value: Option<bool>) -> BoolFact {
    match value {
        Some(value) => BoolFact {
            status: FactStatus::Known,
            value: Some(value),
            reason: None,
        },
        None => BoolFact {
            status: FactStatus::Unknown,
            value: None,
            reason: Some("The document does not record it".into()),
        },
    }
}
