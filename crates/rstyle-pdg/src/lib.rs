//! Compiler-independent PDG/CFG interchange and validation.
//!
//! This crate owns the typed interchange models (facts, unknown evidence,
//! provenance, definitions, extensions, directed edge multiplicity) and the
//! structural plus milestone semantic validation. Matching semantics live in
//! `rstyle-ccgraph`. The rustc HIR walk lives in `rstyle-driver`; only its
//! compiler-independent fact records ([`frontend`]) and their analysis live
//! here.

pub(crate) mod bitset_cardinality;
pub mod bool_constants;
pub(crate) mod bool_usize;
pub(crate) mod borrowed_result_errors;
pub mod borrowed_result_propagation;
pub(crate) mod borrowed_result_types;
mod builtin_scalars;
pub mod byte_literals;
pub mod call_summaries;
pub mod cfg;
mod character_ascii;
pub mod character_ranges;
pub mod character_scalars;
pub(crate) mod checked_index_bytes;
pub(crate) mod checked_lengths;
mod checked_option_products;
pub mod checked_options;
pub(crate) mod checked_slice_views;
pub mod collection;
mod concrete_calls;
pub mod concrete_results;
pub mod concrete_types;
mod copy_conversion_members;
mod copy_tagged_values;
mod cyclic_values;
mod data_aggregates;
mod data_bounds;
pub(crate) mod data_bytes;
mod data_component_origins;
mod data_dereferences;
mod data_discards;
pub mod data_documents;
mod data_enums;
pub mod data_frontend;
mod data_index_reads;
pub mod data_library;
mod data_locals;
pub mod data_origins;
pub mod data_places;
pub mod data_receivers;
mod data_reference_locals;
mod data_structs;
mod data_subslices;
pub mod data_types;
mod data_value_fields;
pub mod declaration_origins;
pub(crate) mod default_calls;
pub mod dependency_namespaces;
pub mod diagnostics;
pub mod eligibility;
mod evidence_budget;
pub mod extraction_profile;
pub mod field_locations;
mod fieldless_enums;
pub mod float_scalars;
pub mod frontend;
pub mod generated_sources;
pub mod helper_calls;
pub mod helper_component_outputs;
mod helper_normal_flow;
mod helper_option_callbacks;
pub mod helper_reads;
mod helper_references;
mod helper_stored_slices;
mod helper_views;
pub mod id_error_maps;
pub mod id_reader_constants;
pub mod id_reader_types;
pub mod inherent_calls;
pub mod iterator_position;
pub(crate) mod local_value_trait_calls;
pub(crate) mod lossless_integer_calls;
pub mod model;
pub(crate) mod mutable_scalars;
pub(crate) mod named_callbacks;
mod named_error_payloads;
pub mod named_error_result_propagation;
pub mod nominal_binding;
pub mod nominal_boundary;
pub mod nominal_constants;
pub mod nonzero_values;
pub mod option_callbacks;
pub mod option_propagation;
mod owned_product_locals;
mod owned_product_values;
pub mod parsed_item_callbacks;
pub mod primitive_constants;
pub mod producer;
pub mod product_constant_values;
pub mod product_instances;
pub mod product_result_propagation;
pub mod range_values;
mod reader_products;
pub mod reference_instances;
pub mod result_propagation;
pub mod scalar_constants;
mod scalar_exhaustiveness;
pub(crate) mod scalar_from;
pub mod scalar_instances;
mod scalar_loop_helpers;
pub mod scalar_patterns;
pub mod scalar_product_instances;
pub mod scalar_result_propagation;
mod scalar_tuple_bindings;
pub mod size_calls;
pub mod size_constants;
pub mod slice_length_instances;
pub mod slice_patterns;
pub mod source_evidence;
pub mod source_identity;
mod source_table;
mod static_string_values;
pub(crate) mod tryfrom_calls;
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

mod tuple_assignment;

mod owned_receiver;
pub mod result_maps;

mod structural_json;

mod nonzero_digit_calls;
