//! Check original source, Copy closure, conversion and concrete value projection.
use rstyle_pdg::{product_instances, unit_store};
use serde_json::Value;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let raw = args.next().ok_or("Expected original raw snapshot")?;
    let observed = args.next().ok_or("Expected product instance inventory")?;
    if args.next().is_some() {
        return Err("Unexpected product probe argument".into());
    }
    let facts = unit_store::read_diagnostic_snapshot(&std::fs::read(raw)?)?;
    let inventory: Value = serde_json::from_slice(&std::fs::read(observed)?)?;
    probe(&facts, &inventory)?;
    rstyle_pdg::verification_policy::strict(|| owning_probe(facts, &inventory))?;
    println!(
        "Both original constructor instances passed owning admission and strict sidecar replay"
    );
    Ok(())
}

fn owning_probe(facts: Vec<rstyle_pdg::HirFnFacts>, inventory: &Value) -> Result<(), String> {
    let request = &inventory["requests"][0];
    let name = request["request"]["function"]
        .as_str()
        .ok_or("Missing function")?;
    let ordinal = facts
        .iter()
        .position(|facts| facts.name == name)
        .ok_or("Missing owner")?;
    let owner = Arc::new(unit_store::UnitStore::capture(facts)?);
    eprintln!(
        "Original complete owner captured: {} definitions",
        owner.functions().len()
    );
    let replay = unit_store::ProductConstructorReplay::new(owner, |_| Ok(()))?;
    for (index, argument) in [
        product_instances::SpanArgument::Span,
        product_instances::SpanArgument::RangeUsize,
    ]
    .into_iter()
    .enumerate()
    {
        owning_instance(&replay, ordinal, &request["instances"][index], argument)?;
    }
    Ok(())
}

fn owning_instance(
    replay: &unit_store::ProductConstructorReplay,
    ordinal: usize,
    observation: &Value,
    argument: product_instances::SpanArgument,
) -> Result<(), String> {
    let instance = unit_store::UnitStore::capture_product_constructor(
        Arc::clone(replay.owner()),
        ordinal,
        observation,
        argument,
    )?;
    let analysis = instance.analysis();
    let view = analysis.function(0).ok_or("Missing checked instance")?;
    let pair = admitted_pair(&view)?;
    verify_call_attacks(&pair, &view, argument)?;
    eprintln!("Original {argument:?} owning pair independently verified");
    let mut sidecar = Vec::new();
    instance.write_product_constructor_sidecar(&mut sidecar)?;
    let restored = replay.replay(&sidecar)?;
    let restored_analysis = restored.analysis();
    let restored_view = restored_analysis
        .function(0)
        .ok_or("Missing replayed instance")?;
    if rstyle_pdg::producer::verify(&pair, &restored_view).is_ok() {
        return Err("Original pair unexpectedly accepted a different instance namespace".into());
    }
    admitted_pair(&restored_view)?;
    if instance.functions() != restored.functions() {
        return Err("Strict replay changed the complete checked function".into());
    }
    Ok(())
}

fn admitted_pair(
    facts: &dyn unit_store::EvidenceFacts,
) -> Result<rstyle_pdg::producer::Pair, String> {
    let pair = rstyle_pdg::producer::produce(facts).map_err(|errors| errors.join("; "))?;
    let bytes = pair.serialize().map_err(|error| error.to_string())?;
    rstyle_pdg::producer::verify_serialized(&bytes, facts).map_err(|errors| errors.join("; "))?;
    rstyle_ccgraph::admit_produced(&pair, facts).map_err(|errors| errors.join("; "))?;
    Ok(pair)
}

fn probe(facts: &[rstyle_pdg::HirFnFacts], inventory: &Value) -> Result<(), String> {
    if inventory["identity"] != "rstyle.diagnostic.product-instance-bodies/1"
        || inventory["admission_authority"] != false
    {
        return Err("Expected evidence-only product inventory".into());
    }
    let requests = inventory["requests"].as_array().ok_or("Missing requests")?;
    let [request] = requests.as_slice() else {
        return Err("Expected exactly one complete product request".into());
    };
    let name = request["request"]["function"]
        .as_str()
        .ok_or("Missing function")?;
    let owners: Vec<_> = facts.iter().filter(|facts| facts.name == name).collect();
    let [canonical] = owners.as_slice() else {
        return Err("Missing or ambiguous original owner".into());
    };
    let instances = request["instances"].as_array().ok_or("Missing instances")?;
    let [span, range] = instances.as_slice() else {
        return Err("Expected both selected product substitutions".into());
    };
    if request["status"] != "observed"
        || span["argument"] != "Span"
        || range["argument"] != "Range<usize>"
    {
        return Err("Original product selection differs".into());
    }
    let checked = product_instances::verify_constructor_projection(
        canonical,
        span,
        product_instances::SpanArgument::Span,
    )?;
    if rstyle_pdg::producer::produce(*canonical).is_ok()
        || rstyle_pdg::producer::produce(checked.facts()).is_ok()
    {
        return Err("Projection unexpectedly granted ordinary graph admission".into());
    }
    if product_instances::verify_constructor_projection(
        canonical,
        range,
        product_instances::SpanArgument::RangeUsize,
    )
    .is_ok()
    {
        return Err("Range unexpectedly acquired consuming ownership authority".into());
    }
    Ok(())
}

fn verify_call_attacks(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn unit_store::EvidenceFacts,
    argument: product_instances::SpanArgument,
) -> Result<(), String> {
    if argument != product_instances::SpanArgument::RangeUsize {
        return Ok(());
    }
    for field in ["operator", "callee", "construct"] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        let call = changed
            .pdg
            .nodes
            .iter_mut()
            .find(|node| {
                node.attributes.construct.as_deref() == Some("data.product-instance.range-into")
            })
            .ok_or("Missing Range conversion CALL")?;
        match field {
            "operator" => {
                call.attributes.operator =
                    Some(rstyle_pdg::StringFact::known("different operation"))
            }
            "callee" => {
                call.attributes.callee = Some(rstyle_pdg::StringFact::known("different callee"))
            }
            _ => call.attributes.construct = Some("data.product-instance.identity-into".into()),
        }
        let sealed = changed.serialize().map_err(|error| error.to_string())?;
        if rstyle_pdg::producer::verify_serialized(&sealed, facts).is_ok() {
            return Err(format!("Resealed changed Range {field} was accepted"));
        }
    }
    Ok(())
}
