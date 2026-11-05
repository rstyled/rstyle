//! Complete subtrees for the bounded immediate Option callback contract.
//! Ordinary escaping and unrelated closure values retain their unsupported
//! source boundary; eligibility and graph checking remain separate stages.

use super::hir::Context;
use rustc_hir::{self as hir, Expr, ExprKind};

impl<'tcx> Context<'tcx> {
    pub(super) fn immediate_core_callback(&self, expression: &Expr<'tcx>, paths: &[&str]) -> bool {
        let hir::Node::Expr(parent) = self.tcx.parent_hir_node(expression.hir_id) else {
            return false;
        };
        let ExprKind::MethodCall(_, _, arguments, _) = parent.kind else {
            return false;
        };
        if !matches!(arguments, [argument] if argument.hir_id == expression.hir_id) {
            return false;
        }
        let Some(definition) = self.typeck.type_dependent_def_id(parent.hir_id) else {
            return false;
        };

        !definition.is_local()
            && self.tcx.crate_name(definition.krate).as_str() == "core"
            && paths.contains(&self.tcx.def_path_str(definition).as_str())
    }
}
