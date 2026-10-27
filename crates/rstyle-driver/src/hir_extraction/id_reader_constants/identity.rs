//! Original local declaration coordinates; expansion coordinates stay separate.
use super::super::hir::Context;
use rstyle_pdg::id_reader_constants::Site;
use rustc_hir::{Node, def_id::DefId};

pub(super) fn declaration(context: &Context<'_>, definition: DefId) -> Option<Site> {
    let span = match context.tcx.hir_node_by_def_id(definition.as_local()?) {
        Node::Item(item) => item.span,
        Node::ImplItem(item) => item.span,
        _ => return None,
    };
    if span.from_expansion() || span.is_dummy() {
        return None;
    }
    let site =
        super::super::module_expansions::file_span(context.tcx, &context.sources, span).ok()?;
    Some(Site {
        file: site.path.to_str()?.into(),
        span: site.span,
    })
}
