//! Observations only: selected generic calls do not certify instantiated bodies.
use super::hir::{Context, reference};
use rstyle_pdg::{HirFnFacts, extraction_profile::ExtractionProfile};
use rustc_hir::{
    self as hir, Expr, ExprKind,
    def_id::DefId,
    intravisit::{self, Visitor},
};
use rustc_middle::ty::{self, GenericArgsRef, TyCtxt, TypeVisitableExt};
use serde::Serialize;

mod adjustments;
mod borrowed_operations;
mod calls;
mod comparisons;
pub(super) mod diagnostic;
mod diagnostic_support;
mod projection;
mod provenance;
pub(super) mod publication;
pub(super) mod reference_diagnostic;
mod reference_owners;
pub(super) use publication::capture as capture_projections;
mod types;

const KEY: &str = "rstyle.rust.call-instances";

pub(super) fn capture(
    context: &Context<'_>,
    owner: hir::def_id::LocalDefId,
    mut facts: HirFnFacts,
) -> Result<HirFnFacts, String> {
    if context.profile != ExtractionProfile::DataV3 {
        return Ok(facts);
    }
    let node = context.tcx.hir_node_by_def_id(owner);
    let body = context
        .tcx
        .hir_body(node.body_id().ok_or("Missing instance observation body")?);
    let mut scan = Scan {
        context,
        calls: Vec::new(),
    };
    scan.visit_expr(body.value);

    let inventory = Inventory {
        identity: "rstyle.rust.call-instance-observations/1",
        scope: "owning-body direct FnDef and method generic/trait calls; closure/function-pointer calls and nested owners excluded",
        complete: true,
        calls: scan.calls,
    };
    let value = serde_json::to_value(inventory).map_err(|error| error.to_string())?;
    facts.build.frontend_options.insert(KEY.into(), value);
    Ok(facts)
}

#[derive(Serialize)]
struct Inventory {
    identity: &'static str,
    scope: &'static str,
    complete: bool,
    calls: Vec<Observation>,
}

#[derive(Serialize)]
struct Observation {
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    expanded: bool,
    requested: Definition,
    requested_arguments: types::Arguments,
    #[serde(skip_serializing_if = "Option::is_none")]
    compiled_dependency: Option<super::crate_association::Dependency>,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Serialize)]
pub(super) struct Definition {
    path: String,
    definition_identity: String,
    #[serde(rename = "crate")]
    crate_name: String,
    stable_crate_id: String,
    local: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Resolved,
    Unresolved,
    Error,
    Unsupported,
}

#[derive(Serialize)]
struct Outcome {
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected: Option<Definition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_arguments: Option<types::Arguments>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<Signature>,
}

#[derive(Serialize)]
struct Signature {
    display_only: bool,
    inputs: Vec<String>,
    output: String,
}

struct Scan<'a, 'tcx> {
    context: &'a Context<'tcx>,
    calls: Vec<Observation>,
}

impl<'v> Visitor<'v> for Scan<'_, '_> {
    fn visit_expr(&mut self, expression: &'v Expr<'v>) {
        // These bodies have their own typeck owner and must never inherit this one.
        if matches!(
            expression.kind,
            ExprKind::Closure(_) | ExprKind::ConstBlock(_)
        ) {
            return;
        }
        if let Some((definition, arguments)) = self.call(expression) {
            self.calls
                .push(observe(self.context, expression, definition, arguments));
        }
        intravisit::walk_expr(self, expression);
    }
}

impl<'tcx> Scan<'_, 'tcx> {
    fn call(&self, expression: &Expr<'_>) -> Option<(DefId, GenericArgsRef<'tcx>)> {
        let (definition, arguments) = match expression.kind {
            ExprKind::Call(callee, _) => {
                let ty::FnDef(definition, arguments) = self.context.typeck.expr_ty(callee).kind()
                else {
                    return None;
                };
                (*definition, arguments.skip_binder())
            }
            ExprKind::MethodCall(..) => (
                self.context
                    .typeck
                    .type_dependent_def_id(expression.hir_id)?,
                self.context.typeck.node_args(expression.hir_id),
            ),
            _ => return None,
        };
        let trait_member = self.context.tcx.trait_of_assoc(definition).is_some()
            || self.context.tcx.trait_item_of(definition).is_some();
        (trait_member
            || arguments
                .iter()
                .any(|arg| !matches!(arg.kind(), ty::GenericArgKind::Lifetime(_))))
        .then_some((definition, arguments))
    }
}

fn observe<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'_>,
    definition: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> Observation {
    let environment = ty::TypingEnv::post_analysis(context.tcx, expression.hir_id.owner.def_id);
    let (requested_arguments, outcome, compiled_dependency) =
        match select(context.tcx, environment, definition, arguments) {
            Ok((arguments, instance)) => {
                let requested = types::arguments(context.tcx, arguments);
                let outcome =
                    selected_outcome(context.tcx, environment, instance, requested.complete());
                let artifact = (matches!(outcome.status, Status::Resolved)
                    && instance.def_id() == definition)
                    .then(|| {
                        super::crate_association::scalar_dependency(
                            context.tcx,
                            instance.def_id(),
                            instance.args,
                        )
                    })
                    .flatten();
                (requested, outcome, artifact)
            }
            Err((status, reason)) => (
                types::arguments(context.tcx, arguments),
                failure(status, reason),
                None,
            ),
        };

    Observation {
        hir: reference(expression.hir_id),
        span: context.span(expression.span),
        expanded: expression.span.from_expansion(),
        requested: definition_record(context.tcx, definition),
        requested_arguments,
        compiled_dependency,
        outcome,
    }
}

fn select<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    definition: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> Result<(GenericArgsRef<'tcx>, ty::Instance<'tcx>), (Status, &'static str)> {
    let arguments = tcx
        .try_normalize_erasing_regions(
            environment,
            ty::EarlyBinder::bind(tcx, arguments).instantiate_identity(),
        )
        .map_err(|_| (Status::Error, "requested argument normalization failed"))?;
    if !concrete(arguments) {
        return Err((
            Status::Unresolved,
            "requested arguments remain generic or inferred",
        ));
    }

    let instance = ty::Instance::try_resolve(tcx, environment, definition, arguments)
        .map_err(|_| (Status::Error, "compiler instance resolution failed"))?
        .ok_or((Status::Unresolved, "compiler did not select an instance"))?;
    if !matches!(instance.def, ty::InstanceKind::Item(_)) {
        return Err((
            Status::Unsupported,
            "selected instance is not an ordinary Item",
        ));
    }

    if !concrete(instance.args) {
        return Err((
            Status::Unresolved,
            "selected arguments remain generic or inferred",
        ));
    }
    Ok((arguments, instance))
}

fn concrete(arguments: GenericArgsRef<'_>) -> bool {
    !arguments.has_non_region_param()
        && !arguments.has_infer()
        && !arguments.has_escaping_bound_vars()
}

fn selected_outcome<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
    requested_complete: bool,
) -> Outcome {
    let selected_arguments = types::arguments(tcx, instance.args);
    let result = if requested_complete && selected_arguments.complete() {
        signature(tcx, environment, instance)
    } else {
        Err((
            Status::Unsupported,
            "instance argument structural identity is incomplete",
        ))
    };
    let (status, reason, signature) = match result {
        Ok(signature) => (Status::Resolved, None, Some(signature)),
        Err((status, reason)) => (status, Some(reason), None),
    };

    Outcome {
        status,
        reason,
        selected: Some(definition_record(tcx, instance.def_id())),
        selected_arguments: Some(selected_arguments),
        signature,
    }
}

fn signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
) -> Result<Signature, (Status, &'static str)> {
    let signature = tcx
        .try_normalize_erasing_regions(
            environment,
            tcx.fn_sig(instance.def_id())
                .instantiate(tcx, instance.args),
        )
        .map_err(|_| (Status::Error, "selected signature normalization failed"))?
        .skip_binder();

    Ok(Signature {
        display_only: true,
        inputs: signature.inputs().iter().map(ToString::to_string).collect(),
        output: signature.output().to_string(),
    })
}

fn failure(status: Status, reason: &'static str) -> Outcome {
    Outcome {
        status,
        reason: Some(reason),
        selected: None,
        selected_arguments: None,
        signature: None,
    }
}

fn definition_record(tcx: TyCtxt<'_>, definition: DefId) -> Definition {
    Definition {
        path: tcx.def_path_str(definition),
        definition_identity: format!("{:?}", tcx.def_path_hash(definition)),
        crate_name: tcx.crate_name(definition.krate).to_string(),
        stable_crate_id: format!("{:?}", tcx.stable_crate_id(definition.krate)),
        local: definition.is_local(),
    }
}
