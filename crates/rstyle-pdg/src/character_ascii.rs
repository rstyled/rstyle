//! Pinned char ASCII classification with a method-local immutable autoborrow.

use crate::{
    HirNode, ProfileId,
    call_summaries::Summary,
    concrete_types::{Observation, TypeTree},
    data_receivers::{Kind, Method},
    unit_store::EvidenceFacts,
};
use syn::spanned::Spanned;

pub(crate) const CAPABILITY: &str = "rust.character-ascii-predicate";
pub(crate) const CONSTRUCT: &str = "data.char-is-ascii";
const PATHS: &[&str] = &[
    "std::char::methods::<impl char>::is_ascii",
    "core::char::methods::<impl char>::is_ascii",
];

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::data_receivers::candidate(
        node,
        facts,
        crate::data_receivers::InvocationSyntax {
            capability: CAPABILITY,
            kind: "expr_method_call",
            paths: PATHS,
        },
    )
}

/// Only this Method inventory may contain an internal shared character type.
/// This checks no other receiver inventory, avoiding recursive verification.
pub(crate) fn verify_receiver(
    record: &Method,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if !candidate(node, facts)
        || node
            .children
            .first()
            .and_then(|receiver| crate::frontend::normalized(receiver))
            != Some("char")
    {
        return Ok(false);
    }
    if !crate::extraction_profile::facts_support(facts, crate::character_scalars::CAPABILITY) {
        return Err("Character ASCII requires the character scalar policy".into());
    }

    values(node)?;
    authority(node, facts)?;
    let character = scalar("char");
    let borrowed = Observation::Captured {
        ty: TypeTree::Shared(Box::new(TypeTree::Scalar("char".into()))),
    };
    let [adjustment] = record.adjustments.as_slice() else {
        return Err("Character ASCII requires exactly one observed shared borrow".into());
    };
    if record.call != node.hir
        || record.source_span != node.span
        || record.receiver != node.children[0].hir
        || record.receiver_type != character
        || record.adjusted_type != borrowed
        || record.signature_inputs != [borrowed.clone()]
        || record.signature_result != scalar("bool")
        || adjustment.kind != Kind::SharedBorrow
        || adjustment.target != borrowed
    {
        return Err(
            "Character ASCII loses its exact immutable receiver adjustment/signature".into(),
        );
    }
    Ok(true)
}

fn scalar(name: &str) -> Observation {
    Observation::Captured {
        ty: TypeTree::Scalar(name.into()),
    }
}

fn values(node: &HirNode) -> Result<(), String> {
    let [receiver] = node.children.as_slice() else {
        return Err("Character ASCII requires one saved scalar receiver".into());
    };
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing character ASCII dispatch")?;
    if node.expansion.is_some()
        || node.is_unsafe_block
        || crate::frontend::normalized(receiver) != Some("char")
        || crate::frontend::normalized(node) != Some("bool")
        || dispatch.kind != "resolved_method"
        || !PATHS.contains(&dispatch.op.as_str())
        || dispatch.operand_tys != ["char"]
        || dispatch.result_ty != "bool"
    {
        return Err("Character ASCII requires an original char receiver and bool result".into());
    }
    crate::data_receivers::verify_dispatch(node, &dispatch.op, &["bool"])
}

fn authority(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let callee = crate::data_library::callee(node)?;
    if node.dispatch.as_ref().map(|dispatch| &dispatch.op) != Some(&callee.path) {
        return Err("Character ASCII dispatch disagrees with its resolved callee".into());
    }
    crate::data_library::verify_core_definition(
        &callee,
        facts,
        crate::data_library::CoreDefinition {
            paths: PATHS,
            unavailable: "Character ASCII requires its pinned compiler/target authority",
            mismatch: "Character ASCII disagrees with the pinned core diagnostic definition",
        },
    )
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    if !candidate(node, facts) {
        return Ok(None);
    }
    // Source correspondence itself resolves summaries. Check observations here;
    // the unconditional document boundary verifies full receiver correspondence.
    let capture = crate::data_receivers::verify_capture(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|record| record.call == node.hir)
        .ok_or("Missing character ASCII receiver evidence")?;
    if !verify_receiver(record, node, facts)? {
        return Err("Character ASCII requires a saved scalar receiver".into());
    }
    source(node, facts)?;

    Ok(Some(Summary {
        id: "rstyle.data.char.is-ascii".into(),
        version: "1".into(),
        callee_path: node
            .dispatch
            .as_ref()
            .ok_or("Missing character ASCII dispatch")?
            .op
            .clone(),
        callee_crate: "core".into(),
        argument_types: vec!["char".into()],
        result_type: "bool".into(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "unicode-codepoint-le-0x7f".into(),
    }))
}

fn source(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::MethodCall(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Character ASCII requires original method-call syntax".into());
    };
    if call.method != "is_ascii" || call.turbofish.is_some() || !call.args.is_empty() {
        return Err("Character ASCII source has unsupported arguments or generic syntax".into());
    }

    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Character ASCII source span is outside its function")?;
    let span = crate::data_aggregates::ungroup(&call.receiver).span();
    let expected = crate::ByteSpan {
        start: node.span.start + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(
        &facts.source.bytes,
        expected,
        node.children[0].span,
    ) {
        return Err("Character ASCII loses its original receiver correspondence".into());
    }
    Ok(())
}

pub(crate) fn verify_document_policy(
    policy: &ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        policy,
        CAPABILITY,
        || has_call(&facts.body, facts),
        "Character ASCII requires its registered document policy",
    )
}

fn has_call(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    candidate(node, facts) || node.children.iter().any(|child| has_call(child, facts))
}

#[cfg(test)]
mod tests {
    #[test]
    fn registered_ascii_authority_binds_reviewed_core_source() {
        let output = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let sysroot = String::from_utf8(output.stdout).unwrap();
        let path = std::path::Path::new(sysroot.trim())
            .join("lib/rustlib/src/rust/library/core/src/char/methods.rs");
        assert_eq!(
            crate::concrete_types::digest(&std::fs::read(path).unwrap()),
            "sha256:17ced91be9e97808deffcb1686e213cbe18a71cd86b3b5243cc94c0cb6a9455f"
        );
    }
}
