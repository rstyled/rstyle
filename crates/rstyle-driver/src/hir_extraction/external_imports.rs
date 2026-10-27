//! Observe the exact complete macro namespace of original alloc imports.
use super::{library_resolution::definition, module_expansions::file_span, sources::Sources};
use rstyle_pdg::nominal_boundary::census::external_imports::{Exports, Import, Macro};
use rustc_hir::{
    ItemKind,
    def::{DefKind, MacroKinds, Res},
    def_id::DefId,
};
use rustc_middle::ty::TyCtxt;
use rustc_span::ExpnId;

pub(super) fn capture(tcx: TyCtxt<'_>, sources: &Sources) -> Result<Vec<Import>, String> {
    let mut imports = Vec::new();
    for id in tcx.hir_free_items() {
        let item = tcx.hir_item(id);
        let ItemKind::ExternCrate(None, ident) = item.kind else {
            continue;
        };
        if ident.name.as_str() != "alloc"
            || item.span.from_expansion()
            || tcx.expn_that_defined(item.owner_id.to_def_id()) != ExpnId::root()
            || !tcx.hir_attrs(item.hir_id()).iter().any(|attribute| {
                matches!(
                    attribute,
                    rustc_attr_ir::Attribute::Parsed(rustc_attr_ir::AttributeKind::MacroUse { .. })
                )
            })
        {
            continue;
        }
        let krate = tcx
            .extern_mod_stmt_cnum(item.owner_id.def_id)
            .ok_or("Missing compiler-resolved external macro crate")?;
        imports.push(Import {
            declaration: file_span(tcx, sources, item.span)?,
            exports: exports(tcx, krate.as_def_id())?,
        });
    }
    imports.sort_by(|a, b| {
        (&a.declaration.path, a.declaration.span.start)
            .cmp(&(&b.declaration.path, b.declaration.span.start))
    });
    Ok(imports)
}

fn exports(tcx: TyCtxt<'_>, root: DefId) -> Result<Exports, String> {
    let mut macros = Vec::new();
    for child in tcx.module_children(root) {
        let Res::Def(DefKind::Macro(kind), target) = child.res else {
            continue;
        };
        if macros.len() >= rstyle_pdg::nominal_boundary::census::MAX_FILES {
            return Err("External macro inventory exceeds capture budget".into());
        }
        macros.push(Macro {
            name: child.ident.name.to_string(),
            kind: if kind == MacroKinds::BANG {
                "bang".into()
            } else {
                format!("{kind:?}")
            },
            public: child.vis.is_public(),
            definition: definition(tcx, target),
        });
    }
    macros.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Exports {
        library: definition(tcx, root),
        macros,
    })
}
