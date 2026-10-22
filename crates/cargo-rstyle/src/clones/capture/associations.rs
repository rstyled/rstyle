//! Run-local compiled-artifact associations, separate from semantic call authority.
use super::{Unit, invocation_features};
use rstyle_core::cli_args::flag_value;
use rstyle_pdg::{HirFnFacts, HirNode, unit_store::UnitNamespace};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

const COMPILED: &str = "rstyle.rust.compiled-crate";
const IDENTITY: &str = "rstyle.rust.compiled-crate/1";
const COMPILER_COMMIT: &str = "5c543b0b8c73c7b72bc8284ced4fb22ead15734d";
const METADATA_HEADER: &[u8; 8] = b"rust\0\0\0\x0b";

pub(in crate::clones) struct ObservedUnit<'a> {
    pub namespace: UnitNamespace,
    pub functions: &'a [HirFnFacts],
}

pub(in crate::clones) struct Request<'a> {
    pub graph: &'a Value,
    pub cargo: &'a [Value],
    pub units: &'a [Unit],
    pub observations: &'a [ObservedUnit<'a>],
}

struct Prepared<'a> {
    graph_index: usize,
    artifact: &'a Value,
    local: Option<&'a Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArtifactSeal {
    bytes: u64,
    checksum: String,
    crate_hash: String,
}

type Seals = BTreeMap<PathBuf, ArtifactSeal>;
type Descriptor = (String, String, String);
type Associations = BTreeMap<(usize, usize), Value>;

struct Targets<'a> {
    cargo: BTreeMap<Descriptor, Vec<&'a Value>>,
    graph: BTreeMap<Descriptor, Vec<(usize, &'a Value)>>,
}

type CrateRoutes<'a> = BTreeMap<&'a str, BTreeMap<&'a str, Option<usize>>>;
type DependencyRoutes<'a> = BTreeMap<(usize, usize), Option<&'a str>>;

struct Routes<'a> {
    crates: CrateRoutes<'a>,
    edges: DependencyRoutes<'a>,
}

pub(in crate::clones) fn associate(request: Request<'_>) -> Result<Vec<Value>, String> {
    let reachable = validate_graph(request.graph)?;
    if request.units.len() != request.observations.len() {
        return Err("Compiled association observations do not match ledger inventory".into());
    }

    let targets = Targets::new(&request)?;
    let prepared = request
        .units
        .iter()
        .zip(request.observations)
        .map(|(unit, observed)| prepare(&targets, unit, observed))
        .collect::<Result<Vec<_>, _>>()?;
    unique_units(&prepared, request.observations, &reachable)?;
    let routes = Routes::new(&request, &prepared)?;
    let context = CallContext {
        request: &request,
        prepared: &prepared,
        routes: &routes,
    };

    let mut seals = Seals::new();
    let mut associations = Associations::new();
    for (caller, observed) in request.observations.iter().enumerate() {
        for function in observed.functions {
            visit_calls(&function.body, &mut |resolution| {
                associate_call(
                    &context,
                    caller,
                    resolution,
                    &mut Output {
                        seals: &mut seals,
                        associations: &mut associations,
                    },
                )
            })?;
        }
    }

    Ok(associations.into_values().collect())
}

impl<'a> Targets<'a> {
    fn new(request: &Request<'a>) -> Result<Self, String> {
        let mut cargo: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for artifact in request
            .cargo
            .iter()
            .filter(|value| value["reason"] == "compiler-artifact")
        {
            cargo
                .entry(descriptor(artifact, "package_id")?)
                .or_default()
                .push(artifact);
        }

        let mut graph: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (index, unit) in request.graph["units"]
            .as_array()
            .ok_or("Missing graph units")?
            .iter()
            .enumerate()
        {
            graph
                .entry(descriptor(unit, "pkg_id")?)
                .or_default()
                .push((index, unit));
        }

        Ok(Self { cargo, graph })
    }
}

fn descriptor(value: &Value, package: &str) -> Result<Descriptor, String> {
    Ok((
        string(value, package)?.into(),
        string(&value["target"], "name")?.into(),
        string(&value["target"], "src_path")?.into(),
    ))
}

impl<'a> Routes<'a> {
    fn new(request: &'a Request<'_>, prepared: &'a [Prepared<'_>]) -> Result<Self, String> {
        Ok(Self {
            crates: Self::crate_routes(request, prepared)?,
            edges: Self::dependency_routes(request.graph)?,
        })
    }

    fn crate_routes(
        request: &'a Request<'_>,
        prepared: &'a [Prepared<'_>],
    ) -> Result<CrateRoutes<'a>, String> {
        let mut crates: BTreeMap<_, BTreeMap<_, Option<usize>>> = BTreeMap::new();
        for (index, unit) in request.units.iter().enumerate() {
            let identities = crates.entry(unit.target.name.as_str()).or_default();
            if let Some(local) = prepared[index].local {
                let stable = string(local, "stable_crate_id")?;
                identities
                    .entry(stable)
                    .and_modify(|value| *value = None)
                    .or_insert(Some(index));
            }
        }

        Ok(crates)
    }

    fn dependency_routes(graph: &'a Value) -> Result<DependencyRoutes<'a>, String> {
        let mut edges = BTreeMap::new();
        let graph = graph["units"].as_array().ok_or("Missing graph units")?;
        for (caller, unit) in graph.iter().enumerate() {
            for edge in unit["dependencies"]
                .as_array()
                .ok_or("Missing graph edges")?
            {
                let key = (caller, integer(&edge["index"], graph.len())?);
                edges
                    .entry(key)
                    .and_modify(|value| *value = None)
                    .or_insert(Some(string(edge, "extern_crate_name")?));
            }
        }

        Ok(edges)
    }

    fn foreign<'b>(&self, resolution: &'b Value) -> Result<Option<(usize, &'b Value)>, String> {
        let name = string(resolution, "crate")?;
        let Some(identities) = self.crates.get(name) else {
            return Ok(None);
        };

        let foreign = resolution
            .get("compiled_dependency")
            .ok_or("Selected dependency call lacks compiler identity")?;
        check_foreign(foreign, name)?;
        let callee = identities
            .get(string(foreign, "stable_crate_id")?)
            .copied()
            .flatten()
            .ok_or("Selected dependency has ambiguous or wrong stable crate identity")?;

        Ok(Some((callee, foreign)))
    }

    fn alias(&self, caller: usize, callee: usize) -> Result<&str, String> {
        self.edges
            .get(&(caller, callee))
            .copied()
            .flatten()
            .ok_or_else(|| "Selected foreign call lacks one exact Cargo dependency edge".into())
    }
}

fn unique_units(
    prepared: &[Prepared<'_>],
    observed: &[ObservedUnit<'_>],
    reachable: &BTreeSet<usize>,
) -> Result<(), String> {
    let mut graphs = BTreeSet::new();
    let mut namespaces = BTreeSet::new();
    for (index, unit) in prepared.iter().enumerate() {
        let namespace =
            serde_json::to_string(&observed[index].namespace).map_err(|error| error.to_string())?;
        if !reachable.contains(&unit.graph_index)
            || !graphs.insert(unit.graph_index)
            || !namespaces.insert(namespace)
        {
            return Err("Unreachable or duplicate compiled graph-unit/evidence namespace".into());
        }
    }
    Ok(())
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Missing compiled association {key}"))
}

fn integer(value: &Value, count: usize) -> Result<usize, String> {
    value
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .filter(|index| *index < count)
        .ok_or_else(|| "Invalid compiled association graph index".into())
}

fn validate_graph(graph: &Value) -> Result<BTreeSet<usize>, String> {
    if graph["version"] != 1 {
        return Err("Unsupported compiled association graph".into());
    }
    let units = graph["units"]
        .as_array()
        .ok_or("Missing association graph units")?;
    let roots = graph["roots"]
        .as_array()
        .ok_or("Missing association graph roots")?;
    if roots.is_empty() {
        return Err("Missing association graph root authority".into());
    }

    for root in roots {
        integer(root, units.len())?;
    }
    for unit in units {
        string(unit, "pkg_id")?;
        for edge in unit["dependencies"]
            .as_array()
            .ok_or("Missing association dependency edges")?
        {
            integer(&edge["index"], units.len())?;
            string(edge, "extern_crate_name")?;
        }
    }

    graph_reachability(units, roots)
}

fn graph_reachability(units: &[Value], roots: &[Value]) -> Result<BTreeSet<usize>, String> {
    let mut pending = roots
        .iter()
        .map(|root| integer(root, units.len()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    while let Some(current) = pending.pop() {
        if seen.insert(current) {
            for edge in units[current]["dependencies"]
                .as_array()
                .ok_or("Missing graph edges")?
            {
                pending.push(integer(&edge["index"], units.len())?);
            }
        }
    }
    Ok(seen)
}

fn prepare<'a>(
    targets: &Targets<'a>,
    unit: &Unit,
    observed: &'a ObservedUnit<'_>,
) -> Result<Prepared<'a>, String> {
    let key = (
        unit.target.package.clone(),
        unit.target.name.clone(),
        unit.target.source.to_string_lossy().into_owned(),
    );
    let artifacts = targets
        .cargo
        .get(&key)
        .ok_or("Missing successful Cargo artifact for captured ledger unit")?;
    let [artifact] = artifacts.as_slice() else {
        return Err("Ambiguous Cargo artifact for captured ledger unit".into());
    };
    if unit.test
        || artifact["profile"]["test"] != false
        || !target_matches(&artifact["target"], unit)
    {
        return Err("Compiled dependency association requires exact non-test library units".into());
    }

    let candidates = targets
        .graph
        .get(&key)
        .ok_or("Missing ledger/compiler-artifact graph context")?;
    let candidates: Vec<_> = candidates
        .iter()
        .filter(|(_, candidate)| graph_matches(candidate, artifact, unit))
        .collect();
    let [(graph_index, _)] = candidates.as_slice() else {
        return Err("Ambiguous or missing ledger/compiler-artifact graph context".into());
    };

    let local = local_identity(unit, observed)?;
    Ok(Prepared {
        graph_index: *graph_index,
        artifact,
        local,
    })
}

fn target_matches(target: &Value, unit: &Unit) -> bool {
    target["name"] == unit.target.name
        && target["src_path"] == unit.target.source.to_string_lossy().as_ref()
        && target["kind"].as_array().is_some_and(|values| {
            values.iter().map(Value::as_str).eq(unit
                .target
                .kind
                .iter()
                .map(|value| Some(value.as_str())))
        })
}

fn graph_matches(candidate: &Value, artifact: &Value, unit: &Unit) -> bool {
    candidate["pkg_id"] == unit.target.package
        && target_matches(&candidate["target"], unit)
        && candidate["target"]["crate_types"]
            .as_array()
            .is_some_and(|kinds| {
                !kinds.is_empty()
                    && kinds
                        .iter()
                        .all(|kind| matches!(kind.as_str(), Some("lib" | "rlib")))
            })
        && candidate["target"]["crate_types"] == artifact["target"]["crate_types"]
        && candidate["mode"] == "check"
        && candidate["features"] == artifact["features"]
        && candidate["features"] == json!(invocation_features(&unit.invocation))
        && candidate.get("platform").is_some_and(|platform| {
            match flag_value(&unit.invocation, "--target") {
                Some(target) => platform.as_str() == Some(target),
                None => platform.is_null(),
            }
        })
        && [
            "opt_level",
            "debuginfo",
            "debug_assertions",
            "overflow_checks",
        ]
        .iter()
        .all(|key| {
            candidate["profile"]
                .get(key)
                .is_some_and(|value| artifact["profile"].get(key) == Some(value))
        })
}

fn local_identity<'a>(
    unit: &Unit,
    observed: &'a ObservedUnit<'_>,
) -> Result<Option<&'a Value>, String> {
    let mut identity = None;
    let features = invocation_features(&unit.invocation);
    for function in observed.functions {
        if function.build.rustc_commit.status != rstyle_pdg::FactStatus::Known
            || function.build.rustc_commit.value.as_deref() != Some(COMPILER_COMMIT)
        {
            return Err("Unpinned compiler for native metadata identity codec".into());
        }
        let options = &function.build.frontend_options;
        let local = options
            .get(COMPILED)
            .ok_or("Missing captured local compiler identity")?;
        check_identity(local)?;
        if string(local, "crate_name")? != unit.target.name
            || !invocation_matches(options.get("invocation"), &unit.invocation)
            || !function
                .build
                .features
                .iter()
                .map(String::as_str)
                .eq(features.iter().filter_map(Value::as_str))
        {
            return Err("Compiler observation disagrees with exact ledger invocation".into());
        }
        if identity.is_some_and(|previous| previous != local) {
            return Err("Conflicting local compiler identities in captured unit".into());
        }
        identity = Some(local);
    }
    Ok(identity)
}

fn invocation_matches(observed: Option<&Value>, expected: &[String]) -> bool {
    let Some(observed) = observed.and_then(Value::as_array) else {
        return false;
    };
    let Some(prefix) = observed.get(..expected.len()) else {
        return false;
    };
    if !prefix
        .iter()
        .zip(expected)
        .all(|(actual, expected)| actual.as_str() == Some(expected.as_str()))
    {
        return false;
    }
    match &observed[expected.len()..] {
        [] => true,
        [flag, path] => {
            flag.as_str() == Some("--sysroot") && path.as_str().is_some_and(|path| !path.is_empty())
        }
        _ => false,
    }
}

fn check_identity(identity: &Value) -> Result<(), String> {
    if identity["identity"] != IDENTITY {
        return Err("Unsupported compiled crate identity observation".into());
    }
    string(identity, "crate_name")?;
    string(identity, "stable_crate_id")?;
    Ok(())
}

fn visit_calls(
    node: &HirNode,
    visit: &mut impl FnMut(&Value) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(resolution) = &node.res
        && resolution.kind == "resolved_direct_callee"
    {
        let resolution: Value =
            serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
        if resolution["local"] == false {
            visit(&resolution)?;
        }
    }
    for child in &node.children {
        visit_calls(child, visit)?;
    }
    Ok(())
}

struct Output<'a> {
    seals: &'a mut Seals,
    associations: &'a mut Associations,
}

struct CallContext<'a, 'b> {
    request: &'a Request<'b>,
    prepared: &'a [Prepared<'b>],
    routes: &'a Routes<'a>,
}

fn associate_call(
    context: &CallContext<'_, '_>,
    caller: usize,
    resolution: &Value,
    output: &mut Output<'_>,
) -> Result<(), String> {
    let request = context.request;
    let prepared = context.prepared;
    let Some((callee, foreign)) = context.routes.foreign(resolution)? else {
        return Ok(());
    };
    let key = (caller, callee);
    if let Some(previous) = output.associations.get(&key) {
        if previous["compiled_dependency"] != *foreign {
            return Err("Conflicting compiled dependency associations".into());
        }
        return Ok(());
    }

    let alias = context
        .routes
        .alias(prepared[caller].graph_index, prepared[callee].graph_index)?;
    let loaded = PathBuf::from(string(foreign, "rmeta")?);
    let external = extern_path(&request.units[caller].invocation, alias)?;
    if loaded != external {
        return Err("Loaded dependency metadata disagrees with caller --extern".into());
    }
    verify_emission(&request.units[callee], prepared[callee].artifact, &loaded)?;
    let artifact = seal(&loaded, output.seals)?;
    if artifact.crate_hash != string(foreign, "crate_hash")? {
        return Err(
            "Native emitted metadata hash disagrees with loaded compiler dependency".into(),
        );
    }

    let edge = association_value(
        context,
        AssociatedCall {
            caller,
            callee,
            alias,
            foreign,
        },
        artifact.emitted(&loaded),
    )?;

    output.associations.insert(key, edge);
    Ok(())
}

fn check_foreign(foreign: &Value, crate_name: &str) -> Result<(), String> {
    check_identity(foreign)?;
    string(foreign, "crate_hash")?;
    string(foreign, "rmeta")?;

    if foreign["crate_name"] != crate_name
        || !foreign["instance"]["type_arguments"]
            .as_array()
            .is_some_and(Vec::is_empty)
        || !foreign["instance"]["const_arguments"]
            .as_array()
            .is_some_and(Vec::is_empty)
    {
        return Err("Compiled dependency requires exact nongeneric instance identity".into());
    }
    Ok(())
}

fn extern_path(invocation: &[String], alias: &str) -> Result<PathBuf, String> {
    let mut matches = Vec::new();
    for (index, argument) in invocation.iter().enumerate() {
        let value = argument.strip_prefix("--extern=").or_else(|| {
            (argument == "--extern")
                .then(|| invocation.get(index + 1).map(String::as_str))
                .flatten()
        });
        if let Some((name, path)) = value.and_then(|value| value.split_once('='))
            && name == alias
        {
            if path.is_empty() {
                return Err("Empty selected --extern path".into());
            }
            matches.push(PathBuf::from(path));
        }
    }
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        _ => Err("Missing or ambiguous caller --extern alias".into()),
    }
}

fn verify_emission(unit: &Unit, artifact: &Value, loaded: &Path) -> Result<(), String> {
    let filenames = artifact["filenames"]
        .as_array()
        .ok_or("Missing emitted compiler artifact filenames")?;
    if filenames
        .iter()
        .filter(|value| {
            value
                .as_str()
                .is_some_and(|value| Path::new(value) == loaded)
        })
        .count()
        != 1
        || loaded.extension().is_none_or(|value| value != "rmeta")
    {
        return Err("Loaded rmeta is not the selected emitted Cargo artifact".into());
    }

    let output = flag_value(&unit.invocation, "--out-dir")
        .ok_or("Missing callee compiler output directory")?;
    let name =
        flag_value(&unit.invocation, "--crate-name").ok_or("Missing callee compiler crate name")?;
    let suffix = extra_filename(&unit.invocation)?;
    if name != unit.target.name
        || Path::new(output).join(format!("lib{name}{suffix}.rmeta")) != loaded
    {
        return Err("Emitted rmeta disagrees with exact callee ledger output".into());
    }
    Ok(())
}

fn extra_filename(invocation: &[String]) -> Result<&str, String> {
    let matches: Vec<_> = invocation
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            argument.strip_prefix("-Cextra-filename=").or_else(|| {
                (argument == "-C")
                    .then(|| invocation.get(index + 1)?.strip_prefix("extra-filename="))
                    .flatten()
            })
        })
        .collect();
    match matches.as_slice() {
        [suffix] => Ok(suffix),
        _ => Err("Missing or ambiguous callee extra-filename".into()),
    }
}

fn seal(path: &Path, seals: &mut Seals) -> Result<ArtifactSeal, String> {
    if let Some(seal) = seals.get(path) {
        return Ok(seal.clone());
    }

    let bytes = std::fs::read(path)
        .map_err(|error| format!("Missing emitted dependency rmeta: {error}"))?;
    let result = ArtifactSeal {
        bytes: u64::try_from(bytes.len()).map_err(|_| "Oversized dependency rmeta")?,
        checksum: rstyle_pdg::unit_store::transport_checksum(&bytes),
        crate_hash: native_crate_hash(&bytes)?,
    };

    seals.insert(path.to_owned(), result.clone());
    Ok(result)
}

// Pinned rustc_metadata::rmeta fixed header, not a reconstruction of rustc's hash.
fn native_crate_hash(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() < 40 || !bytes.starts_with(METADATA_HEADER) {
        return Err("Unsupported or truncated native metadata header".into());
    }

    let root = u64::from_le_bytes(
        bytes[8..16]
            .try_into()
            .map_err(|_| "Missing metadata root")?,
    );
    let unhashed = u64::from_le_bytes(
        bytes[16..24]
            .try_into()
            .map_err(|_| "Missing unhashed metadata root")?,
    );
    let length = u64::try_from(bytes.len()).map_err(|_| "Oversized native metadata")?;
    if root < 40 || unhashed <= root || unhashed >= length {
        return Err("Native metadata positions are outside the pinned header/body contract".into());
    }

    let native = u128::from_le_bytes(
        bytes[24..40]
            .try_into()
            .map_err(|_| "Missing native metadata hash")?,
    );
    Ok(format!("{native:032x}"))
}

struct AssociatedCall<'a> {
    caller: usize,
    callee: usize,
    alias: &'a str,
    foreign: &'a Value,
}

fn association_value(
    context: &CallContext<'_, '_>,
    call: AssociatedCall<'_>,
    artifact: EmittedMetadata<'_>,
) -> Result<Value, String> {
    serde_json::to_value(Association {
        identity: "rstyle.rust.compiled-dependency-association/1",
        caller: unit_reference(context.request, context.prepared, call.caller),
        callee: unit_reference(context.request, context.prepared, call.callee),
        extern_alias: call.alias,
        artifact,
        local_compiled_crate: context.prepared[call.callee].local,
        compiled_dependency: call.foreign,
    })
    .map_err(|error| error.to_string())
}

#[derive(Serialize)]
struct Association<'a> {
    identity: &'static str,
    caller: UnitReference<'a>,
    callee: UnitReference<'a>,
    extern_alias: &'a str,
    artifact: EmittedMetadata<'a>,
    local_compiled_crate: Option<&'a Value>,
    compiled_dependency: &'a Value,
}

#[derive(Serialize)]
struct EmittedMetadata<'a> {
    path: &'a Path,
    bytes: u64,
    checksum: &'a str,
    native_crate_hash: &'a str,
}

impl ArtifactSeal {
    fn emitted<'a>(&'a self, path: &'a Path) -> EmittedMetadata<'a> {
        EmittedMetadata {
            path,
            bytes: self.bytes,
            checksum: &self.checksum,
            native_crate_hash: &self.crate_hash,
        }
    }
}

#[derive(Serialize)]
struct UnitReference<'a> {
    unit: usize,
    graph_unit: usize,
    namespace: UnitNamespace,
    package_id: &'a str,
}

fn unit_reference<'a>(
    request: &Request<'a>,
    prepared: &[Prepared<'_>],
    index: usize,
) -> UnitReference<'a> {
    UnitReference {
        unit: index,
        graph_unit: prepared[index].graph_index,
        namespace: request.observations[index].namespace,
        package_id: &request.units[index].target.package,
    }
}

#[cfg(test)]
mod tests;
