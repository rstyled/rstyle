//! Strictly check an unchanged original Range-to-Span callee in its whole owner.
use rstyle_pdg::{producer, unit_store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let raw = args.next().ok_or("Expected original raw snapshot")?;
    if args.next().is_some() {
        return Err("Unexpected Range probe argument".into());
    }
    let facts = unit_store::read_diagnostic_snapshot(&std::fs::read(raw)?)?;
    rstyle_pdg::verification_policy::strict(|| probe(facts))?;
    println!("Original Range-to-Span callee strictly admitted");
    Ok(())
}

fn probe(facts: Vec<rstyle_pdg::HirFnFacts>) -> Result<(), String> {
    let selected: Vec<_> = facts
        .iter()
        .enumerate()
        .filter(|(_, facts)| {
            facts.signature.params.len() == 1
                && facts.signature.params[0].ty.normalized.as_deref()
                    == Some(r#"{"kind":"range_usize"}"#)
                && facts.name
                    == "<util::search::Span as core::convert::From<core::ops::Range<usize>>>::from"
        })
        .map(|(ordinal, _)| ordinal)
        .collect();
    let [ordinal] = selected.as_slice() else {
        return Err("Missing or ambiguous original selected Range conversion".into());
    };
    let owner = unit_store::UnitStore::capture(facts)?;
    let analysis = owner.analysis();
    let view = analysis
        .function(*ordinal)
        .ok_or("Missing selected callee")?;
    let pair = producer::produce(&view).map_err(|errors| errors.join("; "))?;
    let bytes = pair.serialize().map_err(|error| error.to_string())?;
    producer::verify_serialized(&bytes, &view).map_err(|errors| errors.join("; "))?;
    rstyle_ccgraph::admit_produced(&pair, &view).map_err(|errors| errors.join("; "))?;
    Ok(())
}
