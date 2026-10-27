//! Scoped selection receipts, never synthetic nested calls or effect authority.
use super::*;
use rustc_span::Symbol;

const IDENTITY: &str = "rstyle.rust.usize-reader-conversion-closure/1";
const BASIS: &str = "independent concrete trait obligations corresponding to reviewed core convert/mod.rs blanket TryFrom/Into bodies; not nested source/HIR calls or an effect proof";

#[derive(Serialize)]
pub(super) struct Observation {
    identity: &'static str,
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    selection_basis: &'static str,
    try_from: Selection,
    #[serde(skip_serializing_if = "Option::is_none")]
    into: Option<Selection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<Selection>,
}

#[derive(Serialize)]
struct Selection {
    requested: Definition,
    requested_trait: Definition,
    requested_arguments: types::Arguments,
    #[serde(flatten)]
    outcome: Outcome,
    #[serde(flatten)]
    structure: StructuralEvidence,
}

#[derive(Default, Serialize)]
struct StructuralEvidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_impl: Option<Definition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_trait: Option<Definition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    substituted_impl_trait_arguments: Option<types::Arguments>,
    #[serde(skip_serializing_if = "Option::is_none")]
    structural_signature: Option<StructuralSignature>,
}

#[derive(Serialize)]
struct StructuralSignature {
    inputs: Vec<types::Arguments>,
    output: types::Arguments,
    bound_variables: usize,
}

#[derive(Clone, Copy)]
enum Stage {
    TryFrom,
    Into,
    From,
}

/// The coordinator supplies the actual original call selected by the ordinary
/// inventory scan. An unrecognized root receives no additional observation.
pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'_>,
    requested: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> Option<Observation> {
    if !candidate(context, expression, requested, arguments) {
        return None;
    }

    let environment = ty::TypingEnv::post_analysis(context.tcx, expression.hir_id.owner.def_id);
    let (root, selected) = selection(context.tcx, environment, requested, arguments);
    let mut observation = Observation::root(context, expression, root);

    let Some(selected) = selected else {
        return Some(observation);
    };
    if !stage_matches(context.tcx, environment, selected, Stage::TryFrom) {
        unsupported(
            &mut observation.try_from,
            "Root differs from the reviewed blanket TryFrom obligation",
        );
        return Some(observation);
    }

    nested(context.tcx, environment, &mut observation);
    Some(observation)
}

impl Observation {
    fn root(context: &Context<'_>, expression: &Expr<'_>, root: Selection) -> Self {
        Self {
            identity: IDENTITY,
            hir: reference(expression.hir_id),
            span: context.span(expression.span),
            selection_basis: BASIS,
            try_from: root,
            into: None,
            from: None,
        }
    }
}

fn candidate<'tcx>(
    context: &Context<'tcx>,
    expression: &Expr<'_>,
    requested: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> bool {
    let tcx = context.tcx;
    !expression.span.from_expansion()
        && matches!(expression.kind, ExprKind::Call(_, [_]))
        && tcx.get_diagnostic_item(Symbol::intern("try_from_fn")) == Some(requested)
        && tcx.trait_of_assoc(requested) == tcx.get_diagnostic_item(Symbol::intern("TryFrom"))
        && arguments.len() == 2
        && arguments.type_at(0) == tcx.types.usize
        && arguments.type_at(1) == tcx.types.u16
}

fn nested<'tcx>(tcx: TyCtxt<'tcx>, environment: ty::TypingEnv<'tcx>, receipt: &mut Observation) {
    let Some(into) = member(tcx, "Into", "into") else {
        unsupported(
            &mut receipt.try_from,
            "Missing diagnostic core Into obligation",
        );
        return;
    };

    let arguments = tcx.mk_args(&[tcx.types.u16.into(), tcx.types.usize.into()]);
    let (mut record, selected) = selection(tcx, environment, into, arguments);
    let follows =
        selected.is_some_and(|instance| stage_matches(tcx, environment, instance, Stage::Into));
    if selected.is_some() && !follows {
        unsupported(
            &mut record,
            "Nested Into differs from the reviewed blanket obligation",
        );
    }
    receipt.into = Some(record);

    if !follows {
        return;
    }
    from_obligation(tcx, environment, receipt);
}

fn from_obligation<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    receipt: &mut Observation,
) {
    let Some(from) = member(tcx, "From", "from") else {
        unsupported(
            receipt.into.as_mut().unwrap(),
            "Missing diagnostic core From obligation",
        );
        return;
    };

    let arguments = tcx.mk_args(&[tcx.types.usize.into(), tcx.types.u16.into()]);
    let (mut record, selected) = selection(tcx, environment, from, arguments);
    if selected.is_some_and(|instance| !stage_matches(tcx, environment, instance, Stage::From)) {
        unsupported(
            &mut record,
            "Nested From differs from the concrete integer obligation",
        );
    }
    receipt.from = Some(record);
}

fn member(tcx: TyCtxt<'_>, trait_name: &str, method_name: &str) -> Option<DefId> {
    let owner = tcx.get_diagnostic_item(Symbol::intern(trait_name))?;
    tcx.associated_items(owner)
        .in_definition_order()
        .find(|item| item.name().as_str() == method_name)
        .map(|item| item.def_id)
}

fn selection<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    requested: DefId,
    arguments: GenericArgsRef<'tcx>,
) -> (Selection, Option<ty::Instance<'tcx>>) {
    let (arguments, selected) = match select(tcx, environment, requested, arguments) {
        Ok(selected) => selected,
        Err((status, reason)) => {
            return (
                Selection::new(tcx, requested, arguments, failure(status, reason)),
                None,
            );
        }
    };
    let outcome = selected_outcome(
        tcx,
        environment,
        selected,
        types::arguments(tcx, arguments).complete(),
    );
    let record = Selection::new(tcx, requested, arguments, outcome);

    if !matches!(record.outcome.status, Status::Resolved) {
        return (record, None);
    }

    match structural(tcx, environment, selected, &record.requested_trait) {
        Ok(structure) => (
            Selection {
                structure,
                ..record
            },
            Some(selected),
        ),
        Err((status, reason)) => (selection_failure(record, status, reason), None),
    }
}

impl Selection {
    fn new<'tcx>(
        tcx: TyCtxt<'tcx>,
        requested: DefId,
        arguments: GenericArgsRef<'tcx>,
        outcome: Outcome,
    ) -> Self {
        Self {
            requested: definition_record(tcx, requested),
            requested_trait: definition_record(tcx, tcx.trait_of_assoc(requested).unwrap()),
            requested_arguments: types::arguments(tcx, arguments),
            outcome,
            structure: StructuralEvidence::default(),
        }
    }
}

fn selection_failure(mut record: Selection, status: Status, reason: &'static str) -> Selection {
    record.outcome.status = status;
    record.outcome.reason = Some(reason);
    record
}

fn structural<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
    requested_trait: &Definition,
) -> Result<StructuralEvidence, (Status, &'static str)> {
    let implementation = tcx.impl_of_assoc(instance.def_id()).ok_or((
        Status::Unsupported,
        "Selected conversion has no implementation owner",
    ))?;
    let trait_id = tcx.impl_opt_trait_id(implementation).ok_or((
        Status::Unsupported,
        "Selected conversion implementation has no trait",
    ))?;
    let arguments = implementation_arguments(tcx, environment, instance, implementation)?;

    if !arguments.complete()
        || requested_trait.definition_identity != format!("{:?}", tcx.def_path_hash(trait_id))
    {
        return Err((
            Status::Unsupported,
            "Selected implementation trait identity is incomplete",
        ));
    }

    Ok(StructuralEvidence {
        selected_impl: Some(definition_record(tcx, implementation)),
        selected_trait: Some(definition_record(tcx, trait_id)),
        substituted_impl_trait_arguments: Some(arguments),
        structural_signature: Some(structural_signature(tcx, environment, instance)?),
    })
}

fn implementation_arguments<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
    implementation: DefId,
) -> Result<types::Arguments, (Status, &'static str)> {
    let original = tcx
        .impl_trait_ref(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    let substituted = tcx
        .try_normalize_erasing_regions(
            environment,
            ty::EarlyBinder::bind(tcx, original.args).instantiate(tcx, instance.args),
        )
        .map_err(|_| {
            (
                Status::Error,
                "Selected implementation trait normalization failed",
            )
        })?;
    Ok(types::arguments(tcx, substituted))
}

fn structural_signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
) -> Result<StructuralSignature, (Status, &'static str)> {
    let signature = normalized_signature(tcx, environment, instance)?;
    let inputs = signature
        .inputs()
        .iter()
        .map(|value| types::endpoint(tcx, *value))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            (
                Status::Unsupported,
                "Conversion input structural identity is incomplete",
            )
        })?;
    let output = types::endpoint(tcx, signature.output()).map_err(|_| {
        (
            Status::Unsupported,
            "Conversion output structural identity is incomplete",
        )
    })?;
    Ok(StructuralSignature {
        inputs,
        output,
        bound_variables: 0,
    })
}

fn normalized_signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
) -> Result<ty::FnSig<'tcx>, (Status, &'static str)> {
    let signature = tcx
        .try_normalize_erasing_regions(
            environment,
            tcx.fn_sig(instance.def_id())
                .instantiate(tcx, instance.args),
        )
        .map_err(|_| {
            (
                Status::Error,
                "Conversion structural signature normalization failed",
            )
        })?;
    if !signature.bound_vars().is_empty() {
        return Err((
            Status::Unsupported,
            "Conversion signature has bound variables",
        ));
    }
    Ok(signature.skip_binder())
}

fn stage_matches<'tcx>(
    tcx: TyCtxt<'tcx>,
    environment: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
    stage: Stage,
) -> bool {
    if !stage_implementation(tcx, instance, stage) {
        return false;
    }
    let Ok(signature) = normalized_signature(tcx, environment, instance) else {
        return false;
    };
    if signature.inputs() != [tcx.types.u16] {
        return false;
    }
    match stage {
        Stage::TryFrom => super::super::results::payloads(tcx, signature.output())
            .is_some_and(|(ok, err)| ok == tcx.types.usize && matches!(err.kind(), ty::Never)),
        Stage::Into | Stage::From => signature.output() == tcx.types.usize,
    }
}

fn stage_implementation<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: ty::Instance<'tcx>,
    stage: Stage,
) -> bool {
    let Some(implementation) = tcx.impl_of_assoc(instance.def_id()) else {
        return false;
    };
    let Some(trait_id) = tcx.impl_opt_trait_id(implementation) else {
        return false;
    };
    let name = match stage {
        Stage::TryFrom => "TryFrom",
        Stage::Into => "Into",
        Stage::From => "From",
    };
    if tcx.get_diagnostic_item(Symbol::intern(name)) != Some(trait_id)
        || tcx.crate_name(instance.def_id().krate).as_str() != "core"
        || tcx.crate_name(implementation.krate).as_str() != "core"
    {
        return false;
    }

    let original = tcx
        .impl_trait_ref(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    original_arguments(original.args, stage, tcx)
}

fn original_arguments<'tcx>(
    arguments: GenericArgsRef<'tcx>,
    stage: Stage,
    tcx: TyCtxt<'tcx>,
) -> bool {
    if arguments.len() != 2 {
        return false;
    }
    match stage {
        Stage::TryFrom | Stage::Into => matches!(
            (arguments.type_at(0).kind(), arguments.type_at(1).kind()),
            (ty::Param(first), ty::Param(second)) if first.index == 0 && second.index == 1),
        Stage::From => {
            arguments.type_at(0) == tcx.types.usize && arguments.type_at(1) == tcx.types.u16
        }
    }
}

fn unsupported(record: &mut Selection, reason: &'static str) {
    record.outcome.status = Status::Unsupported;
    record.outcome.reason = Some(reason);
}
