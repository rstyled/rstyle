//! Bounded compiler lens; erased regions provide no lifetime authority.
use super::super::{diagnostic_support::ArgumentsProjection, types};
use rustc_hir::{
    HirId,
    def::DefKind,
    def_id::{DefId, LocalDefId},
};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt, TypeckResults};

pub(in crate::hir_extraction::instances) struct Projection<'tcx> {
    owner: LocalDefId,
    arguments: GenericArgsRef<'tcx>,
}

impl<'tcx> Projection<'tcx> {
    pub(in crate::hir_extraction::instances) fn new(
        tcx: TyCtxt<'tcx>,
        owner: LocalDefId,
        haystack: &str,
    ) -> Result<Self, String> {
        let generics = tcx.generics_of(owner);
        if tcx.def_kind(owner) != DefKind::AssocFn
            || generics.count() != 2
            || !matches!(
                generics.param_at(0, tcx).kind,
                ty::GenericParamDefKind::Lifetime
            )
            || !matches!(
                generics.param_at(1, tcx).kind,
                ty::GenericParamDefKind::Type {
                    has_default: false,
                    synthetic: false
                }
            )
        {
            return Err(
                "Reference projection requires an associated owner with lifetime0/type1 parameters"
                    .into(),
            );
        }

        let element = match haystack {
            "[u8]" => Ty::new_slice(tcx, tcx.types.u8),
            "str" => tcx.types.str_,
            _ => return Err("Reference projection haystack is outside [u8]/str".into()),
        };
        let arguments = tcx.mk_args(&[tcx.lifetimes.re_erased.into(), element.into()]);
        Ok(Self { owner, arguments })
    }

    pub(in crate::hir_extraction::instances) fn arguments(&self) -> GenericArgsRef<'tcx> {
        self.arguments
    }

    pub(in crate::hir_extraction::instances) fn ty(
        &self,
        tcx: TyCtxt<'tcx>,
        original: Ty<'tcx>,
    ) -> Result<Ty<'tcx>, String> {
        let projected = self.normalize(tcx, original)?;
        let observed = types::arguments(tcx, tcx.mk_args(&[projected.into()]));
        if !observed.complete() {
            return Err("Projected reference type lacks bounded structural identity".into());
        }
        Ok(projected)
    }

    pub(in crate::hir_extraction::instances) fn node_ty(
        &self,
        tcx: TyCtxt<'tcx>,
        typeck: &TypeckResults<'tcx>,
        node: HirId,
    ) -> Result<Ty<'tcx>, String> {
        super::super::projection::project_node(self.owner, typeck, node, |original| {
            self.ty(tcx, original)
        })
    }

    pub(in crate::hir_extraction::instances) fn select(
        &self,
        tcx: TyCtxt<'tcx>,
        definition: DefId,
        original: GenericArgsRef<'tcx>,
    ) -> Result<ty::Instance<'tcx>, String> {
        let args = self.args(tcx, original)?;
        let selected =
            ty::Instance::try_resolve(tcx, ty::TypingEnv::fully_monomorphized(), definition, args)
                .map_err(|_| "Reference instance compiler selection failed")?
                .ok_or("Reference instance compiler selection is unresolved")?;
        if !matches!(selected.def, ty::InstanceKind::Item(_)) {
            return Err("Reference selected instance is not an ordinary Item".into());
        }

        if !types::arguments(tcx, selected.args).complete() {
            return Err("Reference selected arguments lack structural identity".into());
        }
        Ok(selected)
    }

    fn normalize<T: ty::TypeFoldable<TyCtxt<'tcx>>>(
        &self,
        tcx: TyCtxt<'tcx>,
        original: T,
    ) -> Result<T, String> {
        super::super::projection::normalize(tcx, self.arguments, original).map_err(|error| {
            if matches!(error, super::super::projection::Error::Unresolved) {
                "Reference substitution remains generic, inferred or bound".into()
            } else {
                "Reference substitution normalization failed".into()
            }
        })
    }
}

impl<'tcx> ArgumentsProjection<'tcx> for Projection<'tcx> {
    fn args(
        &self,
        tcx: TyCtxt<'tcx>,
        original: GenericArgsRef<'tcx>,
    ) -> Result<GenericArgsRef<'tcx>, String> {
        Ok(
            super::super::projection::complete_arguments(tcx, self.normalize(tcx, original))?
                .arguments,
        )
    }
}
