//! Observational mutable scalar roots and accesses; no graph or alias authority.
mod hir;
mod source;

use crate::{ByteSpan, HirFnFacts, HirRef};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarFormalAccesses {
    pub status: InventoryStatus,
    pub formals: Vec<Formal>,
    pub accesses: Vec<Access>,
    pub possibly_aliasing_formals: Vec<[u32; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
pub enum InventoryStatus {
    Complete,
    Unsupported { reason: String },
    Unknown { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Formal {
    pub parameter: u32,
    pub binding: HirRef,
    pub observation: FormalObservation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum FormalObservation {
    Shared { referent: ScalarObservation },
    Mutable { referent: ScalarObservation },
    NonReference,
    Unknown { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
pub enum ScalarObservation {
    Captured { scalar: String },
    Unsupported { reason: String },
    Unknown { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    pub hir: HirRef,
    pub span: ByteSpan,
    pub snippet: String,
    pub parameter: u32,
    pub kind: AccessKind,
    pub compound: Option<Compound>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessKind {
    Load,
    Store,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compound {
    pub hir: HirRef,
    pub operator: String,
    pub rhs: HirRef,
}

impl Formal {
    pub fn mutable_usize(&self) -> bool {
        matches!(&self.observation, FormalObservation::Mutable {
            referent: ScalarObservation::Captured { scalar }
        } if scalar == "usize")
    }
}

/// Observe the complete raw HIR inventory. Unavailability is explicit data.
pub fn observe_accesses(facts: &HirFnFacts, formals: Vec<Formal>) -> ScalarFormalAccesses {
    let possibly_aliasing_formals = if formals.len() <= 64 {
        aliases(&formals)
    } else {
        Vec::new()
    };
    let mut evidence = ScalarFormalAccesses {
        status: InventoryStatus::Complete,
        formals,
        accesses: Vec::new(),
        possibly_aliasing_formals,
    };
    match hir::observe(facts, &evidence.formals) {
        Ok(accesses) => evidence.accesses = accesses,
        Err(failure) => evidence.status = failure.status(),
    }
    if evidence.status == InventoryStatus::Complete
        && let Err(failure) = validate(facts, &evidence)
    {
        evidence.status = failure.status();
    }

    evidence
}

/// An independent source check validates observations without activating memory.
pub fn verify(facts: &HirFnFacts, evidence: &ScalarFormalAccesses) -> Result<(), String> {
    validate(facts, evidence).map_err(|failure| failure.reason())
}

fn validate(facts: &HirFnFacts, evidence: &ScalarFormalAccesses) -> Result<(), Failure> {
    match &evidence.status {
        InventoryStatus::Complete => {}
        InventoryStatus::Unsupported { reason } => {
            return Err(Failure::Unsupported(reason.clone()));
        }
        InventoryStatus::Unknown { reason } => return Err(Failure::Unknown(reason.clone())),
    }
    if evidence.formals.len() > 64 || evidence.accesses.len() > super::MAX_OPERATIONS {
        return Err(Failure::Unknown(
            "Mutable scalar inventory exceeds its budget".into(),
        ));
    }
    if !facts.source.bytes.matches_identity(&facts.source.digest) {
        return Err(Failure::Unknown(
            "Mutable scalar source identity differs".into(),
        ));
    }
    if evidence.possibly_aliasing_formals != aliases(&evidence.formals) {
        return Err("Mutable scalar inventory omits or changes conservative formal aliases".into());
    }
    let expected_accesses = hir::observe(facts, &evidence.formals)?;
    source::verify(facts, evidence)?;
    if expected_accesses != evidence.accesses {
        return Err("Mutable scalar access roster differs from complete resolved HIR".into());
    }

    Ok(())
}

#[derive(Debug)]
pub(super) enum Failure {
    Unsupported(String),
    Unknown(String),
}

impl Failure {
    fn status(self) -> InventoryStatus {
        match self {
            Self::Unsupported(reason) => InventoryStatus::Unsupported { reason },
            Self::Unknown(reason) => InventoryStatus::Unknown { reason },
        }
    }

    fn reason(self) -> String {
        match self {
            Self::Unsupported(reason) | Self::Unknown(reason) => reason,
        }
    }
}

impl From<&str> for Failure {
    fn from(reason: &str) -> Self {
        Self::Unsupported(reason.into())
    }
}

impl From<String> for Failure {
    fn from(reason: String) -> Self {
        Self::Unsupported(reason)
    }
}

fn aliases(formals: &[Formal]) -> Vec<[u32; 2]> {
    let roots: Vec<_> = formals
        .iter()
        .filter(|formal| !matches!(formal.observation, FormalObservation::NonReference))
        .map(|formal| formal.parameter)
        .collect();
    roots
        .iter()
        .enumerate()
        .flat_map(|(index, left)| roots[index..].iter().map(move |right| [*left, *right]))
        .collect()
}
