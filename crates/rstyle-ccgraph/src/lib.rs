//! Imported-graph matching for `compat.ccgraph.strict/0.1.0`.
//!
//! The extracted graph and the matching projection stay separate objects.
//! This milestone covers imported-graph matching only: no HIR producer, no
//! call/memory support, no grouping or abstraction proposals, and no
//! semantic-equivalence claims.

pub mod config;
pub mod features;
pub mod frac;
pub mod graph;
pub mod interface;
pub mod pipeline;
pub mod projection;
pub mod scoring;
pub mod wl;

pub use config::{CcgraphConfig, ConfigError, Scoring, parse_config};
pub use features::{FEATURE_NAMES, Vector9, cosine_parts, cosine_passes, numerical_features};
pub use frac::{ArithError, Frac};
pub use graph::{
    CcLabel, ImportedEdge, ImportedFunction, ImportedNode, InterfaceElement, ProjectedEdge,
    ProjectedGraph, ProjectedNode, family_rank, initial_label,
};
pub use interface::{jaro_winkler, serialize_interface, string_mean};
pub use pipeline::{
    Decision, FunctionRecord, PairRecord, RunReport, StageName, StageOutcome, StageRecord,
    evaluate_all,
};
pub use projection::{
    EdgeDisposition, EdgeTrace, NodeDisposition, Projection, ProjectionFacts, ProjectionFault,
    ProjectionTrace, project,
};
pub use scoring::{Histogram, RoundScores, cosine_score, overlap_score, score_rounds};
pub use wl::{diameter, pair_depth, refine, weights};
