//! Compiler-independent PDG/CFG interchange and validation.
//!
//! This crate owns the typed interchange models (facts, unknown evidence,
//! provenance, definitions, extensions, directed edge multiplicity) and the
//! structural plus milestone semantic validation. Matching semantics live in
//! `rstyle-ccgraph`; no HIR producer lives here.

pub mod cfg;
pub mod model;
pub mod validate;

pub use cfg::{
    CfgDocument, CfgEdge, CfgEdgeKind, CfgExitKind, CfgExtractionEvidence, CfgFunction, CfgIssue,
    CfgNode, CfgNodeKind, CfgOperation, CfgProducer, CfgProfile, CfgSource, EvaluationOrder,
    ExceptionalFlow, ExtractionStatus, Granularity, LoopRole, OPERATION_MAP_KEY, OperationMapEntry,
    PAIRED_GRAPH_KEY, Sequencing, ShortCircuit, UnreachableCode, UnspecifiedOrder, r0_profile,
    reachable_vertices, read_operation_map, validate_cfg, validate_correspondence,
};
pub use model::{
    Access, BoolFact, CFG_SCHEMA_VERSION, CallForm, Capability, CapabilityStatus,
    ConformanceRecord, ConformanceStatus, CountFact, Definition, Diagnostic, DiagnosticSeverity,
    EdgeFamily, ExitKind, Extensions, ExtractionEvidence, FactStatus, FunctionInfo, MemoryLocation,
    NodeAttributes, NodeCategory, PDG_SCHEMA_VERSION, ParamRole, Parameter, PdgDocument, PdgEdge,
    PdgNode, Producer, ProfileId, Provenance, ProvenanceOrigin, R0_POLICY_ID, R0_POLICY_VERSION,
    REFERENCE_COUNT_POLICY_ID, REFERENCE_COUNT_POLICY_VERSION, REQUIRED_CAPABILITIES,
    REQUIRED_NORMAL_FLOW, Source, Span, StringFact, Symbol, SymbolRole, invalid_extension_keys,
    is_valid_extension_key,
};
pub use validate::{
    Eligibility, Issue, ProducerConformance, ValidationContext, ValidationReport, validate,
};
