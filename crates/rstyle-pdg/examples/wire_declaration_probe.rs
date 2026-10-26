//! Source-only generated error proof on unchanged selected wire functions.
use rstyle_pdg::{
    concrete_types::{InterfaceObservations, Observation, TypeTree, VariantShape},
    nominal_boundary::resolution::probe_generated_declaration_sources,
    unit_store::UnitStore,
};
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [bundle, selection] = args.as_slice() else {
        return Err("Expected original unit bundle and wire selection".into());
    };
    let store = UnitStore::from_bundle(&std::fs::read(bundle)?)?;
    let selection: Value = serde_json::from_slice(&std::fs::read(selection)?)?;
    let functions = selection["functions"]
        .as_array()
        .ok_or("Missing selection")?;
    if functions.len() != 3 {
        return Err("Expected complete three-function wire selection".into());
    }
    let analysis = store.analysis();
    let mut proofs = 0;
    for expected in functions {
        let indices: Vec<_> = store
            .functions()
            .iter()
            .enumerate()
            .filter_map(|(i, facts)| {
                (expected["name"] == facts.name
                    && expected["source"] == facts.source.bytes.as_str())
                .then_some(i)
            })
            .collect();
        let [index] = indices.as_slice() else {
            return Err("Missing or ambiguous unchanged original".into());
        };
        let facts = analysis.function(*index).ok_or("Missing owning view")?;
        let interface = store.functions()[*index]
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;
        let Observation::Captured { ty } = &interface.result else {
            return Err("Missing complete error observation".into());
        };
        proofs += probe_generated_declaration_sources(&facts, &children(ty)?)?;
    }
    println!(
        "{}",
        serde_json::json!({"originals":3,"declaration_source_proofs":proofs,"grants_admission":false})
    );
    Ok(())
}

fn children(tree: &TypeTree) -> Result<Vec<&TypeTree>, String> {
    let error = match tree {
        TypeTree::Result { err, .. } => err.as_ref(),
        error => error,
    };
    let TypeTree::Struct { fields, .. } = error else {
        return Err("Missing error owner".into());
    };
    let TypeTree::MixedEnum { variants, .. } = &fields[0].ty else {
        return Err("Missing error enum".into());
    };
    if variants.len() != 10 {
        return Err("Incomplete error variants".into());
    }
    [8, 9]
        .into_iter()
        .map(|index| {
            let VariantShape::Named { fields } = &variants[index].shape else {
                return Err("Missing named error payload".into());
            };
            if !matches!(fields[0].1.ty, TypeTree::GeneratedStruct { .. }) {
                return Err("Missing generated error child".into());
            }
            Ok(&fields[0].1.ty)
        })
        .collect()
}
