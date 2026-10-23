//! Check original mutable-formal evidence without claiming memory admission.
use rstyle_pdg::{field_locations, unit_store::UnitStore};
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let bundle = arguments.next().ok_or("Expected original unit bundle")?;
    let selection = arguments
        .next()
        .ok_or("Expected frozen development selection")?;
    if arguments.next().is_some() {
        return Err("Unexpected readiness probe argument".into());
    }
    let selection: Value = serde_json::from_slice(&std::fs::read(selection)?)?;
    let store = rstyle_pdg::verification_policy::strict(|| {
        UnitStore::from_bundle(&std::fs::read(bundle).map_err(|error| error.to_string())?)
    })?;
    check(&store, &selection)?;
    println!(
        "Original scalar-formal source/access evidence verified; memory admission remains closed"
    );
    Ok(())
}

fn check(store: &UnitStore, selection: &Value) -> Result<(), String> {
    let source = selection["function"]["source"]
        .as_str()
        .ok_or("Missing selected source")?;
    let candidates = store
        .functions()
        .iter()
        .enumerate()
        .filter(|(_, facts)| facts.source.bytes.as_str() == source)
        .collect::<Vec<_>>();
    let [(ordinal, facts)] = candidates.as_slice() else {
        return Err("Expected exactly one unchanged original function".into());
    };
    check_context(facts, selection)?;
    let fields = facts
        .build
        .frontend_options
        .data::<field_locations::FieldObservations>()?;
    let accesses = fields
        .scalar_formal_accesses
        .as_ref()
        .ok_or("Original function lacks mutable scalar access evidence")?;
    field_locations::verify(*facts, &fields)?;
    if accesses.accesses.len() != 8 || accesses.possibly_aliasing_formals.len() != 10 {
        return Err("Selected source lacks its complete eight accesses / ten alias pairs".into());
    }
    if store.pair(*ordinal).is_some() || rstyle_pdg::producer::produce(*facts).is_ok() {
        return Err("Readiness evidence unexpectedly authorized memory admission".into());
    }
    println!(
        "Retained owner: {} definitions; selected function: {}",
        store.functions().len(),
        facts.name
    );
    Ok(())
}

fn check_context(facts: &rstyle_pdg::HirFnFacts, selection: &Value) -> Result<(), String> {
    let build = &selection["build_selection"];
    let features: Vec<String> =
        serde_json::from_value(build["features"].clone()).map_err(|error| error.to_string())?;
    let target = facts
        .build
        .frontend_options
        .get("target")
        .ok_or("Missing compiler target")?;
    if facts.fn_span.start as u64 != selection["function"]["start"].as_u64().unwrap_or(u64::MAX)
        || facts.fn_span.end as u64 != selection["function"]["end"].as_u64().unwrap_or(u64::MAX)
        || facts.build.features != features
        || !build["targets"]
            .as_array()
            .is_some_and(|targets| targets.contains(target))
        || facts.build.rustc_commit.known_value() != build["rustc_commit"].as_str()
        || facts.build.edition.known_value() != build["edition"].as_str()
        || facts.build.panic_strategy.known_value() != Some("unwind")
        || facts.build.opt_level.known_value() != Some("No")
        || facts.build.overflow_checks.status != rstyle_pdg::FactStatus::Known
        || facts.build.overflow_checks.value != Some(true)
        || facts.build.debug_assertions.status != rstyle_pdg::FactStatus::Known
        || facts.build.debug_assertions.value != Some(true)
        || facts.build.pointer_width.status != rstyle_pdg::FactStatus::Known
        || facts.build.pointer_width.value != Some(64)
    {
        return Err(
            "Original function differs from its selected range/compiler/build context".into(),
        );
    }
    Ok(())
}
