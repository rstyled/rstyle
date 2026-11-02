//! Standard iterator methods that dispatch to trait impls (`Clone`, `Ord`, `Sum`,
//! `FromIterator`) are modeled only when every dispatched type is standard, since
//! workspace impls would otherwise run uncounted inside the synthetic scan.
use super::*;
use rustc_span::sym;

const DEPTH: usize = 32;

impl<'tcx> Collector<'_, 'tcx> {
    /// Whether `method` on `receiver` can only reach standard trait implementations.
    pub(super) fn standard_dispatch(
        &self,
        expression: &Expr<'tcx>,
        receiver: &Expr<'tcx>,
        method: &str,
    ) -> bool {
        let arguments = self.ctx.typeck.node_args(expression.hir_id);
        let generic = |index: usize| arguments.types().nth(index);
        let item = || self.item(self.ctx.typeck.expr_ty_adjusted(receiver));
        let standard = |ty: Option<Ty<'tcx>>| ty.is_some_and(|ty| self.standard(ty, DEPTH));

        match method {
            "sum" | "product" => standard(item()) && standard(generic(1)),
            "min" | "max" => standard(item()),
            "min_by_key" | "max_by_key" => standard(generic(1)),
            "cloned" => standard(arguments.types().last()),
            "collect" => generic(1).is_some_and(|target| {
                self.item_agnostic(target) || (self.standard(target, DEPTH) && standard(item()))
            }),
            _ => true,
        }
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
    fn item_agnostic(&self, target: Ty<'tcx>) -> bool {
        let ty::Adt(definition, _) = target.kind() else {
            return false;
        };
        let tcx = self.ctx.tcx;
        [
            sym::Vec,
            rustc_span::Symbol::intern("VecDeque"),
            sym::String,
        ]
        .into_iter()
        .any(|name| tcx.is_diagnostic_item(name, definition.did()))
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
