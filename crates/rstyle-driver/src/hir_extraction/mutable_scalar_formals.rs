//! Compiler observations connected to existing formal-location evidence only.
use super::hir::Context;
use rstyle_pdg::{
    HirFnFacts, HirRef,
    field_locations::{FieldObservations, scalar_formals::*},
};
use rustc_hir::{self as hir, Mutability};
use rustc_middle::ty::{self, Ty};

pub(super) fn capture(ctx: &Context<'_>, facts: &mut HirFnFacts) -> Result<(), String> {
    if !facts
        .build
        .frontend_options
        .contains_key(rstyle_pdg::field_locations::KEY)
    {
        return Ok(());
    }
    let formals: Vec<_> = facts
        .signature
        .params
        .iter()
        .map(|parameter| Formal {
            parameter: parameter.position,
            binding: parameter.pat.hir.clone(),
            observation: observation(ctx.typeck.node_type_opt(hir_id(&parameter.pat.hir))),
        })
        .collect();
    if !formals.iter().any(Formal::mutable_usize) {
        return Ok(());
    }
    let evidence = observe_accesses(facts, formals);
    let mut fields = facts
        .build
        .frontend_options
        .data::<FieldObservations>()?
        .into_owned();
    fields.scalar_formal_accesses = Some(evidence);
    facts.build.frontend_options.insert_data(fields);

    Ok(())
}

fn hir_id(reference: &HirRef) -> hir::HirId {
    hir::HirId {
        owner: hir::OwnerId {
            def_id: hir::def_id::LocalDefId {
                local_def_index: hir::def_id::DefIndex::from_u32(reference.owner),
            },
        },
        local_id: hir::ItemLocalId::from_u32(reference.local),
    }
}

fn observation(ty: Option<Ty<'_>>) -> FormalObservation {
    match ty.map(|ty| ty.kind()) {
        Some(ty::Ref(_, referent, Mutability::Not)) => FormalObservation::Shared {
            referent: scalar(*referent),
        },
        Some(ty::Ref(_, referent, Mutability::Mut)) => FormalObservation::Mutable {
            referent: scalar(*referent),
        },
        Some(_) => FormalObservation::NonReference,
        None => FormalObservation::Unknown {
            reason: "Missing compiler formal type".into(),
        },
    }
}

fn scalar(ty: Ty<'_>) -> ScalarObservation {
    if matches!(ty.kind(), ty::Uint(ty::UintTy::Usize)) {
        ScalarObservation::Captured {
            scalar: "usize".into(),
        }
    } else {
        ScalarObservation::Unsupported {
            reason: "Referent is outside the observed usize location family".into(),
        }
    }
}
