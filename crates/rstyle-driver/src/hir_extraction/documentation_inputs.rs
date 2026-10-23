//! Observe direct builtin crate documentation expansion and its complete buffer.
use super::{module_expansions::file_span, sources::Sources};
use rstyle_pdg::{nominal_boundary::census::documentation::Input, source_identity::checksum};
use rustc_ast::token::DocFragmentKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::{
    ExpnId, Symbol,
    hygiene::{ExpnKind, MacroKind},
};

pub(super) fn capture(tcx: TyCtxt<'_>, sources: &Sources) -> Result<Vec<Input>, String> {
    let mut inputs = Vec::new();
    for attribute in tcx.hir_krate_attrs() {
        let Some((comment, DocFragmentKind::Raw(span))) = attribute.doc_str_and_fragment_kind()
        else {
            continue;
        };
        if !span.from_expansion() {
            continue;
        }
        let data = span.ctxt().outer_expn_data();
        let Some(definition) = data.macro_def_id else {
            continue;
        };
        if data.parent != ExpnId::root()
            || data.call_site.from_expansion()
            || !matches!(data.kind, ExpnKind::Macro(MacroKind::Bang, name) if name.as_str() == "include_str")
            || definition.is_local()
            || tcx.crate_name(definition.krate).as_str() != "core"
            || !tcx.is_diagnostic_item(Symbol::intern("include_str_macro"), definition)
        {
            continue;
        }
        let input = file_span(tcx, sources, span)?;
        let physical_path = input
            .path
            .canonicalize()
            .map_err(|error| error.to_string())?;
        inputs.push(Input {
            invocation: file_span(tcx, sources, data.call_site)?,
            input,
            physical_path,
            builtin: super::library_resolution::definition(tcx, definition),
            expanded_digest: checksum(comment.as_str().as_bytes()),
        });
    }
    Ok(inputs)
}
