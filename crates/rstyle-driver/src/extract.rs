//! Turns every local function body into a plain `FnFacts` record.

use crate::{effects, ids, mutations, pdg, sig};
use rstyle_core::facts::{FnFacts, ParamFacts};
use rstyle_core::lines::preceding_comment_line;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{Body, ExprKind, FnSig, PatKind};
use rustc_middle::ty::TyCtxt;
use rustc_span::{FileName, SourceFile, Span};
use std::path::Path;

pub(crate) struct Options<'a> {
    pub root: &'a Path,
    pub with_pdg: bool,
    pub complexity: Option<&'a crate::driver_complexity::Functions>,
}

/// How many lines above a function we look for an `rstyle-allow` comment.
const LOOKBACK_LINES: usize = 40;

/// 1-based line of a span's outermost macro call site, so expanded code maps to user source.
pub fn line_of(tcx: TyCtxt<'_>, span: Span) -> usize {
    tcx.sess
        .source_map()
        .lookup_char_pos(span.source_callsite().lo())
        .line
}

/// Inside a macro expansion, as opposed to a compiler desugaring of source syntax.
pub fn macro_expanded(span: Span) -> bool {
    span.from_expansion()
        && !matches!(
            span.ctxt().outer_expn_data().kind,
            rustc_span::hygiene::ExpnKind::Desugaring(_)
        )
}

/// The block rustc lowers a destructuring assignment `(a, b) = value` to:
/// `{ let (lhs, lhs) = value; a = lhs; b = lhs; }`. Nothing marks it as a desugaring.
pub fn destructuring_assignment(block: &rustc_hir::Block<'_>) -> bool {
    matches!(
        block.stmts.first().map(|stmt| &stmt.kind),
        Some(rustc_hir::StmtKind::Let(local))
            if matches!(local.source, rustc_hir::LocalSource::AssignDesugar)
    )
}

pub(crate) fn function_facts(
    tcx: TyCtxt<'_>,
    def_id: LocalDefId,
    options: &Options<'_>,
) -> Option<FnFacts> {
    let root = options.root;
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
    let path = source_path(tcx, &start.file, root)?;
    let source = source_map
        .span_to_snippet(sig.span.to(body.value.span))
        .ok()?;
    let has_self = sig.decl.implicit_self().has_implicit_self();
    let body_facts = effects::collect(tcx, def_id, body, options);
    let (mutations, mutated_types) = mutations::collect(tcx, def_id, options);

    Some(FnFacts {
        id: ids::def_id_str(tcx, def_id.to_def_id()),
        implements: implemented_trait_method(tcx, def_id),
        path,
        line: start.line,
        column: start.col.0 + 1,
        name: tcx.def_path_str(def_id.to_def_id()),
        params: sig.decl.inputs.len() - usize::from(has_self),
        param_list: param_list(tcx, sig, body, &body_facts.param_uses),
        has_self,
        statements: body_facts.statements,
        top_statements: top_statements(body),
        complexity: options.measure_complexity(sig.span, body_facts.recursions),
        source,
        preceding_line: lines_above(&start.file, start.line),
        mutations,
        calls: body_facts.calls,
        effects: body_facts.effects,
        edges: body_facts.edges,
        mutated_types,
        sig: sig::sig_facts(tcx, def_id, sig),
        pdg: options
            .with_pdg
            .then(|| pdg::pdg_of(tcx, def_id, body, root)),
    })
}

impl Options<'_> {
    fn measure_complexity(
        &self,
        span: Span,
        recursions: Vec<rstyle_core::complexity::Node>,
    ) -> Option<rstyle_core::complexity::Measurement> {
        let nodes = self.complexity?.get(&span.lo())?;
        Some(rstyle_core::complexity::Measurement::score(
            nodes.iter().chain(&recursions),
        ))
    }
}

pub(crate) fn implemented_trait_method(tcx: TyCtxt<'_>, def_id: LocalDefId) -> Option<String> {
    tcx.opt_associated_item(def_id.to_def_id())
        .and_then(|item| item.trait_item_def_id())
        .map(|trait_fn| ids::def_id_str(tcx, trait_fn))
}

/// Statements plus the tail expression directly in the body block: its "phases".
fn top_statements(body: &Body<'_>) -> usize {
    match body.value.kind {
        ExprKind::Block(block, _) => block.stmts.len() + usize::from(block.expr.is_some()),
        _ => 1,
    }
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

/// A local file's path relative to `root`, climbing out of it with `..` for a file
/// outside it, as a document records every file a unit compiles.
pub(crate) fn document_path(tcx: TyCtxt<'_>, file: &SourceFile, root: &Path) -> Option<String> {
    if let Some(path) = source_path(tcx, file, root) {
        return Some(path);
    }
    let FileName::Real(real) = &file.name else {
        return None;
    };
    let absolute = tcx
        .sess
        .source_map()
        .working_dir()
        .local_path()?
        .join(real.local_path()?);
    let normalized = rstyle_pdg::nominal_binding::absolute_path(&absolute).ok()?;
    let common = root
        .components()
        .zip(normalized.components())
        .take_while(|(left, right)| left == right)
        .count();
    let ups = root.components().count() - common;
    let rest = normalized
        .components()
        .skip(common)
        .map(|part| part.as_os_str().to_string_lossy().into_owned());
    let parts: Vec<String> = std::iter::repeat_n("..".to_owned(), ups)
        .chain(rest)
        .collect();

    Some(parts.join("/"))
}

pub(crate) fn source_path(tcx: TyCtxt<'_>, file: &SourceFile, root: &Path) -> Option<String> {
    let FileName::Real(real) = &file.name else {
        return None;
    };

    let path = real.local_path()?;
    let absolute = tcx.sess.source_map().working_dir().local_path()?.join(path);
    let normalized = rstyle_pdg::nominal_binding::absolute_path(&absolute).ok()?;
    let relative = normalized.strip_prefix(root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}
