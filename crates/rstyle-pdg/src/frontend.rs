//! Compiler-independent R0 frontend facts.
//!
//! Plain-data records of what the rustc-based frontend observed for one
//! assessed function: exact source bytes, signature and binding tables with
//! resolved primitive types, a faithful HIR construct tree, and captured
//! build evidence. No rustc type appears here; the HIR walk lives in
//! `rstyle-driver`, while the eligibility classifier and the graph
//! derivation consume these facts. Unknown evidence keeps an explicit
//! reason and never becomes a fabricated default.

mod options;
pub(crate) use options::bundle as option_bundle;
mod source_snapshot;
pub use source_snapshot::SourceSnapshot;
mod source_text;
pub use options::{DataEvidence, FrontendOptions};
pub use source_text::SourceText;

use crate::model::{BoolFact, CountFact, FactStatus, StringFact};
use serde::{Deserialize, Serialize};

/// Reference to one HIR node: crate plus owner and local identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HirRef {
    /// Crate number (`0` for the assessed local crate).
    pub krate: u32,
    /// Owner definition index.
    pub owner: u32,
    /// Item-local node identifier.
    pub local: u32,
}

/// Half-open byte span relative to the assessed function's exact bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteSpan {
    /// Zero-based offset of the first byte.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

/// A compiler-resolved type: normalized primitive token plus evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedTy {
    /// Normalized scalar token (`i32`, `bool`, `unit`, ...) when supported.
    /// `None` means unresolvable or outside the R0 closed type set.
    pub normalized: Option<String>,
    /// Compiler-observed type for forensics (never a matching label).
    pub observed: String,
    /// Original source spelling (`""` where no annotation exists).
    pub spelling: String,
}

/// How a path expression or pattern resolved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathRes {
    /// `local` for a lexical binding read, `other` for anything else.
    pub kind: String,
    /// Referred binding node for local resolutions.
    pub target: Option<HirRef>,
    /// Resolution description for forensics (never a matching label).
    pub description: String,
}

/// Operator dispatch evidence: built-in facts, never spelling inference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispatch {
    /// `builtin`, `overloaded`, or `not_applicable`.
    pub kind: String,
    /// Operator rendering from the compiler (`+`, `&&`, `+=`, ...).
    pub op: String,
    /// Normalized operand types in semantic position order.
    pub operand_tys: Vec<String>,
    /// Normalized result type.
    pub result_ty: String,
}

/// Pre-expansion site of an expanded construct (macros, desugaring).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpansionSite {
    /// Outermost call-site span in function-relative bytes.
    pub callsite: ByteSpan,
    /// Source text at the call site.
    pub snippet: String,
}

/// One faithfully recorded HIR construct with resolved evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HirNode {
    /// HIR identity for cross-referencing within this document.
    pub hir: HirRef,
    /// Faithful HIR kind tag (for example `block`, `binary`, `call`).
    /// The classifier interprets these; the walk never pre-classifies.
    pub kind: String,
    /// Function-relative source span.
    pub span: ByteSpan,
    /// Exact source text covered by the span.
    pub snippet: String,
    /// Resolved value type for typed nodes.
    pub ty: Option<ResolvedTy>,
    /// Path resolution for path expressions and patterns.
    pub res: Option<PathRes>,
    /// Dispatch evidence for operator expressions.
    pub dispatch: Option<Dispatch>,
    /// Call-site evidence for expanded constructs.
    pub expansion: Option<ExpansionSite>,
    /// Binding symbol defined by a binding pattern.
    pub binding: Option<String>,
    /// Binding-pattern mutability (`mut x` versus `x`).
    pub binding_mut: Option<bool>,
    /// Rendered literal value for literal nodes.
    pub lit: Option<String>,
    /// Source-order position for parameter nodes.
    pub param_index: Option<u32>,
    /// Whether a block carries an `unsafe` modifier.
    #[serde(default)]
    pub is_unsafe_block: bool,
    /// Nested constructs in source order.
    pub children: Vec<HirNode>,
}

impl HirNode {
    /// Observed resolution tag only; this accessor grants no semantic permission.
    pub(crate) fn resolution_kind(&self) -> Option<&str> {
        self.res.as_ref().map(|resolution| resolution.kind.as_str())
    }

    /// Search the complete recorded subtree, including its root, in source order.
    pub(crate) fn any(&self, predicate: &impl Fn(&Self) -> bool) -> bool {
        predicate(self) || self.children.iter().any(|child| child.any(predicate))
    }
}

/// One lexical binding: identity is the declaring-token span plus scope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingFact {
    /// Graph-local symbol identifier assigned in walk order.
    pub symbol: String,
    /// Declared source name (navigation only, never identity).
    pub name: String,
    /// Declaring identifier token span.
    pub decl_span: ByteSpan,
    /// Owning scope key (`scope:<source>:<kind>:<start>:<end>`).
    pub scope: String,
    /// HIR node of the binding pattern.
    pub hir: HirRef,
    /// Whether the binding is a function parameter.
    pub is_param: bool,
    /// Whether declared `mut` (parameters included).
    pub is_mut: bool,
    /// Resolved binding type.
    pub ty: ResolvedTy,
}

/// One named parameter in source order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamFact {
    /// Binding symbol identifier.
    pub symbol: String,
    /// Zero-based source-order position.
    pub position: u32,
    /// Parameter identifier token span.
    pub name_span: ByteSpan,
    /// Type annotation span.
    pub ty_span: ByteSpan,
    /// Resolved parameter type (spelling kept from the annotation).
    pub ty: ResolvedTy,
    /// Faithful pattern subtree (`pat_binding`, `pat_wild`, ...).
    pub pat: HirNode,
}

/// A receiver (`self`) that R0 always rejects, with its source evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverFact {
    /// Receiver token span.
    pub span: ByteSpan,
    /// Exact source text covered by the span.
    pub snippet: String,
}

/// A function-level generic parameter that R0 always rejects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenericFact {
    /// Declared generic name.
    pub name: String,
    /// Generic parameter span.
    pub span: ByteSpan,
}

/// Owned signature evidence: parameters, return type, and shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureFacts {
    /// Whole-signature span for signature-shape diagnostics.
    pub sig_span: ByteSpan,
    /// Parameters in source order (`self` excluded; receivers rejected).
    pub params: Vec<ParamFact>,
    /// Receiver evidence when present.
    pub receiver: Option<ReceiverFact>,
    /// Resolved return type (`unit` for a default return).
    pub return_ty: ResolvedTy,
    /// Return type annotation span (function end for a default return).
    pub return_ty_span: ByteSpan,
    /// Whether a receiver (`self`) is present.
    pub has_self: bool,
    /// ABI name (`Rust` when standard).
    pub abi: StringFact,
    /// Declared function-level generic parameters (empty when none).
    pub generics: Vec<GenericFact>,
    /// Whether the signature is variadic (C-style `...`).
    pub is_variadic: bool,
    /// Whether the function is `async`.
    pub is_async: bool,
    /// Whether the function body is an `unsafe` block or `unsafe fn`.
    pub is_unsafe: bool,
    /// Whether the function is a `const fn`.
    pub is_const: bool,
}

/// Exact assessed-function bytes with their verified identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFacts {
    /// Exact UTF-8 bytes of the assessed function (no added newline).
    pub bytes: SourceText,
    /// Explicitly tagged byte checksum: XXH3 for new Data-v3 captures; SHA for historical claims.
    pub digest: String,
}

/// Captured build and semantic-setting evidence (never fabricated).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildFacts<Options = FrontendOptions> {
    /// Actual producer source revision captured when the compiler driver built.
    pub producer_revision: StringFact,
    /// Relevant effective frontend options captured from the active session.
    pub frontend_options: Options,
    /// Exact rustc version string.
    pub rustc_version: StringFact,
    /// rustc commit hash.
    pub rustc_commit: StringFact,
    /// Rust edition.
    pub edition: StringFact,
    /// Target triple.
    pub target_triple: StringFact,
    /// Pointer width in bits.
    pub pointer_width: CountFact,
    /// Target data layout string.
    pub data_layout: StringFact,
    /// Active `--cfg` flags.
    pub cfgs: Vec<String>,
    /// Active Cargo features (`cfg(feature = ...)` flags observed).
    pub features: Vec<String>,
    /// Overflow checks (`-C overflow-checks`).
    pub overflow_checks: BoolFact,
    /// Debug assertions (`-C debug-assertions`).
    pub debug_assertions: BoolFact,
    /// Panic strategy (`unwind` or `abort`).
    pub panic_strategy: StringFact,
    /// Optimization level (`-C opt-level`).
    pub opt_level: StringFact,
}

impl<Options> BuildFacts<Options> {
    pub(crate) fn try_map_options<T>(
        self,
        map: impl FnOnce(Options) -> Result<T, String>,
    ) -> Result<BuildFacts<T>, String> {
        Ok(BuildFacts {
            frontend_options: map(self.frontend_options)?,
            producer_revision: self.producer_revision,
            rustc_version: self.rustc_version,
            rustc_commit: self.rustc_commit,
            edition: self.edition,
            target_triple: self.target_triple,
            pointer_width: self.pointer_width,
            data_layout: self.data_layout,
            cfgs: self.cfgs,
            features: self.features,
            overflow_checks: self.overflow_checks,
            debug_assertions: self.debug_assertions,
            panic_strategy: self.panic_strategy,
            opt_level: self.opt_level,
        })
    }
}

/// One assessed function's complete frontend evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HirFnFacts {
    /// Function path for navigation (never a matching label).
    pub name: String,
    /// Span of the whole function within its file, in file bytes.
    pub fn_span: ByteSpan,
    /// Exact function bytes with their verified identity.
    pub source: SourceFacts,
    /// Owned signature evidence.
    pub signature: SignatureFacts,
    /// Lexical bindings in walk order.
    pub bindings: Vec<BindingFact>,
    /// Faithful construct tree of the owned body.
    pub body: HirNode,
    /// Captured build and semantic-setting evidence.
    pub build: BuildFacts,
    /// Findings from the owned source before expansion and cfg removal.
    #[serde(default)]
    pub source_diagnostics: Vec<HirDiagnostic>,
    /// Whether the whole owned source was available to the pre-expansion scan.
    #[serde(default)]
    pub source_scan_complete: bool,
}

/// Eligibility verdict over frontend facts (classifier output).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityVerdict {
    /// Closed R0 subset with complete required evidence.
    Eligible,
    /// A deferred construct was encountered; diagnostics say where.
    Ineligible,
    /// Compilation, resolution, or required evidence failed.
    Unknown,
}

/// One actionable eligibility diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HirDiagnostic {
    /// Stable capability ID from the backlog.
    pub capability: String,
    /// Offending construct tag.
    pub construct: String,
    /// Function-relative source span.
    pub span: ByteSpan,
    /// Exact source text covered by the span.
    pub snippet: String,
    /// Severity (`error` blocks conformance).
    pub severity: String,
    /// Reason with deferred scope or remediation.
    pub reason: String,
}

/// Classifier output: verdict plus visible diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HirEligibility {
    /// Verdict over the assessed function.
    pub status: EligibilityVerdict,
    /// Every encountered rejection or evidence gap.
    pub diagnostics: Vec<HirDiagnostic>,
}

/// Build a known string fact.
pub fn known_str(value: &str) -> StringFact {
    StringFact {
        status: FactStatus::Known,
        value: Some(value.to_string()),
        reason: None,
    }
}

/// Build an unknown string fact with an explicit reason.
pub fn unknown_str(reason: &str) -> StringFact {
    StringFact {
        status: FactStatus::Unknown,
        value: None,
        reason: Some(reason.to_string()),
    }
}

/// Build a known boolean fact.
pub fn known_bool(value: bool) -> BoolFact {
    BoolFact {
        status: FactStatus::Known,
        value: Some(value),
        reason: None,
    }
}

/// Build an unknown boolean fact with an explicit reason.
pub fn unknown_bool(reason: &str) -> BoolFact {
    BoolFact {
        status: FactStatus::Unknown,
        value: None,
        reason: Some(reason.to_string()),
    }
}

/// Build a known count fact.
pub fn known_count(value: u64) -> CountFact {
    CountFact::known(value, crate::ProfileId::hir_r0())
}

/// Build an unknown count fact with an explicit reason.
pub fn unknown_count(reason: &str) -> CountFact {
    CountFact::missing(FactStatus::Unknown, reason)
}

/// Shared exact-kind traversal for independent frontend inventory verifiers.
pub fn collect_kind<'a>(node: &'a HirNode, kind: &str, result: &mut Vec<&'a HirNode>) {
    if node.kind == kind {
        result.push(node);
    }
    for child in &node.children {
        collect_kind(child, kind, result);
    }
}

pub(crate) fn normalized(node: &HirNode) -> Option<&str> {
    node.ty.as_ref()?.normalized.as_deref()
}
