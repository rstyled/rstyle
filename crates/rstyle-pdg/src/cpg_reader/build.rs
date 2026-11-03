//! The build settings a document's unit records, as the driver's build facts.

use crate::frontend::BuildFacts;
use crate::model::{BoolFact, CountFact, FactStatus, StringFact};
use rstyle_core::cpg::model::{Producer, Unit};

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
