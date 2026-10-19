//! Collapse only compiler-resolved standard assertion expansions.

use super::hir::{Context, resolved};
use rstyle_pdg::{ExpansionSite, HirNode, PathRes, assertions::Assertion};
use rustc_hir::{Expr, ExprKind, StmtKind, UnOp};

impl<'tcx> Context<'tcx> {
    pub(super) fn assertion(&self, expression: &Expr<'tcx>) -> Option<HirNode> {
        if !self.profile.supports("rust.scalar-assertions") || !expression.span.from_expansion() {
            return None;
        }
        let expansion = expression.span.ctxt().outer_expn_data();
        let definition = expansion.macro_def_id?;
        if definition.is_local() || self.tcx.crate_name(definition.krate).as_str() != "core" {
            return None;
        }

        let source = self.snippet(expression.span);
        let assertion = Assertion::parse(&source).ok()?;
        if self.tcx.item_name(definition).as_str() != assertion.name {
            return None;
        }

        let active = assertion.name == "assert" || self.tcx.sess.opts.debug_assertions;
        let condition = if assertion.name == "debug_assert" {
            debug_condition(expression, active)?
        } else {
            assert_condition(expression)?
        };
        let condition = self.expression(condition);
        let node = self.node(expression.hir_id, expression.span, "expr_assert");

        Some(HirNode {
            ty: Some(resolved(self.typeck.expr_ty(expression), String::new())),
            res: Some(PathRes {
                kind: "standard_assertion".into(),
                target: None,
                description: format!("core::{}", assertion.name),
            }),
            lit: Some(active.to_string()),
            expansion: Some(ExpansionSite {
                callsite: node.span,
                snippet: source,
            }),
            children: vec![condition],
            ..node
        })
    }
}

fn assert_condition<'a, 'tcx>(expression: &'a Expr<'tcx>) -> Option<&'a Expr<'tcx>> {
    let ExprKind::If(test, _, None) = expression.kind else {
        return None;
    };
    let ExprKind::Unary(UnOp::Not, condition) = test.kind else {
        return None;
    };
    Some(condition)
}

fn debug_condition<'a, 'tcx>(expression: &'a Expr<'tcx>, active: bool) -> Option<&'a Expr<'tcx>> {
    let ExprKind::If(test, body, None) = expression.kind else {
        return None;
    };
    let ExprKind::Lit(literal) = test.kind else {
        return None;
    };
    if literal.node != rustc_ast::LitKind::Bool(active) {
        return None;
    }

    let ExprKind::Block(block, _) = body.kind else {
        return None;
    };
    let [statement] = block.stmts else {
        return None;
    };
    let StmtKind::Semi(inner) = statement.kind else {
        return None;
    };
    assert_condition(inner)
}
