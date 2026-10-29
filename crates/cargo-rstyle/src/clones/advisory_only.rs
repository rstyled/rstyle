//! Compiler observations have independent ownership and never grant strict admission.
use super::*;

pub(super) fn consume(request: &UnitRequest<'_>, result: &mut Handoff) -> Result<(), String> {
    let unit = request.unit;
    if !unit.compiler_advisory_only
        || unit.extraction_profile != request.capture.plan.extraction_profile
    {
        return Err("Compiler advisory unit mode or extraction profile disagrees with plan".into());
    }

    let snapshot = capture::load_inventory(unit)?;
    let facts = rstyle_pdg::unit_store::read_diagnostic_snapshot_phase(
        snapshot.bundle.bytes(),
        "compiler-advisory-only",
    )?;
    let selected = selected_context::SelectedContext::new(unit)?;

    let mut builds = rstyle_pdg::nominal_binding::BuildContexts::default();
    for function in &facts {
        verify_frontend_invocation(function, unit, request.capture, &selected)?;
        builds.intern(&function.build)?;
    }

    result
        .source_inventory
        .extend(source_inventory(unit, &snapshot)?);
    result.build_contexts.push(UnitBuildContexts {
        unit_bundle: unit.directory.join("unit.evidence.json"),
        builds,
    });
    retain(Arc::new(facts), request, result)
}

fn retain(
    facts: Arc<Vec<HirFnFacts>>,
    request: &UnitRequest<'_>,
    result: &mut Handoff,
) -> Result<(), String> {
    let keys = identity::functions(request.id, &facts)?;
    for (index, key) in keys.into_iter().enumerate() {
        if !result.keys.insert(key) {
            return Err(format!("duplicate function key {key}"));
        }
        let facts = ReportFacts {
            owner: FactOwner::Advisory(Arc::clone(&facts)),
            index,
        };
        let mut record = function_record(facts, key, request.unit, index);
        record.outcome = "not_requested".into();
        result.assessed.push(record);
    }
    Ok(())
}
