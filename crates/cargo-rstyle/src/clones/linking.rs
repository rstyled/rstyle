//! Explicit selected dependency closure; original unit bundles remain separate.
use super::*;
use capture::associations::{ObservedUnit, Request};
use rstyle_pdg::unit_store::{DependencyClosure, UnitNamespace, UnitStore};

const SCHEMA: &str = "rstyle.cargo.dependency-closure/1";
const FILE: &str = "dependency-closure.json";

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    ledgers: Vec<PathBuf>,
    namespaces: Vec<UnitNamespace>,
    associations: Vec<Value>,
}

pub(super) fn load_capture(directory: &Path, args: &Args) -> Result<capture::Capture, String> {
    if args.release
        || args.locked
        || args.offline
        || args.features.is_some()
        || args.all_features
        || args.no_default_features
        || args.target.is_some()
        || args.jobs.is_some()
    {
        return Err("Saved-run replay uses its original build options".into());
    }
    let manifest = manifest(directory)?;
    let plan: capture::Plan = read_json(&directory.join("plan.json"))?;
    if plan.extraction_profile != args.extraction_profile || plan.dependency_units.is_empty() {
        return Err("Saved-run replay requires its selected dependency extraction profile".into());
    }
    let mut units = Vec::new();

    for path in &manifest.ledgers {
        let unit: capture::Unit = read_json(path)?;
        if capture::ledger::path(&plan, directory, &unit.target, unit.test)? != *path
            || unit.directory.parent() != Some(directory)
        {
            return Err("Saved dependency ledger belongs to another run or target".into());
        }
        units.push(unit);
    }

    let mut capture = capture::Capture {
        directory: directory.into(),
        plan,
        units,
        failures: Vec::new(),
        expected_unit_graph: read_json(&directory.join("expected-unit-graph.json"))?,
    };
    capture::check_expected(&mut capture);
    if !capture.failures.is_empty() {
        return Err(capture.failures.join("; "));
    }

    Ok(capture)
}

fn manifest(directory: &Path) -> Result<Manifest, String> {
    let manifest: Manifest = read_json(&directory.join(FILE))?;
    if manifest.schema != SCHEMA
        || manifest.ledgers.is_empty()
        || manifest.ledgers.len() != manifest.namespaces.len()
    {
        return Err("Invalid saved dependency closure inventory".into());
    }
    Ok(manifest)
}

pub(super) fn consume(
    capture: &capture::Capture,
    args: &Args,
    assignments: &[identity::Assignment<'_>],
    result: &mut Handoff,
) -> Result<(), String> {
    let requests = requests(capture, args, assignments)?;
    // Keep ledger ordinals distinct from sorted report unit IDs.
    let (stores, bindings): (Vec<_>, Vec<_>) = requests
        .iter()
        .map(|request| load_unit(request, result))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .unzip();

    let checked = check_closure(capture, args, &stores)?;
    let closure =
        DependencyClosure::from_units_with(stores, &checked.associations, |_, claims| {
            if claims != checked.associations {
                return Err("Dependency association claims changed after validation".into());
            }
            Ok(())
        })?;

    consume_closure(&closure, bindings, &requests, result)?;
    if args.replay_run.is_none() {
        write_json(&capture.directory.join(FILE), &checked)?;
    }
    Ok(())
}

fn requests<'a>(
    capture: &'a capture::Capture,
    args: &'a Args,
    assignments: &[identity::Assignment<'a>],
) -> Result<Vec<UnitRequest<'a>>, String> {
    let mut requests: Vec<_> = (0..capture.units.len()).map(|_| None).collect();
    for assignment in assignments {
        let slot = requests
            .get_mut(assignment.context.observed_unit)
            .ok_or("Invalid dependency report assignment")?;
        if slot.is_some() {
            return Err("Duplicate dependency report assignment".into());
        }
        *slot = Some(UnitRequest {
            id: assignment.context.id,
            unit: assignment.unit,
            capture,
            args,
        });
    }
    requests
        .into_iter()
        .collect::<Option<_>>()
        .ok_or_else(|| "Missing dependency report assignment".into())
}

fn check_closure(
    capture: &capture::Capture,
    args: &Args,
    stores: &[UnitStore],
) -> Result<Manifest, String> {
    let observations: Vec<_> = stores
        .iter()
        .map(|store| ObservedUnit {
            namespace: *store.namespace(),
            functions: store.functions(),
        })
        .collect();

    let cargo = cargo_artifacts(&capture.directory)?;
    let associations = capture::associations::associate(Request {
        graph: &capture.expected_unit_graph,
        cargo: &cargo,
        units: &capture.units,
        observations: &observations,
    })?;

    let checked = Manifest {
        schema: SCHEMA.into(),
        ledgers: capture
            .units
            .iter()
            .map(|unit| {
                capture::ledger::path(&capture.plan, &capture.directory, &unit.target, unit.test)
            })
            .collect::<Result<_, _>>()?,
        namespaces: observations.iter().map(|unit| unit.namespace).collect(),
        associations,
    };
    if args.replay_run.is_some() && manifest(&capture.directory)? != checked {
        return Err(
            "Saved dependency closure disagrees with independently checked artifacts".into(),
        );
    }
    Ok(checked)
}

fn cargo_artifacts(directory: &Path) -> Result<Vec<Value>, String> {
    let file = std::fs::File::open(directory.join("cargo.jsonl")).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let records: Vec<Value> = serde_json::Deserializer::from_reader(reader)
        .into_iter()
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    let completion = records
        .last()
        .ok_or("Dependency closure lacks Cargo completion")?;
    if completion["reason"] != "build-finished" || completion["success"] != true {
        return Err("Dependency closure lacks successful Cargo completion".into());
    }
    Ok(records)
}

fn consume_closure(
    closure: &DependencyClosure,
    bindings: Vec<rstyle_pdg::nominal_binding::UnitBindings>,
    requests: &[UnitRequest<'_>],
    result: &mut Handoff,
) -> Result<(), String> {
    for (ordinal, binding) in bindings.into_iter().enumerate() {
        let request = requests
            .get(ordinal)
            .ok_or("Missing dependency report assignment")?;
        let store = Arc::clone(
            closure
                .unit(ordinal)
                .ok_or("Missing linked dependency owner")?,
        );
        let first = result.assessed.len();
        consume_store(store, binding, request, result)?;
        for record in &mut result.assessed[first..] {
            record.unit_evidence.dependency_closure = Some(request.capture.directory.join(FILE));
        }
    }
    Ok(())
}
