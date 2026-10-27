//! Use the constructor's own HIR/typeck; reject any additional computation.
use super::{CaptureFailure, Context, identity};
use rstyle_pdg::{concrete_types::TypeTree, nominal_constants::Constructor};
use rustc_hir::{
    self as hir, Body, ExprKind, HirId, Node,
    def::{DefKind, Res},
    def_id::{DefId, LocalDefId},
};
use rustc_middle::ty::Ty;

pub(super) fn capture<'tcx>(
    context: &Context<'tcx>,
    definition: DefId,
    owner: Ty<'tcx>,
    observed: &TypeTree,
) -> Result<Constructor, CaptureFailure> {
    let (implementation, implementation_source) = identity::implementation(context, definition)?;
    if context
        .tcx
        .type_of(implementation)
        .instantiate_identity()
        .skip_norm_wip()
        != owner
    {
        return Err(CaptureFailure::Unsupported(
            "constructor belongs to a different product",
        ));
    }
    let (body, binding) = signature(context, definition, owner)?;
    let tuple_constructor = tuple(context, body, binding, owner)?;
    let parameter_span = identity::original_range(context, body.params[0].pat.span)?;
    let result_type = rstyle_pdg::data_types::token(observed).map_err(|_| {
        CaptureFailure::Unsupported("constructor result lacks its complete type token")
    })?;

    Ok(Constructor {
        definition: identity::local(context, definition)?,
        implementation: identity::local(context, implementation)?,
        declaration: identity::declaration(context, definition)?,
        implementation_source,
        parameter_span,
        tuple_constructor: identity::local(context, tuple_constructor)?,
        parameter_type: context.tcx.types.usize.to_string(),
        result_type,
        field_type: context.tcx.types.u32.to_string(),
    })
}

fn signature<'tcx>(
    context: &Context<'tcx>,
    definition: DefId,
    owner: Ty<'tcx>,
) -> Result<(&'tcx Body<'tcx>, HirId), CaptureFailure> {
    let tcx = context.tcx;
    let local = definition
        .as_local()
        .ok_or(CaptureFailure::Unsupported("constructor is nonlocal"))?;
    let Node::ImplItem(item) = tcx.hir_node_by_def_id(local) else {
        return Err(CaptureFailure::Unsupported(
            "constructor lacks its original impl member",
        ));
    };
    let hir::ImplItemKind::Fn(signature, _) = item.kind else {
        return Err(CaptureFailure::Unsupported(
            "selected constructor is not a function",
        ));
    };

    if tcx.def_kind(definition) != DefKind::AssocFn
        || tcx.generics_of(definition).count() != 0
        || !tcx.is_const_fn(definition)
        || signature.header.is_unsafe()
        || signature.header.is_async()
        || signature.header.abi != rustc_abi::ExternAbi::Rust
        || signature.decl.c_variadic()
    {
        return Err(CaptureFailure::Unsupported(
            "constructor is not an ordinary safe const function",
        ));
    }

    let observed = tcx
        .fn_sig(definition)
        .instantiate_identity()
        .skip_norm_wip()
        .skip_binder();
    if observed.inputs() != [tcx.types.usize] || observed.output() != owner {
        return Err(CaptureFailure::Unsupported(
            "constructor signature differs from usize to product",
        ));
    }

    parameter(context, local)
}

fn parameter<'tcx>(
    context: &Context<'tcx>,
    local: LocalDefId,
) -> Result<(&'tcx Body<'tcx>, HirId), CaptureFailure> {
    let body = context
        .tcx
        .hir_maybe_body_owned_by(local)
        .ok_or(CaptureFailure::Unsupported(
            "constructor body is unavailable",
        ))?;
    let [parameter] = body.params else {
        return Err(CaptureFailure::Unsupported(
            "constructor requires exactly one parameter",
        ));
    };
    let hir::PatKind::Binding(
        hir::BindingMode(hir::ByRef::No, hir::Mutability::Not),
        binding,
        _,
        None,
    ) = parameter.pat.kind
    else {
        return Err(CaptureFailure::Unsupported(
            "constructor parameter is not an immutable value binding",
        ));
    };
    Ok((body, binding))
}

fn tuple<'tcx>(
    context: &Context<'tcx>,
    body: &'tcx Body<'tcx>,
    binding: HirId,
    owner: Ty<'tcx>,
) -> Result<DefId, CaptureFailure> {
    let value = tail(context, body)?;
    let ExprKind::Call(callee, [cast]) = value.kind else {
        return Err(CaptureFailure::Unsupported(
            "constructor tail is not one tuple field construction",
        ));
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return Err(CaptureFailure::Unsupported(
            "tuple constructor lacks an original path",
        ));
    };

    let typeck = context.tcx.typeck(body.value.hir_id.owner.def_id);
    let Res::Def(DefKind::Ctor(..), selected) = typeck.qpath_res(path, callee.hir_id) else {
        return Err(CaptureFailure::Unsupported(
            "product field call does not resolve to a tuple constructor",
        ));
    };
    let rustc_middle::ty::Adt(adt, _) = owner.kind() else {
        return Err(CaptureFailure::Unsupported(
            "constructor owner is not an ADT",
        ));
    };
    if adt.non_enum_variant().ctor_def_id() != Some(selected)
        || typeck.node_type(value.hir_id) != owner
    {
        return Err(CaptureFailure::Unsupported(
            "tuple constructor selects a different owner",
        ));
    }

    field_cast(context, cast, binding)?;
    identity::original_range(context, callee.span)?;
    Ok(selected)
}

fn tail<'tcx>(
    context: &Context<'tcx>,
    body: &'tcx Body<'tcx>,
) -> Result<&'tcx hir::Expr<'tcx>, CaptureFailure> {
    let ExprKind::Block(block, None) = body.value.kind else {
        return Err(CaptureFailure::Unsupported(
            "constructor requires one original block",
        ));
    };
    if !block.stmts.is_empty() || !matches!(block.rules, hir::BlockCheckMode::DefaultBlock) {
        return Err(CaptureFailure::Unsupported(
            "constructor block contains additional work or unsafe rules",
        ));
    }

    let value = block.expr.ok_or(CaptureFailure::Unsupported(
        "constructor tail is unavailable",
    ))?;
    identity::original_range(context, body.value.span)?;
    identity::original_range(context, value.span)?;

    Ok(value)
}

fn field_cast(
    context: &Context<'_>,
    cast: &hir::Expr<'_>,
    binding: HirId,
) -> Result<(), CaptureFailure> {
    let ExprKind::Cast(operand, _) = cast.kind else {
        return Err(CaptureFailure::Unsupported(
            "tuple field requires the original u32 cast",
        ));
    };
    let ExprKind::Path(ref path) = operand.kind else {
        return Err(CaptureFailure::Unsupported(
            "tuple field does not use the original parameter",
        ));
    };

    let typeck = context.tcx.typeck(cast.hir_id.owner.def_id);
    if typeck.node_type(cast.hir_id) != context.tcx.types.u32
        || typeck.node_type(operand.hir_id) != context.tcx.types.usize
        || typeck.qpath_res(path, operand.hir_id) != Res::Local(binding)
    {
        return Err(CaptureFailure::Unsupported(
            "tuple field cast or parameter identity differs",
        ));
    }

    identity::original_range(context, cast.span)?;
    identity::original_range(context, operand.span)?;
    Ok(())
}
