//! Observations of local tuple-variant patterns, without admission permission.
use super::hir::{Context, resolved};
use rstyle_pdg::{extraction_profile::ExtractionProfile, frontend::*};
use rustc_hir::{
    self as hir,
    def::{DefKind, Res},
};
use rustc_middle::ty;

impl<'tcx> Context<'tcx> {
    pub(super) fn observed_pattern(&self, pattern: &hir::Pat<'tcx>) -> Option<HirNode> {
        self.option_pattern(pattern)
            .or_else(|| self.local_variant_pattern(pattern))
            .or_else(|| self.slice_prefix_pattern(pattern))
    }

    fn local_variant_pattern(&self, pattern: &hir::Pat<'tcx>) -> Option<HirNode> {
        if self.profile != ExtractionProfile::DataV3 {
            return None;
        }
        let hir::PatKind::TupleStruct(ref path, children, rest) = pattern.kind else {
            return None;
        };
        if rest.as_opt_usize().is_some() {
            return None;
        }
        let value = self.typeck.node_type_opt(pattern.hir_id)?;
        let ty::Adt(owner, _) = value.kind() else {
            return None;
        };
        if !owner.is_enum() || !owner.did().is_local() {
            return None;
        }
        let Res::Def(DefKind::Ctor(..), constructor) = self.typeck.qpath_res(path, pattern.hir_id)
        else {
            return None;
        };
        let variant = self.tcx.parent(constructor);
        if self.tcx.parent(variant) != owner.did() {
            return None;
        }
        let mut resolution = self.library_callee(constructor);
        resolution.kind = "resolved_local_variant".into();
        let mut node = self.node(pattern.hir_id, pattern.span, "pat_local_variant");
        node.ty = Some(resolved(value, String::new()));
        node.res = Some(resolution);
        node.children = children.iter().map(|child| self.pattern(child)).collect();

        Some(node)
    }
}
