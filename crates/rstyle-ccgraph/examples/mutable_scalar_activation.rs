//! Strict original/replay matching for the frozen backtracking selection.
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

    verification_policy::strict(|| check([&store, &replay], &selection))?;
    Ok(())
}

fn check(stores: [&UnitStore; 2], selection: &Value) -> Result<(), String> {
    let source = selection["function"]["source"]
        .as_str()
        .ok_or("Missing source")?;
    let mut imports = Vec::new();
    for (replay, store) in stores.into_iter().enumerate() {
        let selected: Vec<_> = store
            .functions()
            .iter()
            .enumerate()
            .filter(|(_, facts)| facts.source.bytes.as_str() == source)
            .collect();
        let [(index, _)] = selected.as_slice() else {
            return Err("Expected one unchanged selected original".into());
        };
        let pair = store.pair(*index).ok_or("Missing selected memory pair")?;
        let analysis = store.analysis();
        let view = analysis
            .function(*index)
            .ok_or("Missing owning function view")?;
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
    Ok(())
}
