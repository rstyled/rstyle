//! Reconcile one standalone saved run without executing Cargo or the driver.
use super::*;
use std::collections::BTreeSet;

pub(in crate::clones) fn load_saved(directory: &Path, plan: Plan) -> Result<Capture, String> {
    let graph: Value = read_json(&directory.join("expected-unit-graph.json"))?;
    check_plan(&plan, &graph)?;
    let cargo = std::fs::read(directory.join("cargo.jsonl")).map_err(|e| e.to_string())?;
    let records = check_completion(&cargo)?;
    check_artifacts(&plan, &records)?;

    let mut capture = Capture {
        directory: directory.into(),
        plan,
        units: Vec::new(),
        failures: Vec::new(),
        expected_unit_graph: graph,
    };
    reconcile(&mut capture, &cargo)?;
    if !capture.failures.is_empty() {
        return Err(capture.failures.join("; "));
    }
    check_units(&capture)?;
    Ok(capture)
}

fn check_completion(cargo: &[u8]) -> Result<Vec<Value>, String> {
    let records = serde_json::Deserializer::from_slice(cargo)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let completion = records.last().ok_or("Saved run lacks Cargo completion")?;
    if completion["reason"] != "build-finished" || completion["success"] != true {
        return Err("Saved run lacks successful final Cargo completion".into());
    }
    if records[..records.len() - 1]
        .iter()
        .any(|record| record["reason"] == "build-finished")
    {
        return Err("Saved run has multiple Cargo completions".into());
    }
    Ok(records)
}

fn check_plan(plan: &Plan, graph: &Value) -> Result<(), String> {
    if plan.ledger_schema != ledger::SCHEMA
        || (!plan.compiler_advisory_only && !plan.dependency_units.is_empty())
        || (plan.style_inputs_only
            && (!plan.compiler_advisory_only
                || !plan.compiler_advisory
                || plan.local_source_facts
                || plan.local_mutation_facts))
    {
        return Err("Standalone replay requires a current standalone capture plan".into());
    }
    if graph["version"] != 1 {
        return Err("Unsupported saved Cargo unit graph".into());
    }

    let roots = graph["roots"]
        .as_array()
        .ok_or("Missing saved Cargo roots")?;
    let mut unique = BTreeSet::new();
    if roots.is_empty() || roots.iter().any(|root| !unique.insert(root.as_u64())) {
        return Err("Saved Cargo roots must be nonempty and unique".into());
    }
    let selected = primary_units(graph)?;
    if selected != plan.primary_units {
        return Err("Saved Cargo roots disagree with the capture plan".into());
    }

    // Distinct Cargo root indices can have identical metadata for check/test
    // units. Their actual compiler modes remain distinct in the unit ledgers.
    check_targets(plan)?;
    for unit in &plan.dependency_units {
        if !graph["units"]
            .as_array()
            .is_some_and(|units| units.contains(unit))
        {
            return Err("Saved selected dependency is absent from Cargo unit graph".into());
        }
    }
    for unit in plan.planned_units() {
        check_root(plan, unit)?;
    }
    Ok(())
}

fn check_targets(plan: &Plan) -> Result<(), String> {
    if plan.targets.is_empty() {
        return Err("Saved capture has no selected targets".into());
    }
    for (index, target) in plan.targets.iter().enumerate() {
        if plan.targets[..index].contains(target)
            || target.package.is_empty()
            || target.name.is_empty()
            || target.kind.is_empty()
            || target.source.as_os_str().is_empty()
            || !plan.planned_units().any(|unit| target_agrees(target, unit))
        {
            return Err("Duplicate or unexplained saved capture target".into());
        }
    }
    Ok(())
}

fn check_root(plan: &Plan, unit: &Value) -> Result<(), String> {
    let matches = plan
        .targets
        .iter()
        .filter(|target| target_agrees(target, unit))
        .count();
    if matches != 1
        || !matches!(unit["mode"].as_str(), Some("check" | "build"))
        || !unit["features"]
            .as_array()
            .is_some_and(|features| features.iter().all(Value::is_string))
        || ![
            "opt_level",
            "debuginfo",
            "debug_assertions",
            "overflow_checks",
        ]
        .iter()
        .all(|key| !unit["profile"][key].is_null())
    {
        return Err("Saved root lacks one exact selected target and build context".into());
    }
    Ok(())
}

fn target_agrees(target: &Target, unit: &Value) -> bool {
    unit["pkg_id"] == target.package
        && unit["target"]["src_path"] == target.source.to_string_lossy().as_ref()
        && unit["target"]["name"] == target.name
        && unit["target"]["kind"].as_array().is_some_and(|kinds| {
            kinds.len() == target.kind.len()
                && kinds
                    .iter()
                    .zip(&target.kind)
                    .all(|(kind, expected)| kind == expected)
        })
        && unit["target"]["test"] == target.test
}

fn check_artifacts(plan: &Plan, records: &[Value]) -> Result<(), String> {
    for record in records {
        if !plan
            .targets
            .iter()
            .any(|target| matches_artifact(record, target))
        {
            continue;
        }
        if !plan.planned_units().any(|unit| {
            unit["pkg_id"] == record["package_id"] && unit["target"] == record["target"]
        }) {
            return Err("Saved Cargo artifact target disagrees with selected roots".into());
        }
    }
    Ok(())
}

fn check_units(capture: &Capture) -> Result<(), String> {
    let mut ledgers = BTreeSet::new();
    let mut directories = BTreeSet::new();
    for unit in &capture.units {
        let path = ledger::path(&capture.plan, &capture.directory, &unit.target, unit.test)?;
        if !ledgers.insert(path)
            || !directories.insert(&unit.directory)
            || unit.compiler_advisory_only != capture.plan.compiler_advisory_only
            || unit.style_inputs_only != capture.plan.style_inputs_only
            || unit.extraction_profile != capture.plan.extraction_profile
            || unit.test != unit.invocation.iter().any(|argument| argument == "--test")
            || !intended_invocation(&capture.plan, &unit.target, &unit.invocation)
        {
            return Err("Duplicate or contradictory saved compiler-unit ledger".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
