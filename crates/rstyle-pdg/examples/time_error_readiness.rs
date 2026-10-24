//! Inspect each frozen original error-conversion function without authorizing it.
use rstyle_pdg::{HirFnFacts, classify, nominal_boundary::resolution, unit_store};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [snapshot, selection] = arguments.as_slice() else {
        return Err("Expected raw shared snapshot and frozen time selection".into());
    };
    let facts = unit_store::read_diagnostic_snapshot(&std::fs::read(snapshot)?)?;
    let selection: Value = serde_json::from_slice(&std::fs::read(selection)?)?;
    let results = selection["functions"]
        .as_array()
        .ok_or("Missing original selection")?
        .iter()
        .map(|selected| inspect(&facts, selected, &selection))
        .collect::<Result<Vec<_>, _>>()?;
    println!(
        "{}",
        json!({"definitions":facts.len(), "functions":results,
        "scope":"diagnostic evidence only; no strict admission or matching claim"})
    );
    Ok(())
}

fn inspect(facts: &[HirFnFacts], selected: &Value, selection: &Value) -> Result<Value, String> {
    let expected = selected["source"]
        .as_str()
        .ok_or("Missing original bytes")?;
    let candidates = facts
        .iter()
        .filter(|facts| facts.source.bytes.as_str() == expected)
        .collect::<Vec<_>>();
    let [facts] = candidates.as_slice() else {
        return Err("Expected exactly one original function".into());
    };
    check_context(facts, selected, selection)?;
    let verdict = rstyle_pdg::verification_policy::strict(|| classify(*facts));
    let diagnostics = verdict
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.reason.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let copy = resolution::probe_copy_components_interface(facts);
    Ok(json!({"name":selected["name"], "classifier":verdict.status,
        "diagnostics":diagnostics, "copy_source":copy}))
}

fn check_context(facts: &HirFnFacts, selected: &Value, selection: &Value) -> Result<(), String> {
    let build = &selection["build_selection"];
    let contexts = build["contexts"]
        .as_array()
        .ok_or("Missing selected contexts")?;
    if facts.fn_span.start as u64 != selected["start"].as_u64().unwrap_or(u64::MAX)
        || facts.fn_span.end as u64 != selected["end"].as_u64().unwrap_or(u64::MAX)
        || facts.build.rustc_commit.known_value() != build["rustc_commit"].as_str()
        || facts.build.edition.known_value() != build["edition"].as_str()
        || facts.build.panic_strategy.known_value() != Some("unwind")
        || facts.build.opt_level.known_value() != Some("No")
        || facts.build.debug_assertions.value != Some(true)
        || facts.build.overflow_checks.value != Some(true)
        || facts.build.pointer_width.value != Some(64)
        || !contexts
            .iter()
            .any(|context| json!(facts.build.features) == context["expected_feature_closure"])
    {
        return Err("Original function differs from frozen source/build selection".into());
    }
    Ok(())
}
