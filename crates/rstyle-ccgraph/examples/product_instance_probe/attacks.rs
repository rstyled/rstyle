//! Independently specified changes must fail after fresh document sealing.
use rstyle_pdg::{
    producer::{Pair, verify_serialized},
    product_instances::SpanArgument,
    unit_store::{EvidenceFacts, ProductConstructorReplay},
};
use serde_json::Value;

pub(super) fn documents(
    pair: &Pair,
    facts: &dyn EvidenceFacts,
    argument: SpanArgument,
) -> Result<(), String> {
    if argument != SpanArgument::RangeUsize {
        return Ok(());
    }
    for attack in [
        "operator",
        "callee",
        "construct",
        "policy",
        "dependency",
        "operand",
    ] {
        let mut forged = Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        forge(&mut forged, attack)?;
        let sealed = forged.serialize().map_err(|error| error.to_string())?;
        if verify_serialized(&sealed, facts).is_ok() {
            return Err(format!("Resealed changed Range {attack} was accepted"));
        }
    }

    eprintln!("Range resealed operator/callee/construct/3.91/dependency/operand attacks rejected");
    Ok(())
}

fn forge(pair: &mut Pair, attack: &str) -> Result<(), String> {
    match attack {
        "policy" => {
            // Historical 3.91 predates consuming Range authority.
            pair.pdg.profile.version = "3.91.0".into();
            pair.cfg.profile.version = "3.91.0".into();
        }
        "dependency" | "operand" => forge_dependency(pair, attack)?,
        _ => forge_call(pair, attack)?,
    }

    Ok(())
}

fn forge_call(pair: &mut Pair, attack: &str) -> Result<(), String> {
    let call = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.attributes.construct.as_deref() == Some("data.product-instance.range-into")
        })
        .ok_or("Missing Range conversion CALL")?;
    match attack {
        "operator" => {
            call.attributes.operator = Some(rstyle_pdg::StringFact::known("different operation"))
        }
        "callee" => {
            call.attributes.callee = Some(rstyle_pdg::StringFact::known("different callee"))
        }
        "construct" => {
            call.attributes.construct = Some("data.product-instance.identity-into".into())
        }
        _ => return Err(format!("Unknown Range CALL attack {attack}")),
    }

    Ok(())
}

fn forge_dependency(pair: &mut Pair, attack: &str) -> Result<(), String> {
    let records = pair
        .pdg
        .extensions
        .get_mut(rstyle_pdg::call_summaries::KEY)
        .and_then(Value::as_array_mut)
        .ok_or("Missing Range call records")?;
    let mut dependencies = records
        .iter_mut()
        .filter(|record| record.get("dependency").is_some());
    let record = dependencies
        .next()
        .ok_or("Missing Range dependency record")?;
    if dependencies.next().is_some() {
        return Err("Expected exactly one Range dependency record".into());
    }
    match attack {
        "dependency" => record["dependency"] = Value::Null,
        "operand" => record["operand_pdg_node_ids"] = serde_json::json!([]),
        _ => return Err(format!("Unknown Range dependency attack {attack}")),
    }
    let changed = pair.pdg.extensions[rstyle_pdg::call_summaries::KEY].clone();
    pair.cfg
        .extensions
        .insert(rstyle_pdg::call_summaries::KEY.into(), changed);

    Ok(())
}

pub(super) fn sidecar(
    replay: &ProductConstructorReplay,
    bytes: &[u8],
    argument: SpanArgument,
) -> Result<(), String> {
    if argument != SpanArgument::RangeUsize {
        return Ok(());
    }
    let original: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    for attack in ["missing-local", "callee", "source"] {
        let mut forged = original.clone();
        forge_observation(&mut forged, attack)?;
        let bytes = serde_json::to_vec(&forged).map_err(|error| error.to_string())?;
        if replay.replay(&bytes).is_ok() {
            return Err(format!(
                "Range sidecar accepted selected From {attack} attack"
            ));
        }
    }

    eprintln!("Range selected From omission/callee/source replay attacks rejected");
    Ok(())
}

fn forge_observation(sidecar: &mut Value, attack: &str) -> Result<(), String> {
    let observation = sidecar
        .get_mut("observation")
        .ok_or("Missing observation")?;
    let local = observation
        .get_mut("selected_local_from")
        .ok_or("Missing selected From")?;
    match attack {
        "missing-local" => *local = Value::Null,
        "callee" => local["definition"]["definition_identity"] = "another::from".into(),
        "source" => local["source"]["bytes"] =
            "fn from(range: Range<usize>) -> Span { Span { start: range.end, end: range.start } }"
                .into(),
        _ => return Err(format!("Unknown selected From attack {attack}")),
    }

    Ok(())
}
