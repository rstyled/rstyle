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

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Box<dyn std::error::Error>> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    Ok(serde_json::from_reader(reader)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let [original, projected] = arguments.as_slice() else {
        return Err("usage: scalar_instance_probe ORIGINAL_SNAPSHOT PROJECTION_SNAPSHOT".into());
    };
    let originals: Originals = read(Path::new(original))?;
    let inventory: Inventory = read(Path::new(projected))?;
    let mut functions = BTreeMap::new();
    for facts in &originals.facts {
        if functions.insert(facts.name.as_str(), facts).is_some() {
            return Err("Ambiguous original definition name".into());
        }
    }
    for mut entry in inventory.requests {
        let request = entry["request"].clone();
        if entry["status"] != "projected" {
            return Err(format!("Unavailable projection: {request}").into());
        }
        let name = request["function"]
            .as_str()
            .ok_or("Missing requested function")?;
        let original = functions.get(name).ok_or("Missing original definition")?;
        let object = entry.as_object_mut().ok_or("Missing projection object")?;
        object.remove("request");
        object.remove("status");
        let projection = serde_json::from_value(entry)?;
        let checked = scalar_instances::verify(original, &projection)?;
        println!(
            "{request}: verified {} comparisons; admission unavailable",
            checked.comparison_count()
        );
    }
    Ok(())
}
