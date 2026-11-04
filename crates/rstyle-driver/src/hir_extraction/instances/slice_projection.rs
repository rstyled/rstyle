//! Proof inputs for selected local shared-slice instances; never admission authority.
use super::{definition_record, diagnostic_support, projection, types};
use crate::hir_extraction::hir::Context;
use rustc_hir::{self as hir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt};
use serde::Serialize;

mod methods;
pub(in crate::hir_extraction) mod record;

fn candidate(tcx: TyCtxt<'_>, owner: LocalDefId) -> bool {
    let generics = tcx.generics_of(owner);
    if tcx.def_kind(owner) != DefKind::Fn
        || generics.parent.is_some()
        || generics.own_params.len() != 1
        || !matches!(
            generics.own_params[0].kind,
            ty::GenericParamDefKind::Type {
                has_default: false,
                synthetic: false
            }
        )
    {
        return false;
    }

    let signature = tcx.fn_sig(owner).instantiate_identity().skip_binder();
    let [slice, length, what] = signature.inputs() else {
        return false;
    };
    let ty::Ref(_, slice, hir::Mutability::Not) = slice.kind() else {
        return false;
    };
    let ty::Slice(element) = slice.kind() else {
        return false;
    };

    if !matches!(element.kind(), ty::Param(parameter) if parameter.index == 0)
        || !matches!(length.kind(), ty::Uint(ty::UintTy::Usize))
        || !matches!(what.kind(), ty::Ref(region, inner, hir::Mutability::Not)
            if matches!(region.kind(), ty::ReStatic) && matches!(inner.kind(), ty::Str))
    {
        return false;
    }

    let ty::Adt(result, args) = signature.output().kind() else {
        return false;
    };
    if !tcx.is_diagnostic_item(rustc_span::sym::Result, result.did())
        || args.len() != 2
        || !matches!(args.type_at(0).kind(), ty::Tuple(elements) if elements.is_empty())
    {
        return false;
    }

    matches!(args.type_at(1).kind(), ty::Adt(error, arguments)
        if error.is_struct() && error.did().is_local() && arguments.is_empty())
}

#[derive(Serialize)]
struct Typed {
    hir: rstyle_pdg::HirRef,
    concrete: TypeObservation,
    adjustments: Vec<TypeObservation>,
    dispatch_operands: Vec<TypeObservation>,
}

/// A function item's definition and complete arguments identify its type without
/// claiming that its higher-ranked callable signature has been independently proved.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TypeObservation {
    Structural {
        arguments: types::Arguments,
    },
    FunctionItem {
        definition: super::Definition,
        arguments: types::Arguments,
    },
}

struct Lens<'tcx> {
    owner: LocalDefId,
    arguments: GenericArgsRef<'tcx>,
}

impl<'tcx> Lens<'tcx> {
    fn endpoint(&self, tcx: TyCtxt<'tcx>, original: Ty<'tcx>) -> Result<TypeObservation, String> {
        let concrete = projection::normalize(tcx, self.arguments, original)
            .map_err(|error| error.to_string())?;
        if let ty::FnDef(definition, arguments) = concrete.kind() {
            let arguments = projection::complete_arguments(
                tcx,
                projection::normalize(tcx, self.arguments, arguments.skip_binder()),
            )?
            .observation;
            return Ok(TypeObservation::FunctionItem {
                definition: definition_record(tcx, *definition),
                arguments,
            });
        }
        Ok(TypeObservation::Structural {
            arguments: types::endpoint(tcx, concrete)?,
        })
    }

    fn typed(
        &self,
        context: &Context<'tcx>,
        reference: &rstyle_pdg::HirRef,
    ) -> Result<Typed, String> {
        let id = diagnostic_support::hir_id(reference, "Shared-slice projection HIR is not local")?;
        let concrete = projection::project_node(self.owner, context.typeck, id, |ty| {
            projection::normalize(context.tcx, self.arguments, ty)
                .map_err(|error| error.to_string())
        })?;

        let mut adjustments = Vec::new();
        let mut dispatch_operands = Vec::new();
        if let hir::Node::Expr(expression) = context.tcx.hir_node(id) {
            for adjustment in context.typeck.expr_adjustments(expression) {
                adjustments.push(self.endpoint(context.tcx, adjustment.target)?);
            }
            if let Some((_, operands)) = context.dispatch_operands(expression) {
                for operand in operands {
                    dispatch_operands
                        .push(self.endpoint(context.tcx, context.typeck.expr_ty(operand))?);
                }
            }
        }

        Ok(Typed {
            hir: reference.clone(),
            concrete: self.endpoint(context.tcx, concrete)?,
            adjustments,
            dispatch_operands,
        })
    }
}
