//! Mutation events from rustc's own expression-use analysis (`ExprUseVisitor`).
//! This sees autoref'd `&mut self` method calls, so no method-name heuristics.

use rstyle_core::facts::MutationEvent;
use rustc_hir::HirId;
use rustc_hir::def_id::LocalDefId;
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor};
use rustc_lint::LateContext;
use rustc_middle::hir::place::{Place, PlaceBase, PlaceWithHirId, ProjectionKind};
use rustc_middle::ty::{self, BorrowKind, Ty, TyCtxt};

pub fn collect<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> Vec<MutationEvent> {
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
    let mut collector = Collector {
        tcx,
        events: Vec::new(),
    };
    let visitor = ExprUseVisitor::for_clippy(&cx, def_id, &mut collector);
    let _ = visitor.consume_body(tcx.hir_body(body_id));
    collector.events
}

struct Collector<'tcx> {
    tcx: TyCtxt<'tcx>,
    events: Vec<MutationEvent>,
}

impl<'tcx> Collector<'tcx> {
    fn record(&mut self, place: &PlaceWithHirId<'tcx>, at: HirId, is_helper_call: bool) {
        let Some(root) = self.root_name(&place.place) else {
            return;
        };
        let span = self.tcx.hir_span(at);
        let line = self
            .tcx
            .sess
            .source_map()
            .lookup_char_pos(span.source_callsite().lo())
            .line;
        let (field_path, escapes) = self.describe(&place.place);
        let escapes = escapes && !(is_helper_call && is_local_reborrow(&place.place));
        self.events.push(MutationEvent {
            root,
            field_path,
            line,
            escapes,
        });
    }

    /// Statics are reported as `global` effects by the expression visitor instead.
    fn root_name(&self, place: &Place<'tcx>) -> Option<String> {
        let id = match place.base {
            PlaceBase::Local(id) => id,
            PlaceBase::Upvar(upvar) => upvar.var_path.hir_id,
            PlaceBase::Rvalue | PlaceBase::StaticItem => return None,
        };
        Some(self.tcx.hir_name(id).to_string())
    }

    /// Dotted field path below the root, and whether the write escapes the
    /// function (a deref of a reference or raw pointer; `Box` derefs stay owned).
    fn describe(&self, place: &Place<'tcx>) -> (String, bool) {
        let before = std::iter::once(place.base_ty).chain(place.projections.iter().map(|p| p.ty));
        let steps: Vec<_> = before.zip(&place.projections).collect();
        let names: Vec<String> = steps
            .iter()
            .filter_map(|(ty, p)| match p.kind {
                ProjectionKind::Field(index, variant) => {
                    Some(field_name(*ty, index.as_usize(), variant.as_usize()))
                }
                _ => None,
            })
            .collect();
        let escapes = steps.iter().any(|(ty, p)| {
            p.kind == ProjectionKind::Deref && matches!(ty.kind(), ty::Ref(..) | ty::RawPtr(..))
        });
        (names.join("."), escapes)
    }
}

/// `&mut *r` of a whole crate-local object (e.g. autoref for `self.helper()`).
/// The callee's writes are attributed to the callee, so they don't count as
/// escaping side effects of the caller (they still count toward the budget).
fn is_local_reborrow(place: &Place<'_>) -> bool {
    let Some(last) = place.projections.last() else {
        return false;
    };
    last.kind == ProjectionKind::Deref
        && place.ty().ty_adt_def().is_some_and(|adt| adt.did().is_local())
}

fn field_name(ty: Ty<'_>, index: usize, variant: usize) -> String {
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

impl<'tcx> Delegate<'tcx> for Collector<'tcx> {
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
