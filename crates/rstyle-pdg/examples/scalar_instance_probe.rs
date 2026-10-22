//! Check retained owner-local projection evidence; never performs admission.
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
    index: usize,
    projection: &scalar_instances::Projection,
    arguments: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    use rstyle_pdg::{scalar_instances::ScalarArgument, unit_store::UnitStore};
    let expected = match arguments.as_array().map(Vec::as_slice) {
        Some([value]) if value == "i32" => ScalarArgument::I32,
        Some([value]) if value == "u32" => ScalarArgument::U32,
        _ => return Err("Unsupported requested scalar argument".into()),
    };
    let instance =
        UnitStore::capture_scalar_instance(std::sync::Arc::clone(owner), index, projection)?;
    let (retained, definition, argument) = instance
        .scalar_instance_origin()
        .ok_or("Missing retained scalar owner")?;
    if !std::sync::Arc::ptr_eq(retained, owner) || definition != index || argument != expected {
        return Err("Scalar instance changed its original owner or structural argument".into());
    }
    closed_instance(instance, projection)
}

fn closed_instance(
    instance: rstyle_pdg::unit_store::UnitStore,
    projection: &scalar_instances::Projection,
) -> Result<(), Box<dyn std::error::Error>> {
    use rstyle_pdg::unit_store::{DependencyClosure, UnitStore};
    if instance.graph_count() != 0 {
        return Err("Unregistered scalar instance unexpectedly produced graphs".into());
    }
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let [original, projected, options @ ..] = arguments.as_slice() else {
        return Err("usage: scalar_instance_probe ORIGINAL_SNAPSHOT PROJECTION_SNAPSHOT [--shared-sources] [--owned]".into());
    };
    if options
        .iter()
        .any(|option| option != "--shared-sources" && option != "--owned")
    {
        return Err("Unknown projection probe option".into());
    }
    let owned = options.iter().any(|option| option == "--owned");
    let shared = options.iter().any(|option| option == "--shared-sources");
    let facts = original_facts(Path::new(original), shared)?;
    let functions = function_indexes(&facts)?;
    let (owner, originals) = if owned {
        (
            Some(std::sync::Arc::new(
                rstyle_pdg::unit_store::UnitStore::capture(facts)?,
            )),
            None,
        )
    } else {
        (None, Some(facts))
    };
    let inventory: Inventory = read(Path::new(projected))?;
    let count = inventory.requests.len();
    let mut comparisons = 0;
    for entry in inventory.requests {
        let entry = requested_projection(entry, &functions)?;
        if let Some(owner) = &owner {
            owned_projection(
                owner,
                entry.index,
                &entry.projection,
                &entry.request["arguments"],
            )?;
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
        "comparison_count": (!owned).then_some(comparisons), "admissions": "unavailable"})
    );
    Ok(())
}
