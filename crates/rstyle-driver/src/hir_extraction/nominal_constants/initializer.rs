//! The const owner's actual selected call, typed subtraction and core dependency.
use super::{CaptureFailure, Context, identity};
use rstyle_pdg::nominal_constants::{CoreConstant, Initializer, ScalarValue};
use rustc_hir::{
    Expr, ExprKind,
    def::{DefKind, Res},
    def_id::DefId,
};
use rustc_middle::ty::Ty;
use rustc_span::Symbol;

pub(super) fn capture<'tcx>(
    context: &Context<'tcx>,
    definition: DefId,
    owner: Ty<'tcx>,
) -> Result<(Initializer, DefId), CaptureFailure> {
    let body = context
        .tcx
        .hir_maybe_body_owned_by(definition.expect_local())
        .ok_or(CaptureFailure::Unsupported(
            "constant initializer body is unavailable",
        ))?;
    let ExprKind::Call(callee, [argument]) = body.value.kind else {
        return Err(CaptureFailure::Unsupported(
            "constant initializer is not one constructor call",
        ));
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return Err(CaptureFailure::Unsupported(
            "constant constructor lacks an original path",
        ));
    };
    let typeck = context.tcx.typeck(definition.expect_local());
    let Res::Def(DefKind::AssocFn, selected) = typeck.qpath_res(path, callee.hir_id) else {
        return Err(CaptureFailure::Unsupported(
            "constant initializer does not select an associated function",
        ));
    };
    if !selected.is_local() || typeck.node_type(body.value.hir_id) != owner {
        return Err(CaptureFailure::Unsupported(
            "constant initializer returns a different or nonlocal product",
        ));
    }
    identity::original_range(context, callee.span)?;
    let span = identity::original_range(context, body.value.span)?;
    let (core_constant, offset) = arithmetic(context, argument)?;

    Ok((
        Initializer {
            span,
            core_constant,
            offset,
            cast_from: context.tcx.types.i32.to_string(),
            cast_to: context.tcx.types.usize.to_string(),
            subtraction_type: typeck.node_type(argument.hir_id).to_string(),
        },
        selected,
    ))
}

fn arithmetic(
    context: &Context<'_>,
    expression: &Expr<'_>,
) -> Result<(CoreConstant, String), CaptureFailure> {
    let ExprKind::Binary(operation, cast, offset) = expression.kind else {
        return Err(CaptureFailure::Unsupported(
            "constant argument is not original subtraction",
        ));
    };
    let typeck = context.tcx.typeck(expression.hir_id.owner.def_id);
    if operation.node != rustc_hir::BinOpKind::Sub
        || typeck.node_type(expression.hir_id) != context.tcx.types.usize
        || typeck.node_type(cast.hir_id) != context.tcx.types.usize
        || typeck.node_type(offset.hir_id) != context.tcx.types.usize
    {
        return Err(CaptureFailure::Unsupported(
            "constant arithmetic lacks exact usize subtraction",
        ));
    }

    identity::original_range(context, expression.span)?;
    let core = core_operand(context, cast)?;
    let offset = literal(context, offset)?;

    Ok((core, offset))
}

fn core_operand(context: &Context<'_>, cast: &Expr<'_>) -> Result<CoreConstant, CaptureFailure> {
    let ExprKind::Cast(operand, _) = cast.kind else {
        return Err(CaptureFailure::Unsupported(
            "constant dependency lacks its original usize cast",
        ));
    };
    let ExprKind::Path(ref path) = operand.kind else {
        return Err(CaptureFailure::Unsupported(
            "constant scalar dependency lacks an original path",
        ));
    };
    let tcx = context.tcx;
    let typeck = tcx.typeck(cast.hir_id.owner.def_id);
    let Res::Def(DefKind::Const, definition) = typeck.qpath_res(path, operand.hir_id) else {
        return Err(CaptureFailure::Unsupported(
            "constant scalar dependency is not a standalone const",
        ));
    };
    if definition.is_local()
        || tcx.crate_name(definition.krate).as_str() != "core"
        || !tcx.is_diagnostic_item(Symbol::intern("i32_legacy_const_max"), definition)
        || typeck.node_type(operand.hir_id) != tcx.types.i32
        || typeck.node_type(cast.hir_id) != tcx.types.usize
        || tcx
            .type_of(definition)
            .instantiate_identity()
            .skip_norm_wip()
            != tcx.types.i32
    {
        return Err(CaptureFailure::Unsupported(
            "constant scalar dependency lacks exact legacy i32 authority",
        ));
    }
    identity::original_range(context, cast.span)?;
    identity::original_range(context, operand.span)?;
    let scalar = tcx
        .const_eval_poly(definition)
        .map_err(|error| CaptureFailure::Evaluation(format!("{error:?}")))?
        .try_to_scalar_int()
        .ok_or(CaptureFailure::Unsupported(
            "legacy i32 constant has no scalar value",
        ))?;

    Ok(CoreConstant {
        definition: super::super::library_resolution::definition(tcx, definition),
        scalar: ScalarValue {
            ty: tcx.types.i32.to_string(),
            value: scalar.to_i32().to_string(),
        },
    })
}

fn literal(context: &Context<'_>, expression: &Expr<'_>) -> Result<String, CaptureFailure> {
    let ExprKind::Lit(literal) = expression.kind else {
        return Err(CaptureFailure::Unsupported(
            "constant offset is not an integer literal",
        ));
    };
    let rustc_ast::LitKind::Int(value, _) = literal.node else {
        return Err(CaptureFailure::Unsupported(
            "constant offset is not an integer literal",
        ));
    };
    let value = u32::try_from(value.get()).map_err(|_| {
        CaptureFailure::Unsupported("constant offset exceeds the bounded u32 domain")
    })?;

    identity::original_range(context, expression.span)?;
    let (source, _) =
        context
            .sources
            .range(context.tcx, expression.span)
            .ok_or(CaptureFailure::Unsupported(
                "constant offset source is unavailable",
            ))?;
    let canonical = value.to_string();
    if source.as_str() != canonical && source.as_str() != format!("{canonical}usize") {
        return Err(CaptureFailure::Unsupported(
            "constant offset requires canonical decimal source",
        ));
    }

    Ok(canonical)
}
