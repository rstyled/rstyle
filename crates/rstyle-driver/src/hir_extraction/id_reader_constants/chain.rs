//! Actual rustc selected definitions, not constants inferred from displayed names.
use super::super::{
    generated_sources, hir::Context, instances::definition_record, library_resolution,
};
use super::identity;
use rstyle_pdg::{
    generated_sources::capture::{Definition, Origin},
    id_reader_constants::{IDENTITY, Layout, Observation},
};
use rustc_hir::{
    ExprKind, Node,
    def::{DefKind, Res},
    def_id::DefId,
};
use rustc_middle::ty::{self, Ty};

pub(super) fn capture(context: &Context<'_>, resolution: Res) -> Option<Observation> {
    let Res::Def(DefKind::AssocConst, definition) = resolution else {
        return None;
    };
    let (implementation, owner) = constant_owner(context, definition)?;
    let generated = generated(context, definition, implementation)?;
    let original = dependency(context, definition)?;
    let (original_implementation, size_argument) = constant_owner(context, original)?;
    let (size_of, argument) = initializer(context, original)?;
    if argument != size_argument {
        return None;
    }

    Some(Observation {
        identity: IDENTITY.into(),
        definition: generated.definition,
        implementation: generated.implementation,
        origin: generated.origin,
        implementation_origin: generated.implementation_origin,
        defining_expansion: generated.defining_expansion,
        owner: definition_record(context.tcx, owner),
        owner_source: identity::declaration(context, owner)?,
        original: definition_record(context.tcx, original),
        original_source: identity::declaration(context, original)?,
        original_implementation: definition_record(context.tcx, original_implementation),
        original_implementation_source: identity::declaration(context, original_implementation)?,
        size_of: library_resolution::definition(context.tcx, size_of),
        size_argument: definition_record(context.tcx, argument),
        size_argument_source: identity::declaration(context, argument)?,
        layout: layout(context, original, argument)?,
        value: value(context, definition)
            .filter(|evaluated| Some(*evaluated) == value(context, original))?,
    })
}

struct Generated {
    definition: Definition,
    implementation: Definition,
    origin: Origin,
    implementation_origin: Origin,
    defining_expansion: String,
}

fn generated(context: &Context<'_>, definition: DefId, implementation: DefId) -> Option<Generated> {
    let node = context.tcx.hir_node_by_def_id(definition.as_local()?);
    let Node::ImplItem(item) = node else {
        return None;
    };
    if !item.span.from_expansion() {
        return None;
    }
    let (selected, origin, defining_expansion) =
        generated_sources::declaration_origin(context, definition, item.span).ok()?;
    let Node::Item(implementation_item) =
        context.tcx.hir_node_by_def_id(implementation.as_local()?)
    else {
        return None;
    };
    let (implementation_record, implementation_origin, implementation_expansion) =
        generated_sources::declaration_origin(context, implementation, implementation_item.span)
            .ok()?;
    if implementation_expansion != defining_expansion {
        return None;
    }

    Some(Generated {
        definition: selected,
        implementation: implementation_record,
        origin,
        implementation_origin,
        defining_expansion,
    })
}

fn layout(context: &Context<'_>, original: DefId, argument: DefId) -> Option<Layout> {
    let ty = context
        .tcx
        .type_of(argument)
        .instantiate_identity()
        .skip_norm_wip();
    let layout = context
        .tcx
        .layout_of(ty::TypingEnv::post_analysis(context.tcx, original).as_query_input(ty))
        .ok()?;
    Some(Layout {
        size: layout.size.bytes(),
        align: layout.align.abi.bytes(),
    })
}

fn constant_owner(context: &Context<'_>, definition: DefId) -> Option<(DefId, DefId)> {
    if !context.local_constant(context.tcx.def_kind(definition), definition)
        || context
            .tcx
            .type_of(definition)
            .instantiate_identity()
            .skip_norm_wip()
            != context.tcx.types.usize
    {
        return None;
    }
    let implementation = context.tcx.parent(definition);
    let ty = context
        .tcx
        .type_of(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    Some((implementation, owner(ty)?))
}

fn owner(ty: Ty<'_>) -> Option<DefId> {
    let ty::Adt(owner, arguments) = ty.kind() else {
        return None;
    };
    (owner.is_struct() && owner.did().is_local() && arguments.is_empty()).then_some(owner.did())
}

fn dependency(context: &Context<'_>, definition: DefId) -> Option<DefId> {
    let body = context
        .tcx
        .hir_maybe_body_owned_by(definition.as_local()?)?;
    let ExprKind::Path(ref path) = body.value.kind else {
        return None;
    };

    let typeck = context.tcx.typeck(definition.as_local()?);
    let Res::Def(DefKind::AssocConst, original) = typeck.qpath_res(path, body.value.hir_id) else {
        return None;
    };
    Some(original)
}

fn initializer(context: &Context<'_>, definition: DefId) -> Option<(DefId, DefId)> {
    let body = context
        .tcx
        .hir_maybe_body_owned_by(definition.as_local()?)?;
    if body.value.span.from_expansion() {
        return None;
    }
    let ExprKind::Call(callee, []) = body.value.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };

    let typeck = context.tcx.typeck(definition.as_local()?);
    let Res::Def(DefKind::Fn, size_of) = typeck.qpath_res(path, callee.hir_id) else {
        return None;
    };
    let arguments = typeck.node_args(callee.hir_id);
    if arguments.len() != 1 || size_of.is_local() {
        return None;
    }
    Some((size_of, owner(arguments.type_at(0))?))
}

fn value(context: &Context<'_>, definition: DefId) -> Option<u64> {
    let scalar = context
        .tcx
        .const_eval_poly(definition)
        .ok()?
        .try_to_scalar_int()?;
    u64::try_from(scalar.to_bits(scalar.size())).ok()
}
