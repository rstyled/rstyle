//! Complete subtrees for the bounded immediate Option callback contract.
//! Ordinary escaping and unrelated closure values retain their unsupported
//! source boundary; eligibility and graph checking remain separate stages.

use super::hir::Context;
use rstyle_pdg::frontend::HirNode;
use rustc_hir::{self as hir, Expr, ExprKind};
use rustc_middle::ty;

impl<'tcx> Context<'tcx> {
    pub(super) fn callback_children(&self, expr: &Expr<'tcx>) -> Option<Vec<HirNode>> {
        if !self.profile.supports("rust.immediate-option-callbacks")
            || !self.immediate_option_callback(expr)
        {
            return None;
        }
        let ExprKind::Closure(closure) = expr.kind else {
            return None;
        };
        if !matches!(closure.kind, hir::ClosureKind::Closure) {
            return None;
        }
        let ty::Closure(definition, _) = self.typeck.expr_ty(expr).kind() else {
            return None;
        };
        if *definition != closure.def_id.to_def_id() {
            return None;
        }

        let body = self.tcx.hir_body(closure.body);
        Some(
            body.params
                .iter()
                // These bindings belong to the callback, not the outer
                // function interface. In particular, leave param_index unset.
                .map(|parameter| self.callback_pattern(parameter.pat))
                .chain(std::iter::once(self.expression(body.value)))
                .collect(),
        )
    }

    fn callback_pattern(&self, pattern: &hir::Pat<'tcx>) -> HirNode {
        let mut node = self.pattern(pattern);
        if self.profile.supports("rust.byte-iterator-position")
            && let hir::PatKind::Ref(inner, hir::Pinnedness::Not, hir::Mutability::Not) =
                pattern.kind
        {
            node.kind = "pat_shared_copy".into();
            node.children = vec![self.pattern(inner)];
        }
        node
    }

    fn immediate_option_callback(&self, expression: &Expr<'tcx>) -> bool {
        let hir::Node::Expr(parent) = self.tcx.parent_hir_node(expression.hir_id) else {
            return false;
        };
        let ExprKind::MethodCall(_, _, arguments, _) = parent.kind else {
            return false;
        };
        if arguments
            .last()
            .is_none_or(|argument| argument.hir_id != expression.hir_id)
        {
            return false;
        }
        let Some(definition) = self.typeck.type_dependent_def_id(parent.hir_id) else {
            return false;
        };

        let path = self.tcx.def_path_str(definition);
        let option = arguments.len() == 2
            && matches!(
                path.as_str(),
                "core::option::Option::<T>::map_or" | "std::option::Option::<T>::map_or"
            );
        let position = self.profile.supports("rust.byte-iterator-position")
            && arguments.len() == 1
            && matches!(
                path.as_str(),
                "core::iter::Iterator::position" | "std::iter::Iterator::position"
            );
        !definition.is_local()
            && self.tcx.crate_name(definition.krate).as_str() == "core"
            && (option || position)
    }
}
