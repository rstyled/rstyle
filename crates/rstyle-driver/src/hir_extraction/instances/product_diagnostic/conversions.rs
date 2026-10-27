//! Complete selected Into/From observations; neither name nor trait implies purity.
use super::*;
use rustc_hir::{Expr, ExprKind, def_id::DefId};
use rustc_middle::ty::GenericArgsRef;
use rustc_span::{Symbol, sym};

#[derive(Serialize)]
pub(super) struct Observation {
    identity: &'static str,
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    receiver: rstyle_pdg::HirRef,
    receiver_type: types::Arguments,
    result_type: types::Arguments,
    adjustments: Vec<Adjustment>,
    from_selection_basis: &'static str,
    into: Selection,
    from: Selection,
}

#[derive(Serialize)]
struct Adjustment {
    kind: String,
    target: types::Arguments,
}

#[derive(Serialize)]
struct Selection {
    requested: CompilerDefinition,
    requested_trait: CompilerDefinition,
    requested_arguments: types::Arguments,
    selected: CompilerDefinition,
    selected_impl: CompilerDefinition,
    selected_trait: CompilerDefinition,
    selected_arguments: types::Arguments,
    original_impl_trait_arguments: Vec<projection::OriginalType>,
    substituted_impl_trait_arguments: types::Arguments,
    requested_signature: Signature,
    selected_signature: Signature,
}

#[derive(Serialize)]
struct Signature {
    inputs: Vec<types::Arguments>,
    output: types::Arguments,
    bound_variables: usize,
}

pub(super) struct Captured {
    pub(super) observation: Observation,
    pub(super) local_from: Option<LocalDefId>,
}

pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    lens: &projection::Projection<'tcx>,
) -> Result<Option<Captured>, Failure> {
    let ExprKind::MethodCall(_, receiver, operands, _) = expression.kind else {
        return Ok(None);
    };
    let requested = requested_into(context, expression, operands.is_empty())?;
    let tcx = context.tcx;

    let arguments = lens.args(tcx, context.typeck.node_args(expression.hir_id))?;
    let selected_into = projection::select(tcx, requested, arguments)?;
    let from_method = tcx
        .get_diagnostic_item(Symbol::intern("from_fn"))
        .ok_or("Missing From method diagnostic item")?;
    let from_trait = tcx
        .get_diagnostic_item(sym::From)
        .ok_or("Missing From trait diagnostic item")?;
    let from_arguments = tcx.mk_args(&[lens.span.into(), lens.input.into()]);
    let selected_from = projection::select(tcx, from_method, from_arguments)?;

    check_receiver(context, expression, receiver, lens)?;
    let observation = Observation {
        identity: "rstyle.rust.product-into-from-observation/1",
        hir: super::super::super::hir::reference(expression.hir_id),
        span: context.span(expression.span),
        receiver: super::super::super::hir::reference(receiver.hir_id),
        receiver_type: types::endpoint(tcx, lens.node_ty(context, receiver.hir_id)?)?,
        result_type: types::endpoint(tcx, lens.node_ty(context, expression.hir_id)?)?,
        adjustments: adjustments(context, receiver, lens)?,
        from_selection_basis: "independent compiler selection of From<concrete input> for actual Span; not a core Into body/effect proof",
        into: selection(tcx, requested, arguments, selected_into)?,
        from: selection(tcx, from_method, from_arguments, selected_from)?,
    };
    if tcx.trait_of_assoc(from_method) != Some(from_trait) {
        return Err("From method/trait identity disagrees".into());
    }
    Ok(Some(Captured {
        observation,
        local_from: selected_from.def_id().as_local(),
    }))
}

fn requested_into(
    context: &Context<'_>,
    expression: &Expr<'_>,
    no_operands: bool,
) -> Result<DefId, String> {
    let tcx = context.tcx;
    let requested = context
        .typeck
        .type_dependent_def_id(expression.hir_id)
        .ok_or("Product method call lacks its original requested method")?;
    let requested_trait = tcx
        .trait_of_assoc(requested)
        .ok_or("Product method has no original trait")?;

    if tcx.get_diagnostic_item(Symbol::intern("Into")) != Some(requested_trait)
        || tcx.item_name(requested).as_str() != "into"
        || !no_operands
        || expression.span.from_expansion()
    {
        return Err("Product observation requires one original Into receiver".into());
    }
    Ok(requested)
}

fn check_receiver<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'tcx>,
    receiver: &Expr<'tcx>,
    lens: &projection::Projection<'tcx>,
) -> Result<(), String> {
    if lens.node_ty(context, receiver.hir_id)? != lens.input
        || lens.node_ty(context, expression.hir_id)? != lens.span
    {
        return Err(
            "Product Into receiver/result differ from the requested concrete substitution".into(),
        );
    }
    Ok(())
}

fn adjustments<'tcx>(
    context: &Context<'tcx>,
    receiver: &Expr<'tcx>,
    lens: &projection::Projection<'tcx>,
) -> Result<Vec<Adjustment>, String> {
    context
        .typeck
        .expr_adjustments(receiver)
        .iter()
        .map(|adjustment| {
            Ok(Adjustment {
                kind: format!("{:?}", adjustment.kind),
                target: types::endpoint(context.tcx, lens.ty(context.tcx, adjustment.target)?)?,
            })
        })
        .collect()
}

fn selection<'tcx>(
    tcx: TyCtxt<'tcx>,
    requested: DefId,
    arguments: GenericArgsRef<'tcx>,
    selected: ty::Instance<'tcx>,
) -> Result<Selection, String> {
    let requested_trait = tcx
        .trait_of_assoc(requested)
        .ok_or("Requested conversion has no trait owner")?;
    let implementation = tcx
        .impl_of_assoc(selected.def_id())
        .ok_or("Selected conversion has no impl owner")?;
    let selected_trait = tcx
        .impl_opt_trait_id(implementation)
        .ok_or("Selected conversion impl has no trait")?;
    if selected_trait != requested_trait {
        return Err("Selected conversion changes its requested trait".into());
    }
    let original_trait = tcx
        .impl_trait_ref(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    let substituted_trait =
        super::super::projection::normalize(tcx, selected.args, original_trait.args)
            .map_err(|error| error.to_string())?;

    Ok(Selection {
        requested: compiler_definition(tcx, requested),
        requested_trait: compiler_definition(tcx, requested_trait),
        requested_arguments: types::arguments(tcx, arguments),
        selected: compiler_definition(tcx, selected.def_id()),
        selected_impl: compiler_definition(tcx, implementation),
        selected_trait: compiler_definition(tcx, selected_trait),
        selected_arguments: types::arguments(tcx, selected.args),
        original_impl_trait_arguments: original_trait
            .args
            .iter()
            .map(|argument| match argument.kind() {
                ty::GenericArgKind::Type(value) => projection::original_type(tcx, value, 0),
                _ => Err(
                    "Conversion impl trait arguments are outside the bounded type grammar".into(),
                ),
            })
            .collect::<Result<_, _>>()?,
        substituted_impl_trait_arguments: types::arguments(tcx, substituted_trait),
        requested_signature: signature(tcx, requested, arguments)?,
        selected_signature: signature(tcx, selected.def_id(), selected.args)?,
    })
}

fn signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    definition: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> Result<Signature, String> {
    let original = tcx.fn_sig(definition).instantiate(tcx, arguments);
    let bound_variables = original.skip_normalization().bound_vars().len();
    let normalized = tcx
        .try_normalize_erasing_regions(ty::TypingEnv::fully_monomorphized(), original)
        .map_err(|_| "Product conversion signature normalization failed")?
        .skip_binder();
    Ok(Signature {
        inputs: normalized
            .inputs()
            .iter()
            .map(|value| types::endpoint(tcx, *value))
            .collect::<Result<_, _>>()?,
        output: types::endpoint(tcx, normalized.output())?,
        bound_variables,
    })
}

pub(super) fn local_owner(
    tcx: TyCtxt<'_>,
    owner: LocalDefId,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Result<LocalBody, String> {
    let (facts, _) = function_source(tcx, owner, scanned, options)
        .ok_or("Selected local From body/source is unavailable")?;
    Ok(LocalBody {
        definition: compiler_definition(tcx, owner.to_def_id()),
        source: facts.source,
        fn_span: facts.fn_span,
        signature: facts.signature,
        bindings: facts.bindings,
        body: facts.body,
        source_diagnostics: facts.source_diagnostics,
        source_scan_complete: facts.source_scan_complete,
    })
}

#[derive(Serialize)]
pub(super) struct LocalBody {
    definition: CompilerDefinition,
    source: rstyle_pdg::SourceFacts,
    fn_span: rstyle_pdg::ByteSpan,
    signature: rstyle_pdg::SignatureFacts,
    bindings: Vec<rstyle_pdg::BindingFact>,
    body: HirNode,
    source_diagnostics: Vec<rstyle_pdg::HirDiagnostic>,
    source_scan_complete: bool,
}
