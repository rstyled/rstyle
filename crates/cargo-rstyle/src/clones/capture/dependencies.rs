//! Explicit dependency capture selection; this grants no cross-unit semantic authority.
mod metadata;
pub(super) use metadata::select as from_cargo;

use super::{Target, package_context, target_record};
use rstyle_pdg::extraction_profile::ExtractionProfile;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) struct Selected {
    pub unit: Value,
    pub target: Target,
}

pub(super) struct Selection<'a> {
    pub requests: &'a [String],
    pub profile: ExtractionProfile,
    pub target: Option<&'a str>,
}

pub(super) fn select(
    metadata: &Value,
    graph: &Value,
    selection: Selection<'_>,
) -> Result<Vec<Selected>, String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("Missing resolved packages")?;
    let inventory = Inventory::new(graph)?;

    let mut requested = BTreeSet::new();
    let mut selected = Vec::new();

    for request in selection.requests {
        let package = resolve(packages, request)?;
        let identity = text(package, "id")?;
        if !requested.insert(identity) {
            return Err(format!("Duplicate dependency capture request: {request}"));
        }
        selected.push(select_unit(
            package,
            &inventory,
            selection.profile,
            selection.target,
        )?);
    }

    Ok(selected)
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("Missing dependency {key}"))
}

fn resolve<'a>(packages: &'a [Value], request: &str) -> Result<&'a Value, String> {
    let mut exact = Vec::new();
    let mut named = Vec::new();

    for package in packages {
        let identity = text(package, "id")?;
        let name = text(package, "name")?;
        let version = text(package, "version")?;
        if identity == request {
            exact.push(package);
        }
        if request == name || request == format!("{name}@{version}") {
            named.push(package);
        }
    }

    let matches = if exact.is_empty() { named } else { exact };
    match matches.as_slice() {
        [package] => Ok(package),
        [] => Err(format!(
            "Dependency request is absent from resolved metadata: {request}"
        )),
        _ => Err(format!("Ambiguous dependency capture request: {request}")),
    }
}

struct Inventory<'a> {
    units: &'a [Value],
    roots: BTreeSet<usize>,
    reachable: BTreeSet<usize>,
}

impl<'a> Inventory<'a> {
    fn new(graph: &'a Value) -> Result<Self, String> {
        if graph["version"] != 1 {
            return Err("Unsupported Cargo dependency unit graph".into());
        }
        let units = graph["units"].as_array().ok_or("Missing Cargo units")?;
        let roots = graph["roots"].as_array().ok_or("Missing Cargo roots")?;
        if roots.is_empty() {
            return Err("Dependency capture requires explicit Cargo roots".into());
        }

        let roots = roots
            .iter()
            .map(|value| index(value, units.len()))
            .collect::<Result<_, _>>()?;

        let adjacency = units
            .iter()
            .map(|unit| edges(unit, units.len()))
            .collect::<Result<Vec<_>, _>>()?;
        let reachable = reachable(&roots, &adjacency);

        Ok(Self {
            units,
            roots,
            reachable,
        })
    }
}

fn index(value: &Value, count: usize) -> Result<usize, String> {
    let index = value
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or("Invalid Cargo dependency unit index")?;
    if index >= count {
        return Err("Cargo dependency unit index outside inventory".into());
    }
    Ok(index)
}

fn edges(unit: &Value, count: usize) -> Result<Vec<usize>, String> {
    text(unit, "pkg_id")?;
    unit["dependencies"]
        .as_array()
        .ok_or("Missing Cargo dependency edges")?
        .iter()
        .map(|edge| index(&edge["index"], count))
        .collect()
}

fn reachable(roots: &BTreeSet<usize>, adjacency: &[Vec<usize>]) -> BTreeSet<usize> {
    let mut pending: Vec<_> = roots.iter().copied().collect();
    let mut seen = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if seen.insert(index) {
            pending.extend(&adjacency[index]);
        }
    }
    seen
}

fn select_unit(
    package: &Value,
    inventory: &Inventory<'_>,
    profile: ExtractionProfile,
    target: Option<&str>,
) -> Result<Selected, String> {
    let identity = text(package, "id")?;
    let matches: Vec<_> = inventory
        .reachable
        .iter()
        .copied()
        .filter(|index| inventory.units[*index]["pkg_id"] == identity)
        .collect();
    let [selected] = matches.as_slice() else {
        return Err(format!(
            "Dependency requires one reachable compilation unit: {identity}"
        ));
    };
    if inventory.roots.contains(selected) {
        return Err("Dependency capture request selects a primary root unit".into());
    }

    let unit = &inventory.units[*selected];
    check_unit(unit, target)?;
    let target = metadata_target(package, &unit["target"])?;

    Ok(Selected {
        unit: unit.clone(),
        target: target_record(package, target, package_context(package, profile)?)?,
    })
}

fn check_unit(unit: &Value, target: Option<&str>) -> Result<(), String> {
    let kinds = unit["target"]["kind"]
        .as_array()
        .ok_or("Missing dependency target kinds")?;
    let crates = unit["target"]["crate_types"]
        .as_array()
        .ok_or("Missing dependency crate types")?;

    let ordinary = |values: &[Value]| {
        !values.is_empty()
            && values
                .iter()
                .all(|value| matches!(value.as_str(), Some("lib" | "rlib")))
    };

    let platform = unit.get("platform").ok_or("Missing dependency platform")?;
    if !ordinary(kinds)
        || !ordinary(crates)
        || unit["mode"] != "check"
        || match target {
            Some(target) => platform.as_str() != Some(target),
            None => !platform.is_null(),
        }
    {
        return Err(
            "Dependency requires one ordinary check library on the exact requested platform".into(),
        );
    }

    if !unit["profile"]
        .as_object()
        .is_some_and(|profile| !profile.is_empty())
    {
        return Err("Missing dependency compilation profile".into());
    }

    let features = unit["features"]
        .as_array()
        .ok_or("Missing dependency features")?;
    let mut seen = BTreeSet::new();
    for feature in features {
        let feature = feature.as_str().ok_or("Invalid dependency feature")?;
        if !seen.insert(feature) {
            return Err("Duplicate dependency feature".into());
        }
    }

    Ok(())
}

fn metadata_target<'a>(package: &'a Value, captured: &Value) -> Result<&'a Value, String> {
    text(captured, "name")?;
    text(captured, "src_path")?;
    let targets = package["targets"]
        .as_array()
        .ok_or("Missing resolved dependency targets")?;

    let matches: Vec<_> = targets
        .iter()
        .filter(|target| {
            ["name", "src_path", "kind", "crate_types"]
                .iter()
                .all(|key| target.get(key) == captured.get(key))
        })
        .collect();
    let [target] = matches.as_slice() else {
        return Err("Dependency unit lacks one complete exact metadata target".into());
    };
    if target["test"].as_bool().is_none() {
        return Err("Missing dependency target test capability".into());
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn package(name: &str, version: &str) -> Value {
        json!({
            "id": format!("registry+https://example.invalid#{name}@{version}"),
            "name": name, "version": version,
            "manifest_path": format!("/fixture/{name}/Cargo.toml"),
            "targets": [{"name": name, "kind": ["lib"], "crate_types": ["lib"],
                "src_path": format!("/fixture/{name}/src/lib.rs"), "test": true}]
        })
    }

    fn unit(package: &Value, dependencies: &[usize]) -> Value {
        json!({"pkg_id": package["id"], "target": package["targets"][0],
            "mode": "check", "platform": null,
            "profile": {"opt_level": "0", "debug_assertions": true}, "features": [],
            "dependencies": dependencies.iter().map(|index| json!({"index":index})).collect::<Vec<_>>()})
    }

    fn fixture() -> (Value, Value) {
        let root = package("consumer", "1.0.0");
        let middle = package("calendar", "0.3.47");
        let leaf = package("calendar-core", "0.1.8");
        let graph = json!({"version":1, "roots":[0], "units":[
            unit(&root, &[1]), unit(&middle, &[2]), unit(&leaf, &[1])
        ]});
        (json!({"packages":[root, middle, leaf]}), graph)
    }

    fn choose(metadata: &Value, graph: &Value, names: &[&str]) -> Result<Vec<Selected>, String> {
        select(
            metadata,
            graph,
            Selection {
                requests: &names.iter().map(|name| (*name).into()).collect::<Vec<_>>(),
                profile: ExtractionProfile::R0,
                target: None,
            },
        )
    }

    #[test]
    fn exact_names_versions_and_package_ids_select_transitive_units() {
        let (metadata, graph) = fixture();
        let identity = metadata["packages"][2]["id"].as_str().unwrap();
        for request in ["calendar-core", "calendar-core@0.1.8", identity] {
            let selected = choose(&metadata, &graph, &[request]).unwrap();
            assert_eq!(selected.len(), 1);
            assert_eq!(selected[0].unit, graph["units"][2]);
            assert_eq!(
                selected[0].target.source,
                PathBuf::from("/fixture/calendar-core/src/lib.rs")
            );
            assert_eq!(selected[0].target.package, identity);
        }
    }

    #[test]
    fn primary_missing_unreachable_and_duplicate_requests_reject() {
        let (metadata, mut graph) = fixture();
        for requests in [
            vec!["consumer"],
            vec!["missing"],
            vec!["calendar", "calendar@0.3.47"],
        ] {
            assert!(choose(&metadata, &graph, &requests).is_err());
        }
        graph["units"][0]["dependencies"] = json!([]);
        assert!(choose(&metadata, &graph, &["calendar"]).is_err());
    }

    #[test]
    fn ambiguous_metadata_names_require_exact_version_or_identity() {
        let (mut metadata, graph) = fixture();
        metadata["packages"]
            .as_array_mut()
            .unwrap()
            .push(package("calendar", "0.3.46"));
        assert!(choose(&metadata, &graph, &["calendar"]).is_err());
        assert!(choose(&metadata, &graph, &["calendar@0.3.47"]).is_ok());
        metadata["packages"][3]["version"] = json!("0.3.47");
        assert!(choose(&metadata, &graph, &["calendar@0.3.47"]).is_err());
    }

    #[test]
    fn multiple_reachable_profile_feature_and_platform_variants_reject() {
        let (metadata, graph) = fixture();
        for (field, value) in [
            ("features", json!(["large-dates"])),
            ("profile", json!({"opt_level":"3"})),
            ("platform", json!("other-target")),
        ] {
            let mut changed = graph.clone();
            let mut variant = changed["units"][1].clone();
            variant[field] = value;
            changed["units"].as_array_mut().unwrap().push(variant);
            changed["units"][0]["dependencies"]
                .as_array_mut()
                .unwrap()
                .push(json!({"index":3}));
            assert!(choose(&metadata, &changed, &["calendar"]).is_err());
        }
    }

    #[test]
    fn exact_requested_platform_is_required_and_missing_is_not_host() {
        let (metadata, mut graph) = fixture();
        let requests = ["calendar".into()];
        let selected = |graph: &Value| {
            select(
                &metadata,
                graph,
                Selection {
                    requests: &requests,
                    profile: ExtractionProfile::R0,
                    target: Some("aarch64-apple-darwin"),
                },
            )
        };
        assert!(selected(&graph).is_err());
        graph["units"][1]["platform"] = json!("aarch64-apple-darwin");
        assert!(selected(&graph).is_ok());
        assert!(choose(&metadata, &graph, &["calendar"]).is_err());
        graph["units"][1]
            .as_object_mut()
            .unwrap()
            .remove("platform");
        assert!(choose(&metadata, &graph, &["calendar"]).is_err());
    }

    #[test]
    fn build_and_proc_macro_targets_and_noncheck_modes_reject() {
        let (metadata, graph) = fixture();
        for kind in ["custom-build", "proc-macro", "bin", "dylib"] {
            let mut changed = graph.clone();
            changed["units"][1]["target"]["kind"] = json!([kind]);
            assert!(choose(&metadata, &changed, &["calendar"]).is_err());
        }
        for mode in ["build", "test", "run-custom-build"] {
            let mut changed = graph.clone();
            changed["units"][1]["mode"] = json!(mode);
            assert!(choose(&metadata, &changed, &["calendar"]).is_err());
        }
    }

    #[test]
    fn invalid_graph_roots_edges_and_missing_inventories_reject() {
        let (metadata, graph) = fixture();
        for bad_index in [json!(99), json!(-1), json!("1"), Value::Null] {
            let mut changed = graph.clone();
            changed["roots"][0] = bad_index.clone();
            assert!(choose(&metadata, &changed, &["calendar"]).is_err());
            changed = graph.clone();
            changed["units"][2]["dependencies"][0]["index"] = bad_index;
            assert!(choose(&metadata, &changed, &["calendar"]).is_err());
        }
        for field in ["units", "roots"] {
            let mut changed = graph.clone();
            changed.as_object_mut().unwrap().remove(field);
            assert!(choose(&metadata, &changed, &["calendar"]).is_err());
        }
        let mut changed = graph;
        changed["units"][2]
            .as_object_mut()
            .unwrap()
            .remove("dependencies");
        assert!(choose(&metadata, &changed, &["calendar"]).is_err());
    }

    #[test]
    fn target_matching_requires_exact_source_name_kind_and_crate_type() {
        let (metadata, graph) = fixture();
        for (field, value) in [
            ("src_path", json!("/other/lib.rs")),
            ("name", json!("other")),
            ("kind", json!(["rlib"])),
            ("crate_types", json!(["rlib"])),
        ] {
            let mut changed = graph.clone();
            changed["units"][1]["target"][field] = value;
            assert!(choose(&metadata, &changed, &["calendar"]).is_err());
        }
    }
}
