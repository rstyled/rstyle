//! File-relative original coordinates and actual selected definitions.
use super::{CaptureFailure, Context};
use rstyle_pdg::{ByteSpan, data_library::Callee, nominal_constants::SourceSite};
use rustc_hir::{Node, def::DefKind, def_id::DefId};
use rustc_span::{FileName, Span};

pub(super) fn local(context: &Context<'_>, definition: DefId) -> Result<Callee, CaptureFailure> {
    if !definition.is_local() {
        return Err(CaptureFailure::Unsupported(
            "selected product definition is nonlocal",
        ));
    }
    Ok(Callee {
        path: context.tcx.def_path_str(definition),
        crate_name: context.tcx.crate_name(definition.krate).to_string(),
        crate_hash: "local-unencoded".into(),
        definition_hash: format!("{:?}", context.tcx.def_path_hash(definition)),
        implementation: None,
    })
}

pub(super) fn site(context: &Context<'_>, span: Span) -> Result<SourceSite, CaptureFailure> {
    let range = original_range(context, span)?;
    let file = context.tcx.sess.source_map().lookup_source_file(span.lo());
    let FileName::Real(real) = &file.name else {
        return Err(CaptureFailure::Unsupported(
            "product constant lacks a real source file",
        ));
    };
    let path = real.local_path().ok_or(CaptureFailure::Unsupported(
        "product source lacks a local path",
    ))?;
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| CaptureFailure::Unsupported("compiler working directory is unavailable"))?
            .join(path)
    };

    Ok(SourceSite {
        file: path.to_string_lossy().into_owned(),
        span: range,
    })
}

pub(super) fn original_range(
    context: &Context<'_>,
    span: Span,
) -> Result<ByteSpan, CaptureFailure> {
    if span.from_expansion() || span.is_dummy() {
        return Err(CaptureFailure::Unsupported(
            "product constant source is generated or missing",
        ));
    }
    let file = context.tcx.sess.source_map().lookup_source_file(span.lo());
    if !matches!(&file.name, FileName::Real(_)) {
        return Err(CaptureFailure::Unsupported(
            "product constant lacks a real source file",
        ));
    }
    context
        .sources
        .range(context.tcx, span)
        .map(|(_, range)| range)
        .ok_or(CaptureFailure::Unsupported(
            "original product source range is unavailable",
        ))
}

pub(super) fn declaration(
    context: &Context<'_>,
    definition: DefId,
) -> Result<SourceSite, CaptureFailure> {
    let local = definition.as_local().ok_or(CaptureFailure::Unsupported(
        "missing local product declaration",
    ))?;
    let span = match context.tcx.hir_node_by_def_id(local) {
        Node::ImplItem(item) => item.span,
        Node::Item(item) => item.span,
        _ => return Err(CaptureFailure::Unsupported("missing original product item")),
    };
    site(context, span)
}

pub(super) fn implementation(
    context: &Context<'_>,
    member: DefId,
) -> Result<(DefId, SourceSite), CaptureFailure> {
    let parent = context.tcx.parent(member);
    if !parent.is_local()
        || !matches!(
            context.tcx.def_kind(parent),
            DefKind::Impl { of_trait: false }
        )
        || context.tcx.generics_of(parent).count() != 0
    {
        return Err(CaptureFailure::Unsupported(
            "product member requires a concrete inherent impl",
        ));
    }
    Ok((parent, declaration(context, parent)?))
}
