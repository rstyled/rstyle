//! Diagnostic exact generated source/instance proof; deliberately no admission claim.
use rstyle_pdg::{nominal_boundary::resolution, unit_store::UnitStore};
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [bundle, selection] = args.as_slice() else {
        return Err("Expected original unit bundle and source-selected functions".into());
    };
    let store = UnitStore::from_bundle(&std::fs::read(bundle)?)?;
    let selection: Value = serde_json::from_slice(&std::fs::read(selection)?)?;
    let expected = selection["functions"]
        .as_array()
        .ok_or("Missing selected functions")?;
    if expected.len() != 8 {
        return Err("Expected the complete eight-function conversion family".into());
    }
    let analysis = store.analysis();
    let mut admitted = 0;
    for expected in expected {
        let matches: Vec<_> = store
            .functions()
            .iter()
            .enumerate()
            .filter(|(_, facts)| {
                expected["name"] == facts.name && expected["source"] == facts.source.bytes.as_str()
            })
            .map(|(index, _)| index)
            .collect();
        let [index] = matches.as_slice() else {
            return Err("Missing or ambiguous unchanged original".into());
        };
        let facts = analysis
            .function(*index)
            .ok_or("Missing selected owning facts")?;
        if resolution::probe_generated_conversion_source(&facts)? != 8 {
            return Err("Generated source proof omits a parameter/body role".into());
        }
        admitted += usize::from(store.pair(*index).is_some());
    }
    println!(
        "{}",
        serde_json::json!({"owner_definitions":store.functions().len(),
        "selected_originals":expected.len(),"source_and_instance_proofs":expected.len(),
        "retained_admitted_selected":admitted,"probe_grants_admission":false})
    );
    Ok(())
}
