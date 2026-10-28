use super::super::{Availability, mutations};
use crate::{ByteSpan, HirRef};
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub functions: usize,
    pub calls: usize,
    pub summaries: usize,
    pub sites: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            functions: 4096,
            calls: 16_384,
            summaries: 4096,
            sites: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Statistics {
    pub observations: usize,
    pub leaf_checks: usize,
    pub wrapper_checks: usize,
    pub calls: usize,
    pub mapped_sites: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Assessment {
    pub function_index: usize,
    pub availability: Availability<()>,
    pub signature: Availability<()>,
    pub direct_site_count: usize,
    pub direct_coverage: mutations::Coverage,
    pub coverage: mutations::Coverage,
    pub execution: mutations::Execution,
    pub roots: Vec<mutations::Root>,
    pub calls: Vec<CallEvent>,
    pub unresolved_boundaries: Vec<usize>,
    pub reasons: Vec<String>,
    pub caller_visible_mutation: CallerVisibleMutation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallerVisibleMutation {
    NoneProven,
    WritesFormal,
    WitnessedWritesFormalWithUnknownRemainder,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct CallEvent {
    pub call_index: usize,
    pub span: ByteSpan,
    pub native: HirRef,
    pub summary_index: usize,
    pub mappings: Vec<Mapping>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Mapping {
    pub summary_site_index: usize,
    pub caller_root: mutations::Root,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub function_index: usize,
    pub sites: Vec<SiteWitness>,
    pub parameters: Vec<Parameter>,
}

/// Original write identity and its projection into the summary owner's bindings.
#[derive(Debug, Serialize)]
pub struct SiteWitness {
    pub function_index: usize,
    pub site_index: usize,
    pub call_path: Vec<CallReference>,
    pub root: mutations::Root,
}

/// One original call occurrence; paths are ordered from the summary owner outward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CallReference {
    pub function_index: usize,
    pub call_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Parameter {
    OwnedU32,
    MutableU32,
}
