//! Ordered projected receiver adjustments; unsupported coercions remain unavailable.
use super::*;
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability, DerefAdjustKind};

#[derive(Serialize)]
pub(super) struct Observation {
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    source_type: types::Arguments,
    steps: Vec<Step>,
    adjusted_type: types::Arguments,
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
}

pub(super) fn capture<'tcx>(
    context: &Context<'tcx>,
    receiver: &Expr<'tcx>,
    projection: &Projection<'tcx>,
) -> Result<Observation, String> {
    let source = projection.node_ty(context.tcx, context.typeck, receiver.hir_id)?;
    let mut current = source;
    let mut steps = Vec::new();

    for adjustment in context.typeck.expr_adjustments(receiver) {
        let target = projection.ty(context.tcx, adjustment.target)?;
        let kind = match &adjustment.kind {
            Adjust::Deref(DerefAdjustKind::Builtin) if matches!(current.kind(), ty::Ref(_, referent, Mutability::Not) if *referent == target) => {
                Kind::BuiltinDeref
            }
            Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) if matches!(target.kind(), ty::Ref(_, referent, Mutability::Not) if *referent == current) => {
                Kind::SharedBorrow
            }
            _ => return Err("AsRef receiver has an unsupported or inconsistent adjustment".into()),
        };
        steps.push(Step {
            kind,
            target: endpoint(context.tcx, target)?,
        });
        current = target;
    }

    Ok(Observation {
        hir: reference(receiver.hir_id),
        span: context.span(receiver.span),
        source_type: endpoint(context.tcx, source)?,
        steps,
        adjusted_type: endpoint(context.tcx, current)?,
    })
}
