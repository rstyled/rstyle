//! Preserve complete immediate map_err callbacks and actual immutable capture storage.
mod environment;

use super::hir::{Context, hir_id, reference};
use rstyle_pdg::{HirNode, concrete_types::Observation, id_error_maps as contract};
use rustc_hir::{self as hir, Expr, ExprKind};
use rustc_middle::ty::{self, Ty};

impl<'tcx> Context<'tcx> {
    pub(super) fn immediate_error_map(&self, expression: &Expr<'tcx>) -> bool {
        self.immediate_core_callback(expression, &["core::result::Result::<T, E>::map_err"])
    }

    pub(super) fn capture_id_error_maps(
        &self,
        node: &mut HirNode,
        observe: &mut impl FnMut(Ty<'tcx>) -> Result<Observation, String>,
    ) -> Result<(), String> {
        if !self.profile.supports(contract::CAPABILITY) {
            return Ok(());
        }

        if node.kind == "expr_closure" && !node.children.is_empty() {
            let id = hir_id(&node.hir);
            if let hir::Node::Expr(expression) = self.tcx.hir_node(id)
                && self.immediate_error_map(expression)
            {
                node.res = Some(self.id_error_map_environment(expression, observe)?);
            }
        }
        node.children
            .iter_mut()
            .try_for_each(|child| self.capture_id_error_maps(child, observe))
    }
}
