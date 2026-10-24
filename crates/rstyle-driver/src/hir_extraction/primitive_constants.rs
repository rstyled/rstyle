//! Concrete primitive constants have pinned authority, separate from local items.
use super::hir::Context;
use rstyle_pdg::{PathRes, primitive_constants::PrimitiveConstant};
use rustc_hir::{
    def::{DefKind, Res},
    def_id::DefId,
};
use rustc_middle::ty;

impl Context<'_> {
    pub(super) fn primitive_constant_resolution(&self, resolution: Res) -> Option<PathRes> {
        if !self
            .profile
            .supports(rstyle_pdg::primitive_constants::CAPABILITY)
        {
            return None;
        }

        let Res::Def(DefKind::AssocConst, definition) = resolution else {
            return None;
        };
        if !self.primitive_u32_max(definition) {
            return None;
        }

        let value = self
            .tcx
            .const_eval_poly(definition)
            .ok()?
            .try_to_scalar_int()?;
        let definition = self.library_callee(definition);
        let constant = PrimitiveConstant {
            definition: serde_json::from_str(&definition.description).ok()?,
            primitive: "u32".into(),
            member: "MAX".into(),
            value: value.to_bits(value.size()).to_string(),
        };

        Some(PathRes {
            kind: rstyle_pdg::primitive_constants::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&constant).ok()?,
        })
    }

    fn primitive_u32_max(&self, definition: DefId) -> bool {
        if definition.is_local()
            || self.tcx.crate_name(definition.krate).as_str() != "core"
            || self.tcx.item_name(definition).as_str() != "MAX"
            || self.tcx.generics_of(definition).count() != 0
        {
            return false;
        }

        let owner = self.tcx.parent(definition);
        if !matches!(self.tcx.def_kind(owner), DefKind::Impl { of_trait: false }) {
            return false;
        }

        let environment = ty::TypingEnv::post_analysis(self.tcx, definition);
        [owner, definition].into_iter().all(|item| {
            self.tcx
                .try_normalize_erasing_regions(
                    environment,
                    self.tcx.type_of(item).instantiate_identity(),
                )
                .is_ok_and(|ty| ty == self.tcx.types.u32)
        })
    }
}
