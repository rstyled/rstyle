//! Complete subtrees for the bounded immediate Option callback contract.
//! Ordinary escaping and unrelated closure values retain their unsupported
//! source boundary; eligibility and graph checking remain separate stages.

use super::hir::Context;
use rstyle_pdg::frontend::HirNode;
use rustc_hir::{self as hir, Expr, ExprKind};
use rustc_middle::ty;

impl<'tcx> Context<'tcx> {
    pub(super) fn callback_children(&self, expr: &Expr<'tcx>) -> Option<Vec<HirNode>> {
        if self.usize_reader_callback(expr) {
            return self.usize_reader_children(expr);
        }

        if self.profile.supports(rstyle_pdg::id_error_maps::CAPABILITY)
            && self.immediate_error_map(expr)
        {
            return self.closure_children(expr, |pattern| self.pattern(pattern));
        }

        if !self.profile.supports("rust.immediate-option-callbacks")
            || !self.immediate_option_callback(expr)
        {
            return None;
        }

        self.closure_children(expr, |pattern| self.callback_pattern(pattern))
    }

    pub(super) fn closure_children(
        &self,
        expr: &Expr<'tcx>,
        pattern: impl Fn(&hir::Pat<'tcx>) -> HirNode,
    ) -> Option<Vec<HirNode>> {
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
                .map(|parameter| pattern(parameter.pat))
                .chain(std::iter::once(self.expression(body.value)))
                .collect(),
        )
    }

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

    pub(super) fn named_option_function(
        &self,
        expr: &Expr<'tcx>,
    ) -> Option<(rstyle_pdg::PathRes, rstyle_pdg::Dispatch)> {
        let ty::FnDef(definition, arguments) = self.typeck.expr_ty(expr).kind() else {
            return None;
        };
        if !self.profile.supports("rust.named-option-callbacks")
            || !definition.is_local()
            || !arguments.skip_binder().is_empty()
            || self.tcx.generics_of(*definition).count() != 0
            || self.tcx.def_kind(*definition) != rustc_hir::def::DefKind::Fn
            || !matches!(expr.kind, ExprKind::Path(_))
            || expr.span.from_expansion()
            || !self.typeck.expr_adjustments(expr).is_empty()
            || !self.immediate_option_callback(expr)
        {
            return None;
        }
        let hir::Node::Expr(parent) = self.tcx.parent_hir_node(expr.hir_id) else {
            return None;
        };
        if !matches!(parent.kind, ExprKind::MethodCall(_, _, arguments, _) if arguments.len() == 2)
        {
            return None;
        }
        let signature = self
            .tcx
            .fn_sig(*definition)
            .instantiate_identity()
            .skip_binder();
        if signature.inputs() != [self.tcx.types.u8] || signature.output() != self.tcx.types.bool {
            return None;
        }

        let resolution = self.helper_callee(*definition, "resolved_function_item", None);
        let dispatch = rstyle_pdg::Dispatch {
            kind: "resolved_function_item".into(),
            op: self.tcx.def_path_str(*definition),
            operand_tys: vec!["u8".into()],
            result_ty: "bool".into(),
        };
        Some((resolution, dispatch))
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
