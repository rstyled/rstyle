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
        (
            Kind::Native | Kind::NativeUsize | Kind::NativeU16 | Kind::NativeU128 | Kind::LittleU32,
            syn::Expr::Call(call),
        ) if native_invocation(call, kind) && node.kind == "expr_call" => &call.args[0],
        (
            Kind::Copy(_) | Kind::Unwrap(_) | Kind::Borrow(_) | Kind::UnwrapBorrow(_),
            syn::Expr::MethodCall(call),
        ) if call.args.is_empty()
            && call.turbofish.is_none()
            && node.kind == "expr_method_call"
            && call.method
                == if kind.conversion() {
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

pub(super) fn native_invocation(call: &syn::ExprCall, kind: Kind) -> bool {
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return false;
    };

    let method = if kind == Kind::LittleU32 {
        "from_le_bytes"
    } else {
        "from_ne_bytes"
    };
    call.args.len() == 1
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == method && segment.arguments.is_empty())
}
