//! The build settings a document's unit records, and the navigation types reports share.
use rstyle_capture::CONTEXT_KEY;
use rstyle_core::cpg::model::{Document, Producer, Unit};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Known,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringFact {
    pub status: FactStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl StringFact {
    pub fn known_value(&self) -> Option<&str> {
        match self.status {
            FactStatus::Known => self.value.as_deref(),
            FactStatus::Unknown => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoolFact {
    pub status: FactStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CountFact {
    pub status: FactStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Half-open byte span relative to the assessed function's exact bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteSpan {
    pub start: u32,
    pub end: u32,
}

/// A compiler-resolved type: normalized token plus what the compiler and source spell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedTy {
    pub normalized: Option<String>,
    pub observed: String,
    pub spelling: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuildFacts {
    pub producer_revision: StringFact,
    pub frontend_options: BTreeMap<String, Value>,
    pub rustc_version: StringFact,
    pub rustc_commit: StringFact,
    pub edition: StringFact,
    pub target_triple: StringFact,
    pub pointer_width: CountFact,
    pub data_layout: StringFact,
    pub cfgs: Vec<String>,
    pub features: Vec<String>,
    pub overflow_checks: BoolFact,
    pub debug_assertions: BoolFact,
    pub panic_strategy: StringFact,
    pub opt_level: StringFact,
}

/// The exact compiler settings of a unit's documents, each kept once.
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
    pub fn intern(&mut self, build: &BuildFacts) {
        if !self.contexts.contains(build) {
            self.contexts.push(build.clone());
        }
    }
}

/// The build settings a document's unit records.
pub fn unit_build(document: &Document) -> Result<BuildFacts, String> {
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let mut facts = facts(unit, &document.producer);
    let session = unit
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.get("rust:compiler-session"));

    session_options(unit, session, &mut facts.frontend_options);
    Ok(facts)
}

/// The absolute path a unit's capture session records of a file.
pub fn session_path<'u>(unit: &'u Unit, file: &str) -> Option<&'u str> {
    let extensions = unit.extensions.as_ref()?;
    extensions
        .get("rust:compiler-session")?
        .get("files")?
        .get(file)?
        .as_str()
}

fn facts(unit: &Unit, producer: &Producer) -> BuildFacts {
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
        frontend_options: BTreeMap::new(),
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

// rstyle-allow(mutation_per_target): installs each session option the frontend records
/// The options the frontend records from its session: the edition, target and compiler
/// invocation, and the unit it selected.
fn session_options(unit: &Unit, session: Option<&Value>, options: &mut BTreeMap<String, Value>) {
    options.insert("edition".into(), json!(unit.language.version));
    options.insert("target".into(), json!(unit.target.triple));

    let Some(session) = session else {
        return;
    };
    if let Some(invocation) = session.get("invocation") {
        options.insert("invocation".into(), invocation.clone());
    }
    if let Some(unit) = session.get("selected_unit").filter(|unit| !unit.is_null()) {
        options.insert(CONTEXT_KEY.into(), unit.clone());
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
