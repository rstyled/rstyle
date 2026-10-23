//! Check retained owner-local projections and strict instance admission/replay.
use rstyle_pdg::{HirFnFacts, scalar_instances};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
struct Originals {
    facts: Vec<HirFnFacts>,
}

#[derive(Deserialize)]
struct Inventory {
    requests: Vec<serde_json::Value>,
}

struct RequestedProjection {
    request: serde_json::Value,
    index: usize,
    projection: scalar_instances::Projection,
}

fn requested_projection(
    mut entry: serde_json::Value,
    functions: &BTreeMap<String, usize>,
) -> Result<RequestedProjection, Box<dyn std::error::Error>> {
    let request = entry["request"].clone();
    if entry["status"] != "projected" {
        return Err(format!("Unavailable projection: {request}").into());
    }
    let name = request["function"]
        .as_str()
        .ok_or("Missing requested function")?;
    let index = *functions.get(name).ok_or("Missing original definition")?;
    let object = entry.as_object_mut().ok_or("Missing projection object")?;
    object.remove("request");
    object.remove("status");
    Ok(RequestedProjection {
        request,
        index,
        projection: serde_json::from_value(entry)?,
    })
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Box<dyn std::error::Error>> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    Ok(serde_json::from_reader(reader)?)
}

fn original_facts(
    path: &Path,
    shared: bool,
) -> Result<Vec<HirFnFacts>, Box<dyn std::error::Error>> {
    if shared {
        Ok(rstyle_pdg::unit_store::read_diagnostic_snapshot(
            &std::fs::read(path)?,
        )?)
    } else {
        Ok(read::<Originals>(path)?.facts)
    }
}

fn function_indexes(
    facts: &[HirFnFacts],
) -> Result<BTreeMap<String, usize>, Box<dyn std::error::Error>> {
    let mut functions = BTreeMap::new();
    for (index, facts) in facts.iter().enumerate() {
        if functions.insert(facts.name.clone(), index).is_some() {
            return Err("Ambiguous original definition name".into());
        }
    }
    Ok(functions)
}

fn owned_projection(
    owner: &std::sync::Arc<rstyle_pdg::unit_store::UnitStore>,
    entry: &RequestedProjection,
    replay: Option<&rstyle_pdg::unit_store::ScalarInstanceReplay>,
) -> Result<(), Box<dyn std::error::Error>> {
    use rstyle_pdg::{scalar_instances::ScalarArgument, unit_store::UnitStore};
    let expected = match entry.request["arguments"].as_array().map(Vec::as_slice) {
        Some([value]) if value == "i32" => ScalarArgument::I32,
        Some([value]) if value == "u32" => ScalarArgument::U32,
        _ => return Err("Unsupported requested scalar argument".into()),
    };
    let instance = UnitStore::capture_scalar_instance(
        std::sync::Arc::clone(owner),
        entry.index,
        &entry.projection,
    )?;
    let (retained, definition, argument) = instance
        .scalar_instance_origin()
        .ok_or("Missing retained scalar owner")?;
    if !std::sync::Arc::ptr_eq(retained, owner) || definition != entry.index || argument != expected
    {
        return Err("Scalar instance changed its original owner or structural argument".into());
    }
    if let Some(session) = replay {
        replay_projection(session, &instance, &entry.projection)?;
    }
    closed_instance(instance, &entry.projection)
}

fn replay_projection(
    session: &rstyle_pdg::unit_store::ScalarInstanceReplay,
    instance: &rstyle_pdg::unit_store::UnitStore,
    projection: &scalar_instances::Projection,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    instance.write_scalar_instance_sidecar(&mut bytes, projection)?;
    let replayed = session.replay(&bytes)?;
    let (owner, ordinal, scalar) = instance
        .scalar_instance_origin()
        .ok_or("Missing captured instance origin")?;
    let (restored, definition, argument) = replayed
        .scalar_instance_origin()
        .ok_or("Missing replayed instance origin")?;
    if !std::sync::Arc::ptr_eq(owner, restored)
        || ordinal != definition
        || scalar != argument
        || instance.functions() != replayed.functions()
        || instance.graph_count() != replayed.graph_count()
    {
        return Err("Scalar sidecar replay changed its checked origin, facts or graph".into());
    }
    replay_attacks(session, &bytes)?;
    closed_instance(replayed, projection)
}

fn replay_attacks(
    session: &rstyle_pdg::unit_store::ScalarInstanceReplay,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let original: serde_json::Value = serde_json::from_slice(bytes)?;
    for attack in ["owner", "ordinal", "arguments", "build", "trust"] {
        let mut forged = original.clone();
        match attack {
            "owner" => forged["owner_namespace"]["ordinal"] = serde_json::json!(u64::MAX),
            "ordinal" => forged["definition_ordinal"] = serde_json::json!(usize::MAX),
            "arguments" => {
                forged["projection"]["arguments"][0]["identity"]["name"] = serde_json::json!("u64")
            }
            "build" => forged["projection"]["build"] = serde_json::json!({"trusted":true}),
            _ => forged["verified"] = serde_json::json!(true),
        }
        if session.replay(&serde_json::to_vec(&forged)?).is_ok() {
            return Err(format!("Scalar sidecar accepted {attack} forgery").into());
        }
    }
    Ok(())
}

fn closed_instance(
    instance: rstyle_pdg::unit_store::UnitStore,
    projection: &scalar_instances::Projection,
) -> Result<(), Box<dyn std::error::Error>> {
    use rstyle_pdg::unit_store::{DependencyClosure, UnitStore};
    let analysis = instance.analysis();
    let view = analysis.function(0).ok_or("Missing scalar instance body")?;
    let classification = rstyle_pdg::classify(&view);
    if classification.status != rstyle_pdg::EligibilityVerdict::Eligible
        || instance.graph_count() != 1
    {
        return Err(format!("Scalar instance failed admission: {classification:?}").into());
    }
    drop(analysis);
    if instance.write_bundle(std::io::sink()).is_ok() {
        return Err("Ordinary bundle export accepted scalar instance".into());
    }
    let instance = std::sync::Arc::new(instance);
    if UnitStore::capture_scalar_instance(std::sync::Arc::clone(&instance), 0, projection).is_ok() {
        return Err("Scalar instance was accepted as an original definition owner".into());
    }
    let instance = std::sync::Arc::try_unwrap(instance)
        .map_err(|_| "Unexpected retained scalar analysis owner")?;
    if DependencyClosure::from_units_with(vec![instance], &[], |_, associations| {
        if associations.is_empty() {
            Ok(())
        } else {
            Err("Unexpected associations".into())
        }
    })
    .is_ok()
    {
        return Err("Ordinary dependency closure accepted scalar instance".into());
    }
    Ok(())
}

struct Roster {
    owner: Option<std::sync::Arc<rstyle_pdg::unit_store::UnitStore>>,
    originals: Option<Vec<HirFnFacts>>,
}

impl Roster {
    fn new(facts: Vec<HirFnFacts>, owned: bool) -> Result<Self, String> {
        if owned {
            Ok(Self {
                owner: Some(std::sync::Arc::new(
                    rstyle_pdg::unit_store::UnitStore::capture(facts)?,
                )),
                originals: None,
            })
        } else {
            Ok(Self {
                owner: None,
                originals: Some(facts),
            })
        }
    }
}

fn replay_session(
    owner: Option<&std::sync::Arc<rstyle_pdg::unit_store::UnitStore>>,
    replay: bool,
    definitions: usize,
) -> Result<Option<rstyle_pdg::unit_store::ScalarInstanceReplay>, String> {
    owner
        .filter(|_| replay)
        .map(|owner| {
            rstyle_pdg::unit_store::ScalarInstanceReplay::new(
                std::sync::Arc::clone(owner),
                |facts| {
                    if facts.len() != definitions {
                        return Err("Original definition roster changed".into());
                    }
                    Ok(())
                },
            )
        })
        .transpose()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let [original, projected, options @ ..] = arguments.as_slice() else {
        return Err("usage: scalar_instance_probe ORIGINAL_SNAPSHOT PROJECTION_SNAPSHOT [--shared-sources] [--owned] [--replay]".into());
    };
    if options
        .iter()
        .any(|option| option != "--shared-sources" && option != "--owned" && option != "--replay")
    {
        return Err("Unknown projection probe option".into());
    }
    let replay = options.iter().any(|option| option == "--replay");
    let owned = replay || options.iter().any(|option| option == "--owned");
    let shared = options.iter().any(|option| option == "--shared-sources");
    let facts = original_facts(Path::new(original), shared)?;
    let functions = function_indexes(&facts)?;
    let roster = Roster::new(facts, owned)?;
    let owner = roster.owner;
    let originals = roster.originals;
    let session = replay_session(owner.as_ref(), replay, functions.len())?;
    let inventory: Inventory = read(Path::new(projected))?;
    let count = inventory.requests.len();
    let mut comparisons = 0;
    for entry in inventory.requests {
        let entry = requested_projection(entry, &functions)?;
        if let Some(owner) = &owner {
            owned_projection(owner, &entry, session.as_ref())?;
        } else {
            comparisons += scalar_instances::verify(
                &originals.as_ref().ok_or("Missing raw original roster")?[entry.index],
                &entry.projection,
            )?
            .comparison_count();
        }
    }
    println!(
        "{}",
        serde_json::json!({"projections_verified": count, "owned": owned,
        "comparison_count": (!owned).then_some(comparisons), "sidecars_replayed": if replay {count} else {0}, "admissions": owned.then_some(count)})
    );
    Ok(())
}
