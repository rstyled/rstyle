//! Alias-conservative referent dependencies and explicit per-exit memory state.
//! Root locations describe possible overlap, not additional indexed reads.
use super::*;
use crate::data_types::TypeToken;
use crate::field_locations::scalar_formals::{FormalObservation, ScalarFormalAccesses};
use crate::producer::{self, memory};
use crate::structural_json::object;
use crate::{Definition, EdgeFamily, MemoryLocation, PdgEdge};
use serde_json::Value;

pub(super) const KEY: &str = "rstyle.data:mutable-memory-flow";

type Roots = BTreeMap<memory::Location, Root>;
type Locations = BTreeMap<String, MemoryLocation>;

struct Root {
    location_id: String,
    node_id: String,
    definition_id: String,
}

struct Inventory<'a> {
    roots: BTreeMap<memory::Location, Root>,
    nodes: BTreeMap<&'a str, &'a PdgNode>,
    definitions: BTreeMap<&'a str, Vec<&'a Definition>>,
}

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
    graph: &producer::Graph,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let Some(analysis) = memory::from_graph(graph, context.facts)? else {
        return Ok(BTreeMap::new());
    };

    if pair.pdg.extensions.contains_key(KEY) || pair.cfg.extensions.contains_key(KEY) {
        return Err("Mutable memory flow was already installed".into());
    }

    let fields = observations(context.facts)?;

    let formals = fields
        .scalar_formal_accesses
        .as_ref()
        .ok_or("Mutable memory flow lacks verified formal observations")?;

    let (roots, locations) = roots(pair.as_view(), context, formals)?;

    let inventory = Inventory::new(pair.as_view(), roots)?;

    let edges = analysis
        .dependencies
        .iter()
        .enumerate()
        .map(|(ordinal, dependence)| inventory.edge(ordinal, dependence))
        .collect::<Result<Vec<_>, _>>()?;

    let record = inventory.record(pair.as_view(), &analysis, formals)?;
    check_edge_ids(pair.as_view(), &edges)?;
    install_loop_capability(pair, &analysis);
    publish(pair, edges, record);

    Ok(locations)
}

fn check_edge_ids(pair: PairView<'_>, edges: &[PdgEdge]) -> Result<(), String> {
    let mut ids: BTreeSet<_> = pair.pdg.edges.iter().map(|edge| edge.id.as_str()).collect();

    for edge in edges {
        if !ids.insert(&edge.id) {
            return Err("Mutable memory dependence ID collides".into());
        }
    }

    Ok(())
}

fn publish(pair: &mut Pair, edges: Vec<PdgEdge>, record: Value) {
    pair.pdg.edges.extend(edges);
    pair.pdg.extensions.insert(KEY.into(), record.clone());
    pair.cfg.extensions.insert(KEY.into(), record);
}

struct RootInventory {
    roots: Roots,
    locations: Locations,
}

impl RootInventory {
    fn insert(
        &mut self,
        parameter: u32,
        root: Root,
        location: MemoryLocation,
    ) -> Result<(), String> {
        let key = memory::Location(parameter);
        if self.roots.contains_key(&key) || self.locations.contains_key(&location.id) {
            return Err("Duplicated mutable memory root".into());
        }

        self.roots.insert(key, root);
        self.locations.insert(location.id.clone(), location);

        Ok(())
    }
}

fn roots(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    formals: &ScalarFormalAccesses,
) -> Result<(Roots, Locations), String> {
    let mut result = RootInventory {
        roots: BTreeMap::new(),
        locations: BTreeMap::new(),
    };

    for formal in &formals.formals {
        let (path, ty) = match &formal.observation {
            FormalObservation::Mutable { .. } if formal.mutable_usize() => (
                Projection::MutableReferent,
                TypeToken::Scalar("usize".into()),
            ),
            FormalObservation::Shared { .. }
                if context
                    .facts
                    .signature
                    .params
                    .get(formal.parameter as usize)
                    .is_some_and(|parameter| parameter.ty.observed == "&[u8]") =>
            {
                (
                    Projection::SharedReferent,
                    TypeToken::Slice(Box::new(TypeToken::Scalar("u8".into()))),
                )
            }
            FormalObservation::NonReference => continue,
            _ => return Err("Mutable memory root is outside its bounded referent contract".into()),
        };
        let (id, location) = context.formal_location(formal.parameter, &[path], ty)?;
        let root = entry_root(pair, formal.parameter, id.clone())?;
        result.insert(formal.parameter, root, location)?;
    }

    Ok((result.roots, result.locations))
}

fn entry_root(pair: PairView<'_>, parameter: u32, location_id: String) -> Result<Root, String> {
    let matches: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == NodeCategory::FormalInput
                && node.attributes.position == Some(u64::from(parameter))
        })
        .collect();

    let [node] = matches.as_slice() else {
        return Err("Memory entry root lacks one existing formal input".into());
    };

    let definitions: Vec<_> = pair
        .pdg
        .definitions
        .iter()
        .filter(|definition| definition.node_id == node.id)
        .collect();

    let [definition] = definitions.as_slice() else {
        return Err("Memory entry root lacks one existing formal definition".into());
    };

    if node.attributes.defines != [definition.id.clone()]
        || definition.location_id.is_some()
        || definition
            .symbol_id
            .as_ref()
            .is_none_or(|symbol| node.attributes.symbol_ids != [symbol.clone()])
    {
        return Err("Memory entry root changes its lexical formal descriptor".into());
    }

    Ok(Root {
        location_id,
        node_id: node.id.clone(),
        definition_id: definition.id.clone(),
    })
}

fn checked_nodes<'a>(pair: PairView<'a>) -> Result<BTreeMap<&'a str, &'a PdgNode>, String> {
    let mut nodes = BTreeMap::new();

    for node in &pair.pdg.nodes {
        if nodes.insert(node.id.as_str(), node).is_some() {
            return Err("Duplicated memory graph node".into());
        }
    }

    Ok(nodes)
}

fn checked_definitions<'a>(
    pair: PairView<'a>,
    nodes: &BTreeMap<&str, &PdgNode>,
) -> Result<BTreeMap<&'a str, Vec<&'a Definition>>, String> {
    let mut definitions: BTreeMap<&str, Vec<&Definition>> = BTreeMap::new();

    let mut ids = BTreeSet::new();

    for definition in &pair.pdg.definitions {
        if !ids.insert(&definition.id) || !nodes.contains_key(definition.node_id.as_str()) {
            return Err("Memory definition is duplicated or has no actual node".into());
        }
        definitions
            .entry(&definition.node_id)
            .or_default()
            .push(definition);
    }

    Ok(definitions)
}

impl<'a> Inventory<'a> {
    fn new(pair: PairView<'a>, roots: BTreeMap<memory::Location, Root>) -> Result<Self, String> {
        let nodes = checked_nodes(pair)?;

        let definitions = checked_definitions(pair, &nodes)?;

        Ok(Self {
            roots,
            nodes,
            definitions,
        })
    }

    fn root(&self, location: memory::Location) -> Result<&Root, String> {
        self.roots
            .get(&location)
            .ok_or_else(|| "Memory analysis names an unverified formal root".into())
    }

    fn source(
        &self,
        source: &memory::Definition,
        writes: bool,
    ) -> Result<(String, Option<String>), String> {
        match source {
            memory::Definition::Entry(location) => {
                let root = self.root(*location)?;
                Ok((root.node_id.clone(), Some(root.definition_id.clone())))
            }
            memory::Definition::Operation(id) => self.operation_source(id, writes),
        }
    }

    fn operation_source(&self, id: &str, writes: bool) -> Result<(String, Option<String>), String> {
        let node = self
            .nodes
            .get(id)
            .ok_or("Memory source operation is absent")?;

        if !writes {
            if node.category != NodeCategory::Read {
                return Err("Memory WAR source is not an actual load".into());
            }
            return Ok((id.to_owned(), None));
        }

        let definitions = self
            .definitions
            .get(id)
            .ok_or("Memory store lacks a definition")?;

        let [definition] = definitions.as_slice() else {
            return Err("Memory store definition is ambiguous".into());
        };

        let location = definition
            .location_id
            .as_ref()
            .ok_or("Memory store definition is lexical")?;

        if node.attributes.construct.as_deref() != Some(crate::mutable_scalars::STORE)
            || node.attributes.defines != [definition.id.to_owned()]
            || !node
                .attributes
                .writes
                .iter()
                .any(|write| write.location_id.as_ref() == Some(location))
            || !self
                .roots
                .values()
                .any(|root| &root.location_id == location)
        {
            return Err("Memory reaching store lacks its actual referent definition".into());
        }

        Ok((id.to_owned(), Some(definition.id.to_owned())))
    }

    fn edge(
        &self,
        ordinal: usize,
        dependence: &memory::MemoryDependence,
    ) -> Result<PdgEdge, String> {
        let raw = dependence.kind == memory::DependenceKind::Raw;

        let (source, definition_id) = self.source(
            &dependence.source,
            dependence.kind != memory::DependenceKind::War,
        )?;

        if !self.nodes.contains_key(dependence.target.as_str()) {
            return Err("Memory dependence target is absent".into());
        }

        let description = "Alias-conservative referent-root memory dependence; precise indexed reads remain separate";

        let (evidence, provenance) = self.edge_proof(description);

        Ok(PdgEdge {
            id: format!("edge:mutable-memory:{ordinal}"),
            source,
            target: dependence.target.clone(),
            kind: if raw {
                EdgeFamily::Data
            } else {
                EdgeFamily::Execution
            },
            subkind: match dependence.kind {
                memory::DependenceKind::Raw => "reaching_definition",
                memory::DependenceKind::War => "memory_war",
                memory::DependenceKind::Waw => "memory_waw",
            }
            .into(),
            evidence,
            provenance,
            operand_position: None,
            symbol_id: None,
            definition_id,
            location_id: Some(self.root(dependence.location)?.location_id.clone()),
            outcome: None,
            loop_carried: Some(dependence.loop_carried),
            annotations: BTreeMap::new(),
        })
    }

    fn edge_proof(&self, description: &str) -> (crate::Capability, crate::Provenance) {
        let description = description.to_owned();
        (
            crate::Capability {
                status: crate::CapabilityStatus::Supported,
                description: description.clone(),
            },
            crate::Provenance {
                origin: crate::ProvenanceOrigin::Analysis,
                description,
                origin_node_ids: Vec::new(),
                origin_spans: Vec::new(),
            },
        )
    }

    fn record(
        &self,
        pair: PairView<'_>,
        analysis: &memory::MemoryAnalysis,
        formals: &ScalarFormalAccesses,
    ) -> Result<Value, String> {
        let roots: Vec<_> = self
            .roots
            .iter()
            .map(|(position, root)| {
                json!({
                    "parameter": position.0, "location_id": root.location_id,
                    "node_id": root.node_id, "definition_id": root.definition_id, "entry": true,
                })
            })
            .collect();

        let aliases = formals
            .possibly_aliasing_formals
            .iter()
            .map(|[left, right]| {
                Ok(json!({
                    "left": {"location_id": self.root(memory::Location(*left))?.location_id},
                    "right": {"location_id": self.root(memory::Location(*right))?.location_id},
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;

        let exits = analysis
            .exits
            .iter()
            .map(|(id, state)| self.exit(pair, id, state))
            .collect::<Result<Vec<_>, _>>()?;

        if exits.len() != pair.cfg.exit_node_ids.len() {
            return Err("Mutable memory state omits a CFG exit".into());
        }

        Ok(object([
            ("identity", "rstyle.rust.mutable-memory-flow/1".into()),
            ("interpretation", "alias-conservative referent roots; separate precise indexed reads; no cleanup model".into()),
            ("roots", roots.into()), ("aliases", aliases.into()), ("exits", exits.into()),
        ]))
    }

    fn exit(
        &self,
        pair: PairView<'_>,
        id: &str,
        state: &memory::ExitState,
    ) -> Result<Value, String> {
        let exits: Vec<_> = pair.cfg.nodes.iter().filter(|node| node.id == id).collect();

        let [exit] = exits.as_slice() else {
            return Err("Memory exit lacks one actual CFG node".into());
        };

        let expected = match state.kind {
            memory::ExitKind::Normal => crate::cfg::CfgExitKind::Normal,
            memory::ExitKind::Failure => crate::cfg::CfgExitKind::Exceptional,
        };

        if exit.kind != crate::cfg::CfgNodeKind::Exit
            || exit.exit_kind != Some(expected)
            || !pair.cfg.exit_node_ids.iter().any(|exit| exit == id)
            || !exit.operations.is_empty()
            || state.reachable && state.reaching.len() != self.roots.len()
            || !state.reachable && !state.reaching.is_empty()
        {
            return Err(
                "Memory exit kind, reachability or complete root state differs from CFG".into(),
            );
        }

        let reaching = state.reaching.iter().map(|(location, alternatives)| {
            if alternatives.is_empty() { return Err("Reachable memory root lacks reaching alternatives".into()); }
            let definitions = alternatives.iter().map(|alternative| self.alternative(alternative))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"location_id": self.root(*location)?.location_id, "definitions": definitions}))
        }).collect::<Result<Vec<_>, String>>()?;

        Ok(object([
            ("cfg_node_id", id.into()),
            (
                "exit_kind",
                serde_json::to_value(exit.exit_kind).map_err(|error| error.to_string())?,
            ),
            ("reachable", state.reachable.into()),
            ("reaching", reaching.into()),
        ]))
    }

    fn alternative(&self, alternative: &memory::ReachingDefinition) -> Result<Value, String> {
        let (node_id, definition_id) = self.source(&alternative.definition, true)?;

        let mut record = object([
            ("node_id", node_id.into()),
            ("definition_id", definition_id.into()),
            (
                "entry",
                matches!(alternative.definition, memory::Definition::Entry(_)).into(),
            ),
            ("loop_carried", alternative.loop_carried.into()),
        ]);

        if let memory::Definition::Entry(location) = alternative.definition {
            record["parameter"] = json!(location.0);
        }

        Ok(record)
    }
}

/// Recompute memory flow from serialized execution order and checked source
/// accesses. This path never invokes producer lowering or document emission.
pub(super) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    if !crate::mutable_scalars::enabled(context.facts) {
        if pair.pdg.extensions.contains_key(KEY) || pair.cfg.extensions.contains_key(KEY) {
            return Err("Historical family carries unexpected mutable memory flow".into());
        }
        return Ok(BTreeMap::new());
    }

    let reconstructed = Reconstruction::new(pair, context.facts)?;

    let analysis = memory::from_graph(&reconstructed.graph, context.facts)?
        .ok_or("Mutable memory analysis was not activated")?;

    let analysis = reconstructed.restore(analysis)?;

    let fields = observations(context.facts)?;

    let formals = fields
        .scalar_formal_accesses
        .as_ref()
        .ok_or("Mutable memory verification lacks formal inventory")?;

    let (roots, locations) = roots(pair, context, formals)?;

    let inventory = Inventory::new(pair, roots)?;
    verify_edges(pair, &inventory, &analysis)?;
    verify_loop_capability(pair, &analysis)?;

    let record = inventory.record(pair, &analysis, formals)?;

    if !record_matches(pair.pdg.extensions.get(KEY), &record)
        || !record_matches(pair.cfg.extensions.get(KEY), &record)
    {
        return Err(
            "Mutable memory root/alias/exit state differs from independent CFG analysis".into(),
        );
    }

    Ok(locations)
}

type EdgeKey = (
    EdgeFamily,
    String,
    String,
    String,
    Option<String>,
    Option<bool>,
);

fn edge_key(edge: &PdgEdge) -> EdgeKey {
    (
        edge.kind,
        edge.subkind.clone(),
        edge.source.clone(),
        edge.target.clone(),
        edge.location_id.clone(),
        edge.loop_carried,
    )
}

fn verify_edges(
    pair: PairView<'_>,
    inventory: &Inventory<'_>,
    analysis: &memory::MemoryAnalysis,
) -> Result<(), String> {
    let root_ids: BTreeSet<_> = inventory
        .roots
        .values()
        .map(|root| root.location_id.as_str())
        .collect();

    let mut actual = BTreeMap::new();

    for edge in &pair.pdg.edges {
        let memory_edge = edge.subkind == "memory_war"
            || edge.subkind == "memory_waw"
            || edge
                .location_id
                .as_deref()
                .is_some_and(|id| root_ids.contains(id));
        if memory_edge && actual.insert(edge_key(edge), edge).is_some() {
            return Err("Duplicated referent-root memory dependence".into());
        }
    }

    for (ordinal, dependence) in analysis.dependencies.iter().enumerate() {
        let mut expected = inventory.edge(ordinal, dependence)?;
        let edge = actual
            .remove(&edge_key(&expected))
            .ok_or("Missing independently required memory dependence")?;
        expected.id = edge.id.clone();
        if *edge != expected {
            return Err(
                "Memory dependence changes source, definition, root or loop evidence".into(),
            );
        }
    }

    if !actual.is_empty() {
        return Err("Unexpected referent-root memory dependence".into());
    }

    Ok(())
}

struct Reconstruction {
    graph: producer::Graph,
    operations: BTreeMap<String, String>,
    boundaries: BTreeMap<String, String>,
}

impl Reconstruction {
    fn new(
        pair: PairView<'_>,
        facts: &dyn crate::unit_store::EvidenceFacts,
    ) -> Result<Self, String> {
        let context = ReconstructionContext::new(pair, facts)?;

        let mut result = Self {
            graph: producer::Graph {
                nodes: Vec::new(),
                transfers: Vec::new(),
                value_sites: Vec::new(),
            },
            operations: BTreeMap::new(),
            boundaries: BTreeMap::new(),
        };

        let mut cfg_ids = BTreeMap::new();
        result.add_operations(pair, &context, &mut cfg_ids)?;

        if result.operations.values().collect::<BTreeSet<_>>().len() != result.operations.len() {
            return Err("Mutable operation map repeats a PDG operation".into());
        }
        result.boundaries = boundary_ids(pair, &mut cfg_ids)?;

        if cfg_ids.len() != context.cfg_nodes.len() {
            return Err("Mutable reconstruction omits a CFG operation or boundary".into());
        }
        result.graph.transfers = pair
            .cfg
            .edges
            .iter()
            .map(|edge| reconstruct_transfer(edge, &cfg_ids))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    fn add_operations(
        &mut self,
        pair: PairView<'_>,
        context: &ReconstructionContext<'_>,
        cfg_ids: &mut BTreeMap<String, String>,
    ) -> Result<(), String> {
        for (ordinal, entry) in crate::read_operation_map(&pair.pdg.extensions)?
            .iter()
            .enumerate()
        {
            let operation = context.operation(entry)?;
            let id = format!("memory-operation:{ordinal}");
            if self
                .operations
                .insert(id.clone(), operation.id.clone())
                .is_some()
                || cfg_ids
                    .insert(entry.cfg_node_id.clone(), id.clone())
                    .is_some()
            {
                return Err("Mutable operation correspondence is ambiguous".into());
            }
            self.graph
                .nodes
                .push(producer::Operation { id, ..operation });
        }

        Ok(())
    }

    fn restore(&self, analysis: memory::MemoryAnalysis) -> Result<memory::MemoryAnalysis, String> {
        let dependencies = analysis
            .dependencies
            .into_iter()
            .map(|dependence| self.restore_dependence(dependence))
            .collect::<Result<Vec<_>, _>>()?;

        let exits = analysis
            .exits
            .into_iter()
            .map(|(id, state)| self.restore_exit(&id, state))
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        Ok(memory::MemoryAnalysis {
            dependencies,
            exits,
        })
    }

    fn restore_dependence(
        &self,
        dependence: memory::MemoryDependence,
    ) -> Result<memory::MemoryDependence, String> {
        Ok(memory::MemoryDependence {
            source: self.restored_definition(dependence.source)?,
            target: self
                .operations
                .get(&dependence.target)
                .ok_or("Analyzed memory target has no actual PDG node")?
                .clone(),
            ..dependence
        })
    }

    fn restore_exit(
        &self,
        id: &str,
        state: memory::ExitState,
    ) -> Result<(String, memory::ExitState), String> {
        let actual = self
            .boundaries
            .get(id)
            .ok_or("Analyzed memory exit has no actual CFG node")?;

        let reaching = state
            .reaching
            .into_iter()
            .map(|(location, alternatives)| {
                let definitions = alternatives
                    .into_iter()
                    .map(|alternative| {
                        Ok(memory::ReachingDefinition {
                            definition: self.restored_definition(alternative.definition)?,
                            loop_carried: alternative.loop_carried,
                        })
                    })
                    .collect::<Result<BTreeSet<_>, String>>()?;
                Ok((location, definitions))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;

        Ok((actual.clone(), memory::ExitState { reaching, ..state }))
    }

    fn restored_definition(
        &self,
        definition: memory::Definition,
    ) -> Result<memory::Definition, String> {
        match definition {
            memory::Definition::Operation(id) => Ok(memory::Definition::Operation(
                self.operations
                    .get(&id)
                    .ok_or("Memory definition has no actual PDG operation")?
                    .clone(),
            )),
            entry => Ok(entry),
        }
    }
}

fn operand_index(pair: PairView<'_>) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut grouped: BTreeMap<String, BTreeMap<u64, String>> = BTreeMap::new();

    let nodes: BTreeSet<_> = pair.pdg.nodes.iter().map(|node| node.id.as_str()).collect();

    for edge in &pair.pdg.edges {
        if edge.kind == EdgeFamily::Data && edge.subkind == "value_operand" {
            let position = edge
                .operand_position
                .ok_or("Saved memory operand lacks its ordinal")?;
            if !nodes.contains(edge.source.as_str())
                || !nodes.contains(edge.target.as_str())
                || grouped
                    .entry(edge.target.clone())
                    .or_default()
                    .insert(position, edge.source.clone())
                    .is_some()
            {
                return Err("Saved memory operand is missing or duplicated".into());
            }
        }
    }
    grouped
        .into_iter()
        .map(|(id, operands)| {
            if operands.keys().copied().ne(0..operands.len() as u64) {
                return Err("Saved memory operands omit an ordered position".into());
            }
            Ok((id, operands.into_values().collect()))
        })
        .collect()
}

struct ReconstructionContext<'a> {
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    source_id: &'a str,
    pdg_nodes: BTreeMap<&'a str, &'a PdgNode>,
    cfg_nodes: BTreeMap<&'a str, &'a crate::cfg::CfgNode>,
    operands: BTreeMap<String, Vec<String>>,
}

impl<'a> ReconstructionContext<'a> {
    fn new(
        pair: PairView<'a>,
        facts: &'a dyn crate::unit_store::EvidenceFacts,
    ) -> Result<Self, String> {
        if pair.cfg.nodes.len() > 4096
            || pair.cfg.edges.len() > 16384
            || pair.pdg.nodes.len() > 4096
        {
            return Err("Serialized mutable CFG exceeds its budget".into());
        }
        crate::mutable_scalars::verify(facts)?;

        let pdg_nodes: BTreeMap<_, _> = pair
            .pdg
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();

        let cfg_nodes: BTreeMap<_, _> = pair
            .cfg
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();

        if pdg_nodes.len() != pair.pdg.nodes.len() || cfg_nodes.len() != pair.cfg.nodes.len() {
            return Err("Mutable reconstruction has duplicated graph identities".into());
        }

        Ok(Self {
            facts,
            source_id: checked_source_id(pair, facts)?,
            pdg_nodes,
            cfg_nodes,
            operands: operand_index(pair)?,
        })
    }

    fn operation(
        &self,
        entry: &crate::cfg::OperationMapEntry,
    ) -> Result<producer::Operation, String> {
        let node = self
            .pdg_nodes
            .get(entry.pdg_node_id.as_str())
            .ok_or("Memory operation map lacks PDG node")?;

        let cfg = self
            .cfg_nodes
            .get(entry.cfg_node_id.as_str())
            .ok_or("Memory operation map lacks CFG node")?;

        let source = checked_operation_source(node, cfg, entry)?;

        let span = original_span(&source.source_span, self.source_id)?;

        if self
            .facts
            .source
            .bytes
            .get(span.start as usize..span.end as usize)
            != Some(source.text.as_str())
        {
            return Err("Memory CFG operation does not retain exact original bytes".into());
        }

        let mut result = reconstruct_operation(node, span, self.source_id)?;
        result.operands = self
            .operands
            .get(node.id.as_str())
            .cloned()
            .unwrap_or_default();

        Ok(result)
    }
}

fn checked_source_id<'a>(
    pair: PairView<'a>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<&'a str, String> {
    let [pdg] = pair.pdg.sources.as_slice() else {
        return Err("Memory flow requires one original PDG source".into());
    };

    let [cfg] = pair.cfg.sources.as_slice() else {
        return Err("Memory flow requires one original CFG source".into());
    };

    let identity = pdg
        .content_identity
        .known_value()
        .ok_or("Memory source identity is unknown")?;

    if pdg.id != cfg.id
        || pdg.path != cfg.path
        || pdg.content_identity != cfg.content_identity
        || !facts.source.bytes.matches_identity(identity)
        || cfg
            .content
            .as_ref()
            .is_none_or(|content| content.as_str() != facts.source.bytes.as_str())
        || pair.pdg.function.source_span.source_id != pdg.id
        || pair.cfg.function.source_span.source_id != cfg.id
    {
        return Err("Memory graph sources disagree with exact original source ownership".into());
    }

    Ok(&pdg.id)
}

fn checked_operation_source<'a>(
    node: &PdgNode,
    cfg: &'a crate::cfg::CfgNode,
    entry: &crate::cfg::OperationMapEntry,
) -> Result<&'a crate::cfg::CfgOperation, String> {
    let [operation] = cfg.operations.as_slice() else {
        return Err("Memory CFG requires one ordered operation per node".into());
    };

    if operation.id != entry.cfg_operation_id
        || node.source_span.as_ref() != Some(&operation.source_span)
        || node.original_kind.known_value() != Some(operation.original_kind.as_str())
        || cfg.exit_kind.is_some()
        || matches!(
            cfg.kind,
            crate::cfg::CfgNodeKind::Entry | crate::cfg::CfgNodeKind::Exit
        )
    {
        return Err("Memory CFG/PDG operation source correspondence differs".into());
    }

    Ok(operation)
}

fn reconstruct_operation(
    node: &PdgNode,
    span: crate::ByteSpan,
    source_id: &str,
) -> Result<producer::Operation, String> {
    let provenance = node
        .attributes
        .extensions
        .get("compat.rust.hir-r0:source-operation")
        .ok_or("Memory operation lacks its source provenance")?;

    let construct: crate::Span = serde_json::from_value(
        provenance
            .get("construct_span")
            .ok_or("Memory operation lacks its construct span")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;

    let ty: crate::StringFact = serde_json::from_value(
        node.annotations
            .get("compat.rust.hir-r0:type")
            .ok_or("Memory operation lacks its concrete result type")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;

    let mut result = operation_attributes(node, span)?;
    result.construct_span = original_span(&construct, source_id)?;
    result.role = provenance
        .get("role")
        .and_then(Value::as_str)
        .ok_or("Memory operation lacks its source role")?
        .into();
    result.ty = ty
        .known_value()
        .ok_or("Memory operation type is unknown")?
        .into();

    Ok(result)
}

fn operation_attributes(
    node: &PdgNode,
    span: crate::ByteSpan,
) -> Result<producer::Operation, String> {
    Ok(producer::Operation {
        id: node.id.clone(),
        category: node.category,
        span,
        construct_span: span,
        original_kind: node
            .original_kind
            .known_value()
            .ok_or("Memory operation kind is unknown")?
            .into(),
        role: String::new(),
        ty: String::new(),
        symbol: node.attributes.symbol_ids.first().cloned(),
        definition: None,
        operator: node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            .map(str::to_owned),
        literal: node
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            .map(str::to_owned),
        construct: node.attributes.construct.clone(),
        write_form: node.attributes.write_form.clone(),
        return_form: None,
        position: node.attributes.position,
        operands: Vec::new(),
    })
}

fn original_span(span: &crate::Span, source_id: &str) -> Result<crate::ByteSpan, String> {
    if span.source_id != source_id || span.start > span.end {
        return Err("Memory operation span has the wrong source or order".into());
    }

    Ok(crate::ByteSpan {
        start: u32::try_from(span.start).map_err(|_| "Memory source start exceeds its budget")?,
        end: u32::try_from(span.end).map_err(|_| "Memory source end exceeds its budget")?,
    })
}

fn boundary_ids(
    pair: PairView<'_>,
    cfg_ids: &mut BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    install_entry_id(pair, cfg_ids)?;

    let mut boundaries = BTreeMap::new();

    let normal = pair
        .cfg
        .nodes
        .iter()
        .filter(|node| node.exit_kind == Some(crate::cfg::CfgExitKind::Normal))
        .count();

    for exit in pair
        .cfg
        .nodes
        .iter()
        .filter(|node| node.kind == crate::cfg::CfgNodeKind::Exit)
    {
        if !exit.operations.is_empty() {
            return Err("Mutable exit includes hidden operations".into());
        }
        let canonical = match exit.exit_kind {
            Some(crate::cfg::CfgExitKind::Normal) => "X".into(),
            Some(crate::cfg::CfgExitKind::Exceptional) => failure_id(pair, exit, cfg_ids)?,
            _ => return Err("Mutable CFG has an unsupported exit kind".into()),
        };
        if boundaries
            .insert(canonical.clone(), exit.id.clone())
            .is_some()
            || cfg_ids.insert(exit.id.clone(), canonical).is_some()
        {
            return Err("Mutable CFG boundary correspondence is ambiguous".into());
        }
    }

    if normal != 1 {
        return Err("Mutable CFG requires exactly one normal boundary".into());
    }

    Ok(boundaries)
}

fn install_entry_id(
    pair: PairView<'_>,
    cfg_ids: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let entry = pair
        .cfg
        .nodes
        .iter()
        .find(|node| node.id == pair.cfg.entry_node_id)
        .ok_or("Mutable CFG entry is absent")?;

    if entry.kind != crate::cfg::CfgNodeKind::Entry
        || !entry.operations.is_empty()
        || entry.exit_kind.is_some()
        || cfg_ids.insert(entry.id.clone(), "E".into()).is_some()
    {
        return Err("Mutable CFG entry contradicts operation correspondence".into());
    }

    Ok(())
}

fn failure_id(
    pair: PairView<'_>,
    exit: &crate::cfg::CfgNode,
    cfg_ids: &BTreeMap<String, String>,
) -> Result<String, String> {
    let incoming: Vec<_> = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| edge.target == exit.id)
        .collect();

    let [edge] = incoming.as_slice() else {
        return Err("Memory failure exit has ambiguous incoming execution".into());
    };

    if edge.kind != crate::cfg::CfgEdgeKind::Exception {
        return Err("Memory failure has a nonexception predecessor".into());
    }

    Ok(format!(
        "P:{}",
        cfg_ids
            .get(&edge.source)
            .ok_or("Memory failure source has no operation")?
    ))
}

fn reconstruct_transfer(
    edge: &crate::cfg::CfgEdge,
    cfg_ids: &BTreeMap<String, String>,
) -> Result<producer::Transfer, String> {
    Ok(producer::Transfer {
        source: cfg_ids
            .get(&edge.source)
            .ok_or("Unknown mutable CFG transfer source")?
            .clone(),
        target: cfg_ids
            .get(&edge.target)
            .ok_or("Unknown mutable CFG transfer target")?
            .clone(),
        kind: transfer_kind(edge.kind)?.into(),
        loop_role: edge.loop_role.map(|role| match role {
            crate::cfg::LoopRole::BackEdge => "back_edge".into(),
            crate::cfg::LoopRole::LoopExit => "loop_exit".into(),
        }),
    })
}

fn loop_capability() -> crate::Capability {
    crate::Capability {
        status: crate::CapabilityStatus::Supported,
        description: "Verified alias-conservative loop-carried referent RAW/WAR/WAW dependencies"
            .into(),
    }
}

fn install_loop_capability(pair: &mut Pair, analysis: &memory::MemoryAnalysis) {
    if analysis
        .dependencies
        .iter()
        .any(|dependence| dependence.loop_carried)
    {
        pair.pdg
            .extraction_evidence
            .capabilities
            .insert("loop_carried_dependencies".into(), loop_capability());
    }
}

fn verify_loop_capability(
    pair: PairView<'_>,
    analysis: &memory::MemoryAnalysis,
) -> Result<(), String> {
    if analysis
        .dependencies
        .iter()
        .any(|dependence| dependence.loop_carried)
        && pair
            .pdg
            .extraction_evidence
            .capabilities
            .get("loop_carried_dependencies")
            != Some(&loop_capability())
    {
        return Err(
            "Loop-carried memory dependencies lack their supported extraction capability".into(),
        );
    }

    Ok(())
}

fn transfer_kind(kind: crate::cfg::CfgEdgeKind) -> Result<&'static str, String> {
    use crate::cfg::CfgEdgeKind;
    match kind {
        CfgEdgeKind::Normal => Ok("normal"),
        CfgEdgeKind::True => Ok("true"),
        CfgEdgeKind::False => Ok("false"),
        CfgEdgeKind::Return => Ok("return"),
        CfgEdgeKind::Break => Ok("break"),
        CfgEdgeKind::Continue => Ok("continue"),
        CfgEdgeKind::Exception => Ok("exception"),
        _ => Err("Mutable memory CFG transfer is outside its bounded contract".into()),
    }
}

// Proof rosters are sets with multiplicity checked by exact comparison. Local
// identity renaming must not turn their serialized array order into semantics.
fn record_matches(actual: Option<&Value>, expected: &Value) -> bool {
    actual.is_some_and(|actual| ordered_record(actual.clone()) == ordered_record(expected.clone()))
}

fn ordered_record(mut record: Value) -> Value {
    order_inventory(&mut record, 0);
    record
}

fn order_inventory(record: &mut Value, depth: usize) {
    // Only these three proof rosters are unordered; all other arrays retain
    // their original order and every duplicate remains in the comparison.
    let levels = [
        ("exits", Some("cfg_node_id")),
        ("reaching", Some("location_id")),
        ("definitions", None),
    ];
    let Some((field, key)) = levels.get(depth) else {
        return;
    };
    let Some(items) = record.get_mut(*field).and_then(Value::as_array_mut) else {
        return;
    };

    order_items(items, depth, *key);
}

fn order_items(items: &mut [Value], depth: usize, key: Option<&str>) {
    for item in items.iter_mut() {
        order_inventory(item, depth + 1);
    }

    items.sort_by(|left, right| match key {
        Some(key) => record_id(left, key).cmp(&record_id(right, key)),
        None => definition_order(left).cmp(&definition_order(right)),
    });
}

fn record_id<'a>(record: &'a Value, field: &str) -> Option<&'a str> {
    record.get(field).and_then(Value::as_str)
}

type DefinitionOrder<'a> = (
    Option<&'a str>,
    Option<&'a str>,
    Option<bool>,
    Option<bool>,
    Option<u64>,
);

fn definition_order(record: &Value) -> DefinitionOrder<'_> {
    (
        record_id(record, "node_id"),
        record_id(record, "definition_id"),
        record.get("entry").and_then(Value::as_bool),
        record.get("loop_carried").and_then(Value::as_bool),
        record.get("parameter").and_then(Value::as_u64),
    )
}
