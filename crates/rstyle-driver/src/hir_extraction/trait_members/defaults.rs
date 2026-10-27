//! Exact concrete Default selections; body/source/effect permission remains separate.
use super::{Outcome, conversions, model};
use crate::hir_extraction::{
    hir::{self, Context},
    instances::{definition_record, types},
};
use rstyle_pdg::{ByteSpan, HirFnFacts, HirRef, PathRes};
use rustc_hir::{
    Expr, ExprKind, Node,
    def_id::{DefId, LocalDefId},
};
use rustc_middle::ty;
use rustc_span::Symbol;
use serde::Serialize;

const KEY: &str = "rstyle.rust.concrete-default-member";
const CAPABILITY: &str = "rust.checked-concrete-default";

pub(super) fn capture(
    context: &Context<'_>,
    owner: LocalDefId,
    facts: &mut HirFnFacts,
) -> Result<(), String> {
    let outcome = match conversions::observe(context, owner, false) {
        Ok(mut observation) => {
            observation.identity = "rstyle.rust.concrete-default-member-observation/1";
            Outcome::Observed {
                observation: Box::new(observation),
            }
        }
        Err(reason) => Outcome::Unsupported { reason },
    };
    facts.build.frontend_options.insert(
        KEY.into(),
        serde_json::to_value(outcome).map_err(|error| error.to_string())?,
    );
    Ok(())
}

#[derive(Serialize)]
struct Call {
    identity: &'static str,
    admission_authority: bool,
    hir: HirRef,
    span: ByteSpan,
    requested: super::instances::Definition,
    trait_definition: super::instances::Definition,
    requested_arguments: types::Arguments,
    selected: super::instances::Definition,
    implementation: super::instances::Definition,
    selected_arguments: types::Arguments,
    requested_signature: model::Signature,
    selected_signature: model::Signature,
}

/// `expression` is the callee path passed to Context::direct_callee.
/// The exact parent call must be direct, original and have zero arguments.
pub(in crate::hir_extraction) fn default_call<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'_>,
    requested: DefId,
) -> Option<PathRes> {
    if !context.profile.supports(CAPABILITY) {
        return None;
    }

    let call = parent_call(context, expression)?;
    let (observed, selected) = observe_call(context, expression, call, requested).ok()?;

    let mut resolution = context.helper_callee(selected.def_id(), "resolved_direct_callee", None);
    let mut record: serde_json::Value = serde_json::from_str(&resolution.description).ok()?;
    record["default_selection"] = serde_json::to_value(observed).ok()?;
    resolution.description = record.to_string();
    Some(resolution)
}

fn parent_call<'tcx>(context: &Context<'tcx>, expression: &Expr<'_>) -> Option<&'tcx Expr<'tcx>> {
    let Node::Expr(parent) = context.tcx.parent_hir_node(expression.hir_id) else {
        return None;
    };
    let ExprKind::Call(callee, operands) = parent.kind else {
        return None;
    };
    (callee.hir_id == expression.hir_id
        && operands.is_empty()
        && !parent.span.from_expansion()
        && !expression.span.from_expansion())
    .then_some(parent)
}

fn selected_definition<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'_>,
    requested: DefId,
) -> Result<ty::Instance<'tcx>, String> {
    let environment = ty::TypingEnv::post_analysis(context.tcx, expression.hir_id.owner.def_id);
    conversions::selected(
        context.tcx,
        environment,
        requested,
        context.typeck.node_args(expression.hir_id),
    )
}

fn observe_call<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'_>,
    call: &Expr<'tcx>,
    requested: DefId,
) -> Result<(Call, ty::Instance<'tcx>), String> {
    let tcx = context.tcx;
    let trait_definition = tcx
        .get_diagnostic_item(Symbol::intern("Default"))
        .ok_or("Missing Default diagnostic trait")?;
    if requested.is_local()
        || tcx.crate_name(requested.krate).as_str() != "core"
        || tcx.get_diagnostic_item(Symbol::intern("default_fn")) != Some(requested)
        || tcx.trait_of_assoc(requested) != Some(trait_definition)
    {
        return Err("Default call lacks exact core trait/member identity".into());
    }

    let selected = selected_definition(context, expression, requested)?;
    let implementation = check_selected(context, call, requested, selected)?;
    let observation = call_record(
        context,
        expression,
        call,
        Selection {
            requested,
            selected,
            implementation,
        },
    )?;
    Ok((observation, selected))
}

fn check_selected<'tcx>(
    context: &Context<'tcx>,
    call: &Expr<'tcx>,
    requested: DefId,
    selected: ty::Instance<'tcx>,
) -> Result<DefId, String> {
    let tcx = context.tcx;
    let Some(owner) = selected.def_id().as_local() else {
        return Err("Default call selected nonlocal body".into());
    };
    if !super::candidate(tcx, owner, &["default_fn"])
        || tcx.generics_of(owner).count() != 0
        || tcx.associated_item(owner.to_def_id()).trait_item_def_id() != Some(requested)
    {
        return Err("Default call requires its concrete original local member".into());
    }

    let implementation = tcx
        .impl_of_assoc(owner.to_def_id())
        .ok_or("Default member has no impl")?;
    super::source_span(tcx, owner.to_def_id(), false)?;
    super::source_span(tcx, implementation, false)?;
    let reference = tcx
        .impl_trait_ref(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    if reference.args.len() != 1 || reference.self_ty() != context.typeck.expr_ty(call) {
        return Err("Default selected Self differs from actual call result".into());
    }
    Ok(implementation)
}

struct Selection<'tcx> {
    requested: DefId,
    selected: ty::Instance<'tcx>,
    implementation: DefId,
}

fn call_record<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'_>,
    call: &Expr<'tcx>,
    selection: Selection<'tcx>,
) -> Result<Call, String> {
    let Selection {
        requested,
        selected,
        implementation,
    } = selection;

    let tcx = context.tcx;
    let arguments = context.typeck.node_args(expression.hir_id);
    let requested_arguments = types::arguments(tcx, arguments);
    let selected_arguments = types::arguments(tcx, selected.args);
    if arguments.len() != 1 || !requested_arguments.complete() || !selected_arguments.complete() {
        return Err("Default call lacks its complete concrete Self identity".into());
    }
    let owner = expression.hir_id.owner.def_id;
    Ok(Call {
        identity: "rstyle.rust.concrete-default-call-observation/1",
        admission_authority: false,
        hir: hir::reference(call.hir_id),
        span: context.span(call.span),
        requested: definition_record(tcx, requested),
        trait_definition: definition_record(
            tcx,
            tcx.trait_of_assoc(requested)
                .ok_or("Missing Default trait")?,
        ),
        requested_arguments,
        selected: definition_record(tcx, selected.def_id()),
        implementation: definition_record(tcx, implementation),
        selected_arguments,
        requested_signature: conversions::signature_observation(tcx, owner, requested, arguments)?,
        selected_signature: conversions::signature_observation(
            tcx,
            owner,
            selected.def_id(),
            selected.args,
        )?,
    })
}
