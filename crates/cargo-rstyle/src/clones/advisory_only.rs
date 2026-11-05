//! Compiler observations have independent ownership and never grant strict admission.
use super::*;

// rstyle-allow(side_effect_density): Capture handoff boundary loads one inventory and routes its decoded facts.
pub(super) fn consume(request: &UnitRequest<'_>, result: &mut Handoff) -> Result<(), String> {
    let unit = request.unit;
    check_mode(unit, &request.capture.plan)?;
    if unit.style_inputs_only {
        return consume_documents(request, result);
    }

    let snapshot = Stage::new("cli.advisory.load_inventory", &unit.target.package, None)
        .observe(|| capture::load_inventory(unit))?;
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
        || unit.compiler_runtime_bounds != plan.compiler_runtime_bounds
        || unit.style_inputs_only != plan.style_inputs_only
        || unit.extraction_profile != plan.extraction_profile
    {
        return Err("Compiler advisory unit mode or extraction profile disagrees with plan".into());
    }

    Ok(())
}

// rstyle-allow(side_effect_density): Capture handoff boundary reads one unit's documents and retains their functions.
/// A unit published as documents: its functions come from the core, its settings are
/// checked once per document rather than once per function.
fn consume_documents(request: &UnitRequest<'_>, result: &mut Handoff) -> Result<(), String> {
    if !header_only(request.args) {
        return Err("Metadata-only inputs did not request original HIR bodies or bindings; requested consumers require a full body capture".into());
    }
    let unit = request.unit;
    let demand = rstyle_core::cpg::records::Demand {
        graphs: request.args.compiler_patterns,
        bounds: request.args.compiler_runtime_bounds,
    };
    let functions = Stage::new("cli.documents.read", &unit.target.package, None)
        .observe(|| documents::read(unit, demand))?;
    let context = Stage::new("cli.documents.context", &unit.target.package, None).enter();
    let builds = document_builds(&functions, request)?;
    result.build_contexts.push(UnitBuildContexts {
        unit_bundle: unit
            .directory
            .join(rstyle_pdg::unit_store::DOCUMENTS_MANIFEST),
        builds,
    });
    context.finish(Ok::<_, String>(()))?;
    let keys = identity::functions(request.id, &functions)?;

    retain(
        FactOwner::Document(Arc::new(functions)),
        keys,
        request,
        result,
    )
}

/// Each document's recorded settings, checked against this run and interned.
fn document_builds(
    functions: &[documents::DocumentFunction],
    request: &UnitRequest<'_>,
) -> Result<rstyle_pdg::nominal_binding::BuildContexts, String> {
    let selected = selected_context::SelectedContext::new(request.unit)?;
    let mut builds = rstyle_pdg::nominal_binding::BuildContexts::default();
    let mut previous = None;
    for function in functions {
        // Every function of a document shares its settings.
        if previous.is_some_and(|build| std::ptr::eq(build, Arc::as_ptr(&function.build))) {
            continue;
        }
        previous = Some(Arc::as_ptr(&function.build));

        verify_frontend_build(&function.build, request.unit, request.capture, &selected)?;
        builds.intern(&function.build)?;
    }
    Ok(builds)
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

// rstyle-allow(side_effect_density): Retain validated unit facts and their build contexts together; optional stage telemetry is intentional.
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
