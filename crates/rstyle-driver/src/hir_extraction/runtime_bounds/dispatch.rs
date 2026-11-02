//! Standard iterator methods that dispatch to trait impls (`Clone`, `Ord`, `Sum`,
//! `FromIterator`) are modeled only when every dispatched type is standard, since
//! workspace impls would otherwise run uncounted inside the synthetic scan. Which types
//! each method dispatches on is contract data; this module only resolves them.
use super::*;
use contracts::{Dispatch, Dispatched};
use rustc_span::sym;

const DEPTH: usize = 32;

impl<'tcx> Collector<'_, 'tcx> {
    /// Whether the call on `receiver` can only reach standard trait implementations.
    pub(super) fn standard_dispatch(
        &self,
        expression: &Expr<'tcx>,
        receiver: &Expr<'tcx>,
        dispatch: &Dispatch,
    ) -> bool {
        let arguments = self.ctx.typeck.node_args(expression.hir_id);
        let generic = |index: u32| arguments.types().nth(index as usize);
        if let Some(agnostic) = &dispatch.item_agnostic
            && generic(agnostic.generic)
                .is_some_and(|target| self.item_agnostic(target, &agnostic.types))
        {
            return true;
        }
        dispatch.standard.iter().all(|dispatched| {
            let ty = match dispatched {
                Dispatched::Item => self.item(self.ctx.typeck.expr_ty_adjusted(receiver)),
                Dispatched::Generic(index) => generic(*index),
            };
            ty.is_some_and(|ty| self.standard(ty, DEPTH))
        })
    }

    // `<iterator as Iterator>::Item`, normalized in the owner's environment.
    fn item(&self, iterator: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let tcx = self.ctx.tcx;
        let declaration = tcx.get_diagnostic_item(sym::IteratorItem)?;
        let projected = ty::Ty::new_projection_from_args(
            tcx,
            ty::IsRigid::No,
            declaration,
            tcx.mk_args(&[iterator.into()]),
        );

        let owner = self.ctx.typeck.hir_owner.def_id;
        tcx.try_normalize_erasing_regions(
            ty::TypingEnv::post_analysis(tcx, owner),
            ty::Unnormalized::new(projected),
        )
        .ok()
    }

    // Collecting into these never consults the items' own trait impls.
    fn item_agnostic(&self, target: Ty<'tcx>, types: &[String]) -> bool {
        super::sysroot::type_key(self.ctx.tcx, target).is_some_and(|key| types.contains(&key))
    }

    // Closures and function items contribute only their outputs, which flow to the consumer.
    fn standard(&self, ty: Ty<'tcx>, depth: usize) -> bool {
        let Some(depth) = depth.checked_sub(1) else {
            return false;
        };
        let tcx = self.ctx.tcx;
        match ty.kind() {
            ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => {
                true
            }
            ty::Ref(_, inner, _) | ty::Slice(inner) | ty::Array(inner, _) => {
                self.standard(*inner, depth)
            }
            ty::Tuple(items) => items.iter().all(|item| self.standard(item, depth)),
            ty::Adt(definition, arguments) => {
                standard_crate(tcx, definition.did())
                    && arguments.types().all(|item| self.standard(item, depth))
            }
            ty::Closure(_, arguments) => {
                let output = arguments.as_closure().sig().output().skip_binder();
                self.standard(output, depth)
            }
            _ => false,
        }
    }
}

fn standard_crate(tcx: ty::TyCtxt<'_>, id: rustc_hir::def_id::DefId) -> bool {
    super::sysroot::sysroot(tcx, id.krate)
}
