//! One concrete, lossless core From instantiation; custom traits stay opaque.
use crate::{HirNode, call_summaries::Summary, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) const CAPABILITY: &str = "rust.scalar-from-conversions";
pub(crate) const CONSTRUCT: &str = "data.scalar-from";
const TRAIT: &str = "std::convert::From::from";
const CORE_TRAIT: &str = "core::convert::From::from";
const CORE_IMPLEMENTATION: &str =
    "core::convert::num::<impl core::convert::From<u16> for u32>::from";
const IMPLEMENTATION: &str = "std::convert::num::<impl std::convert::From<u16> for u32>::from";

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && node.kind == "expr_call"
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| matches!(dispatch.op.as_str(), TRAIT | CORE_TRAIT))
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    if !candidate(node, facts) {
        return Ok(None);
    }

    let [operand] = node.children.as_slice() else {
        return Err("Scalar From requires exactly one evaluated operand".into());
    };
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing scalar From dispatch")?;
    if node.expansion.is_some()
        || dispatch.kind != "resolved_direct"
        || dispatch.operand_tys != ["u16"]
        || dispatch.result_ty != "u32"
        || crate::data_aggregates::token(node)? != "u32"
        || crate::data_aggregates::token(operand)? != "u16"
    {
        return Err("Only concrete u16-to-u32 From is registered".into());
    }
    authority(node, facts)?;
    source(node, operand, facts)?;
    Ok(Some(Summary {
        id: "rstyle.data.scalar.from-u16-u32".into(),
        version: "1.0.0".into(),
        callee_path: dispatch.op.clone(),
        callee_crate: "core".into(),
        argument_types: vec!["u16".into()],
        result_type: "u32".into(),
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "lossless-widening".into(),
    }))
}

fn authority(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts).ok_or("Unpinned scalar From build")?;
    let callee = crate::data_library::callee(node)?;
    let implementation = callee
        .implementation
        .as_ref()
        .ok_or("Missing selected scalar From implementation")?;
    let display = if node
        .dispatch
        .as_ref()
        .is_some_and(|dispatch| dispatch.op == CORE_TRAIT)
    {
        (CORE_TRAIT, CORE_IMPLEMENTATION)
    } else {
        (TRAIT, IMPLEMENTATION)
    };

    for (definition, path, pin) in [
        (&callee, display.0, TRAIT),
        (implementation.as_ref(), display.1, IMPLEMENTATION),
    ] {
        if definition.crate_name != "core"
            || definition.crate_hash != registry.core_hash
            || definition.path != path
            || registry.definitions.get(pin) != Some(&definition.definition_hash)
        {
            return Err("Scalar From disagrees with pinned core trait/implementation".into());
        }
    }
    if implementation.implementation.is_some() {
        return Err("Unexpected nested scalar From implementation".into());
    }
    Ok(())
}

fn source(node: &HirNode, operand: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Scalar From requires original direct call syntax".into());
    };
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return Err("Scalar From requires a concrete primitive path".into());
    };
    let names: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect();
    if call.args.len() != 1
        || path.qself.is_some()
        || path
            .path
            .segments
            .iter()
            .any(|part| !part.arguments.is_empty())
        || !(names == ["u32", "from"]
            || names == ["core", "primitive", "u32", "from"]
            || names == ["std", "primitive", "u32", "from"])
    {
        return Err("Scalar From source is outside the bounded primitive invocation".into());
    }

    let bytes = &facts.source.bytes[node.span.start as usize..node.span.end as usize];
    let span = crate::data_aggregates::ungroup(&call.args[0]).span();
    let expected = crate::ByteSpan {
        start: node.span.start + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, operand.span) {
        return Err("Scalar From loses original operand correspondence".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn numeric_from_contract_binds_the_reviewed_pinned_core_source() {
        let output = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let sysroot = String::from_utf8(output.stdout).unwrap();
        let path = std::path::Path::new(sysroot.trim())
            .join("lib/rustlib/src/rust/library/core/src/convert/num.rs");
        let source = std::fs::read(path).unwrap();
        assert_eq!(
            crate::concrete_types::digest(&source),
            "sha256:8dbf6c64681e2dee556e0844eef3e53d62a40df0280d25f469352870a401e034"
        );
    }
}
