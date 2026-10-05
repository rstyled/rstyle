//! Turns every local function body into a plain `FnFacts` record.

use crate::{effects, ids, mutations, pdg, sig};
use rstyle_core::facts::{FnFacts, ParamFacts};
use rstyle_core::lines::preceding_comment_line;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{Body, FnSig, PatKind};
use rustc_middle::ty::TyCtxt;
use rustc_span::{FileName, SourceFile, Span};
use std::path::Path;

/// How many lines above a function we look for an `rstyle-allow` comment.
const LOOKBACK_LINES: usize = 40;

/// 1-based line of a span's outermost macro call site, so expanded code maps to user source.
pub fn line_of(tcx: TyCtxt<'_>, span: Span) -> usize {
    tcx.sess
        .source_map()
        .lookup_char_pos(span.source_callsite().lo())
        .line
}

pub fn crate_facts(tcx: TyCtxt<'_>, root: &Path, with_pdg: bool) -> Vec<FnFacts> {
    let mut facts: Vec<FnFacts> = tcx
        .hir_body_owners()
        .filter_map(|def_id| function_facts(tcx, def_id, root, with_pdg))
        .collect();
    facts.sort_by(|a, b| {
        (&a.path, a.line, a.column, &a.name).cmp(&(&b.path, b.line, b.column, &b.name))
    });
    facts
}

fn function_facts(
    tcx: TyCtxt<'_>,
    def_id: LocalDefId,
    root: &Path,
    with_pdg: bool,
) -> Option<FnFacts> {
    if !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
        return None;
    }
    let node = tcx.hir_node_by_def_id(def_id);
    let (sig, body_id) = (node.fn_sig()?, node.body_id()?);
    if sig.span.from_expansion() {
        return None;
    }
    let body = tcx.hir_body(body_id);
    let source_map = tcx.sess.source_map();
    let start = source_map.lookup_char_pos(sig.span.lo());
    let source = source_map
        .span_to_snippet(sig.span.to(body.value.span))
        .ok()?;
    let has_self = sig.decl.implicit_self().has_implicit_self();
    let body_facts = effects::collect(tcx, def_id, body, root);
    let (mutations, mutated_types) = mutations::collect(tcx, def_id, root);
    let implements = tcx
        .opt_associated_item(def_id.to_def_id())
        .and_then(|item| item.trait_item_def_id())
        .map(|trait_fn| ids::def_id_str(tcx, trait_fn));
    Some(FnFacts {
        id: ids::def_id_str(tcx, def_id.to_def_id()),
        implements,
        path: display_path(&start.file, root)?,
        line: start.line,
        column: start.col.0 + 1,
        name: tcx.def_path_str(def_id.to_def_id()),
        params: sig.decl.inputs.len() - usize::from(has_self),
        param_list: param_list(tcx, sig, body, &body_facts.param_uses),
        has_self,
        statements: body_facts.statements,
        source,
        preceding_line: lines_above(&start.file, start.line),
        mutations,
        calls: body_facts.calls,
        effects: body_facts.effects,
        edges: body_facts.edges,
        mutated_types,
        sig: sig::sig_facts(tcx, def_id, sig),
        pdg: with_pdg.then(|| pdg::pdg_of(tcx, def_id, body, root)),
    })
}

/// Name, written type and use count of each parameter, `self` excluded.
fn param_list(
    tcx: TyCtxt<'_>,
    sig: &FnSig<'_>,
    body: &Body<'_>,
    uses: &[usize],
) -> Vec<ParamFacts> {
    let skip = usize::from(sig.decl.implicit_self().has_implicit_self());
    let source_map = tcx.sess.source_map();
    sig.decl
        .inputs
        .iter()
        .zip(body.params)
        .skip(skip)
        .enumerate()
        .map(|(index, (ty, param))| ParamFacts {
            name: match param.pat.kind {
                PatKind::Binding(_, _, ident, _) => ident.name.to_string(),
                _ => "_".into(),
            },
            ty: source_map
                .span_to_snippet(ty.span)
                .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
                .unwrap_or_default(),
            uses: uses.get(index).copied().unwrap_or(0),
        })
        .collect()
}

fn lines_above(file: &SourceFile, line: usize) -> String {
    let first = line.saturating_sub(1 + LOOKBACK_LINES);
    let above: Vec<String> = (first..line.saturating_sub(1))
        .filter_map(|index| file.get_line(index).map(|text| text.to_string()))
        .collect();
    let refs: Vec<&str> = above.iter().map(String::as_str).collect();
    preceding_comment_line(&refs, refs.len())
}

/// Workspace-relative, `/`-separated. `None` for files outside the workspace root.
pub(crate) fn display_path(file: &SourceFile, root: &Path) -> Option<String> {
    let FileName::Real(real) = &file.name else {
        return None;
    };
    let path = real.local_path()?;
    let relative = path.strip_prefix(root).unwrap_or(path);
    (!relative.is_absolute()).then(|| relative.to_string_lossy().replace('\\', "/"))
}
