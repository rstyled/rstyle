//! Local cfg_if module ownership references existing immutable census buffers.

use super::sources::Sources;
use rstyle_pdg::nominal_boundary::census::expansions::{ExpansionStep, FileSpan, ModuleExpansion};
use rustc_hir::{ItemKind, def_id::DefId};
use rustc_middle::ty::TyCtxt;
use rustc_span::{
    ExpnId, FileName, Span,
    hygiene::{ExpnKind, MacroKind},
};
use std::path::PathBuf;

pub(super) fn capture(tcx: TyCtxt<'_>, sources: &Sources) -> Result<Vec<ModuleExpansion>, String> {
    let mut records = Vec::new();
    for id in tcx.hir_free_items() {
        let item = tcx.hir_item(id);
        let ItemKind::Mod(_, module) = item.kind else {
            continue;
        };
        let definition = item.owner_id.to_def_id();
        let expansion = tcx.expn_that_defined(definition);
        if expansion == ExpnId::root() {
            continue;
        }
        let Some(chain) = expansion_chain(tcx, sources, expansion)? else {
            continue;
        };

        let declaration = file_span(tcx, sources, item.span)?;
        if !emitted_declaration(&declaration, &chain) {
            continue;
        }
        let loaded = file_span(tcx, sources, module.spans.inner_span)?;
        if declaration.path == loaded.path {
            continue;
        }

        records.push(ModuleExpansion {
            module: tcx.def_path_str(definition),
            parent: tcx.def_path_str(tcx.parent(definition)),
            declaration,
            loaded_file: loaded.path,
            chain,
        });
    }

    records.sort_by(|left, right| left.module.cmp(&right.module));
    Ok(records)
}

fn expansion_chain(
    tcx: TyCtxt<'_>,
    sources: &Sources,
    mut expansion: ExpnId,
) -> Result<Option<Vec<ExpansionStep>>, String> {
    let mut chain = Vec::new();
    while expansion != ExpnId::root() {
        if chain.len() >= 128 {
            return Err("Module expansion chain exceeds capture budget".into());
        }

        let data = expansion.expn_data();
        if !matches!(data.kind, ExpnKind::Macro(MacroKind::Bang, _)) {
            return Ok(None);
        }
        let Some(definition) = data.macro_def_id.filter(|id| local_cfg_if(tcx, *id)) else {
            return Ok(None);
        };

        chain.push(ExpansionStep {
            macro_definition: tcx.def_path_str(definition),
            definition: file_span(tcx, sources, tcx.def_span(definition))?,
            invocation: file_span(tcx, sources, data.call_site)?,
        });
        expansion = data.parent;
    }

    Ok(Some(chain))
}

fn local_cfg_if(tcx: TyCtxt<'_>, definition: DefId) -> bool {
    definition.is_local() && tcx.item_name(definition).as_str() == "cfg_if"
}

pub(super) fn file_span(
    tcx: TyCtxt<'_>,
    sources: &Sources,
    span: Span,
) -> Result<FileSpan, String> {
    let file = tcx.sess.source_map().lookup_source_file(span.lo());
    if file.cnum != rustc_hir::def_id::LOCAL_CRATE {
        return Err("Module expansion references a foreign source buffer".into());
    }

    let FileName::Real(real) = &file.name else {
        return Err("Module expansion source has no original local path".into());
    };
    let path = real
        .local_path()
        .ok_or("Module expansion source path is unavailable")?;
    let snapshot = sources
        .file(&file)
        .ok_or("Module expansion source buffer is unavailable")?;
    let (_, span) = sources
        .range(tcx, span)
        .ok_or("Module expansion span leaves original buffer")?;

    Ok(FileSpan {
        path: absolute(path)?,
        digest: snapshot.identity().into(),
        span,
    })
}

fn absolute(path: &std::path::Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.into());
    }
    Ok(std::env::current_dir()
        .map_err(|error| error.to_string())?
        .join(path))
}

fn emitted_declaration(declaration: &FileSpan, chain: &[ExpansionStep]) -> bool {
    chain.last().is_some_and(|outer| {
        declaration.path == outer.invocation.path
            && declaration.digest == outer.invocation.digest
            && outer.invocation.span.start <= declaration.span.start
            && declaration.span.end <= outer.invocation.span.end
    })
}
