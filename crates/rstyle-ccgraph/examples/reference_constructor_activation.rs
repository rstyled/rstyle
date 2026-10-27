//! Replay complete captured originals through checked constructor admission.
use rstyle_pdg::{
    reference_instances::Haystack,
    unit_store::{self, ReferenceConstructorReplay, ReferenceForwarderReplay, UnitStore},
};
use serde_json::Value;
use std::sync::Arc;

const CONSTRUCTOR: &str = "util::search::Input::<'h>::new";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [raw, observations] = args.as_slice() else {
        return Err("Expected complete raw owner snapshot and reference observations".into());
    };
    let facts = unit_store::read_diagnostic_snapshot(&std::fs::read(raw)?)?;
    let inventory: Value = serde_json::from_slice(&std::fs::read(observations)?)?;
    let ordinal = facts
        .iter()
        .position(|fact| fact.name == CONSTRUCTOR)
        .ok_or("Missing original constructor")?;
    let count = facts.len();
    let owner = Arc::new(UnitStore::capture(facts)?);
    let session = ReferenceConstructorReplay::new(Arc::clone(&owner), |facts| {
        if facts.len() != count || facts[ordinal].name != CONSTRUCTOR {
            return Err("Original constructor roster changed".into());
        }
        Ok(())
    })?;
    for haystack in [Haystack::Bytes, Haystack::Str] {
        check_constructor(&session, &inventory, ordinal, haystack)?;
    }
    let session = ReferenceForwarderReplay::from_constructor(session);
    check_forwarders(&session, &inventory, ordinal)?;
    Ok(())
}

fn check_constructor(
    session: &ReferenceConstructorReplay,
    inventory: &Value,
    ordinal: usize,
    haystack: Haystack,
) -> Result<(), String> {
    let observation = constructor(inventory, haystack)?;
    let instance = UnitStore::capture_reference_constructor(
        Arc::clone(session.owner()),
        ordinal,
        observation,
        haystack,
    )?;
    require_admission(&instance)?;
    expected_component(&instance, haystack)?;
    let mut bytes = Vec::new();
    instance.write_reference_constructor_sidecar(&mut bytes)?;
    let replayed = session.replay(&bytes)?;
    require_admission(&replayed)?;
    expected_component(&replayed, haystack)?;
    matching(&instance, &replayed)?;
    attacks(&replayed, haystack)?;
    println!(
        "Original {} constructor admitted and strictly replayed; complete owner={}, ordinal={ordinal}, sidecar={} bytes",
        haystack.spelling(),
        session.owner().functions().len(),
        bytes.len()
    );
    Ok(())
}

fn constructor(inventory: &Value, haystack: Haystack) -> Result<&Value, String> {
    let requests = inventory["requests"]
        .as_array()
        .ok_or("Missing observations")?;
    let matches: Vec<_> = requests
        .iter()
        .filter(|request| {
            request["request"]["function"] == CONSTRUCTOR
                && request["request"]["haystack"] == haystack.spelling()
                && request["status"] == "observed"
        })
        .collect();
    let [observation] = matches.as_slice() else {
        return Err("Expected exactly one observed selected original constructor".into());
    };
    Ok(observation)
}

fn require_admission(instance: &UnitStore) -> Result<(), String> {
    let analysis = instance.analysis();
    let view = analysis.function(0).ok_or("Missing concrete constructor")?;
    let result = rstyle_pdg::classify(&view);
    if result.status != rstyle_pdg::EligibilityVerdict::Eligible || instance.pair(0).is_none() {
        return Err(format!(
            "Original constructor failed complete admission: {result:?}"
        ));
    }
    if instance.write_bundle(std::io::sink()).is_ok() {
        return Err("Ordinary bundle export accepted reference instance authority".into());
    }
    Ok(())
}

fn expected_component(instance: &UnitStore, haystack: Haystack) -> Result<(), String> {
    let analysis = instance.analysis();
    let view = analysis.function(0).ok_or("Missing checked instance")?;
    let pair = instance.pair(0).ok_or("Missing checked instance graph")?;
    let outputs = rstyle_pdg::helper_component_outputs::derive(&view, pair.into())?
        .ok_or("Constructor lost its complete component summary")?;
    let path = if haystack == Haystack::Str {
        serde_json::json!([{"kind":"SharedReferent"}, {"kind":"Utf8Bytes"}])
    } else {
        serde_json::json!([{"kind":"SharedReferent"}])
    };
    let expected = serde_json::json!([{
        "selectors":[0],
        "ty":{"kind":"shared","value":{"kind":"slice","value":{"kind":"scalar","value":"u8"}}},
        "origins":[{"parameter":0,"path":path}]
    }]);
    if serde_json::to_value(outputs).map_err(|error| error.to_string())? != expected {
        return Err(
            "Constructor differs from independently specified saved byte-view component".into(),
        );
    }
    Ok(())
}

fn imported(instance: &UnitStore) -> Result<rstyle_ccgraph::ImportedFunction, String> {
    let analysis = instance.analysis();
    let view = analysis.function(0).ok_or("Missing checked instance")?;
    let pair = instance
        .pair(0)
        .ok_or("Missing complete constructor graph")?;
    let imported =
        rstyle_ccgraph::admit_produced(pair, &view).map_err(|errors| errors.join("; "))?;
    if imported.matching_profile != rstyle_ccgraph::MatchingProfile::DataCallsV1 {
        return Err("Constructor lost its registered DataCalls matching profile".into());
    }
    Ok(imported)
}

fn matching(original: &UnitStore, replayed: &UnitStore) -> Result<(), String> {
    let mut first = imported(original)?;
    let mut second = imported(replayed)?;
    first.key = "original-constructor".into();
    second.key = "strictly-replayed-constructor".into();
    let report = rstyle_ccgraph::evaluate_all(&[first, second], &Default::default());
    if report.incomplete
        || report.pairs.len() != 1
        || report.pairs[0].decision != rstyle_ccgraph::Decision::Accepted
    {
        return Err(format!(
            "Original/replayed identity matching failed: {report:?}"
        ));
    }
    println!("Strict producer admission and complete selected identity comparison: Accepted");
    Ok(())
}

fn attacks(instance: &UnitStore, haystack: Haystack) -> Result<(), String> {
    let analysis = instance.analysis();
    let view = analysis.function(0).ok_or("Missing checked constructor")?;
    let pair = instance.pair(0).ok_or("Missing checked graph")?;
    for attack in ["tag", "literal", "origin", "summary", "policy"] {
        let mut forged = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        mutate(&mut forged, attack, haystack)?;
        if rstyle_pdg::producer::verify(&forged, &view).is_ok() {
            return Err(format!("Constructor verifier accepted {attack} corruption"));
        }
    }
    if rstyle_pdg::classify(&*view).status == rstyle_pdg::EligibilityVerdict::Eligible {
        return Err("Raw concrete facts gained private reference instance authority".into());
    }
    println!(
        "Graph tag/literal/origin/summary/historical-policy and raw-authority attacks rejected"
    );
    Ok(())
}

fn mutate(
    pair: &mut rstyle_pdg::producer::Pair,
    attack: &str,
    haystack: Haystack,
) -> Result<(), String> {
    if attack == "policy" {
        let historical = if haystack == Haystack::Str {
            "3.85.0"
        } else {
            "3.84.0"
        };
        pair.pdg.profile.version = historical.into();
        pair.cfg.profile.version = historical.into();
        return Ok(());
    }
    if attack == "summary" {
        let key = rstyle_pdg::call_summaries::KEY;
        pair.pdg
            .extensions
            .get_mut(key)
            .ok_or("Missing summaries")?[0]["summary"]["always_returns"] = Value::Bool(false);
        pair.cfg
            .extensions
            .insert(key.into(), pair.pdg.extensions[key].clone());
        return Ok(());
    }
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.attributes.construct.as_deref()
                == Some(if attack == "origin" {
                    "data.reference-instance.as-ref"
                } else {
                    "data.reference-instance-unit"
                })
        })
        .ok_or("Missing attack site")?;
    match attack {
        "tag" => node.attributes.operator = Some(rstyle_pdg::StringFact::known("Yes")),
        "literal" => node.attributes.literal = Some(rstyle_pdg::StringFact::known("0")),
        _ => {
            node.annotations
                .insert(rstyle_pdg::data_origins::KEY.into(), serde_json::json!([]));
        }
    }
    Ok(())
}

fn check_forwarders(
    session: &ReferenceForwarderReplay,
    inventory: &Value,
    constructor_ordinal: usize,
) -> Result<(), String> {
    let requests = inventory["requests"].as_array().ok_or("Missing requests")?;
    let selected: Vec<_> = requests
        .iter()
        .filter(|request| request["status"] == "observed")
        .collect();
    if selected.len() != 4 {
        return Err("Original four-instance selection changed".into());
    }
    for request in selected {
        check_forwarder(session, inventory, constructor_ordinal, request)?;
    }
    println!("Unchanged development selection: 4 admitted out of 4");
    Ok(())
}

fn check_forwarder(
    session: &ReferenceForwarderReplay,
    inventory: &Value,
    constructor_ordinal: usize,
    request: &Value,
) -> Result<(), String> {
    let owner = session.owner();
    let name = request["request"]["function"]
        .as_str()
        .ok_or("Missing function")?;
    if name == CONSTRUCTOR {
        return Ok(());
    }
    let haystack = match request["request"]["haystack"].as_str() {
        Some("str") => Haystack::Str,
        Some("[u8]") => Haystack::Bytes,
        _ => return Err("Unexpected substitution".into()),
    };
    let ordinal = owner
        .functions()
        .iter()
        .position(|facts| facts.name == name)
        .ok_or("Missing From")?;
    let constructor = Arc::new(UnitStore::capture_reference_constructor(
        Arc::clone(owner),
        constructor_ordinal,
        constructor(inventory, haystack)?,
        haystack,
    )?);
    let instance = UnitStore::capture_reference_forwarder(
        Arc::clone(owner),
        ordinal,
        request,
        Arc::clone(&constructor),
    )?;
    let wrong_owner = Arc::new(UnitStore::capture(Vec::new())?);
    if UnitStore::capture_reference_forwarder(wrong_owner, ordinal, request, constructor).is_ok() {
        return Err("From accepted constructor from another original owner".into());
    }
    require_admission(&instance)?;
    expected_component(&instance, haystack)?;
    let mut sidecar = Vec::new();
    instance.write_reference_forwarder_sidecar(&mut sidecar)?;
    let replayed = session.replay(&sidecar)?;
    require_admission(&replayed)?;
    expected_component(&replayed, haystack)?;
    matching(&instance, &replayed)?;
    forwarder_attacks(session, &sidecar, &replayed)?;
    println!(
        "Original From<{}> admitted with exact constructor dependency and independently specified component origin",
        haystack.spelling()
    );
    Ok(())
}

fn forwarder_attacks(
    session: &ReferenceForwarderReplay,
    sidecar: &[u8],
    instance: &UnitStore,
) -> Result<(), String> {
    attack_forwarder_sidecar(session, sidecar)?;
    let analysis = instance.analysis();
    let view = analysis.function(0).ok_or("Missing From")?;
    let pair = instance.pair(0).ok_or("Missing From pair")?;
    for attack in ["callee", "origin", "dependency", "policy"] {
        let mut forged = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        mutate_forwarder_pair(&mut forged, attack)?;
        if rstyle_pdg::verification_policy::strict(|| rstyle_pdg::producer::verify(&forged, &view))
            .is_ok()
        {
            return Err(format!("From graph accepted {attack} attack"));
        }
    }
    if rstyle_pdg::producer::verify(pair, &*view).is_ok() {
        return Err("Raw From facts acquired private authority".into());
    }
    println!(
        "From sidecar substitution/ordinal/dependency/owner and graph callee/dependency/historical/raw-authority attacks rejected"
    );
    Ok(())
}

fn attack_forwarder_sidecar(
    session: &ReferenceForwarderReplay,
    sidecar: &[u8],
) -> Result<(), String> {
    let original: Value = serde_json::from_slice(sidecar).map_err(|e| e.to_string())?;
    for attack in ["substitution", "ordinal", "constructor", "namespace"] {
        let mut forged = original.clone();
        match attack {
            "substitution" => {
                forged["haystack"] = Value::String(
                    if original["haystack"] == "str" {
                        "[u8]"
                    } else {
                        "str"
                    }
                    .into(),
                );
            }
            "ordinal" => {
                forged["definition_ordinal"] = forged["constructor"]["definition_ordinal"].clone()
            }
            "constructor" => {
                forged["constructor"]["definition_ordinal"] = forged["definition_ordinal"].clone()
            }
            "namespace" => forged["owner_namespace"] = Value::Null,
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&forged).map_err(|e| e.to_string())?;
        if session.replay(&bytes).is_ok() {
            return Err(format!("From sidecar accepted {attack} attack"));
        }
    }
    Ok(())
}

fn mutate_forwarder_pair(
    forged: &mut rstyle_pdg::producer::Pair,
    attack: &str,
) -> Result<(), String> {
    match attack {
        "callee" | "origin" => {
            let node = forged
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
                .ok_or("Missing From CALL")?;
            if attack == "origin" {
                node.annotations
                    .insert(rstyle_pdg::data_origins::KEY.into(), serde_json::json!([]));
            } else {
                node.attributes.callee =
                    Some(rstyle_pdg::StringFact::known("another::constructor"));
            }
        }
        "dependency" => {
            forged
                .pdg
                .extensions
                .get_mut(rstyle_pdg::call_summaries::KEY)
                .ok_or("Missing calls")?[0]["dependency"] = Value::Null;
            forged.cfg.extensions.insert(
                rstyle_pdg::call_summaries::KEY.into(),
                forged.pdg.extensions[rstyle_pdg::call_summaries::KEY].clone(),
            );
        }
        "policy" => {
            forged.pdg.profile.version = "3.86.0".into();
            forged.cfg.profile.version = "3.86.0".into();
        }
        _ => unreachable!(),
    }
    Ok(())
}
