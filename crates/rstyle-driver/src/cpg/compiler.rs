//! Compiler judgments a document records so core can lower runtime bounds without
//! rustc: implicit adjustments, binding modes, standard-library identity, library
//! operation keys and iterator item types.

use super::unit::Unit;
use crate::hir_extraction::runtime_bounds::sysroot;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::adjustment::{
    Adjust, Adjustment, AutoBorrow, AutoBorrowMutability, DerefAdjustKind, PatAdjust,
    PatAdjustment, PointerCoercion,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::sym;
use serde_json::{Value, json};

fn mode(mutability: rustc_hir::Mutability) -> &'static str {
    match mutability {
        rustc_hir::Mutability::Not => "shared",
        rustc_hir::Mutability::Mut => "exclusive",
    }
}

/// An expression's implicit adjustments in application order, each with the type it produces.
pub fn adjustments<'tcx>(unit: &mut Unit<'tcx>, adjustments: &[Adjustment<'tcx>]) -> Value {
    let records: Vec<Value> = adjustments
        .iter()
        .map(|adjustment| {
            let (kind, mode) = match adjustment.kind {
                Adjust::NeverToAny => ("other", None),
                Adjust::Deref(DerefAdjustKind::Builtin) => ("auto_deref", None),
                Adjust::Deref(DerefAdjustKind::Overloaded(deref)) => {
                    ("overloaded_deref", Some(mode(deref.mutbl)))
                }
                Adjust::Deref(DerefAdjustKind::Pin) => ("other", None),
                Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) => {
                    ("auto_ref", Some("shared"))
                }
                Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Mut { .. })) => {
                    ("auto_ref", Some("exclusive"))
                }
                Adjust::Borrow(AutoBorrow::RawPtr(_)) => ("auto_ref", Some("raw")),
                Adjust::Borrow(AutoBorrow::Pin(mutability)) => ("pin", Some(mode(mutability))),
                Adjust::Pointer(PointerCoercion::Unsize) => ("unsize", None),
                Adjust::Pointer(_) => ("other", None),
                Adjust::GenericReborrow(mutability) => ("reborrow", Some(mode(mutability))),
            };
            let mut record = json!({"kind": kind, "target": unit.ty(adjustment.target)});
            if let Some(mode) = mode {
                record["mode"] = json!(mode);
            }
            record
        })
        .collect();
    Value::Array(records)
}

/// A pattern's implicit dereferences before it matches, each with the type it applies to.
pub fn pattern_adjustments<'tcx>(unit: &mut Unit<'tcx>, adjustments: &[PatAdjustment<'tcx>]) -> Value {
    let records: Vec<Value> = adjustments
        .iter()
        .map(|adjustment| {
            let kind = match adjustment.kind {
                PatAdjust::BuiltinDeref => "auto_deref",
                PatAdjust::OverloadedDeref => "overloaded_deref",
                _ => "other",
            };
            json!({"kind": kind, "target": unit.ty(adjustment.source)})
        })
        .collect();
    Value::Array(records)
}

/// How a binding pattern binds, after default binding modes: by value or by reference.
pub fn binding_mode(mode: rustc_hir::BindingMode) -> &'static str {
    match mode.0 {
        rustc_hir::ByRef::No => "value",
        rustc_hir::ByRef::Yes(_, mutability) => mode_of(mutability),
    }
}

fn mode_of(mutability: rustc_hir::Mutability) -> &'static str {
    mode(mutability)
}

/// The operation key of a library member: a trait member by its trait's name, any other
/// by its implementation's self type.
pub fn declared_operation(tcx: TyCtxt<'_>, member: DefId) -> Option<String> {
    match tcx.trait_of_assoc(member) {
        Some(owner) => sysroot::trait_operation(tcx, owner, member),
        None => sysroot::operation(tcx, member),
    }
}

/// The key of the implementation member a call selects, by its impl's self type.
pub fn selected_operation(tcx: TyCtxt<'_>, member: DefId) -> Option<String> {
    sysroot::operation(tcx, member)
}

pub fn standard(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    sysroot::sysroot(tcx, def_id.krate)
}

/// `<receiver as Iterator>::Item`, normalized in the owner's environment.
pub fn iterator_item<'tcx>(tcx: TyCtxt<'tcx>, owner: LocalDefId, receiver: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let declaration = tcx.get_diagnostic_item(sym::IteratorItem)?;
    let projected = ty::Ty::new_projection_from_args(
        tcx,
        ty::IsRigid::No,
        declaration,
        tcx.mk_args(&[receiver.into()]),
    );
    tcx.try_normalize_erasing_regions(
        ty::TypingEnv::post_analysis(tcx, owner),
        ty::Unnormalized::new(projected),
    )
    .ok()
}

/// Whether the member belongs to the standard `Iterator` trait.
pub fn iterator_member(tcx: TyCtxt<'_>, member: DefId) -> bool {
    let Some(iterator) = tcx.get_diagnostic_item(sym::Iterator) else {
        return false;
    };
    tcx.trait_of_assoc(member) == Some(iterator) && member.krate == iterator.krate
}
