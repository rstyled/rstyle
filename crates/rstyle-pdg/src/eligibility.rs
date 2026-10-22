//! Strict R0 eligibility over frontend facts.
//!
//! Default-deny classifier for the closed R0 subset from the pinned
//! extraction profile. It accepts only ordinary synchronous safe functions
//! over resolved primitive integer, boolean, and unit types with the
//! enumerated bindings, writes, operators, blocks, branches, short-circuits,
//! and returns. Anything else yields an actionable diagnostic naming the
//! stable capability ID, construct, exact source span, and reason. Missing
//! required evidence yields `Unknown`, never an eligible empty graph.
//!
//! Driver contract for `HirNode.kind` strings: expression nodes use
//! `expr_<lowercase HIR variant>` (`expr_binary`, `expr_call`, ...),
//! statements use `stmt_<variant>` (`stmt_let`, `stmt_item`, ...),
//! patterns use `pat_<variant>` (`pat_binding`, `pat_wild`, ...), and type
//! annotations use `ty_<variant>`. Unlisted kinds are rejected by the
//! fallback arm; the walk stays total over future HIR variants.

use crate::frontend::{
    BindingFact, ByteSpan, EligibilityVerdict, ExpansionSite, HirDiagnostic, HirEligibility,
    HirNode, ParamFact, ResolvedTy, SignatureFacts,
};
use crate::model::{FactStatus, StringFact};
use std::collections::{BTreeMap, BTreeSet};

/// Resolved primitive types admitted by R0 (unit included as a type).
const PRIMITIVE_TYS: [&str; 14] = [
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "bool",
    "unit",
];

/// Signed integer types admitting unary negation.
const SIGNED_INTS: [&str; 6] = ["i8", "i16", "i32", "i64", "i128", "isize"];

/// Unsigned integer types.
const UNSIGNED_INTS: [&str; 6] = ["u8", "u16", "u32", "u64", "u128", "usize"];

/// Ordinary integer binary operators (comparisons included).
const INT_OPS: [&str; 16] = [
    "+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", "==", "!=", "<", "<=", ">", ">=",
];

/// Eager boolean binary operators (comparisons included).
const BOOL_EAGER_OPS: [&str; 9] = ["&", "|", "^", "==", "!=", "<", "<=", ">", ">="];

/// Comparison operators (the only ones even considered for unit operands).
const COMPARISONS: [&str; 6] = ["==", "!=", "<", "<=", ">", ">="];

/// Primitive compound assignment operators (`target OP= rhs`).
const COMPOUND_OPS: [&str; 10] = ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="];

/// One classifier finding: diagnostics plus an evidence-gap flag.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Outcome {
    diagnostics: Vec<HirDiagnostic>,
    gap: bool,
    rejected: bool,
}

impl Outcome {
    fn ok() -> Self {
        Self {
            diagnostics: Vec::new(),
            gap: false,
            rejected: false,
        }
    }

    fn reject(diagnostic: HirDiagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
            gap: false,
            rejected: true,
        }
    }

    fn gap(diagnostic: HirDiagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
            gap: true,
            rejected: false,
        }
    }

    fn merge(self, other: Self) -> Self {
        Self {
            diagnostics: self
                .diagnostics
                .into_iter()
                .chain(other.diagnostics)
                .collect(),
            gap: self.gap || other.gap,
            rejected: self.rejected || other.rejected,
        }
    }

    fn combine(outcomes: Vec<Self>) -> Self {
        outcomes.into_iter().fold(Self::ok(), Self::merge)
    }

    fn check_each<'a, T>(items: &'a [T], check: impl FnMut(&'a T) -> Self) -> Self {
        items.iter().map(check).fold(Self::ok(), Self::merge)
    }
}

/// Read-only classifier context: binding cross-references.
#[derive(Clone)]
struct Ctx<'a> {
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    loops: Vec<crate::HirRef>,
    profile: crate::extraction_profile::ExtractionProfile,
    bindings: BTreeMap<(u32, u32, u32), &'a BindingFact>,
    symbols: BTreeSet<&'a str>,
}

/// Diagnostic site: exact span plus its source text.
struct Site {
    span: ByteSpan,
    snippet: String,
}

fn site_of(node: &HirNode) -> Site {
    Site {
        span: node.span,
        snippet: node.snippet.clone(),
    }
}

fn diagnostic(capability: &str, construct: &str, site: &Site, reason: &str) -> HirDiagnostic {
    HirDiagnostic {
        capability: capability.to_string(),
        construct: construct.to_string(),
        span: site.span,
        snippet: site.snippet.clone(),
        severity: "error".to_string(),
        reason: reason.to_string(),
    }
}

fn error(capability: &str, construct: &str, node: &HirNode, reason: &str) -> Outcome {
    Outcome::reject(diagnostic(capability, construct, &site_of(node), reason))
}

fn evidence_gap(node: &HirNode, what: &str) -> Outcome {
    Outcome::gap(diagnostic(
        "rust.scalar-computation",
        &node.kind,
        &site_of(node),
        &format!("Required {what} evidence is unavailable; cannot certify this operation."),
    ))
}

/// Normalized primitive token of a resolved type, if admitted.
fn norm(ty: &ResolvedTy) -> Option<&str> {
    ty.normalized.as_deref()
}

/// Normalized primitive token of a node's value type, if present.
fn node_norm(node: &HirNode) -> Option<&str> {
    node.ty.as_ref().and_then(norm)
}

/// Classify one assessed function.
pub fn classify(facts: &dyn crate::unit_store::EvidenceFacts) -> HirEligibility {
    let profile = match crate::extraction_profile::ExtractionProfile::from_facts(facts) {
        Ok(profile) => profile,
        Err(reason) => {
            return HirEligibility {
                status: EligibilityVerdict::Unknown,
                diagnostics: vec![diagnostic(
                    "rust.extraction-profile",
                    "profile",
                    &site_of(&facts.body),
                    &reason,
                )],
            };
        }
    };
    let ctx = context(facts, profile);

    let outcome = Outcome::combine(vec![
        check_data_frontend(facts, profile),
        check_loop_values(&facts.body, profile),
        check_source_scan(facts),
        check_signature(facts, &ctx),
        check_bindings(facts, &ctx),
        check_node(&facts.body, false, &ctx),
    ]);
    let status = if outcome.rejected {
        EligibilityVerdict::Ineligible
    } else if outcome.gap {
        EligibilityVerdict::Unknown
    } else {
        EligibilityVerdict::Eligible
    };

    HirEligibility {
        status,
        diagnostics: outcome.diagnostics,
    }
}

fn check_data_frontend(
    facts: &dyn crate::unit_store::EvidenceFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Outcome {
    if !profile.supports("rust.concrete-read-only-data") {
        return Outcome::ok();
    }
    match crate::data_frontend::verify(facts) {
        Ok(()) => Outcome::ok(),
        Err(reason) => error(
            "rust.concrete-read-only-data",
            "data_evidence",
            &facts.body,
            &reason,
        ),
    }
}

fn accepted_type(ty: &str, profile: crate::extraction_profile::ExtractionProfile) -> bool {
    PRIMITIVE_TYS.contains(&ty)
        || profile.supports("rust.concrete-read-only-data")
            && crate::data_types::parse_for(ty, profile).is_ok()
}

fn context(
    facts: &dyn crate::unit_store::EvidenceFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Ctx<'_> {
    Ctx {
        facts,
        loops: Vec::new(),
        profile,
        bindings: facts
            .bindings
            .iter()
            .map(|binding| {
                (
                    (binding.hir.krate, binding.hir.owner, binding.hir.local),
                    binding,
                )
            })
            .collect(),
        symbols: facts
            .bindings
            .iter()
            .map(|binding| binding.symbol.as_str())
            .collect(),
    }
}

fn check_source_scan(facts: &dyn crate::unit_store::EvidenceFacts) -> Outcome {
    let source = Outcome {
        diagnostics: facts.source_diagnostics.clone(),
        gap: false,
        rejected: !facts.source_diagnostics.is_empty(),
    };

    if crate::extraction_profile::ExtractionProfile::from_facts(facts)
        .is_ok_and(|profile| profile.supports("rust.conditional-source-correspondence"))
    {
        return match crate::source_evidence::verify(facts) {
            Ok(()) => source,
            Err(reason) => source.merge(evidence_gap(&facts.body, &reason)),
        };
    }
    if facts.source_scan_complete {
        source
    } else {
        source.merge(evidence_gap(
            &facts.body,
            "complete pre-expansion source and conditional-source resolution",
        ))
    }
}

fn known_str(value: &StringFact) -> Option<&str> {
    match value.status {
        FactStatus::Known => value.value.as_deref(),
        _ => None,
    }
}

/// Signature-shape diagnostics anchor on the signature span.
fn signature_node(signature: &SignatureFacts) -> HirNode {
    HirNode {
        hir: crate::frontend::HirRef {
            krate: 0,
            owner: 0,
            local: 0,
        },
        kind: "signature".to_string(),
        span: signature.sig_span,
        snippet: String::new(),
        ty: None,
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: Vec::new(),
    }
}

fn check_signature(facts: &dyn crate::unit_store::EvidenceFacts, ctx: &Ctx<'_>) -> Outcome {
    Outcome::combine(vec![
        check_sig_subject(&facts.signature, ctx.profile),
        check_sig_header(&facts.signature, ctx.profile),
        check_sig_params(&facts.signature, ctx),
        check_sig_return(&facts.signature, ctx.profile),
    ])
}

fn check_receiver(
    signature: &SignatureFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Option<Outcome> {
    signature
        .receiver
        .as_ref()
        .filter(|_| !profile.supports("rust.concrete-read-only-data"))
        .map(|found| {
            let site = Site {
                span: found.span,
                snippet: found.snippet.clone(),
            };

            Outcome::reject(diagnostic(
                "rust.scalar-interface",
                "receiver",
                &site,
                "Receiver syntax (self) is outside R0 even for primitive types.",
            ))
        })
}

/// Receiver and generic parameters: rejected with their own spans.
fn check_sig_subject(
    signature: &SignatureFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Outcome {
    let anchor = signature_node(signature);
    let receiver = check_receiver(signature, profile);
    let missing = (signature.has_self && signature.receiver.is_none()).then(|| {
        error(
            "rust.scalar-interface",
            "receiver",
            &anchor,
            "Receiver syntax (self) is outside R0 even for primitive types.",
        )
    });
    let generics = signature
        .generics
        .iter()
        .filter(|_| !profile.supports("rust.concrete-read-only-data"))
        .map(|generic| {
            let site = Site {
                span: generic.span,
                snippet: generic.name.clone(),
            };

            Outcome::reject(diagnostic(
                "rust.additional-types",
                "generic_parameter",
                &site,
                "Function-level generics are deferred, even for scalar bodies.",
            ))
        });

    Outcome::combine(
        receiver
            .into_iter()
            .chain(missing)
            .chain(generics)
            .collect(),
    )
}

/// Calling convention and evaluation context: one normal Rust function.
fn check_sig_header(
    signature: &SignatureFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Outcome {
    check_sig_abi(signature).merge(check_sig_flags(signature, profile))
}

fn check_sig_abi(signature: &SignatureFacts) -> Outcome {
    let anchor = signature_node(signature);
    let abi = match known_str(&signature.abi) {
        Some("Rust") => None,
        Some(_) => Some(error(
            "rust.additional-types",
            "abi",
            &anchor,
            "Non-Rust ABIs are outside R0.",
        )),
        None => Some(Outcome::gap(diagnostic(
            "rust.additional-types",
            "abi",
            &Site {
                span: signature.sig_span,
                snippet: String::new(),
            },
            "Required ABI evidence is unavailable; cannot certify this operation.",
        ))),
    };
    Outcome::combine(abi.into_iter().collect())
}

fn check_sig_flags(
    signature: &SignatureFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Outcome {
    let anchor = signature_node(signature);
    let flags = [
        (
            signature.is_variadic,
            "rust.additional-types",
            "variadic_signature",
            "Variadic signatures are outside R0.",
        ),
        (
            signature.is_async,
            "rust.async",
            "async_fn",
            "Async functions are deferred to rust.async.",
        ),
        (
            signature.is_unsafe,
            "rust.unsafe",
            "unsafe_fn",
            "Unsafe functions are deferred to rust.unsafe.",
        ),
        (
            signature.is_const && profile == crate::extraction_profile::ExtractionProfile::R0,
            "rust.additional-types",
            "const_fn",
            "Const evaluation context is outside R0.",
        ),
    ];
    let header = flags
        .into_iter()
        .filter(|flag| flag.0)
        .map(|flag| error(flag.1, flag.2, &anchor, flag.3));

    Outcome::combine(header.collect())
}

fn check_sig_params(signature: &SignatureFacts, ctx: &Ctx<'_>) -> Outcome {
    Outcome::check_each(&signature.params, |param| check_param(param, ctx))
}

fn check_sig_return(
    signature: &SignatureFacts,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Outcome {
    match norm(&signature.return_ty) {
        Some(ty) if accepted_type(ty, profile) => Outcome::ok(),
        _ => Outcome::reject(diagnostic(
            "rust.additional-types",
            "return_type",
            &Site {
                span: signature.return_ty_span,
                snippet: signature.return_ty.spelling.clone(),
            },
            "Return type is outside the resolved primitive interface.",
        )),
    }
}

fn check_param(param: &ParamFact, ctx: &Ctx<'_>) -> Outcome {
    let pat = check_pat_node(&param.pat, ctx);
    let ty = match norm(&param.ty) {
        Some(ty) if accepted_type(ty, ctx.profile) => Outcome::ok(),
        _ => Outcome::reject(diagnostic(
            "rust.additional-types",
            "parameter_type",
            &Site {
                span: param.ty_span,
                snippet: param.ty.spelling.clone(),
            },
            "Parameter type is outside the resolved primitive interface.",
        )),
    };

    Outcome::combine(vec![pat, ty])
}

fn check_bindings(facts: &dyn crate::unit_store::EvidenceFacts, ctx: &Ctx<'_>) -> Outcome {
    let mut seen: BTreeSet<&str> = BTreeSet::new();

    Outcome::check_each(&facts.bindings, |binding| {
        let duplicate = if seen.insert(binding.symbol.as_str()) {
            Outcome::ok()
        } else {
            Outcome::reject(diagnostic(
                "rust.local-bindings",
                "binding",
                &Site {
                    span: binding.decl_span,
                    snippet: binding.name.clone(),
                },
                "Duplicate binding symbol; each definition needs its own ID.",
            ))
        };
        let table = if ctx.symbols.contains(binding.symbol.as_str()) {
            Outcome::ok()
        } else {
            evidence_gap(&binding_node(binding), "binding table entry")
        };
        let ty = match norm(&binding.ty) {
            Some(ty) if accepted_type(ty, ctx.profile) => Outcome::ok(),
            // Parameter interface types are diagnosed at the signature;
            // the table keeps the entry so reads still resolve.
            _ if binding.is_param => Outcome::ok(),
            _ => Outcome::reject(diagnostic(
                "rust.additional-types",
                "binding_type",
                &Site {
                    span: binding.decl_span,
                    snippet: binding.name.clone(),
                },
                "Binding type is outside the resolved primitive interface.",
            )),
        };

        Outcome::combine(vec![duplicate, table, ty])
    })
}

fn binding_node(binding: &BindingFact) -> HirNode {
    HirNode {
        hir: binding.hir.clone(),
        kind: "binding".to_string(),
        span: binding.decl_span,
        snippet: binding.name.clone(),
        ty: None,
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: Vec::new(),
    }
}

fn check_result_propagation(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::result_propagation::verify(node, ctx.facts) {
        Ok(checked) => check_node(checked.operand, false, ctx),
        Err(reason) => error(
            crate::result_propagation::CAPABILITY,
            "result_propagation",
            node,
            &reason,
        ),
    }
}

/// Check one construct; expanded subtrees report once at the outermost site.
/// Closures own their bodies separately, so their subtrees are excluded too.
fn check_node(node: &HirNode, in_expansion: bool, ctx: &Ctx<'_>) -> Outcome {
    if crate::option_propagation::candidate(node, ctx.facts) {
        return check_option_propagation(node, ctx);
    }
    if crate::result_propagation::candidate(node, ctx.facts) {
        return check_result_propagation(node, ctx);
    }
    if let Some(outcome) = check_data_contract(node, ctx) {
        return outcome;
    }
    if crate::data_subslices::candidate(node, ctx.facts) {
        return check_subslice(node, in_expansion, ctx);
    }
    if node.kind == "expr_assert" && ctx.profile.supports("rust.scalar-assertions") {
        return check_assertion(node, ctx);
    }
    if !in_expansion && let Some(site) = &node.expansion {
        return macro_diagnostic(node, site);
    }

    if ctx.profile.supports("rust.scalar-loops")
        && matches!(node.kind.as_str(), "expr_loop" | "expr_while")
    {
        return check_loop(node, ctx);
    }
    if ctx.profile.supports("rust.scalar-loops")
        && matches!(node.kind.as_str(), "expr_break" | "expr_continue")
    {
        return check_loop_exit(node, ctx);
    }
    if crate::data_enums::match_candidate(node, ctx.facts) {
        return check_enum_match(node, ctx);
    }
    if crate::checked_options::match_candidate(node, ctx.facts) {
        return check_option_match(node, ctx);
    }
    if node.kind == "expr_match" && ctx.profile.supports("rust.scalar-match") {
        return check_match(node, ctx);
    }
    check_ordinary_node(node, in_expansion, ctx)
}

fn check_ordinary_node(node: &HirNode, in_expansion: bool, ctx: &Ctx<'_>) -> Outcome {
    let own = check_kind(node, ctx);

    if node.kind == "expr_closure" || node.kind == "stmt_item" {
        return own;
    }

    let inner = in_expansion || node.expansion.is_some();
    let children = Outcome::combine(
        node.children
            .iter()
            .map(|child| check_node(child, inner, ctx))
            .collect(),
    );

    own.merge(children)
}

fn check_option_propagation(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::option_propagation::verify(node, ctx.facts) {
        Ok(checked) => check_node(checked.operand, false, ctx),
        Err(reason) => error(
            crate::option_propagation::CAPABILITY,
            "option_propagation",
            node,
            &reason,
        ),
    }
}

fn check_data_contract(node: &HirNode, ctx: &Ctx<'_>) -> Option<Outcome> {
    if crate::checked_options::if_let_candidate(node, ctx.facts) {
        return Some(check_option_if_let(node, ctx));
    }
    if crate::bool_constants::candidate(node) {
        return Some(match crate::bool_constants::verify_node(node, ctx.facts) {
            Ok(()) => Outcome::ok(),
            Err(reason) => error(
                crate::bool_constants::CAPABILITY,
                "const_table",
                node,
                &reason,
            ),
        });
    }

    if crate::iterator_position::candidate(node, ctx.facts) {
        return Some(match crate::iterator_position::verify(node, ctx.facts) {
            Ok(checked) => check_node(checked.receiver, false, ctx),
            Err(reason) => error(
                crate::iterator_position::CAPABILITY,
                "byte_position",
                node,
                &reason,
            ),
        });
    }
    if crate::named_callbacks::candidate(node, ctx.facts) {
        return Some(check_named_callback(node, ctx));
    }
    if crate::helper_option_callbacks::candidate(node, ctx.facts) {
        return Some(check_helper_callback(node, ctx));
    }
    if crate::option_callbacks::candidate(node, ctx.facts) {
        return Some(check_callback(node, ctx));
    }
    if crate::data_discards::candidate(node, ctx.facts) {
        return Some(match crate::data_discards::verify(node, ctx.facts) {
            Ok(()) => Outcome::ok(),
            Err(reason) => error("rust.scalar-enum-values", "copy_discard", node, &reason),
        });
    }
    None
}

fn check_named_callback(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::named_callbacks::verify(node, ctx.facts) {
        Ok(checked) => Outcome::combine(vec![
            check_node(checked.receiver, false, ctx),
            check_node(checked.default, false, ctx),
        ]),
        Err(reason) => error(
            crate::named_callbacks::CAPABILITY,
            "named_callback",
            node,
            &reason,
        ),
    }
}

fn check_helper_callback(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::helper_option_callbacks::verify(node, ctx.facts) {
        Ok(checked) => Outcome::combine(vec![
            check_node(checked.arguments.receiver, false, ctx),
            check_node(checked.arguments.default, false, ctx),
            check_node(checked.binding, false, ctx),
            check_node(checked.body, false, ctx),
        ]),
        Err(reason) => error(
            crate::helper_option_callbacks::CAPABILITY,
            "shared_helper_callback",
            node,
            &reason,
        ),
    }
}

fn check_callback(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::option_callbacks::verify_map_or(node, ctx.facts) {
        Ok(checked) => Outcome::combine(vec![
            check_node(checked.receiver, false, ctx),
            check_node(checked.default, false, ctx),
            check_node(checked.predicate.binding, false, ctx),
            check_node(checked.predicate.body, false, ctx),
        ]),
        Err(reason) => error(
            "rust.immediate-option-callbacks",
            "immediate_callback",
            node,
            &reason,
        ),
    }
}

fn check_subslice(node: &HirNode, in_expansion: bool, ctx: &Ctx<'_>) -> Outcome {
    match crate::data_subslices::verify(node, ctx.facts) {
        Ok(checked) => Outcome::combine(
            checked
                .operands()
                .map(|operand| check_node(operand, in_expansion, ctx))
                .collect(),
        ),
        Err(reason) => error("rust.shared-subslice-views", "subslice", node, &reason),
    }
}

fn check_assertion(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let verified = ctx
        .facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or_else(|| "Missing original assertion source".to_string())
        .and_then(|source| {
            crate::assertions::verify(
                node,
                source,
                ctx.facts.build.debug_assertions.value,
                ctx.profile,
            )
        });
    let own = match verified {
        Ok(()) => Outcome::ok(),
        Err(reason) => error(
            "rust.scalar-assertions",
            "assertion_evidence",
            node,
            &reason,
        ),
    };
    own.merge(Outcome::combine(
        node.children
            .iter()
            .map(|child| check_node(child, false, ctx))
            .collect(),
    ))
}

fn macro_diagnostic(node: &HirNode, site: &ExpansionSite) -> Outcome {
    Outcome::reject(diagnostic(
        "rust.macros",
        &node.kind,
        &Site {
            span: site.callsite,
            snippet: site.snippet.clone(),
        },
        "Macro invocations are deferred; expansion cannot erase the source construct.",
    ))
}

fn check_kind(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    if matches!(
        node.kind.as_str(),
        "expr_binary" | "expr_unary" | "expr_cast"
    ) && crate::float_scalars::uses(node)
    {
        return checked_operation(
            node,
            crate::float_scalars::verify_node(node, ctx.facts),
            crate::float_scalars::CAPABILITY,
            "float_operation",
        );
    }
    if ctx.profile.supports("rust.concrete-read-only-data")
        && let Some(outcome) = data_kind(node, ctx)
    {
        return outcome;
    }

    if node.kind.starts_with("pat_") {
        return check_pat_node(node, ctx);
    }

    if node.kind.starts_with("ty_") {
        return check_ty_node(node);
    }

    match node.kind.as_str() {
        "stmt_size_const" => checked_operation(
            node,
            crate::size_constants::verify(node, ctx.facts),
            "rust.native-borrowed-byte-readers",
            "local_size",
        ),
        "expr_call" if ctx.profile.supports("rust.verified-helper-calls") => {
            if crate::call_summaries::static_intrinsic(node, ctx.facts) {
                return check_method(node, ctx);
            }
            match crate::helper_calls::verify_use(node, ctx.facts) {
                Ok(_) => Outcome::ok(),
                Err(reason) => error("rust.call-summaries", "unverified_helper", node, &reason),
            }
        }
        "expr_method_call" if ctx.profile.supports("rust.verified-primitive-summaries") => {
            check_method(node, ctx)
        }
        "expr_cast" if ctx.profile.supports("rust.integer-casts") => check_supported(node, ctx),
        "expr_block" | "stmt_expr" | "stmt_semi" | "stmt_let" | "expr_path" | "expr_lit"
        | "expr_tup" | "expr_binary" | "expr_unary" | "expr_assign" | "expr_tuple_assign"
        | "expr_assign_op" | "expr_if" | "expr_ret" | "expr_let" => check_supported(node, ctx),
        _ => check_deferred(node),
    }
}

fn checked_operation(
    node: &HirNode,
    verification: Result<(), String>,
    capability: &str,
    code: &str,
) -> Outcome {
    match verification {
        Ok(()) => Outcome::ok(),
        Err(reason) => error(capability, code, node, &reason),
    }
}

fn check_method(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    if ctx.profile.supports("rust.verified-inherent-helper-calls")
        && crate::helper_calls::direct_resolution(node).is_some()
    {
        return match crate::helper_calls::verify_use(node, ctx.facts) {
            Ok(_) => Outcome::ok(),
            Err(reason) => error(
                "rust.call-summaries",
                "unverified_inherent_helper",
                node,
                &reason,
            ),
        };
    }
    if crate::call_summaries::resolve_for(node, ctx.facts).is_ok_and(|summary| summary.is_some()) {
        Outcome::ok()
    } else {
        error(
            "rust.call-summaries",
            "unregistered_primitive_summary",
            node,
            "Missing or mismatched resolved primitive callee, type/signature, pinned compiler/target/core provenance or named summary.",
        )
    }
}

fn enum_kind(node: &HirNode, ctx: &Ctx<'_>) -> Option<Outcome> {
    let (capability, kind, checked) = if crate::fieldless_enums::value_candidate(node, ctx.facts) {
        (
            crate::fieldless_enums::CAPABILITY,
            "fieldless_value",
            crate::fieldless_enums::value(node, ctx.facts).map(|_| ()),
        )
    } else if crate::fieldless_enums::cast_candidate(node, ctx.facts) {
        (
            crate::fieldless_enums::CAPABILITY,
            "fieldless_cast",
            crate::fieldless_enums::cast(node, ctx.facts),
        )
    } else if crate::data_enums::constructor_candidate(node, ctx.facts) {
        (
            crate::data_enums::CAPABILITY,
            "enum_constructor",
            crate::data_enums::constructor(node, ctx.facts).map(|_| ()),
        )
    } else {
        return None;
    };
    Some(match checked {
        Ok(()) => Outcome::ok(),
        Err(reason) => error(capability, kind, node, &reason),
    })
}

fn data_kind(node: &HirNode, ctx: &Ctx<'_>) -> Option<Outcome> {
    if let Some(outcome) = enum_kind(node, ctx) {
        return Some(outcome);
    }
    if crate::checked_options::constructor_candidate(node, ctx.facts) {
        return Some(check_option_constructor(node, ctx));
    }
    if crate::concrete_results::constructor_candidate(node, ctx.facts) {
        return Some(check_result_constructor(node, ctx));
    }

    let ty = || match node_norm(node) {
        Some(token) if accepted_type(token, ctx.profile) => Outcome::ok(),
        _ => check_expr_ty(node, "data_value"),
    };
    match node.kind.as_str() {
        "expr_lit" if node_norm(node).is_some_and(|ty| matches!(ty, "char" | "f32" | "f64")) => {
            Some(ty())
        }
        "expr_tup" | "expr_array" if ctx.profile.supports("rust.aggregate-literal-values") => {
            Some(ty())
        }
        "expr_repeat" if ctx.profile.supports("rust.repeat-array-values") => Some(ty()),
        "expr_struct" if ctx.profile.supports("rust.named-struct-values") => Some(ty()),
        "expr_field" | "expr_addr_of" => Some(ty()),
        "expr_index" if crate::data_index_reads::candidate(node, ctx.facts) => Some(ty()),
        "expr_unary" if crate::data_dereferences::candidate(node, ctx.facts) => Some(ty()),
        "expr_block" if !node.is_unsafe_block => Some(ty()),
        "expr_ret" | "ty_annotation" => Some(ty()),
        "expr_if"
            if node
                .children
                .first()
                .is_some_and(|condition| node_norm(condition) == Some("bool")) =>
        {
            Some(ty())
        }
        "expr_path"
            if node.res.as_ref().is_some_and(|resolution| {
                matches!(
                    resolution.kind.as_str(),
                    crate::scalar_constants::RESOLUTION | crate::primitive_constants::RESOLUTION
                )
            }) =>
        {
            Some(check_path(node, ctx))
        }
        "expr_path" => Some(check_data_path(node, ctx)),
        _ => None,
    }
}

fn check_result_constructor(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::concrete_results::verify_constructor(node, ctx.facts) {
        Ok(_) => Outcome::ok(),
        Err(reason) => error(
            crate::concrete_results::CAPABILITY,
            "result_constructor",
            node,
            &reason,
        ),
    }
}

fn check_data_path(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match node.res.as_ref().and_then(|resolution| {
        (resolution.kind == "local")
            .then_some(resolution.target.as_ref())
            .flatten()
    }) {
        Some(target)
            if ctx
                .bindings
                .contains_key(&(target.krate, target.owner, target.local)) =>
        {
            match node_norm(node) {
                Some(token) if accepted_type(token, ctx.profile) => Outcome::ok(),
                _ => check_expr_ty(node, "data_value"),
            }
        }
        _ => evidence_gap(node, "resolved data binding read"),
    }
}

fn check_option_constructor(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::checked_options::verify_constructor(node, ctx.facts) {
        Ok(_) => Outcome::ok(),
        Err(reason) => error(
            "rust.scalar-option-values",
            "option_constructor",
            node,
            &reason,
        ),
    }
}

/// Constructs that may be admitted once their evidence checks pass.
fn check_supported(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match node.kind.as_str() {
        "expr_cast" if crate::bool_usize::candidate(node) => checked_operation(
            node,
            crate::bool_usize::verify(node, ctx.facts),
            crate::bool_usize::CAPABILITY,
            "bool_cast",
        ),
        "expr_cast" if ctx.profile.supports("rust.integer-casts") => check_cast(node),
        "expr_block" => check_block(node),
        "stmt_expr" | "stmt_semi" => Outcome::ok(),
        "stmt_let" => check_let(node, ctx),
        "expr_path" => check_path(node, ctx),
        "expr_lit" => check_expr_ty(node, "literal"),
        "expr_tup" => check_tup(node),
        "expr_binary"
            if ctx.profile.supports(crate::character_scalars::CAPABILITY)
                && node
                    .children
                    .iter()
                    .any(|child| node_norm(child) == Some("char")) =>
        {
            checked_operation(
                node,
                crate::character_scalars::comparison(node),
                crate::character_scalars::CAPABILITY,
                "comparison",
            )
        }
        "expr_binary" => check_binary(node),
        "expr_unary" => check_unary(node),
        "expr_assign" => check_assign(node, ctx),
        "expr_tuple_assign" => checked_operation(
            node,
            crate::tuple_assignment::verify(node, ctx.facts),
            "rust.scalar-tuple-assignment",
            "tuple_assignment",
        ),
        "expr_assign_op" => check_assign_op(node, ctx),
        "expr_if" => check_if(node),
        "expr_ret" => check_expr_ty(node, "return"),
        // The `if` arm owns the `if let` rejection; the wrapper stays quiet so
        // one condition yields one diagnostic while its scrutinee still scans.
        "expr_let" => Outcome::ok(),
        _ => evidence_gap(node, "supported-kind dispatch"),
    }
}

/// Deferred constructs: each maps to its stable capability ID.
fn check_deferred(node: &HirNode) -> Outcome {
    match node.kind.as_str() {
        "stmt_item" => error(
            "rust.blocks",
            "nested_item",
            node,
            "Nested items are unsupported in R0; reject at declaration without incorporating the separately owned body.",
        ),
        "expr_call" | "expr_method_call" => error(
            "rust.call-summaries",
            "call",
            node,
            "Arbitrary calls are unsupported even with known signature or presumed purity.",
        ),
        _ => check_deferred_expression(node),
    }
}

fn check_deferred_expression(node: &HirNode) -> Outcome {
    match node.kind.as_str() {
        "expr_field" | "expr_index" | "expr_addr_of" => error(
            "rust.abstract-memory",
            node.kind.as_str(),
            node,
            "Field, index, and borrow operations are deferred to abstract memory.",
        ),
        "expr_cast" => error(
            "rust.additional-types",
            "cast",
            node,
            "Cast expressions are deferred in R0, including built-in `as` casts.",
        ),
        "expr_loop" | "expr_while" | "expr_break" | "expr_continue" => error(
            "rust.loops",
            node.kind.as_str(),
            node,
            "Loops are deferred to rust.loops.",
        ),
        "expr_match" => error(
            "rust.match",
            "match",
            node,
            "Match is deferred to rust.match.",
        ),
        _ => check_deferred_owned_body(node),
    }
}

fn check_deferred_owned_body(node: &HirNode) -> Outcome {
    match node.kind.as_str() {
        "expr_closure" => error(
            "rust.closures",
            "closure",
            node,
            "Closures are deferred; their separately owned body is excluded.",
        ),
        "expr_yield" => error(
            "rust.async",
            "yield",
            node,
            "Async constructs are deferred.",
        ),
        "expr_const_block" => error(
            "rust.additional-types",
            "const_block",
            node,
            "Const blocks are outside R0.",
        ),
        "expr_struct" | "expr_array" | "expr_repeat" => error(
            "rust.additional-types",
            node.kind.as_str(),
            node,
            "Composite construction is outside the scalar interface.",
        ),
        "expr_err" => evidence_gap(node, "typeck error recovery"),
        _ => error(
            "rust.additional-types",
            node.kind.as_str(),
            node,
            "Construct is outside the R0 closed subset.",
        ),
    }
}

/// Pattern nodes are leaves: the outermost shape decides, children excluded.
fn check_pat_node(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match node.kind.as_str() {
        "pat_binding" => match &node.binding {
            Some(symbol) if ctx.symbols.contains(symbol.as_str()) => Outcome::ok(),
            _ => evidence_gap(node, "binding definition"),
        },
        "pat_wild" => error(
            "rust.scalar-interface",
            "wildcard_pattern",
            node,
            "R0 requires a named single binding for each parameter; no fabricated symbol for wildcard.",
        ),
        _ => error(
            "rust.scalar-interface",
            node.kind.as_str(),
            node,
            "Only a single identifier, optionally `mut`, is an accepted binding pattern.",
        ),
    }
}

fn check_ty_node(node: &HirNode) -> Outcome {
    check_scalar_type(node, "annotation_type", "annotation type")
}

/// Every retained expression must carry a resolved primitive value type.
fn check_expr_ty(node: &HirNode, what: &str) -> Outcome {
    if matches!(node.kind.as_str(), "expr_block" | "expr_if" | "expr_match")
        && node.ty.as_ref().is_some_and(|ty| ty.observed == "!")
    {
        return Outcome::ok();
    }

    check_scalar_type(node, what, "expression type")
}

fn check_scalar_type(node: &HirNode, construct: &str, evidence: &str) -> Outcome {
    match node_norm(node) {
        Some(ty) if PRIMITIVE_TYS.contains(&ty) => Outcome::ok(),
        Some(_) => error(
            "rust.additional-types",
            construct,
            node,
            "Type is outside the resolved primitive interface.",
        ),
        None if node.ty.is_none() => evidence_gap(node, evidence),
        None => error(
            "rust.additional-types",
            construct,
            node,
            "Compiler-resolved value type is outside the primitive R0 subset.",
        ),
    }
}

fn check_block(node: &HirNode) -> Outcome {
    if node.is_unsafe_block {
        return error(
            "rust.unsafe",
            "unsafe_block",
            node,
            "Unsafe blocks are deferred to rust.unsafe.",
        );
    }

    check_expr_ty(node, "block")
}

fn check_tup(node: &HirNode) -> Outcome {
    if !node.children.is_empty() {
        return error(
            "rust.additional-types",
            "tuple",
            node,
            "Only the empty unit tuple is admitted; tuples are composite types.",
        );
    }

    check_expr_ty(node, "unit")
}

fn check_path(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let resolution = match &node.res {
        None => return evidence_gap(node, "path resolution"),
        Some(res) => res,
    };

    if let Some(outcome) = constant_path(node, ctx) {
        return outcome;
    }

    if resolution.kind != "local" {
        return error(
            "rust.additional-types",
            "non_binding_path",
            node,
            "Only lexical binding reads are admitted; items and constants are outside R0.",
        );
    }

    match &resolution.target {
        Some(target)
            if ctx
                .bindings
                .contains_key(&(target.krate, target.owner, target.local)) =>
        {
            if node_norm(node).is_some_and(|token| {
                crate::data_locals::supported(token, ctx.profile)
                    || token == "char" && ctx.profile.supports(crate::character_scalars::CAPABILITY)
                    || crate::float_scalars::is_float(token)
                        && ctx.profile.supports(crate::float_scalars::CAPABILITY)
            }) {
                Outcome::ok()
            } else {
                check_expr_ty(node, "read")
            }
        }
        _ => evidence_gap(node, "binding target"),
    }
}

fn constant_path(node: &HirNode, ctx: &Ctx<'_>) -> Option<Outcome> {
    let resolution = node.res.as_ref()?;
    let (capability, result) = match resolution.kind.as_str() {
        crate::primitive_constants::RESOLUTION => (
            crate::primitive_constants::CAPABILITY,
            crate::primitive_constants::verify(node, ctx.facts).map(|_| ()),
        ),
        crate::scalar_constants::RESOLUTION
            if ctx.profile.supports("rust.resolved-scalar-constants") =>
        {
            (
                "rust.resolved-scalar-constants",
                crate::scalar_constants::ScalarConstant::verify(
                    node,
                    ctx.facts.build.pointer_width.value,
                )
                .map(|_| ()),
            )
        }
        _ => return None,
    };

    Some(match result {
        Ok(()) => Outcome::ok(),
        Err(reason) => error(capability, "constant_evidence", node, &reason),
    })
}

fn builtin_or_reject(node: &HirNode, what: &str) -> Outcome {
    match &node.dispatch {
        None => evidence_gap(node, "operator dispatch"),
        Some(dispatch) if dispatch.kind == "builtin" => Outcome::ok(),
        Some(_) => error(
            "rust.scalar-computation",
            what,
            node,
            "Overloaded operators are not admitted; built-in dispatch must be evidenced.",
        ),
    }
}

fn contains_loop(node: &HirNode) -> bool {
    matches!(node.kind.as_str(), "expr_loop" | "expr_while")
        || node.children.iter().any(contains_loop)
}

fn check_loop_values(
    node: &HirNode,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Outcome {
    if !profile.supports("rust.scalar-loops")
        || !contains_loop(node)
        || profile.supports("rust.cyclic-value-merges")
    {
        return Outcome::ok();
    }
    check_no_loop_merges(
        node,
        false,
        profile.supports("rust.outside-loop-value-merges"),
    )
}

fn check_no_loop_merges(node: &HirNode, inside_loop: bool, outside: bool) -> Outcome {
    let inside_loop = inside_loop || matches!(node.kind.as_str(), "expr_loop" | "expr_while");
    let branching = matches!(node.kind.as_str(), "expr_if" | "expr_match")
        && !matches!(node_norm(node), Some("unit") | None)
        && node.ty.as_ref().is_none_or(|ty| ty.observed != "!");
    let short = node
        .dispatch
        .as_ref()
        .is_some_and(|dispatch| matches!(dispatch.op.as_str(), "&&" | "||"));

    let own = if (branching || short) && (inside_loop || !outside) {
        error(
            "rust.scalar-loops",
            "loop_value_merges",
            node,
            "The initial cyclic policy excludes nonunit conditional and short-circuit value merges in loop-containing functions.",
        )
    } else {
        Outcome::ok()
    };
    own.merge(Outcome::combine(
        node.children
            .iter()
            .map(|child| check_no_loop_merges(child, inside_loop, outside))
            .collect(),
    ))
}

fn check_loop(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let expected = if node.kind == "expr_while" { 2 } else { 1 };
    if node.children.len() != expected {
        return evidence_gap(node, "loop source/HIR correspondence");
    }

    let mut nested = ctx.clone();
    nested.loops.push(node.hir.clone());
    let own = if node.kind == "expr_while" && node_norm(&node.children[0]) != Some("bool") {
        error(
            "rust.scalar-loops",
            "while_condition",
            node,
            "While condition must resolve to bool.",
        )
    } else {
        Outcome::ok()
    };
    own.merge(Outcome::combine(
        node.children
            .iter()
            .map(|child| check_node(child, false, &nested))
            .collect(),
    ))
}

fn check_loop_exit(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    if !node.children.is_empty()
        || node.res.as_ref().is_none_or(|resolution| {
            resolution.kind != "innermost_loop" || resolution.target.as_ref() != ctx.loops.last()
        })
    {
        return error(
            "rust.scalar-loops",
            "loop_exit",
            node,
            "Only resolved unlabelled valueless exits to the innermost loop are supported.",
        );
    }
    Outcome::ok()
}

fn check_match(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let Some((scrutinee, arms)) = node.children.split_first() else {
        return evidence_gap(node, "match source/HIR correspondence");
    };
    let mut checks = vec![
        check_option_result(node, ctx, "match_result"),
        check_node(scrutinee, false, ctx),
    ];
    if !node_norm(scrutinee).is_some_and(|ty| {
        is_int(ty)
            || ty == "bool"
            || ty == "char" && ctx.profile.supports(crate::character_scalars::CAPABILITY)
    }) {
        checks.push(error(
            "rust.scalar-match",
            "scrutinee",
            scrutinee,
            "Match requires a resolved integer or bool scrutinee.",
        ));
    }
    if arms.is_empty() || arms.len() > 16 || !match_exhaustive(node, arms, ctx) {
        checks.push(error(
            "rust.scalar-match",
            "match_boundary",
            node,
            "Bounded match requires 1..=16 arms and independently exhaustive unguarded patterns under its registered policy.",
        ));
    }
    for (index, arm) in arms.iter().enumerate() {
        checks.push(check_match_arm(
            arm,
            scrutinee,
            index + 1 == arms.len(),
            ctx,
        ));
    }

    Outcome::combine(checks)
}

fn match_exhaustive(node: &HirNode, arms: &[HirNode], ctx: &Ctx<'_>) -> bool {
    if ctx.profile.supports("rust.exhaustive-scalar-match") {
        return crate::scalar_exhaustiveness::closing_arm(
            node,
            ctx.facts.build.pointer_width.value,
        )
        .is_some();
    }
    arms.last()
        .is_some_and(|arm| arm.children.len() == 2 && arm.children[0].kind == "pat_wild")
}

fn check_match_arm(arm: &HirNode, scrutinee: &HirNode, last: bool, ctx: &Ctx<'_>) -> Outcome {
    let Some((pattern, children)) = arm.children.split_first() else {
        return evidence_gap(arm, "match arm correspondence");
    };
    let guarded = children.len() == 2 && children[0].kind == "match_guard";
    let shape = arm.kind == "match_arm" && (children.len() == 1 || guarded);
    let pattern_valid = scalar_match_pattern(pattern, scrutinee, ctx)
        || (pattern.kind == "pat_wild"
            && (last
                || ctx.profile.supports("rust.exhaustive-scalar-match")
                || (guarded && ctx.profile.supports("rust.scalar-match-guards"))));
    let valid =
        shape && pattern_valid && (!guarded || ctx.profile.supports("rust.scalar-match-guards"));
    let own = if valid {
        Outcome::ok()
    } else {
        error(
            "rust.scalar-match",
            "arm_shape",
            arm,
            "Only registered scalar literal/range/OR/wildcard arms with supported guards are admitted.",
        )
    };
    let body = Outcome::combine(
        children
            .iter()
            .map(|child| {
                if child.kind == "match_guard" {
                    check_match_guard(child, ctx)
                } else {
                    check_node(child, false, ctx)
                }
            })
            .collect(),
    );

    own.merge(body)
}

fn scalar_match_pattern(pattern: &HirNode, scrutinee: &HirNode, ctx: &Ctx<'_>) -> bool {
    let minimum = if ctx.profile.supports("rust.nested-scalar-or-patterns") {
        1
    } else {
        2
    };

    scalar_pattern_leaf(pattern, scrutinee, ctx)
        || (ctx.profile.supports("rust.scalar-or-patterns")
            && pattern.kind == "pat_or"
            && pattern.children.len() >= minimum
            && node_norm(pattern) == node_norm(scrutinee)
            && pattern.children.iter().all(|child| {
                if ctx.profile.supports("rust.nested-scalar-or-patterns") {
                    scalar_match_pattern(child, scrutinee, ctx)
                } else {
                    scalar_pattern_leaf(child, scrutinee, ctx)
                }
            }))
}

fn scalar_pattern_leaf(pattern: &HirNode, scrutinee: &HirNode, ctx: &Ctx<'_>) -> bool {
    if (pattern.kind != "pat_range" && !pattern.children.is_empty())
        || node_norm(pattern) != node_norm(scrutinee)
    {
        return false;
    }
    if pattern.kind == "pat_wild" {
        return ctx.profile.supports("rust.nested-scalar-or-patterns");
    }
    if pattern.kind == "pat_literal" {
        return pattern.lit.is_some()
            && (!ctx.profile.supports("rust.exhaustive-scalar-match")
                || ctx
                    .facts
                    .source
                    .bytes
                    .get(pattern.span.start as usize..pattern.span.end as usize)
                    == pattern.lit.as_deref());
    }
    if pattern.kind == "pat_constant" {
        return ctx.profile.supports("rust.scalar-constant-patterns")
            && crate::scalar_constants::ScalarConstant::verify_pattern(
                pattern,
                ctx.facts.build.pointer_width.value,
            )
            .is_ok();
    }

    scalar_range_evidence(pattern, ctx)
}

fn scalar_range_evidence(pattern: &HirNode, ctx: &Ctx<'_>) -> bool {
    if pattern.kind == "pat_range" && node_norm(pattern) == Some("char") {
        return crate::character_ranges::verify_node(pattern, ctx.facts).is_ok();
    }
    ctx.profile.supports("rust.scalar-range-patterns")
        && pattern.kind == "pat_range"
        && node_norm(pattern)
            .is_some_and(|ty| crate::concrete_types::scalar(ty) && !matches!(ty, "bool" | "unit"))
        && ctx
            .facts
            .source
            .bytes
            .get(pattern.span.start as usize..pattern.span.end as usize)
            .and_then(|source| {
                let resolve = if ctx.profile.supports(crate::byte_literals::CAPABILITY) {
                    crate::scalar_patterns::IntegerRange::from_byte_pattern
                } else {
                    crate::scalar_patterns::IntegerRange::from_pattern
                };
                resolve(
                    source,
                    pattern,
                    ctx.facts.build.pointer_width.value,
                    ctx.profile.supports("rust.scalar-constant-range-bounds"),
                )
                .ok()
            })
            .is_some_and(|range| pattern.lit.as_deref() == Some(range.text().as_str()))
}

fn check_match_guard(wrapper: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let [guard] = wrapper.children.as_slice() else {
        return evidence_gap(wrapper, "guard correspondence");
    };
    let result = if node_norm(guard) == Some("bool") {
        Outcome::ok()
    } else {
        error(
            "rust.scalar-match-guards",
            "guard_type",
            guard,
            "Guard result must resolve to bool.",
        )
    };
    result
        .merge(check_guard_subset(guard))
        .merge(check_node(guard, false, ctx))
}

fn check_guard_subset(node: &HirNode) -> Outcome {
    let own = if matches!(
        node.kind.as_str(),
        "expr_lit" | "expr_path" | "expr_unary" | "expr_binary" | "expr_block" | "expr_if"
    ) {
        Outcome::ok()
    } else {
        error(
            "rust.scalar-match-guards",
            "guard_subset",
            node,
            "Guards are restricted to pure base scalar expressions; writes, declarations, casts, calls and returns remain excluded.",
        )
    };
    own.merge(Outcome::combine(
        node.children.iter().map(check_guard_subset).collect(),
    ))
}

fn check_cast(node: &HirNode) -> Outcome {
    let Some(dispatch) = &node.dispatch else {
        return evidence_gap(node, "resolved built-in integer cast");
    };
    let [operand] = node.children.as_slice() else {
        return evidence_gap(node, "cast operand correspondence");
    };
    let Some(input) = node_norm(operand) else {
        return error(
            "rust.integer-casts",
            "cast_operand",
            node,
            "Only resolved integer-to-integer casts are supported.",
        );
    };
    let Some(result) = node_norm(node) else {
        return error(
            "rust.integer-casts",
            "cast_result",
            node,
            "Only resolved integer-to-integer casts are supported.",
        );
    };
    if !is_int(input) || !is_int(result) {
        return error(
            "rust.integer-casts",
            "cast_domain",
            node,
            "Float, bool, unit, pointer and reference casts are excluded.",
        );
    }
    if dispatch.kind != "builtin"
        || dispatch.op != "as"
        || dispatch.operand_tys != [input]
        || dispatch.result_ty != result
    {
        return evidence_gap(node, "consistent resolved cast types and dispatch");
    }

    Outcome::ok()
}

fn check_binary(node: &HirNode) -> Outcome {
    let dispatch = builtin_or_reject(node, "binary");
    if !dispatch.diagnostics.is_empty() || dispatch.gap {
        return dispatch;
    }

    let dispatch = node.dispatch.clone().expect("dispatch evidenced");
    let operands: Vec<&str> = dispatch.operand_tys.iter().map(String::as_str).collect();
    let [left, right] = operands.as_slice() else {
        return evidence_gap(node, "binary operand types");
    };

    if *left == "unit" || *right == "unit" {
        return unit_comparison(node, &dispatch.op);
    }

    if !operands_supported(&dispatch.op, left, right) {
        return error(
            "rust.additional-types",
            "binary",
            node,
            "Operand types are outside the admitted built-in combinations.",
        );
    }

    check_expr_ty(node, "binary")
}

/// Unit comparisons use library traits, not built-in scalar operations.
fn unit_comparison(node: &HirNode, op: &str) -> Outcome {
    if COMPARISONS.contains(&op) {
        return error(
            "rust.scalar-computation",
            "unit_comparison",
            node,
            "Unit equality uses the standard-library PartialEq implementation, not an admitted built-in integer/boolean comparison; the unit-comparison exception is deferred and unimplemented.",
        );
    }

    error(
        "rust.additional-types",
        "unit_operation",
        node,
        "Unit-typed operands are outside the admitted built-in combinations.",
    )
}

fn operands_supported(op: &str, left: &str, right: &str) -> bool {
    if is_int(left) && is_int(right) {
        return INT_OPS.contains(&op);
    }

    if left == "bool" && right == "bool" {
        return BOOL_EAGER_OPS.contains(&op) || op == "&&" || op == "||";
    }

    false
}

fn is_int(ty: &str) -> bool {
    SIGNED_INTS.contains(&ty) || UNSIGNED_INTS.contains(&ty)
}

fn check_unary(node: &HirNode) -> Outcome {
    let dispatch = builtin_or_reject(node, "unary");
    if !dispatch.diagnostics.is_empty() || dispatch.gap {
        return dispatch;
    }

    let dispatch = node.dispatch.clone().expect("dispatch evidenced");
    let operands: Vec<&str> = dispatch.operand_tys.iter().map(String::as_str).collect();
    let [operand] = operands.as_slice() else {
        return evidence_gap(node, "unary operand type");
    };

    match dispatch.op.as_str() {
        "*" => error(
            "rust.abstract-memory",
            "deref",
            node,
            "Explicit dereference is outside R0.",
        ),
        "!" if *operand == "bool" || is_int(operand) => check_expr_ty(node, "unary"),
        "-" if SIGNED_INTS.contains(operand) => check_expr_ty(node, "unary"),
        _ => error(
            "rust.additional-types",
            "unary",
            node,
            "Unary operator is outside the admitted built-in combinations.",
        ),
    }
}

/// Direct assignment target must be a lexical binding path (never a read).
fn check_assign(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let [target, _value] = node.children.as_slice() else {
        return evidence_gap(node, "assignment shape");
    };

    if target.kind != "expr_path" {
        return error(
            "rust.abstract-memory",
            "indirect_write",
            node,
            "Indirect and memory-backed writes are deferred to abstract memory.",
        );
    }

    let target_check = check_path(target, ctx);
    if !target_check.diagnostics.is_empty() || target_check.gap {
        return target_check;
    }

    check_expr_ty(node, "assignment")
}

fn check_assign_op(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let dispatch = builtin_or_reject(node, "compound_assignment");
    if !dispatch.diagnostics.is_empty() || dispatch.gap {
        return dispatch;
    }

    let dispatch = node.dispatch.clone().expect("dispatch evidenced");
    if !COMPOUND_OPS.contains(&dispatch.op.as_str()) {
        return error(
            "rust.additional-types",
            "compound_assignment",
            node,
            "Compound operator is outside the admitted set.",
        );
    }

    let [target, _rhs] = node.children.as_slice() else {
        return evidence_gap(node, "compound shape");
    };

    if target.kind != "expr_path" {
        return error(
            "rust.abstract-memory",
            "indirect_write",
            node,
            "Indirect and memory-backed writes are deferred to abstract memory.",
        );
    }

    let target_check = check_path(target, ctx);
    if !target_check.diagnostics.is_empty() || target_check.gap {
        return target_check;
    }

    check_expr_ty(node, "compound_assignment")
}

fn check_option_if_let(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::checked_options::verify_if_let(node, ctx.facts) {
        Ok(checked) => Outcome::combine(vec![
            check_node(checked.scrutinee, false, ctx),
            check_node(checked.binding, false, ctx),
            check_node(checked.then_branch, false, ctx),
            check_node(checked.else_branch, false, ctx),
            check_kind(node, ctx),
        ]),
        Err(reason) => error("rust.scalar-option-if-let", "if_let", node, &reason),
    }
}

fn check_if(node: &HirNode) -> Outcome {
    let [cond, ..] = node.children.as_slice() else {
        return evidence_gap(node, "branch shape");
    };

    if cond.kind == "expr_let" {
        return error(
            "rust.branching",
            "let_condition",
            cond,
            "`if let` and let chains are rejected in R0.",
        );
    }

    match node_norm(cond) {
        Some("bool") => check_expr_ty(node, "if"),
        Some(_) => error(
            "rust.additional-types",
            "condition",
            cond,
            "Branch conditions must resolve to `bool`.",
        ),
        None => evidence_gap(cond, "condition type"),
    }
}

fn check_let(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    let pats = node
        .children
        .iter()
        .filter(|child| child.kind.starts_with("pat_"));
    let anns = node
        .children
        .iter()
        .filter(|child| child.kind.starts_with("ty_"));
    let elses = node
        .children
        .iter()
        .filter(|child| child.kind == "let_else");
    let pat = pats.map(|pat| check_pat_node(pat, ctx)).collect();
    let ann = anns.map(|node| check_kind(node, ctx)).collect();
    let els = elses
        .map(|found| {
            error(
                "rust.local-bindings",
                "let_else",
                found,
                "`let else` is rejected in R0.",
            )
        })
        .collect();
    let shape = shape_outcome(node);

    Outcome::combine(vec![
        Outcome::combine(pat),
        Outcome::combine(ann),
        Outcome::combine(els),
        shape,
    ])
}

/// A declaration needs its pattern and a single initializer value.
fn shape_outcome(node: &HirNode) -> Outcome {
    let pats = node
        .children
        .iter()
        .filter(|child| child.kind.starts_with("pat_"))
        .count();
    let values = node
        .children
        .iter()
        .filter(|child| {
            !child.kind.starts_with("pat_")
                && !child.kind.starts_with("ty_")
                && child.kind != "let_else"
        })
        .count();

    if pats == 0 {
        return evidence_gap(node, "declaration pattern");
    }

    if values == 0 {
        return error(
            "rust.local-bindings",
            "uninitialized_declaration",
            node,
            "Uninitialized declarations are rejected in R0.",
        );
    }

    Outcome::ok()
}

/// Function-relative span constructor for tests and signature facts.
pub fn span(start: u32, end: u32) -> ByteSpan {
    ByteSpan { start, end }
}

fn check_option_match(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::checked_options::verify_match(node, ctx.facts) {
        Ok(checked) => {
            let mut checks = vec![
                check_option_result(node, ctx, "option_match_result"),
                check_node(checked.scrutinee, false, ctx),
            ];
            for arm in &node.children[1..] {
                checks.push(check_node(
                    arm.children.last().expect("verified Option arm"),
                    false,
                    ctx,
                ));
            }
            Outcome::combine(checks)
        }
        Err(reason) => error("rust.checked-slice-option", "option_match", node, &reason),
    }
}

fn check_option_result(node: &HirNode, ctx: &Ctx<'_>, construct: &str) -> Outcome {
    if crate::checked_options::scalar_enabled(ctx.facts) && crate::checked_options::owned(node) {
        match node_norm(node) {
            Some(token) if accepted_type(token, ctx.profile) => Outcome::ok(),
            _ => check_expr_ty(node, construct),
        }
    } else {
        check_expr_ty(node, construct)
    }
}

fn check_enum_match(node: &HirNode, ctx: &Ctx<'_>) -> Outcome {
    match crate::data_enums::matched(node, ctx.facts) {
        Ok(checked) => {
            let result = match node_norm(node) {
                Some(token) if accepted_type(token, ctx.profile) => Outcome::ok(),
                _ => check_expr_ty(node, "enum_match_result"),
            };
            Outcome::combine(
                std::iter::once(result)
                    .chain(std::iter::once(check_node(checked.scrutinee, false, ctx)))
                    .chain(node.children[1..].iter().map(|arm| {
                        check_node(arm.children.last().expect("verified enum arm"), false, ctx)
                    }))
                    .collect(),
            )
        }
        Err(reason) => error(crate::data_enums::CAPABILITY, "enum_match", node, &reason),
    }
}
