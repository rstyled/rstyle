//! Pinned compiler bits for original standalone floating literals and constants.

use super::hir::Context;
use rstyle_pdg::PathRes;
use rustc_hir::{ConstItemRhs, Expr, ExprKind, ItemKind, Node, UnOp, def_id::DefId};
use rustc_middle::{
    mir::ConstValue,
    ty::{self, Ty},
};

const CAPABILITY: &str = "rust.float-kernel-scalars";

impl Context<'_> {
    pub(super) fn float_literal(&self, expression: &Expr<'_>) -> Option<PathRes> {
        let ExprKind::Lit(literal) = expression.kind else {
            return None;
        };
        let ty = self.typeck.expr_ty(expression);
        if !self.profile.supports(CAPABILITY)
            || !matches!(literal.node, rustc_ast::LitKind::Float(..))
            || expression.span.from_expansion()
        {
            return None;
        }

        let value = self
            .tcx
            .at(expression.span)
            .lit_to_const(ty::LitToConstInput {
                lit: literal.node,
                ty: Some(ty),
                neg: false,
            })?;
        if value.ty != ty {
            return None;
        }

        let scalar = value.try_to_leaf()?;
        let bits = format_bits(ty, scalar.to_bits(scalar.size()), scalar.size().bits())?;
        Some(PathRes {
            kind: "resolved_float_literal".into(),
            target: None,
            description: serde_json::json!({"ty": ty.to_string(), "bits": bits}).to_string(),
        })
    }

    pub(super) fn float_constant_value(
        &self,
        definition: DefId,
        ty: Ty<'_>,
        value: ConstValue,
    ) -> Option<String> {
        if !self.profile.supports(CAPABILITY)
            || self.tcx.parent(definition)
                != self.tcx.parent(self.typeck.hir_owner.def_id.to_def_id())
        {
            return None;
        }

        let Node::Item(item) = self.tcx.hir_node_by_def_id(definition.expect_local()) else {
            return None;
        };
        let ItemKind::Const(_, _, _, ConstItemRhs::Body(body)) = item.kind else {
            return None;
        };

        let mut expression = self.tcx.hir_body(body).value;
        if let ExprKind::Unary(UnOp::Neg, operand) = expression.kind {
            expression = operand;
        }
        if expression.span.from_expansion()
            || !matches!(expression.kind, ExprKind::Lit(literal) if matches!(literal.node, rustc_ast::LitKind::Float(..)))
        {
            return None;
        }

        let scalar = value.try_to_scalar_int()?;
        format_bits(ty, scalar.to_bits(scalar.size()), scalar.size().bits())
    }
}

fn format_bits(ty: Ty<'_>, bits: u128, width: u64) -> Option<String> {
    match ty.kind() {
        ty::Float(ty::FloatTy::F32) if width == 32 => Some(format!("0x{bits:08x}")),
        ty::Float(ty::FloatTy::F64) if width == 64 => Some(format!("0x{bits:016x}")),
        _ => None,
    }
}
