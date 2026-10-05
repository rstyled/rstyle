//! Raw signature shapes for functions in `impl` blocks. No policy: `Self`
//! substitution and type variables are applied later by `rstyle_core::shape`.

use crate::ids;
use rstyle_core::facts::{Receiver, SigFacts, TyShape};
use rustc_hir::FnSig;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::{self, Ty, TyCtxt};

fn args_of<'tcx>(tcx: TyCtxt<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Vec<TyShape> {
    args.types().map(|arg| shape(tcx, arg)).collect()
}

fn dyn_id<'tcx>(
    tcx: TyCtxt<'tcx>,
    preds: &'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>,
) -> String {
    preds
        .principal_def_id()
        .map(|id| ids::def_id_str(tcx, id))
        .unwrap_or_default()
}

pub fn shape<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> TyShape {
    match *ty.kind() {
        ty::Adt(def, args) => TyShape::Adt {
            id: ids::def_id_str(tcx, def.did()),
            args: args_of(tcx, args),
        },
        ty::Ref(_, inner, mutability) => TyShape::Ref {
            mutable: mutability.is_mut(),
            inner: Box::new(shape(tcx, inner)),
        },
        ty::Tuple(items) => TyShape::Tuple(items.iter().map(|t| shape(tcx, t)).collect()),
        ty::Slice(inner) => TyShape::Slice(Box::new(shape(tcx, inner))),
        ty::Array(inner, _) => TyShape::Array(Box::new(shape(tcx, inner))),
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => {
            TyShape::Prim(ty.to_string())
        }
        ty::Param(param) => TyShape::Param(param.index as usize),
        ty::FnPtr(..) => TyShape::FnPtr,
        ty::Dynamic(preds, ..) => TyShape::Dyn(dyn_id(tcx, preds)),
        ty::Alias(_, alias) if matches!(alias.kind, ty::AliasTyKind::Opaque { .. }) => {
            TyShape::Opaque
        }
        _ => TyShape::Other,
    }
}

/// Id and type arguments of the ADT an `impl` block is for.
fn impl_self_ty(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(String, Vec<TyShape>)> {
    let impl_id = tcx.impl_of_assoc(def_id)?;
    match tcx
        .type_of(impl_id)
        .instantiate_identity()
        .skip_norm_wip()
        .kind()
    {
        ty::Adt(def, args) => Some((ids::def_id_str(tcx, def.did()), args_of(tcx, args))),
        _ => None,
    }
}

fn split(owner: Option<(String, Vec<TyShape>)>) -> (Option<String>, Vec<TyShape>) {
    owner.map_or((None, vec![]), |(id, args)| (Some(id), args))
}

fn receiver<'tcx>(self_ty: Option<&str>, first: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Receiver {
    let of_self = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(def, _) if Some(ids::def_id_str(tcx, def.did())).as_deref() == self_ty);
    match *first.kind() {
        ty::Ref(_, inner, m) if of_self(inner) => {
            if m.is_mut() {
                Receiver::RefMut
            } else {
                Receiver::Ref
            }
        }
        _ if of_self(first) => Receiver::Value,
        _ => Receiver::Other,
    }
}

/// `None` for free functions: only methods in `impl` blocks are interesting.
pub fn sig_facts<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
    sig: &FnSig<'tcx>,
) -> Option<SigFacts> {
    tcx.impl_of_assoc(def_id.to_def_id())?;
    let (self_ty, self_args) = split(impl_self_ty(tcx, def_id.to_def_id()));
    let fn_sig = tcx.fn_sig(def_id).instantiate_identity().skip_binder();
    let inputs = fn_sig.inputs();
    let has_self = sig.decl.implicit_self().has_implicit_self();
    let receiver = match (has_self, inputs.first()) {
        (true, Some(first)) => receiver(self_ty.as_deref(), *first, tcx),
        _ => Receiver::None,
    };
    let generics = tcx.generics_of(def_id);

    Some(SigFacts {
        self_ty,
        self_args,
        receiver,
        params: inputs
            .iter()
            .skip(usize::from(has_self))
            .map(|ty| shape(tcx, *ty))
            .collect(),
        ret: shape(tcx, fn_sig.output()),
        is_async: sig.header.is_async(),
        is_unsafe: sig.header.is_unsafe(),
        is_const: sig.header.constness != rustc_hir::Constness::NotConst,
        generics: generics.own_counts().types,
    })
}

/// Receiver of a trait method or foreign fn, where there is no ADT impl to compare with.
fn shape_receiver(first: Ty<'_>) -> Receiver {
    match *first.kind() {
        ty::Ref(_, _, m) if m.is_mut() => Receiver::RefMut,
        ty::Ref(..) => Receiver::Ref,
        _ => Receiver::Value,
    }
}

/// Signature of any called function (workspace, dependency or std), for call-node labels.
/// `None` for anything that is not a plain fn or associated fn.
pub fn callee_sig(tcx: TyCtxt<'_>, def_id: DefId) -> Option<SigFacts> {
    if !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
        return None;
    }
    let fn_sig = tcx.fn_sig(def_id).instantiate_identity().skip_binder();
    let has_self = tcx
        .opt_associated_item(def_id)
        .is_some_and(|i| i.is_method());
    let (self_ty, self_args) = split(impl_self_ty(tcx, def_id));
    let inputs = fn_sig.inputs();
    let receiver = match (has_self, inputs.first()) {
        (true, Some(first)) if self_ty.is_some() => receiver(self_ty.as_deref(), *first, tcx),
        (true, Some(first)) => shape_receiver(*first),
        _ => Receiver::None,
    };

    Some(SigFacts {
        self_ty,
        self_args,
        receiver,
        params: inputs
            .iter()
            .skip(usize::from(has_self))
            .map(|ty| shape(tcx, *ty))
            .collect(),
        ret: shape(tcx, fn_sig.output()),
        is_async: tcx.asyncness(def_id).is_async(),
        is_unsafe: fn_sig.safety().is_unsafe(),
        is_const: tcx.is_const_fn(def_id),
        generics: tcx.generics_of(def_id).own_counts().types,
    })
}
