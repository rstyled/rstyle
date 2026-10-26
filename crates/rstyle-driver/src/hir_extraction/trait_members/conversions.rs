use super::{errors, model};
use crate::hir_extraction::{
    hir::Context,
    instances::{definition_record, types},
    module_expansions::file_span,
};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::{self, GenericArgsRef, TyCtxt, TypeVisitableExt};
use rustc_span::{Symbol, sym};

pub(super) fn observe(
    context: &Context<'_>,
    owner: LocalDefId,
    generated: bool,
) -> Result<model::Observation, String> {
    let tcx = context.tcx;
    let method = tcx
        .associated_item(owner.to_def_id())
        .trait_item_def_id()
        .ok_or("Missing conversion trait member")?;
    let implementation = tcx
        .impl_of_assoc(owner.to_def_id())
        .ok_or("Missing conversion implementation")?;

    if tcx.generics_of(owner).count() != 0 || !implementation.is_local() {
        return Err("Conversion observation requires a concrete local implementation".into());
    }
    let reference = tcx
        .impl_trait_ref(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    check_trait(tcx, reference.def_id, method)?;

    let environment = ty::TypingEnv::post_analysis(tcx, owner);
    let selected = selected(tcx, environment, method, reference.args)?;
    if selected.def_id() != owner.to_def_id() {
        return Err("Conversion trait selects a different owning method".into());
    }
    observation(
        context,
        Association {
            owner,
            implementation,
            method,
            reference,
            selected,
        },
        generated,
    )
}

fn check_trait(tcx: TyCtxt<'_>, definition: DefId, method: DefId) -> Result<(), String> {
    let expected = if tcx.get_diagnostic_item(sym::From) == Some(definition) {
        "from_fn"
    } else if tcx.get_diagnostic_item(Symbol::intern("TryFrom")) == Some(definition) {
        "try_from_fn"
    } else if tcx.get_diagnostic_item(Symbol::intern("Default")) == Some(definition) {
        "default_fn"
    } else {
        return Err("Conversion trait lacks exact core diagnostic identity".into());
    };
    if definition.is_local()
        || tcx.crate_name(definition.krate).as_str() != "core"
        || tcx.get_diagnostic_item(Symbol::intern(expected)) != Some(method)
        || tcx.trait_of_assoc(method) != Some(definition)
    {
        return Err("Conversion method differs from its core trait declaration".into());
    }
    Ok(())
}

pub(super) fn selected<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    method: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> Result<ty::Instance<'tcx>, String> {
    if arguments.has_non_region_param()
        || arguments.has_infer()
        || arguments.has_escaping_bound_vars()
    {
        return Err("Conversion trait arguments are not concrete".into());
    }
    let selected = ty::Instance::try_resolve(tcx, environment, method, arguments)
        .map_err(|_| "Conversion instance resolution failed")?
        .ok_or("Conversion instance unresolved")?;
    if !matches!(selected.def, ty::InstanceKind::Item(_)) {
        return Err("Conversion instance is not an ordinary item".into());
    }
    Ok(selected)
}

struct Association<'tcx> {
    owner: LocalDefId,
    implementation: DefId,
    method: DefId,
    reference: ty::TraitRef<'tcx>,
    selected: ty::Instance<'tcx>,
}

fn observation<'tcx>(
    context: &Context<'tcx>,
    association: Association<'tcx>,
    generated: bool,
) -> Result<model::Observation, String> {
    let Association {
        owner,
        implementation,
        method,
        reference,
        selected,
    } = association;
    let tcx = context.tcx;
    let trait_arguments = types::arguments(tcx, reference.args);
    let selected_arguments = types::arguments(tcx, selected.args);

    if !trait_arguments.complete() || !selected_arguments.complete() {
        return Err("Conversion arguments lack complete structural identity".into());
    }
    let method_span = method_span(context, owner, implementation, generated)?;
    Ok(model::Observation {
        identity: "rstyle.rust.concrete-conversion-member-observation/1",
        admission_authority: false,
        method: definition_record(tcx, owner.to_def_id()),
        implementation: definition_record(tcx, implementation),
        trait_definition: definition_record(tcx, reference.def_id),
        trait_method: definition_record(tcx, method),
        implementation_source: file_span(
            tcx,
            &context.sources,
            super::source_span(tcx, implementation, generated)?,
        )?,
        method_source: file_span(tcx, &context.sources, method_span)?,
        method_header: header(tcx, owner)?,
        members: members(context, implementation, generated)?,
        trait_arguments,
        selected: definition_record(tcx, selected.def_id()),
        selected_arguments,
        requested_signature: signature_observation(tcx, owner, method, reference.args)?,
        selected_signature: signature_observation(tcx, owner, selected.def_id(), selected.args)?,
        associated_error: errors::observe(context, owner, reference, generated)?,
    })
}

fn method_span(
    context: &Context<'_>,
    owner: LocalDefId,
    implementation: DefId,
    generated: bool,
) -> Result<rustc_span::Span, String> {
    let tcx = context.tcx;
    let node = tcx.hir_node_by_def_id(owner);
    let signature = node.fn_sig().ok_or("Conversion method lacks signature")?;
    let body = tcx.hir_body(node.body_id().ok_or("Conversion method lacks body")?);

    let span = signature.span.to(body.value.span);
    if !generated && (span.from_expansion() || tcx.def_span(implementation).from_expansion()) {
        return Err("Generated conversion member source remains unsupported".into());
    }
    Ok(span)
}

fn members(
    context: &Context<'_>,
    implementation: DefId,
    generated: bool,
) -> Result<Vec<model::Member>, String> {
    context
        .tcx
        .associated_items(implementation)
        .in_definition_order()
        .map(|item| {
            Ok(model::Member {
                definition: definition_record(context.tcx, item.def_id),
                trait_member: item
                    .trait_item_def_id()
                    .map(|id| definition_record(context.tcx, id)),
                source: file_span(
                    context.tcx,
                    &context.sources,
                    super::source_span(context.tcx, item.def_id, generated)?,
                )?,
            })
        })
        .collect()
}

pub(super) fn signature_observation<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
    method: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> Result<model::Signature, String> {
    let original = tcx.fn_sig(method).instantiate(tcx, arguments);
    let bound_variables = original.skip_normalization().bound_vars().len();
    let signature = tcx
        .try_normalize_erasing_regions(ty::TypingEnv::post_analysis(tcx, owner), original)
        .map_err(|_| "Conversion signature normalization failed")?
        .skip_binder();
    Ok(model::Signature {
        inputs: signature
            .inputs()
            .iter()
            .map(|value| types::endpoint(tcx, *value))
            .collect::<Result<_, _>>()?,
        output: types::endpoint(tcx, signature.output())?,
        bound_variables,
        abi: signature.abi().to_string(),
        safety: format!("{:?}", signature.safety()),
        variadic: signature.c_variadic(),
    })
}

fn header(tcx: TyCtxt<'_>, owner: LocalDefId) -> Result<model::Header, String> {
    let node = tcx.hir_node_by_def_id(owner);
    let signature = node
        .fn_sig()
        .ok_or("Conversion method lacks source header")?;
    Ok(model::Header {
        is_const: matches!(
            signature.header.constness,
            rustc_hir::Constness::Const { .. }
        ),
        is_async: signature.header.is_async(),
    })
}
