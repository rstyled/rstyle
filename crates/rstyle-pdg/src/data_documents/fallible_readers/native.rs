//! Verify the selected existing reader body, never a helper name or pure signature.
use super::*;
use crate::native_reader_width::Width;

pub(super) fn verify(
    call: &HirNode,
    width: Width,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let dependency = crate::helper_calls::verify_use(call, facts)?;
    let summary = dependency.summary();
    if summary.argument_types != [crate::data_aggregates::token(&call.children[0])?]
        || summary.result_type != width.scalar()
        || summary.mutates_arguments
        || summary.reference_outputs.is_some()
        || summary.component_outputs.is_some()
        || summary.tagged_output.is_some()
        || summary
            .memory_reads
            .as_ref()
            .is_none_or(|reads| reads.is_empty() || reads.iter().any(|read| read.parameter != 0))
    {
        return Err("Reader helper lacks its exact scalar/read-only byte interface".into());
    }
    // The verified dependency preserves its real panic behavior. This source
    // contract does not infer a panic-free callee from the caller's precheck.
    match &dependency {
        crate::helper_calls::VerifiedDependency::Shared(view) => body(&view.frontend(), width),
        crate::helper_calls::VerifiedDependency::Legacy(dependency) => {
            body(&dependency.frontend, width)
        }
    }
}

fn body(facts: &dyn EvidenceFacts, width: Width) -> Result<(), String> {
    interface(facts, width)?;

    let [statement, native] = facts.body.children.as_slice() else {
        return Err("Native dependency requires the complete array-copy and decode body".into());
    };
    let [binding, annotation, unwrap] = statement.children.as_slice() else {
        return Err("Native dependency changes its complete initialized array declaration".into());
    };
    if statement.kind != "stmt_let"
        || binding.kind != "pat_binding"
        || binding.binding_mut != Some(false)
        || !binding.children.is_empty()
        || annotation.kind != "ty_annotation"
        || crate::data_bytes::verify(unwrap, facts)?
            != crate::data_bytes::Kind::Unwrap(width.bytes() as u64)
        || crate::data_bytes::verify(native, facts)? != width.native()
    {
        return Err("Native dependency requires exact copy unwrap and native scalar decode".into());
    }

    let [copied] = unwrap.children.as_slice() else {
        return Err("Native dependency loses its exact saved copied Result".into());
    };
    if crate::data_bytes::verify(copied, facts)?
        != crate::data_bytes::Kind::Copy(width.bytes() as u64)
    {
        return Err("Native dependency changes its exact-width byte copy".into());
    }

    let [window] = copied.children.as_slice() else {
        return Err("Native dependency changes its sole byte view".into());
    };
    let view = crate::data_subslices::verify(window, facts)?;
    if view.start.is_some() || view.array_length.is_some() {
        return Err("Native dependency must read the original slice prefix".into());
    }
    source::formal(view.base, 0, facts)?;
    source::size(
        view.end.ok_or("Missing native byte prefix length")?,
        width,
        facts,
    )?;

    saved_array(native, binding)
}

fn saved_array(native: &HirNode, binding: &HirNode) -> Result<(), String> {
    let [value] = native.children.as_slice() else {
        return Err("Native dependency decode changes its sole saved array".into());
    };
    if value.kind != "expr_path"
        || !value.children.is_empty()
        || value.expansion.is_some()
        || value.dispatch.is_some()
        || !value.res.as_ref().is_some_and(|resolution| {
            resolution.kind == "local" && resolution.target.as_ref() == Some(&binding.hir)
        })
    {
        return Err("Native dependency decodes a different or recomputed array".into());
    }
    Ok(())
}

fn interface(facts: &dyn EvidenceFacts, width: Width) -> Result<(), String> {
    if facts.signature.params.len() != 1
        || !source::implicit_slice_lifetime(facts)
        || facts.signature.has_self
        || facts.signature.is_async
        || facts.signature.is_unsafe
        || facts.signature.is_const
        || facts.signature.is_variadic
        || facts.signature.return_ty.normalized.as_deref() != Some(width.scalar())
    {
        return Err("Native dependency changes the bounded reader interface".into());
    }
    Ok(())
}
