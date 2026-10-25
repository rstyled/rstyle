//! A compiler-selected builtin binary tuple constructor, never an arbitrary function.
use super::*;

pub(super) struct Constructor<'tcx> {
    pub definition: DefId,
    pub owner: ty::AdtDef<'tcx>,
    pub scalar: Argument,
    pub signature: ty::FnSig<'tcx>,
}

impl<'tcx> Constructor<'tcx> {
    pub(super) fn capture(ctx: &Context<'tcx>, callee: &hir::Expr<'tcx>) -> Option<Self> {
        let ExprKind::Path(ref path) = callee.kind else {
            return None;
        };
        let ty::FnDef(definition, arguments) = ctx.typeck.expr_ty(callee).kind() else {
            return None;
        };
        if ctx.typeck.qpath_res(path, callee.hir_id)
            != Res::Def(DefKind::Ctor(CtorOf::Struct, CtorKind::Fn), *definition)
            || !definition.is_local()
        {
            return None;
        }

        let arguments = ctx.tcx.erase_and_anonymize_regions(arguments.skip_binder());
        let scalar = super::super::concrete_types::scalar_argument(arguments)?;
        let bound = ctx.tcx.fn_sig(*definition).instantiate(ctx.tcx, arguments);
        if !bound.skip_normalization().bound_vars().is_empty() {
            return None;
        }

        let signature = ctx
            .tcx
            .try_normalize_erasing_regions(ty::TypingEnv::fully_monomorphized(), bound)
            .ok()?
            .skip_binder();
        let owner = owner(ctx, *definition, arguments, signature)?;
        Some(Self {
            definition: *definition,
            owner,
            scalar,
            signature,
        })
    }
}

fn owner<'tcx>(
    ctx: &Context<'tcx>,
    constructor: DefId,
    arguments: GenericArgsRef<'tcx>,
    signature: ty::FnSig<'tcx>,
) -> Option<ty::AdtDef<'tcx>> {
    let [first, second] = signature.inputs() else {
        return None;
    };
    let ty::Adt(owner, output_arguments) = signature.output().kind() else {
        return None;
    };
    if !owner.is_struct()
        || owner.did() != ctx.tcx.parent(constructor)
        || owner.non_enum_variant().ctor_def_id() != Some(constructor)
        || owner.non_enum_variant().fields.len() != 2
        || *output_arguments != arguments
        || signature.c_variadic()
        || signature.safety().is_unsafe()
        || signature.abi() != rustc_abi::ExternAbi::Rust
        || !matches!(first.kind(), ty::Ref(_, inner, hir::Mutability::Not)
            if matches!(inner.kind(), ty::Slice(element) if *element == ctx.tcx.types.u8))
        || *second != arguments.type_at(1)
    {
        return None;
    }

    for (field, input) in owner.non_enum_variant().fields.iter().zip([first, second]) {
        let actual = ctx
            .tcx
            .try_normalize_erasing_regions(
                ty::TypingEnv::fully_monomorphized(),
                field.ty(ctx.tcx, arguments),
            )
            .ok()?;
        if actual != *input {
            return None;
        }
    }
    Some(*owner)
}
