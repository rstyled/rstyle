//! Exact original invocation and argument correspondence, separate from authority.

use super::*;
use syn::spanned::Spanned;

pub(super) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let [operand] = node.children.as_slice() else {
        return Err("Byte call requires one operand".into());
    };
    if node.expansion.is_some() {
        return Err("Byte calls require original source".into());
    }
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original byte call")?;
    let syntax: syn::Expr = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let argument = match (kind, crate::data_aggregates::ungroup(&syntax)) {
        (Kind::Native, syn::Expr::Call(call))
            if native_invocation(call) && node.kind == "expr_call" =>
        {
            &call.args[0]
        }
        (Kind::Copy | Kind::Unwrap, syn::Expr::MethodCall(call))
            if call.args.is_empty()
                && call.turbofish.is_none()
                && node.kind == "expr_method_call"
                && call.method
                    == if kind == Kind::Copy {
                        "try_into"
                    } else {
                        "unwrap"
                    } =>
        {
            &call.receiver
        }
        _ => return Err("Byte-call source invocation disagrees".into()),
    };
    let span = crate::data_aggregates::ungroup(argument).span();
    let expected = crate::ByteSpan {
        start: node.span.start + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, operand.span) {
        return Err("Byte call loses exact operand source correspondence".into());
    }

    Ok(())
}

fn native_invocation(call: &syn::ExprCall) -> bool {
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return false;
    };

    call.args.len() == 1
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "from_ne_bytes" && segment.arguments.is_empty())
}
