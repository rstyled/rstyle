//! Preserve every public compiler child and ordered import-declaration route.
use super::*;
use rstyle_pdg::dependency_namespaces::capture::{Binding, Export, Route, RouteKind};
use rustc_hir::def::{DefKind, Res};
use rustc_middle::middle::resolve::{ModChild, Reexport};
use rustc_span::kw;

pub(super) struct Inventory {
    pub exports: Vec<Export>,
    pub complete: bool,
    pub unsupported: Vec<String>,
}

pub(super) fn capture(
    tcx: TyCtxt<'_>,
    module: DefId,
    budget: &mut Budget,
) -> Result<Inventory, String> {
    let mut observer = Observer {
        tcx,
        budget,
        inventory: Inventory {
            exports: Vec::new(),
            // module_children omits the separate foreign ambiguity table.
            // Exporter source closure must establish the exact footprint.
            complete: false,
            unsupported: Vec::new(),
        },
    };

    for child in tcx.module_children(module) {
        if let Some(export) = observer.child(child)? {
            observer.inventory.exports.push(export);
        }
    }
    Ok(observer.inventory)
}

struct Observer<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    budget: &'a mut Budget,
    inventory: Inventory,
}

impl Observer<'_, '_> {
    fn child(&mut self, child: &ModChild) -> Result<Option<Export>, String> {
        if !child.vis.is_public() {
            return Ok(None);
        }
        self.budget.record()?;

        let Res::Def(kind, target) = child.res else {
            self.inventory
                .unsupported
                .push(format!("Unhandled public module child: {:?}", child.res));
            return Ok(None);
        };
        self.export(child, kind, target).map(Some)
    }

    fn export(&mut self, child: &ModChild, kind: DefKind, target: DefId) -> Result<Export, String> {
        let binding = self.binding(child.ident)?;
        let route = self.route(&child.reexport_chain)?;
        let target = self.budget.definition(self.tcx, target)?;

        Ok(Export {
            binding,
            target,
            target_kind: format!("{kind:?}"),
            public: true,
            route,
        })
    }

    fn binding(&mut self, ident: rustc_span::Ident) -> Result<Binding, String> {
        if ident.name == kw::Underscore {
            return Ok(Binding::Anonymous);
        }
        let name = ident.name.to_string();
        self.budget.text(&name)?;

        Ok(Binding::Named(name))
    }

    fn route(&mut self, steps: &[Reexport]) -> Result<Vec<Route>, String> {
        let mut route = Vec::new();
        for step in steps {
            self.budget.record()?;
            let (kind, declaration) = match *step {
                Reexport::Single(id) => (RouteKind::Single, id),
                Reexport::Glob(id) => (RouteKind::Glob, id),
                Reexport::ExternCrate(id) => (RouteKind::ExternCrate, id),
                Reexport::MacroUse | Reexport::MacroExport => {
                    self.inventory
                        .unsupported
                        .push(format!("Unsupported public export route: {step:?}"));
                    continue;
                }
            };
            route.push(Route {
                kind,
                declaration: self.budget.definition(self.tcx, declaration)?,
            });
        }
        Ok(route)
    }
}
