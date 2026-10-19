//! Compiler-independent PDG/CFG interchange and validation.
//!
//! This crate owns the typed interchange models (facts, unknown evidence,
//! provenance, definitions, extensions, directed edge multiplicity) and the
//! structural plus milestone semantic validation. Matching semantics live in
//! `rstyle-ccgraph`. The rustc HIR walk lives in `rstyle-driver`; only its
//! compiler-independent fact records ([`frontend`]) and their analysis live
//! here.

pub mod call_summaries;
pub mod cfg;
pub mod collection;
pub mod concrete_types;
mod cyclic_values;
mod data_aggregates;
pub mod data_documents;
pub mod data_frontend;
pub mod data_library;
pub mod data_origins;
pub mod data_places;
pub mod data_receivers;
pub mod data_types;
pub mod declaration_origins;
pub mod diagnostics;
pub mod eligibility;
mod evidence_budget;
pub mod extraction_profile;
pub mod field_locations;
pub mod frontend;
pub mod helper_calls;
pub mod helper_reads;
mod helper_references;
pub mod inherent_calls;
pub mod model;
pub mod nominal_binding;
pub mod nominal_boundary;
pub mod producer;
pub mod scalar_constants;
mod scalar_exhaustiveness;
pub mod scalar_patterns;
pub mod source_evidence;
mod source_table;
pub mod unit_store;
pub mod validate;
pub mod value_sites;
pub mod verification_policy;
mod verification_session;
pub use value_sites::validate_value_sites;

pub use eligibility::{classify, span};
pub use frontend::{
    BindingFact, BuildFacts, ByteSpan, DataEvidence, Dispatch, EligibilityVerdict, ExpansionSite,
    FrontendOptions, GenericFact, HirDiagnostic, HirEligibility, HirFnFacts, HirNode, HirRef,
    ParamFact, PathRes, ReceiverFact, ResolvedTy, SignatureFacts, SourceFacts, SourceSnapshot,
    SourceText, known_bool, known_count, known_str, unknown_bool, unknown_count, unknown_str,
};

pub use cfg::{
    CfgDocument, CfgEdge, CfgEdgeKind, CfgExitKind, CfgExtractionEvidence, CfgFunction, CfgIssue,
    CfgNode, CfgNodeKind, CfgOperation, CfgProducer, CfgProfile, CfgSource, EvaluationOrder,
    ExceptionalFlow, ExtractionStatus, Granularity, LoopRole, OPERATION_MAP_KEY, OperationMapEntry,
    PAIRED_GRAPH_KEY, PairedGraph, Sequencing, ShortCircuit, UnreachableCode, UnspecifiedOrder,
    r0_profile, reachable_vertices, read_operation_map, read_paired_graph, validate_cfg,
    validate_correspondence, validate_paired_agreement,
};
pub use model::{
    Access, BoolFact, CFG_SCHEMA_VERSION, CallForm, Capability, CapabilityStatus, Conformance,
    ConformanceStatus, CountFact, Definition, Diagnostic, DiagnosticSeverity, EdgeFamily, ExitKind,
    Extensions, ExtractionEvidence, FactStatus, FunctionInfo, MemoryLocation, NodeAttributes,
    NodeCategory, PDG_SCHEMA_VERSION, ParamRole, Parameter, PdgDocument, PdgEdge, PdgNode,
    Producer, ProfileId, Provenance, ProvenanceOrigin, R0_POLICY_ID, R0_POLICY_VERSION,
    REQUIRED_CAPABILITIES, REQUIRED_NORMAL_FLOW, Source, Span, StringFact, Symbol, SymbolRole,
    invalid_extension_keys, is_valid_extension_key,
};
pub use validate::{
    Eligibility, Issue, ProducerConformance, ValidationContext, ValidationReport, validate,
};
pub mod assertions;
pub mod conditional_source;
