//! Strict original/replay matching for source-selected original functions.
use rstyle_ccgraph::{CcgraphConfig, Decision};
use rstyle_pdg::{unit_store::UnitStore, verification_policy};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let bundle = args.next().ok_or("Expected original unit bundle")?;
    let selection = args.next().ok_or("Expected frozen selection")?;
    if args.next().is_some() {
        return Err("Unexpected activation argument".into());
    }
    let selection: Value = serde_json::from_slice(&std::fs::read(selection)?)?;
    let store = UnitStore::from_bundle(&std::fs::read(bundle)?)?;
    let mut replay = Vec::new();
    store.write_bundle(&mut replay)?;
    let replay = UnitStore::from_bundle(&replay)?;

    verification_policy::strict(|| {
        if let Some(functions) = selection["functions"].as_array() {
            check_all([&store, &replay], functions)
        } else {
            check([&store, &replay], &selection).map(|_| ())
        }
    })?;
    Ok(())
}

fn check(
    stores: [&UnitStore; 2],
    selection: &Value,
) -> Result<rstyle_ccgraph::ImportedFunction, String> {
    let mut imports = Vec::new();
    for (replay, store) in stores.into_iter().enumerate() {
        let index = selected_index(store, selection)?;
        let analysis = store.analysis();
        let view = analysis
            .function(index)
            .ok_or("Missing owning function view")?;
        let pair = store.pair(index).ok_or_else(|| {
            format!(
                "Missing selected memory pair for {}: {:?}",
                view.name,
                rstyle_pdg::classify(&view)
            )
        })?;
        let serialized = pair.serialize().map_err(|error| error.to_string())?;
        rstyle_pdg::producer::verify_serialized(&serialized, &view)
            .map_err(|errors| errors.join("; "))?;
        let mut imported =
            rstyle_ccgraph::admit_produced(pair, &view).map_err(|errors| errors.join("; "))?;
        imported.key = format!("backtrack:{replay}");
        imports.push(imported);
    }

    let report = rstyle_ccgraph::evaluate_all(&imports, &CcgraphConfig::default());
    if report.incomplete
        || report.pairs.len() != 1
        || report.functions.iter().any(|function| !function.eligible)
        || report.pairs[0].decision != Decision::Accepted
        || report.pairs[0].displayed_score.as_deref() != Some("1")
    {
        return Err("Original/replay differs from expected complete Accepted score 1".into());
    }
    println!(
        "{}",
        json!({"strict_imports":2,"selected_pairs":1,
        "decision":report.pairs[0].decision,"score":report.pairs[0].displayed_score,
        "owner_definitions":stores[0].functions().len()})
    );
    Ok(imports.remove(0))
}

fn selected_index(store: &UnitStore, selection: &Value) -> Result<usize, String> {
    let source = selection["function"]["source"]
        .as_str()
        .ok_or("Missing source")?;
    let selected: Vec<_> = store
        .functions()
        .iter()
        .enumerate()
        .filter(|(_, facts)| {
            facts.source.bytes.as_str() == source
                && selection["function"]["name"]
                    .as_str()
                    .is_none_or(|name| facts.name == name)
        })
        .map(|(index, _)| index)
        .collect();
    let [index] = selected.as_slice() else {
        return Err("Expected one unchanged selected original".into());
    };
    Ok(*index)
}

fn check_all(stores: [&UnitStore; 2], functions: &[Value]) -> Result<(), String> {
    if functions.is_empty() {
        return Err("Expected at least one selected original".into());
    }
    let mut originals = Vec::new();
    let mut sources = std::collections::BTreeSet::new();
    for (index, function) in functions.iter().enumerate() {
        let source = function["source"]
            .as_str()
            .ok_or("Missing selected source")?;
        if !sources.insert((source, function["name"].as_str())) {
            return Err("Selected original source/name identity is duplicated".into());
        }
        let mut original = check(stores, &json!({"function": function}))?;
        original.key = format!("selected:{index}");
        originals.push(original);
    }
    let report = rstyle_ccgraph::evaluate_all(&originals, &CcgraphConfig::default());
    let expected = originals.len() * (originals.len() - 1) / 2;
    if report.incomplete
        || report.pairs.len() != expected
        || report.functions.iter().any(|function| !function.eligible)
    {
        return Err("Selected original pair enumeration/evaluation is incomplete".into());
    }
    println!(
        "{}",
        json!({"selected_originals":originals.len(),
        "evaluated_original_pairs":report.pairs.len(),"complete":true,
        "pairs":report.pairs.iter().map(|pair| json!({
            "first":pair.first,"second":pair.second,"decision":pair.decision,
            "score":pair.displayed_score,"stages":pair.stages,
        })).collect::<Vec<_>>()})
    );
    Ok(())
}
