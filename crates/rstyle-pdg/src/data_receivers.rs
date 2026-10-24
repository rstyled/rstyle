//! Observational method receiver evidence. This is not a call admission registry.

mod dispatch;
mod transport;
mod verify;
pub(crate) use dispatch::{InvocationSyntax, candidate, verify_dispatch};
pub(crate) use verify::check;
pub use verify::{verify, verify_capture};

use crate::{ByteSpan, HirRef, concrete_types::Observation};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "data_method_receivers";
pub const IDENTITY: &str = "rstyle.rust.method-receiver-capture/1";
pub const MAX_RECORDS: usize = 4096;
pub const MAX_ADJUSTMENTS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub identity: String,
    pub methods: Vec<Method>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub call: HirRef,
    pub source_span: ByteSpan,
    pub receiver: HirRef,
    pub receiver_type: Observation,
    pub adjusted_type: Observation,
    pub adjustments: Vec<Adjustment>,
    pub signature_inputs: Vec<Observation>,
    pub signature_result: Observation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adjustment {
    pub kind: Kind,
    pub target: Observation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "reason",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Kind {
    BuiltinDereference,
    SharedBorrow,
    MutableBorrow,
    Unsupported(String),
}

/// Inspect paired frontend records directly, without invoking producer lowering.
pub fn verify_documents<'a>(
    pair: impl Into<crate::producer::PairView<'a>>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let pair = pair.into();
    let expected = verify(facts)?;
    for extensions in [&pair.pdg.extensions, &pair.cfg.extensions] {
        let value = extensions
            .get(crate::extraction_profile::EVIDENCE_KEY)
            .ok_or("Missing paired Data frontend evidence")?;
        if let Some(reference) = value.get("frontend_ref") {
            if facts.frontend_reference().as_ref() != Some(reference)
                || value.get("frontend").is_some()
            {
                return Err("Frontend reference belongs to another unit/function or contradicts inline facts".into());
            }
            continue;
        }
        let frontend: crate::HirFnFacts =
            serde_json::from_value(value["frontend"].clone()).map_err(|error| error.to_string())?;
        if verify(&frontend)? != expected {
            return Err("Paired receiver inventories disagree with captured evidence".into());
        }
    }
    Ok(())
}
