//! Compiler-independent matching graphs.
//!
//! `ImportedFunction` is the validated input to matching: interface facts,
//! categorized nodes, directed dependency occurrences, and reachability
//! evidence. `ProjectedGraph` is the separate conservative matching view.
//! Extracted graphs are never mutated in place.

use rstyle_pdg::{EdgeFamily, NodeCategory};
use serde::{Deserialize, Serialize};

/// Initial WL label from the profile's explicit category mapping.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub enum CcLabel {
    Decl,
    Assign,
    Ctrl,
    State,
    Other,
}

impl CcLabel {
    pub fn token(self) -> &'static str {
        match self {
            Self::Decl => "DECL",
            Self::Assign => "ASSIGN",
            Self::Ctrl => "CTRL",
            Self::State => "STATE",
            Self::Other => "OTHER",
        }
    }
}

/// Map a validated PDG category to its initial matching label.
pub fn initial_label(category: NodeCategory) -> Option<CcLabel> {
    match category {
        NodeCategory::Declaration => Some(CcLabel::Decl),
        NodeCategory::Assignment => Some(CcLabel::Assign),
        NodeCategory::Control => Some(CcLabel::Ctrl),
        NodeCategory::Call => Some(CcLabel::State),
        NodeCategory::Read
        | NodeCategory::Computation
        | NodeCategory::FormalInput
        | NodeCategory::Return
        | NodeCategory::FormalOutput
        | NodeCategory::Merge
        | NodeCategory::Other => Some(CcLabel::Other),
        NodeCategory::Entry | NodeCategory::Exit | NodeCategory::MemorySummary => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct InterfaceElement {
    pub role: String,
    pub normalized_type: String,
    pub variadic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedNode {
    pub id: String,
    pub category: NodeCategory,
    /// Positively identified administrative bookkeeping, without value semantics.
    pub administrative: bool,
    /// Reachable from the paired CFG entry (or explicit stage evidence).
    pub reachable: bool,
    pub defines_binding: bool,
    pub reads_or_writes: bool,
    /// For merges: validated value sites reachable; formal inputs ignore this.
    pub value_site_reachable: bool,
    /// Symbol IDs read by this node when it is a read.
    pub read_symbols: Vec<String>,
    /// Registered meaning for `other`; `None` means no meaning was supplied.
    pub registered_meaning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedEdge {
    pub source: String,
    pub target: String,
    pub family: EdgeFamily,
    /// Origin reference for projection provenance: the imported occurrence
    /// identity (edge id when the adapter supplies one, else occurrence index).
    pub origin: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedFunction {
    /// Verified local extraction/build compatibility; absent only on legacy/fixture inputs.
    pub extraction_context: Option<String>,
    pub key: String,
    pub nodes: Vec<ImportedNode>,
    pub edges: Vec<ImportedEdge>,
    pub inputs: Vec<InterfaceElement>,
    pub outputs: Vec<InterfaceElement>,
    /// Known nonempty unqualified identifier; `None` is missing evidence.
    pub name: Option<String>,
    /// Known reference count; `None` is unknown and never coerced to zero.
    /// Counts follow the root extraction profile's counting rule.
    pub reference_count: Option<u64>,
    pub target_pointer_width: Option<u16>,
    pub native_width_involved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedNode {
    pub id: String,
    pub label: CcLabel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedEdge {
    pub source: String,
    pub target: String,
    pub family: EdgeFamily,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProjectedGraph {
    pub nodes: Vec<ProjectedNode>,
    pub edges: Vec<ProjectedEdge>,
}

impl ProjectedGraph {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn label_of(&self, id: &str) -> Option<CcLabel> {
        self.nodes.iter().find(|n| n.id == id).map(|n| n.label)
    }

    /// Deterministic vertex order for refinement; scores never depend on it.
    pub fn ordered_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.nodes.iter().map(|n| n.id.clone()).collect();
        ids.sort();
        ids
    }
}

/// Family order for WL tuples: `data < control < execution`.
pub fn family_rank(family: EdgeFamily) -> u8 {
    match family {
        EdgeFamily::Data => 0,
        EdgeFamily::Control => 1,
        EdgeFamily::Execution => 2,
    }
}
