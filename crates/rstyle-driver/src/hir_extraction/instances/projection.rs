//! A compiler-only lens over one original scalar generic body, never new HIR.
use rustc_hir::{HirId, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt, TypeVisitableExt, TypeckResults};
use std::fmt;

#[derive(Clone, Copy)]
pub(super) struct Projection<'tcx> {
    owner: LocalDefId,
    arguments: GenericArgsRef<'tcx>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    UnsupportedOwner,
    UnsupportedArguments,
    UnsupportedType,
    Unresolved,
    Normalization,
    OwnerMismatch,
    MissingNodeType,
    Resolution,
    NoInstance,
    NonItem,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::UnsupportedOwner => {
                "projection requires an original free function with only type parameters"
            }
            Self::UnsupportedArguments => {
                "projection requires exact concrete i32/u32 owner arguments"
            }
            Self::UnsupportedType => "projected type is outside the scalar body contract",
            Self::Unresolved => "projection retains generic, inferred or bound type data",
            Self::Normalization => "compiler could not normalize projected type data",
            Self::OwnerMismatch => "projection and typeck/HIR owners differ",
            Self::MissingNodeType => "original typeck lacks the requested node type",
            Self::Resolution => "compiler instance selection failed",
            Self::NoInstance => "compiler did not select a concrete instance",
            Self::NonItem => "selected instance is not an ordinary Item",
        };
        formatter.write_str(reason)
    }
}

impl std::error::Error for Error {}

impl<'tcx> Projection<'tcx> {
    pub(super) fn new(
        tcx: TyCtxt<'tcx>,
        owner: LocalDefId,
        arguments: GenericArgsRef<'tcx>,
    ) -> Result<Self, Error> {
        let generics = tcx.generics_of(owner);
        if tcx.def_kind(owner) != DefKind::Fn
            || generics.parent.is_some()
            || generics.own_params.len() != 1
            || !generics
                .own_params
                .iter()
                .all(|parameter| matches!(parameter.kind, ty::GenericParamDefKind::Type { .. }))
        {
            return Err(Error::UnsupportedOwner);
        }

        if arguments.len() != 1 {
            return Err(Error::UnsupportedArguments);
        }
        let arguments = normalize(tcx, arguments, arguments)?;
        if arguments.len() != generics.count() || !arguments.iter().all(|argument| {
            matches!(argument.kind(), ty::GenericArgKind::Type(value) if integer(value))
        }) {
            return Err(Error::UnsupportedArguments);
        }
        Ok(Self { owner, arguments })
    }

    pub(super) fn ty(&self, tcx: TyCtxt<'tcx>, original: Ty<'tcx>) -> Result<Ty<'tcx>, Error> {
        let projected = normalize(tcx, self.arguments, original)?;
        if !body_type(projected, 0) {
            return Err(Error::UnsupportedType);
        }
        Ok(projected)
    }

    pub(super) fn args(
        &self,
        tcx: TyCtxt<'tcx>,
        original: GenericArgsRef<'tcx>,
    ) -> Result<GenericArgsRef<'tcx>, Error> {
        let projected = normalize(tcx, self.arguments, original)?;
        if !projected.iter().all(|argument| match argument.kind() {
            ty::GenericArgKind::Lifetime(_) => true,
            ty::GenericArgKind::Type(value) => body_type(value, 0),
            ty::GenericArgKind::Const(_) => false,
        }) {
            return Err(Error::UnsupportedArguments);
        }
        Ok(projected)
    }

    pub(super) fn node_ty(
        &self,
        tcx: TyCtxt<'tcx>,
        typeck: &TypeckResults<'tcx>,
        node: HirId,
    ) -> Result<Ty<'tcx>, Error> {
        let original = original_node_type(self.owner, typeck, node)?;
        self.ty(tcx, original)
    }

    /// Selection remains separate evidence; this never changes operator dispatch.
    pub(super) fn select(
        &self,
        tcx: TyCtxt<'tcx>,
        definition: rustc_hir::def_id::DefId,
        original: GenericArgsRef<'tcx>,
    ) -> Result<ty::Instance<'tcx>, Error> {
        if !matches!(tcx.def_kind(definition), DefKind::Fn | DefKind::AssocFn) {
            return Err(Error::UnsupportedOwner);
        }

        let arguments = self.args(tcx, original)?;
        let instance = ty::Instance::try_resolve(
            tcx,
            ty::TypingEnv::fully_monomorphized(),
            definition,
            arguments,
        )
        .map_err(|_| Error::Resolution)?
        .ok_or(Error::NoInstance)?;
        if !matches!(instance.def, ty::InstanceKind::Item(_)) {
            return Err(Error::NonItem);
        }
        self.args(tcx, instance.args)?;
        Ok(instance)
    }
}

/// Look up only original owner-local node types, before applying a lens grammar.
fn original_node_type<'tcx>(
    owner: LocalDefId,
    typeck: &TypeckResults<'tcx>,
    node: HirId,
) -> Result<Ty<'tcx>, Error> {
    if typeck.hir_owner.def_id != owner || node.owner.def_id != owner {
        return Err(Error::OwnerMismatch);
    }

    typeck.node_type_opt(node).ok_or(Error::MissingNodeType)
}

/// Associated lenses share lookup mechanics, while the callback retains each
/// lens's own normalized type grammar and evidence requirements.
pub(super) fn project_node<'tcx>(
    owner: LocalDefId,
    typeck: &TypeckResults<'tcx>,
    node: HirId,
    project: impl FnOnce(Ty<'tcx>) -> Result<Ty<'tcx>, String>,
) -> Result<Ty<'tcx>, String> {
    let original = original_node_type(owner, typeck, node).map_err(|error| error.to_string())?;
    project(original)
}

/// Preserve the caller's substitution grammar, then require complete structural
/// identities before publishing any observed argument roster.
pub(super) struct CheckedArguments<'tcx> {
    pub(super) arguments: GenericArgsRef<'tcx>,
    pub(super) observation: super::types::Arguments,
}

pub(super) fn complete_arguments<'tcx, E: fmt::Display>(
    tcx: TyCtxt<'tcx>,
    normalized: Result<GenericArgsRef<'tcx>, E>,
) -> Result<CheckedArguments<'tcx>, String> {
    let arguments = normalized.map_err(|error| error.to_string())?;
    let observation = super::types::arguments(tcx, arguments);
    if !observation.complete() {
        return Err("Substituted arguments lack complete bounded structural identities".into());
    }
    Ok(CheckedArguments {
        arguments,
        observation,
    })
}

/// Shared substitution mechanics; callers retain their own owner/type grammar.
pub(super) fn normalize<'tcx, T>(
    tcx: TyCtxt<'tcx>,
    arguments: GenericArgsRef<'tcx>,
    original: T,
) -> Result<T, Error>
where
    T: ty::TypeFoldable<TyCtxt<'tcx>>,
{
    let projected = tcx
        .try_instantiate_and_normalize_erasing_regions(
            arguments,
            ty::TypingEnv::fully_monomorphized(),
            ty::EarlyBinder::bind(tcx, original),
        )
        .map_err(|_| Error::Normalization)?;
    if projected.has_non_region_param()
        || projected.has_infer()
        || projected.has_escaping_bound_vars()
    {
        return Err(Error::Unresolved);
    }
    Ok(projected)
}

fn integer(value: Ty<'_>) -> bool {
    matches!(
        value.kind(),
        ty::Int(ty::IntTy::I32) | ty::Uint(ty::UintTy::U32)
    )
}

fn body_type(value: Ty<'_>, depth: usize) -> bool {
    if depth >= 8 {
        return false;
    }

    integer(value)
        || match value.kind() {
            ty::Bool | ty::Never => true,
            ty::Tuple(elements) => elements.is_empty(),
            ty::Ref(_, referent, rustc_hir::Mutability::Not) => body_type(*referent, depth + 1),
            _ => false,
        }
}
