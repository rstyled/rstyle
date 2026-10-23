//! Probe complete original evidence without creating admission authority.
use rstyle_pdg::{HirFnFacts, nominal_boundary::resolution, reference_instances, unit_store};
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("Expected raw owner snapshot")?;
    let bytes = std::fs::read(path)?;
    let facts = unit_store::read_diagnostic_snapshot(&bytes)?;
    if let Some(path) = args.next() {
        let inventory: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        observations(&facts, &inventory)?;
    }
    let owners: Vec<_> = facts
        .iter()
        .filter(|facts| facts.name == "util::search::Input::<'h>::new")
        .collect();
    let [owner] = owners.as_slice() else {
        return Err("Expected exactly one original Input constructor".into());
    };
    resolution::probe_reference_product_result(owner)?;
    println!("Complete original result cleanup proof passed; no admission authority");
    Ok(())
}

fn observations(facts: &[HirFnFacts], inventory: &Value) -> Result<(), String> {
    if inventory["identity"] != "rstyle.diagnostic.reference-instance-bodies/1"
        || inventory["admission_authority"] != false
    {
        return Err("Expected evidence-only reference instance inventory".into());
    }
    let requests = inventory["requests"].as_array().ok_or("Missing requests")?;
    let mut observed = 0;
    for request in requests {
        match request["status"].as_str() {
            Some("observed") => {}
            Some("unavailable") => {
                println!("{}: unavailable", request["request"]);
                continue;
            }
            _ => return Err("Unknown or incomplete reference observation outcome".into()),
        }
        observation(facts, request)?;
        observed += 1;
    }
    if observed != 4 {
        return Err("Expected four original reference observations".into());
    }
    Ok(())
}

fn observation(facts: &[HirFnFacts], request: &Value) -> Result<(), String> {
    let name = request["request"]["function"]
        .as_str()
        .ok_or("Missing function")?;
    let haystack = request["request"]["haystack"]
        .as_str()
        .ok_or("Missing haystack")?;
    let owners: Vec<_> = facts.iter().filter(|facts| facts.name == name).collect();
    let [owner] = owners.as_slice() else {
        return Err(format!("Missing or ambiguous canonical original: {name}"));
    };
    let result = if name == "util::search::Input::<'h>::new" {
        reference_instances::verify_constructor_body_observations(owner, request, haystack)
    } else {
        reference_instances::verify_forwarder_owner(owner, &request["provenance"], haystack)
    };
    result.map_err(|error| format!("{name}<{haystack}>: {error}"))?;
    println!("{name}<{haystack}>: observation checks passed; no admission authority");
    Ok(())
}
