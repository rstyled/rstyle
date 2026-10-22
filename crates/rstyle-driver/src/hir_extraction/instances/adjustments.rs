//! Ordered compiler adjustments with structural projected endpoints, not authority.
use super::super::hir::{Context, reference};
use super::{projection::Projection, types};
use rustc_hir::{Expr, Mutability};
use rustc_middle::ty::{
    self, Ty, TyCtxt,
    adjustment::{Adjust, AutoBorrow, AutoBorrowMutability, DerefAdjustKind},
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Observation {
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    source_type: types::Arguments,
    steps: Vec<Step>,
    adjusted_type: types::Arguments,
}

impl Observation {
    pub(super) fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[derive(Serialize)]
struct Step {
    kind: Kind,
    target: types::Arguments,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    BuiltinDeref,
    SharedBorrow,
    NeverToAny,
}

/// An empty chain is explicit evidence for unadjusted comparison operands.
pub(super) fn operand<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    projection: &Projection<'tcx>,
) -> Result<Observation, String> {
    // This checks the exact typeck/HIR owner before any adjustment-table lookup.
    let source = projection
        .node_ty(context.tcx, context.typeck, expression.hir_id)
        .map_err(|error| error.to_string())?;
    let (steps, adjusted) = ordered(context, expression, projection, source)?;

    Ok(Observation {
        hir: reference(expression.hir_id),
        span: context.span(expression.span),
        source_type: endpoint(context.tcx, source)?,
        steps,
        adjusted_type: endpoint(context.tcx, adjusted)?,
    })
}

fn ordered<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    projection: &Projection<'tcx>,
    source: Ty<'tcx>,
) -> Result<(Vec<Step>, Ty<'tcx>), String> {
    let mut current = source;
    let mut steps = Vec::new();
    for adjustment in context.typeck.expr_adjustments(expression) {
        let target = projection
            .ty(context.tcx, adjustment.target)
            .map_err(|error| error.to_string())?;
        let kind = classify(&adjustment.kind, current, target)?;
        steps.push(Step {
            kind,
            target: endpoint(context.tcx, target)?,
        });
        current = target;
    }
    Ok((steps, current))
}

fn classify<'tcx>(kind: &Adjust, source: Ty<'tcx>, target: Ty<'tcx>) -> Result<Kind, String> {
    match kind {
        Adjust::Deref(DerefAdjustKind::Builtin) if matches!(source.kind(), ty::Ref(_, referent, Mutability::Not) if *referent == target) => {
            Ok(Kind::BuiltinDeref)
        }
        Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) if matches!(target.kind(), ty::Ref(_, referent, Mutability::Not) if *referent == source) => {
            Ok(Kind::SharedBorrow)
        }
        Adjust::NeverToAny if matches!(source.kind(), ty::Never) => Ok(Kind::NeverToAny),
        _ => {
            Err("Compiler adjustment is unsupported or contradicts its projected endpoints".into())
        }
    }
}

fn endpoint<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> Result<types::Arguments, String> {
    let observed = types::arguments(
        tcx,
        tcx.mk_args_from_iter(std::iter::once(ty::GenericArg::from(value))),
    );
    if !observed.complete() {
        return Err("Projected adjustment endpoint lacks complete structural identity".into());
    }
    Ok(observed)
}
