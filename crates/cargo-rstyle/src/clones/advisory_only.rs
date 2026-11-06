//! A unit's documents become the functions its reports are evaluated over.
use super::*;

// rstyle-allow(side_effect_density): Capture handoff boundary reads one unit's documents and records them.
/// Its functions come from core; its settings are checked once per document, not per function.
pub(super) fn consume(request: &UnitRequest<'_>, result: &mut Handoff) -> Result<(), String> {
    let unit = request.unit;
    check_mode(unit, &request.capture.plan)?;
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

    retain(Arc::new(functions), keys, request, result)
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

fn check_mode(unit: &capture::Unit, plan: &capture::Plan) -> Result<(), String> {
    if unit.compiler_runtime_bounds != plan.compiler_runtime_bounds {
        return Err("Compiler unit mode disagrees with plan".into());
    }
    Ok(())
}

fn retain(
    owner: Arc<Vec<documents::DocumentFunction>>,
    keys: Vec<FunctionKey>,
    request: &UnitRequest<'_>,
    result: &mut Handoff,
) -> Result<(), String> {
    let unit = request.unit;
    for (index, key) in keys.into_iter().enumerate() {
        if !result.keys.insert(key) {
            return Err(format!("duplicate function key {key}"));
        }
        result.assessed.push(Function {
            key,
            compilation_unit: format!(
                "{}:{}:test={}",
                unit.target.package, unit.target.name, unit.test
            ),
            unit_evidence: UnitEvidence {
                report_unit: key.unit(),
                bundle: unit
                    .directory
                    .join(rstyle_pdg::unit_store::DOCUMENTS_MANIFEST),
                function: u32::try_from(index).expect("checked unit function table"),
            },
            facts: ReportFacts {
                owner: Arc::clone(&owner),
                index,
            },
        });
    }
    Ok(())
}
