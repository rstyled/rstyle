//! Compiler observations have independent ownership and never grant strict admission.
use super::*;

pub(super) fn consume(request: &UnitRequest<'_>, result: &mut Handoff) -> Result<(), String> {
    let unit = request.unit;
    check_mode(unit, &request.capture.plan)?;

    let snapshot = Stage::new("cli.advisory.load_inventory", &unit.target.package, None)
        .observe(|| capture::load_inventory(unit))?;
    if unit.style_inputs_only {
        return consume_style(request, &snapshot, result);
    }
    let stage = Stage::new(
        "cli.advisory.decode",
        &unit.target.package,
        Some(snapshot.bundle.bytes()),
    );
    if header_only(request.args) {
        let facts = stage.observe(|| {
            rstyle_pdg::unit_store::read_diagnostic_snapshot_headers_phase(
                snapshot.bundle.bytes(),
                "compiler-advisory-only",
            )
        })?;
        consume_facts(
            Decoded {
                facts,
                owner: FactOwner::Numeric,
            },
            request,
            &snapshot,
            result,
        )
    } else {
        let facts = stage.observe(|| {
            rstyle_pdg::unit_store::read_diagnostic_snapshot_phase(
                snapshot.bundle.bytes(),
                "compiler-advisory-only",
            )
        })?;
        consume_facts(
            Decoded {
                facts,
                owner: FactOwner::Advisory,
            },
            request,
            &snapshot,
            result,
        )
    }
}

fn check_mode(unit: &capture::Unit, plan: &capture::Plan) -> Result<(), String> {
    if !unit.compiler_advisory_only
        || unit.style_inputs_only != plan.style_inputs_only
        || unit.extraction_profile != plan.extraction_profile
    {
        return Err("Compiler advisory unit mode or extraction profile disagrees with plan".into());
    }

    Ok(())
}

fn consume_style(
    request: &UnitRequest<'_>,
    snapshot: &capture::Snapshot,
    result: &mut Handoff,
) -> Result<(), String> {
    if request.args.compiler_patterns && !request.capture.plan.compiler_patterns {
        return Err("Root style inputs did not request compiler pattern graphs; use a compiler-pattern capture".into());
    }
    if !header_only(request.args) {
        return Err("Metadata-only inputs did not request original HIR bodies or bindings; requested consumers require a full body capture".into());
    }
    let facts = rstyle_pdg::unit_store::read_style_snapshot_phase(
        snapshot.bundle.bytes(),
        "compiler-style-inputs",
    )?;
    consume_facts(
        Decoded {
            facts,
            owner: FactOwner::Style,
        },
        request,
        snapshot,
        result,
    )
}

fn header_only(args: &Args) -> bool {
    matches!(args.report_detail, report::Detail::Compact)
        && args.compiler_inputs_requested()
        && !args.source_rules
        && !args.guard_clauses
        && !args.mutation_rules
        && !args.mutation_effects
        && !args.effect_density
        && !args.enum_dispatch
        && !args.capability_proposals
}

type FactsOwner<B, L> = fn(Arc<Vec<HirFnFacts<B, L>>>) -> FactOwner;

struct Decoded<B, L> {
    facts: Vec<HirFnFacts<B, L>>,
    owner: FactsOwner<B, L>,
}

fn consume_facts<B: identity::BodyOwner, L>(
    decoded: Decoded<B, L>,
    request: &UnitRequest<'_>,
    snapshot: &capture::Snapshot,
    result: &mut Handoff,
) -> Result<(), String> {
    let Decoded { facts, owner } = decoded;
    let unit = request.unit;
    let context_stage = Stage::new("cli.advisory.context", &unit.target.package, None).enter();
    let selected = selected_context::SelectedContext::new(unit)?;

    let mut builds = rstyle_pdg::nominal_binding::BuildContexts::default();
    for function in &facts {
        verify_frontend_invocation(function, unit, request.capture, &selected)?;
        builds.intern(&function.build)?;
    }

    result
        .source_inventory
        .extend(source_inventory(unit, snapshot)?);
    result.build_contexts.push(UnitBuildContexts {
        unit_bundle: unit.directory.join("unit.evidence.json"),
        builds,
    });
    context_stage.finish(Ok::<_, String>(()))?;
    Stage::new("cli.advisory.retain", &unit.target.package, None).observe(|| {
        let keys = identity::functions(request.id, &facts)?;
        retain(owner(Arc::new(facts)), keys, request, result)
    })
}

fn retain(
    owner: FactOwner,
    keys: Vec<FunctionKey>,
    request: &UnitRequest<'_>,
    result: &mut Handoff,
) -> Result<(), String> {
    for (index, key) in keys.into_iter().enumerate() {
        if !result.keys.insert(key) {
            return Err(format!("duplicate function key {key}"));
        }
        let facts = ReportFacts {
            owner: owner.clone(),
            index,
        };
        let mut record = function_record(facts, key, request.unit, index);
        record.outcome = "not_requested".into();
        result.assessed.push(record);
    }
    Ok(())
}
