//! Evaluate only referenced local scalar constants; rustc owns const execution.

use super::hir::Context;
use rustc_hir::{def::DefKind, def_id::DefId};

impl Context<'_> {
    pub(super) fn local_constant(&self, kind: DefKind, definition: DefId) -> bool {
        matches!(kind, DefKind::Const | DefKind::AssocConst)
            && definition.is_local()
            && self.tcx.generics_of(definition).count() == 0
            && match kind {
                DefKind::Const => true,
                DefKind::AssocConst => matches!(
                    self.tcx.def_kind(self.tcx.parent(definition)),
                    DefKind::Impl { of_trait: false },
                ),
                _ => false,
            }
    }
}
