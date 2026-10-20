//! Original syntax and operand/pattern correspondence, independent of spellings.

use super::*;
use syn::spanned::Spanned;

fn corresponding(
    expression: &impl Spanned,
    child: &HirNode,
    parent: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let bytes = &facts.source.bytes[parent.span.start as usize..parent.span.end as usize];
    let span = expression.span();
    let expected = crate::ByteSpan {
        start: parent.span.start + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: parent.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    };

    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, child.span) {
        return Err("Checked Option loses original child source correspondence".into());
    }
    Ok(())
}

pub(super) fn get(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::MethodCall(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Checked get lacks original method syntax".into());
    };
    if call.method != "get" || call.args.len() != 1 || call.turbofish.is_some() {
        return Err("Checked get requires one original index argument".into());
    }

    corresponding(&*call.receiver, &node.children[0], node, facts)?;
    corresponding(&call.args[0], &node.children[1], node, facts)
}

pub(super) fn matched(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Match(matched) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Checked Option lacks original match syntax".into());
    };
    if matched.arms.len() != 2 || matched.arms.iter().any(|arm| arm.guard.is_some()) {
        return Err("Checked Option requires two exhaustive unguarded arms".into());
    }

    corresponding(&*matched.expr, &node.children[0], node, facts)?;
    for (source, arm) in matched.arms.iter().zip(&node.children[1..]) {
        let [pattern, body] = arm.children.as_slice() else {
            return Err("Checked Option arm loses exact pattern/body correspondence".into());
        };
        if arm.kind != "match_arm" {
            return Err("Checked Option requires original match arms".into());
        }
        corresponding(&source.pat, pattern, node, facts)?;
        corresponding(&*source.body, body, node, facts)?;
        pattern_syntax(&source.pat, pattern, node, facts)?;
    }
    Ok(())
}

fn pattern_syntax(
    syntax: &syn::Pat,
    pattern: &HirNode,
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    match (syntax, pattern.kind.as_str()) {
        (syn::Pat::Path(_), "pat_option_none") => Ok(()),
        (syn::Pat::Ident(name), "pat_option_none")
            if name.by_ref.is_none() && name.mutability.is_none() && name.subpat.is_none() =>
        {
            Ok(())
        }
        (syn::Pat::TupleStruct(some), "pat_option_some") if some.elems.len() == 1 => {
            let syn::Pat::Reference(reference) = &some.elems[0] else {
                return Err("Checked Option Some requires an explicit shared scalar copy".into());
            };
            let syn::Pat::Ident(binding) = reference.pat.as_ref() else {
                return Err("Checked Option payload requires one scalar binding".into());
            };
            if reference.mutability.is_some()
                || binding.mutability.is_some()
                || binding.by_ref.is_some()
                || binding.subpat.is_some()
            {
                return Err("Checked Option payload requires an immutable scalar binding".into());
            }
            let [shared] = pattern.children.as_slice() else {
                return Err("Missing checked Option reference pattern".into());
            };
            let [child] = shared.children.as_slice() else {
                return Err("Missing checked Option scalar pattern".into());
            };
            corresponding(reference, shared, node, facts)?;
            corresponding(binding, child, node, facts)
        }
        _ => Err("Checked Option source pattern contradicts its resolved shape".into()),
    }
}
