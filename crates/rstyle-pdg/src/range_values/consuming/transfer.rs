use crate::{HirNode, ParamFact, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

/// Affine use evidence only. The caller separately verifies Into/From selection.
pub(crate) struct Transfer<'a> {
    formal: &'a ParamFact,
    receiver: &'a HirNode,
}

impl Transfer<'_> {
    pub(crate) fn formal_position(&self) -> u32 {
        self.formal.position
    }
    pub(crate) fn receiver(&self) -> &HirNode {
        self.receiver
    }
}

pub(crate) fn verify_transfer<'a>(
    facts: &'a dyn EvidenceFacts,
    call: &'a HirNode,
    original: &dyn EvidenceFacts,
) -> Result<Transfer<'a>, String> {
    let formal = super::formal::verify(facts)?;
    let [receiver] = call.children.as_slice() else {
        return Err("Owned Range transfer requires exactly its saved receiver".into());
    };
    if call.kind != "expr_method_call"
        || call.expansion.is_some()
        || !call
            .dispatch
            .as_ref()
            .is_some_and(|d| d.kind == "resolved_method" && d.op == "core::convert::Into::into")
        || !super::formal::read(receiver, formal)
    {
        return Err("Owned Range transfer requires the unborrowed formal at Into".into());
    }

    source(facts, formal, call)?;
    let mut paths = Vec::new();
    uses(&facts.body, formal, &mut paths)?;
    if paths.as_slice() != [receiver] {
        return Err(
            "Owned Range must transfer exactly once with no other whole or field use".into(),
        );
    }

    unadjusted(original, call, receiver)?;
    original_site(original, facts, call)?;
    Ok(Transfer { formal, receiver })
}

pub(super) fn source(
    facts: &dyn EvidenceFacts,
    formal: &ParamFact,
    call: &HirNode,
) -> Result<(), String> {
    let function = super::source::function(facts)?;
    let name = super::source::formal_name(&function, formal, facts)?;
    let Some(syn::Stmt::Local(local)) = function.block.stmts.first() else {
        return Err("Range transfer must be the first straight-line initializer".into());
    };

    shadow(local, &name)?;
    let init = local
        .init
        .as_ref()
        .ok_or("Missing Range conversion initializer")?;
    let syn::Expr::MethodCall(method) = init.expr.as_ref() else {
        return Err("Range initializer must directly consume Into receiver".into());
    };
    if init.diverge.is_some()
        || method.method != "into"
        || method.turbofish.is_some()
        || !method.args.is_empty()
        || !crate::source_evidence::plain_value_path(&method.receiver, &name)
        || !super::source::corresponds(method.span(), call.span, facts)
        || !super::source::corresponds(method.receiver.span(), call.children[0].span, facts)
    {
        return Err("Range transfer differs from the exact original initializer".into());
    }

    let Some(statement) = facts.body.children.first() else {
        return Err("Missing Range initializer HIR".into());
    };
    if facts.body.kind != "expr_block"
        || statement.kind != "stmt_let"
        || statement.children.last() != Some(call)
    {
        return Err("Range transfer is not the exact first HIR initializer".into());
    }

    super::source::node_source(call, facts)?;
    super::source::node_source(&call.children[0], facts)
}

pub(super) fn uses<'a>(
    node: &'a HirNode,
    formal: &ParamFact,
    paths: &mut Vec<&'a HirNode>,
) -> Result<(), String> {
    if matches!(
        node.kind.as_str(),
        "expr_loop"
            | "expr_if"
            | "expr_match"
            | "expr_closure"
            | "expr_assign"
            | "expr_assign_op"
            | "expr_await"
            | "expr_yield"
    ) {
        return Err(
            "Owned Range transfer excludes branching, repetition, capture and reassignment".into(),
        );
    }
    if node.res.as_ref().and_then(|r| r.target.as_ref()) == Some(&formal.pat.hir) {
        if !super::formal::read(node, formal) {
            return Err("Unexpected use of the owned Range formal".into());
        }
        paths.push(node);
    }
    for child in &node.children {
        uses(child, formal, paths)?;
    }
    Ok(())
}

fn unadjusted(facts: &dyn EvidenceFacts, call: &HirNode, receiver: &HirNode) -> Result<(), String> {
    let capture = crate::data_receivers::verify_capture(facts)?;
    let records: Vec<_> = capture
        .methods
        .iter()
        .filter(|m| m.call == call.hir)
        .collect();
    let [record] = records.as_slice() else {
        return Err("Missing unique Range consuming receiver observation".into());
    };
    if record.receiver != receiver.hir
        || record.source_span != call.span
        || !record.adjustments.is_empty()
        || record.receiver_type != record.adjusted_type
        || record.signature_inputs.as_slice() != [record.receiver_type.clone()]
    {
        return Err(
            "Range transfer cannot borrow, dereference, adjust or duplicate its receiver".into(),
        );
    }

    match &record.receiver_type {
        crate::concrete_types::Observation::Unsupported { .. } => Ok(()),
        crate::concrete_types::Observation::Captured {
            ty: crate::concrete_types::TypeTree::RangeUsize { observation },
        } => super::super::authority::verify(observation, facts),
        _ => Err(
            "Range transfer receiver is neither its original generic observation nor owned Range"
                .into(),
        ),
    }
}

fn shadow(local: &syn::Local, name: &str) -> Result<(), String> {
    let syn::Pat::Type(typed) = &local.pat else {
        return Err("Range transfer must shadow the moved source with its concrete result".into());
    };
    let syn::Pat::Ident(binding) = typed.pat.as_ref() else {
        return Err("Range transfer result requires a plain new local".into());
    };
    if binding.ident != name
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
    {
        return Err("Range transfer must make its moved source inaccessible by shadowing".into());
    }
    Ok(())
}

fn original_site(
    original: &dyn EvidenceFacts,
    facts: &dyn EvidenceFacts,
    call: &HirNode,
) -> Result<(), String> {
    let raw = original
        .body
        .children
        .first()
        .and_then(|s| s.children.last())
        .ok_or("Missing immutable original transfer site")?;
    let [receiver] = raw.children.as_slice() else {
        return Err("Original Range transfer must have one receiver".into());
    };
    if original.name != facts.name
        || original.fn_span != facts.fn_span
        || original.source != facts.source
        || raw.hir != call.hir
        || raw.kind != call.kind
        || raw.span != call.span
        || raw.snippet != call.snippet
        || raw.res != call.res
        || receiver.hir != call.children[0].hir
        || receiver.res != call.children[0].res
        || receiver.span != call.children[0].span
        || receiver.snippet != call.children[0].snippet
    {
        return Err("Range transfer differs from its immutable original source site".into());
    }
    Ok(())
}
