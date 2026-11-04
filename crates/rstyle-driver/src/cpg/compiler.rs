//! Compiler judgments a document records so core can lower runtime bounds without
//! rustc: implicit adjustments, binding modes, standard-library identity, library
//! operation keys and iterator item types.

use super::sysroot;
use super::unit::Unit;
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
    records(unit, adjustments, |adjustment| {
        (adjustment_kind(&adjustment.kind), adjustment.target)
    })
}

/// A pattern's implicit dereferences before it matches, each with the type it applies to.
pub fn pattern_adjustments<'tcx>(
    unit: &mut Unit<'tcx>,
    adjustments: &[PatAdjustment<'tcx>],
) -> Value {
    records(unit, adjustments, |adjustment| {
        let kind = match adjustment.kind {
            PatAdjust::BuiltinDeref => "auto_deref",
            PatAdjust::OverloadedDeref => "overloaded_deref",
            _ => "other",
        };
        ((kind, None), adjustment.source)
    })
}

type Kind = (&'static str, Option<&'static str>);

/// Adjustment records: each kind, the mode a borrow or dereference takes, and its type.
fn records<'tcx, T>(
    unit: &mut Unit<'tcx>,
    adjustments: &[T],
    describe: impl Fn(&T) -> (Kind, Ty<'tcx>),
) -> Value {
    let mut records = Vec::with_capacity(adjustments.len());
    for adjustment in adjustments {
        let ((kind, mode), target) = describe(adjustment);
        let mut record = json!({"kind": kind, "target": unit.ty(target)});
        if let Some(mode) = mode {
            record["mode"] = json!(mode);
        }
        records.push(record);
    }
    Value::Array(records)
}

fn adjustment_kind(kind: &Adjust) -> Kind {
    match *kind {
        Adjust::NeverToAny => ("other", None),
        Adjust::Deref(DerefAdjustKind::Builtin) => ("auto_deref", None),
        Adjust::Deref(DerefAdjustKind::Overloaded(deref)) => {
            ("overloaded_deref", Some(mode(deref.mutbl)))
        }
        Adjust::Deref(DerefAdjustKind::Pin) => ("other", None),
        Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) => ("auto_ref", Some("shared")),
        Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Mut { .. })) => {
            ("auto_ref", Some("exclusive"))
        }
        Adjust::Borrow(AutoBorrow::RawPtr(_)) => ("auto_ref", Some("raw")),
        Adjust::Borrow(AutoBorrow::Pin(mutability)) => ("pin", Some(mode(mutability))),
        Adjust::Pointer(PointerCoercion::Unsize) => ("unsize", None),
        Adjust::Pointer(_) => ("other", None),
        Adjust::GenericReborrow(mutability) => ("reborrow", Some(mode(mutability))),
    }
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

pub fn standard(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    sysroot::sysroot(tcx, def_id.krate)
}

/// `<receiver as Iterator>::Item`, normalized in the owner's environment.
pub fn iterator_item<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
    receiver: Ty<'tcx>,
) -> Option<Ty<'tcx>> {
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

/// The frontend's own construct tag for the HIR node at `id`, as rstyle's HIR facts
/// name it before any extraction profile refines it.
pub fn original_kind(tcx: TyCtxt<'_>, id: rustc_hir::HirId) -> &'static str {
    use rustc_hir::{ExprKind, LoopSource, Node, PatKind, StmtKind};
    match tcx.hir_node(id) {
        Node::Expr(expr) => match expr.kind {
            ExprKind::Loop(_, Some(_), _, _) => "expr_labelled_loop",
            ExprKind::Loop(_, None, LoopSource::While, _) => "expr_while",
            ExprKind::Loop(_, None, LoopSource::ForLoop, _) => "expr_for",
            ref kind => crate::hir_extraction::hir_tags::kind(kind),
        },
        Node::Stmt(stmt) => match stmt.kind {
            StmtKind::Let(_) => "stmt_let",
            StmtKind::Semi(_) => "stmt_semi",
            StmtKind::Expr(_) => "stmt_expr",
            StmtKind::Item(_) => "stmt_item",
        },
        Node::LetStmt(_) => "stmt_let",
        Node::Arm(_) => "match_arm",
        Node::Block(_) => "expr_block",
        Node::Ty(_) => "ty_annotation",
        Node::Param(_) => "param",
        Node::Pat(pat) => match pat.kind {
            PatKind::Binding(rustc_hir::BindingMode(rustc_hir::ByRef::No, _), _, _, None) => {
                "pat_binding"
            }
            PatKind::Binding(..) => "pat_binding_pattern",
            PatKind::Wild => "pat_wild",
            PatKind::Or(_) => "pat_or",
            PatKind::Range(..) => "pat_range",
            PatKind::Expr(expr) => match expr.kind {
                rustc_hir::PatExprKind::Lit { .. } => "pat_literal",
                rustc_hir::PatExprKind::Path(_) => "pat_path",
            },
            PatKind::Struct(..) => "pat_struct",
            PatKind::TupleStruct(..) => "pat_tuple_struct",
            PatKind::Tuple(..) => "pat_tuple",
            PatKind::Slice(..) => "pat_slice",
            PatKind::Ref(..) => "pat_ref",
            PatKind::Deref(_) => "pat_deref",
            PatKind::Guard(..) => "pat_guard",
            PatKind::Never => "pat_never",
            PatKind::Missing | PatKind::Err(_) => "pat_unsupported",
        },
        _ => "other",
    }
}
