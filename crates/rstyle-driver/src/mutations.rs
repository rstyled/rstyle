//! Mutation events from rustc's own expression-use analysis (`ExprUseVisitor`).
//! This sees autoref'd `&mut self` method calls, so no method-name heuristics.

use rstyle_core::facts::{MutationEvent, MutationOrigin};
use rustc_hir::HirId;
use rustc_hir::def_id::LocalDefId;
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor};
use rustc_lint::LateContext;
use rustc_middle::hir::place::{Place, PlaceBase, PlaceWithHirId, ProjectionKind};
use rustc_middle::ty::{self, BorrowKind, Ty, TyCtxt};
use rustc_span::DesugaringKind;
use std::collections::BTreeSet;
use std::path::Path;

use crate::extract::{Options, line_of};
use crate::ids;

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

/// Mutation events plus the sorted ids of workspace types written through.
pub fn collect<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
    options: &Options<'_>,
) -> (Vec<MutationEvent>, Vec<String>) {
    let mut collector = Collector {
        tcx,
        root: options.root,
        events: Vec::new(),
        types: BTreeSet::new(),
    };
    for place_use in uses(tcx, def_id) {
        collector.record(&place_use);
    }

    (collector.events, collector.types.into_iter().collect())
}

struct Collector<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    root: &'a Path,
    events: Vec<MutationEvent>,
    types: BTreeSet<String>,
}

impl<'tcx> Collector<'_, 'tcx> {
    fn record(&mut self, place_use: &Use<'tcx>) {
        let Use {
            place,
            root,
            at,
            borrow: is_helper_call,
        } = place_use;
        let (root, at, is_helper_call) = (*root, *at, *is_helper_call);
        let (field_path, escapes) = describe(place);
        // A binding a library macro declares is that macro's working state; a write
        // through it that escapes is still an effect.
        if !escapes && self.library_macro(self.tcx.hir_span(root)) {
            return;
        }

        self.note_type(place.base_ty);
        let origin = if self.tcx.hir_span(root).desugaring_kind() == Some(DesugaringKind::ForLoop) {
            MutationOrigin::ForDesugaring
        } else {
            MutationOrigin::Source
        };
        let root = self.tcx.hir_name(root).to_string();
        let span = self.tcx.hir_span(at);
        let line = line_of(self.tcx, span);
        let delegated = escapes && is_helper_call && self.is_workspace_reborrow(place);
        let escapes = escapes && !delegated;
        self.events.push(MutationEvent {
            origin,
            root,
            field_path,
            line,
            escapes,
            delegated,
        });
    }

    /// Inside the expansion of a macro defined outside the workspace, such as `json!`.
    fn library_macro(&self, span: rustc_span::Span) -> bool {
        crate::extract::macro_expanded(span)
            && span
                .ctxt()
                .outer_expn_data()
                .macro_def_id
                .is_some_and(|def_id| !ids::in_workspace(self.tcx, def_id, self.root))
    }

    /// `&mut *r` of a whole workspace object (e.g. autoref for `self.helper()`).
    /// The callee's writes are attributed to the callee (and inherited through
    /// the call graph), so this is not an escaping write of the caller itself.
    fn is_workspace_reborrow(&self, place: &Place<'tcx>) -> bool {
        place
            .projections
            .last()
            .is_some_and(|last| last.kind == ProjectionKind::Deref)
            && place
                .ty()
                .ty_adt_def()
                .is_some_and(|adt| ids::in_workspace(self.tcx, adt.did(), self.root))
    }

    /// Remembers the workspace type behind the written root (`&mut T`, `Box<T>` peeled).
    fn note_type(&mut self, base: Ty<'tcx>) {
        let peeled = base.peel_refs();
        let peeled = peeled.boxed_ty().unwrap_or(peeled);
        if let Some(adt) = peeled.ty_adt_def()
            && ids::in_workspace(self.tcx, adt.did(), self.root)
        {
            self.types.insert(ids::def_id_str(self.tcx, adt.did()));
        }
    }
}

/// Dotted field path below the root, and whether the write escapes the function.
fn describe(place: &Place<'_>) -> (String, bool) {
    let before = std::iter::once(place.base_ty).chain(place.projections.iter().map(|p| p.ty));
    let names: Vec<String> = before
        .zip(&place.projections)
        .filter_map(|(ty, p)| match p.kind {
            ProjectionKind::Field(index, variant) => {
                Some(field_name(ty, index.as_usize(), variant.as_usize()))
            }
            _ => None,
        })
        .collect();
    (names.join("."), escapes(place))
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
