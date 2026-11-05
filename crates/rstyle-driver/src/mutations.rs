//! Writes and exclusive borrows from rustc's own expression-use analysis
//! (`ExprUseVisitor`), which sees autoref'd `&mut self` method calls.

use rustc_hir::HirId;
use rustc_hir::def_id::LocalDefId;
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor};
use rustc_lint::LateContext;
use rustc_middle::hir::place::{Place, PlaceBase, PlaceWithHirId, ProjectionKind};
use rustc_middle::ty::{self, BorrowKind, Ty, TyCtxt};

/// A write to, or exclusive borrow of, a place rooted at a binding, where rustc reports it.
pub struct Use<'tcx> {
    pub place: Place<'tcx>,
    pub root: HirId,
    pub at: HirId,
    pub borrow: bool,
}

/// Writes and exclusive borrows from rustc's expression-use visitor, in its order.
/// Statics are reported as `global` effects by the expression visitor instead.
pub fn uses<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> Vec<Use<'tcx>> {
    let body_id = tcx
        .hir_node_by_def_id(def_id)
        .body_id()
        .expect("function has a body");
    let cx = LateContext {
        tcx,
        enclosing_body: Some(body_id),
        typeck_results: Some(tcx.typeck_body(body_id)),
        param_env: tcx.param_env(def_id),
        effective_visibilities: tcx.effective_visibilities(()),
        last_node_with_lint_attrs: tcx.local_def_id_to_hir_id(def_id),
        generics: None,
        only_module: false,
    };

    let mut uses = Uses(Vec::new());
    let visitor = ExprUseVisitor::for_clippy(&cx, def_id, &mut uses);
    let _ = visitor.consume_body(tcx.hir_body(body_id));
    uses.0
}

/// A deref of a reference or raw pointer below the root; `Box` derefs stay owned.
pub fn escapes(place: &Place<'_>) -> bool {
    let before = std::iter::once(place.base_ty).chain(place.projections.iter().map(|p| p.ty));
    before.zip(&place.projections).any(|(ty, p)| {
        p.kind == ProjectionKind::Deref && matches!(ty.kind(), ty::Ref(..) | ty::RawPtr(..))
    })
}

pub fn field_name(ty: Ty<'_>, index: usize, variant: usize) -> String {
    match ty.kind() {
        ty::Adt(def, _) => {
            let variant = &def.variants()[rustc_abi::VariantIdx::from_usize(variant)];
            variant.fields[rustc_abi::FieldIdx::from_usize(index)]
                .name
                .to_string()
        }
        _ => index.to_string(),
    }
}

struct Uses<'tcx>(Vec<Use<'tcx>>);

impl<'tcx> Uses<'tcx> {
    fn record(&mut self, place: &PlaceWithHirId<'tcx>, at: HirId, borrow: bool) {
        let root = match place.place.base {
            PlaceBase::Local(id) => id,
            PlaceBase::Upvar(upvar) => upvar.var_path.hir_id,
            PlaceBase::Rvalue | PlaceBase::StaticItem => return,
        };
        self.0.push(Use {
            place: place.place.clone(),
            root,
            at,
            borrow,
        });
    }
}

impl<'tcx> Delegate<'tcx> for Uses<'tcx> {
    fn consume(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn use_cloned(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn borrow(&mut self, place: &PlaceWithHirId<'tcx>, at: HirId, kind: BorrowKind) {
        if kind == BorrowKind::Mutable {
            self.record(place, at, true);
        }
    }

    fn mutate(&mut self, place: &PlaceWithHirId<'tcx>, at: HirId) {
        self.record(place, at, false);
    }

    /// `let x = init;` is an initialisation, not a state change (the default forwards to `mutate`).
    fn bind(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn fake_read(
        &mut self,
        _: &PlaceWithHirId<'tcx>,
        _: rustc_middle::mir::FakeReadCause,
        _: HirId,
    ) {
    }
}
