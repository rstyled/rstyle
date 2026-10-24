//! Source-checked mutable referents never replace lexical pointer descriptors.
use super::*;
use crate::field_locations::scalar_formals::{AccessKind, ScalarFormalAccesses};
use crate::{Access, ByteSpan, Definition, HirNode, MemoryLocation, data_types::TypeToken};

struct Site {
    parameter: u32,
    span: ByteSpan,
    kind: AccessKind,
    operator: Option<String>,
    pointer: Option<ByteSpan>,
}

type SiteKey = (u64, u64, bool);

pub(super) fn is_access(node: &PdgNode) -> bool {
    matches!(
        node.attributes.construct.as_deref(),
        Some(crate::mutable_scalars::LOAD | crate::mutable_scalars::STORE)
    )
}

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let prepared = prepare(pair.as_view(), context)?;
    install_prepared(pair, prepared)
}

struct Prepared {
    position: usize,
    site: Site,
    id: String,
    location: MemoryLocation,
}

fn prepare(pair: PairView<'_>, context: &mut LocationContext<'_>) -> Result<Vec<Prepared>, String> {
    let sites = sites(context.facts)?;
    let index = node_index(pair, sites.len())?;
    sites
        .into_iter()
        .map(|site| {
            let position = *index
                .get(&site.key())
                .ok_or("Missing mutable source access")?;
            check_site(&pair.pdg.nodes[position], &site)?;
            check_pointer(pair, &pair.pdg.nodes[position], &site, context.facts)?;
            let (id, location) = scalar_location(context, site.parameter)?;

            Ok(Prepared {
                position,
                site,
                id,
                location,
            })
        })
        .collect()
}

struct Installation {
    definitions: DefinitionInventory,
    locations: BTreeMap<String, MemoryLocation>,
}

fn install_prepared(
    pair: &mut Pair,
    prepared: Vec<Prepared>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let mut installation = Installation {
        definitions: DefinitionInventory::new(&pair.pdg.definitions)?,
        locations: BTreeMap::new(),
    };
    for prepared in prepared {
        install_one(pair, &mut installation, prepared)?;
    }

    Ok(installation.locations)
}

fn install_one(
    pair: &mut Pair,
    installation: &mut Installation,
    prepared: Prepared,
) -> Result<(), String> {
    let node = &mut pair.pdg.nodes[prepared.position];
    if installation.definitions.nodes.contains(&node.id) {
        return Err("Mutable access already owns a definition".into());
    }
    if let Some(definition) = install_effect(node, &prepared.site, &prepared.id)? {
        if !installation.definitions.ids.insert(definition.id.clone()) {
            return Err("Mutable definition ID collides".into());
        }
        pair.pdg.definitions.push(definition);
    }
    installation
        .locations
        .insert(prepared.id, prepared.location);

    Ok(())
}

pub(super) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let sites = sites(context.facts)?;
    let index = node_index(pair, sites.len())?;
    let definitions = definition_index(pair)?;
    let mut locations = BTreeMap::new();

    for site in sites {
        let position = *index
            .get(&site.key())
            .ok_or("Missing mutable source access")?;
        let node = &pair.pdg.nodes[position];
        check_site(node, &site)?;
        check_pointer(pair, node, &site, context.facts)?;
        let (id, location) = verified_location(
            node,
            &site,
            context,
            definitions
                .get(node.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        )?;
        locations.insert(id, location);
    }

    check_location_definitions(pair, &locations)?;
    Ok(locations)
}

fn definition_index(pair: PairView<'_>) -> Result<BTreeMap<&str, Vec<&Definition>>, String> {
    let mut definitions: BTreeMap<&str, Vec<&Definition>> = BTreeMap::new();
    let mut definition_ids = BTreeSet::new();
    for definition in &pair.pdg.definitions {
        if !definition_ids.insert(&definition.id) {
            return Err("Duplicated definition identity".into());
        }
        definitions
            .entry(&definition.node_id)
            .or_default()
            .push(definition);
    }

    Ok(definitions)
}

fn verified_location(
    node: &PdgNode,
    site: &Site,
    context: &mut LocationContext<'_>,
    definitions: &[&Definition],
) -> Result<(String, MemoryLocation), String> {
    let (id, location) = scalar_location(context, site.parameter)?;
    check_effects(node, site, &id, definitions)?;

    Ok((id, location))
}

struct DefinitionInventory {
    ids: BTreeSet<String>,
    nodes: BTreeSet<String>,
}

impl DefinitionInventory {
    fn new(definitions: &[Definition]) -> Result<Self, String> {
        let mut ids = BTreeSet::new();
        let mut nodes = BTreeSet::new();
        for definition in definitions {
            if !ids.insert(definition.id.clone()) {
                return Err("Duplicated definition identity".into());
            }
            nodes.insert(definition.node_id.clone());
        }

        Ok(Self { ids, nodes })
    }
}

fn install_effect(node: &mut PdgNode, site: &Site, id: &str) -> Result<Option<Definition>, String> {
    if !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
    {
        return Err("Mutable source access already has contradictory effects".into());
    }
    if site.kind == AccessKind::Load {
        node.attributes.reads = vec![effect(id, true)];
        return Ok(None);
    }
    let definition = Definition {
        id: format!("definition:mutable:{}", node.id),
        node_id: node.id.clone(),
        symbol_id: None,
        location_id: Some(id.into()),
    };
    node.attributes.writes = vec![effect(id, false)];
    node.attributes.defines = vec![definition.id.clone()];

    Ok(Some(definition))
}

fn scalar_location(
    context: &mut LocationContext<'_>,
    parameter: u32,
) -> Result<(String, MemoryLocation), String> {
    context.formal_location(
        parameter,
        &[Projection::MutableReferent],
        TypeToken::Scalar("usize".into()),
    )
}

fn effect(location: &str, load: bool) -> Access {
    Access {
        symbol_id: None,
        location_id: Some(location.into()),
        unresolved_reason: None,
        operand_position: load.then_some(0),
    }
}

impl Site {
    fn key(&self) -> SiteKey {
        (
            u64::from(self.span.start),
            u64::from(self.span.end),
            self.kind == AccessKind::Store,
        )
    }
}

fn node_index(pair: PairView<'_>, count: usize) -> Result<BTreeMap<SiteKey, usize>, String> {
    let mut index = BTreeMap::new();
    for (position, node) in pair
        .pdg
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| is_access(node))
    {
        let span = node
            .source_span
            .as_ref()
            .ok_or("Mutable access lacks source span")?;
        let key = (
            span.start,
            span.end,
            node.attributes.construct.as_deref() == Some(crate::mutable_scalars::STORE),
        );
        if index.len() >= crate::field_locations::MAX_OPERATIONS {
            return Err("Mutable serialized access inventory exceeds its budget".into());
        }
        if index.insert(key, position).is_some() {
            return Err("Duplicated mutable source access".into());
        }
    }
    if index.len() != count {
        return Err("Mutable source access inventory differs".into());
    }

    Ok(index)
}

fn sites(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Vec<Site>, String> {
    if !crate::mutable_scalars::enabled(facts) {
        return Ok(Vec::new());
    }

    crate::mutable_scalars::verify(facts)?;
    let fields = observations(facts)?;
    let evidence = fields
        .scalar_formal_accesses
        .as_ref()
        .ok_or("Missing mutable scalar source inventory")?;

    source_sites(facts, evidence)
}

fn source_sites(
    facts: &dyn crate::unit_store::EvidenceFacts,
    evidence: &ScalarFormalAccesses,
) -> Result<Vec<Site>, String> {
    let mut sources = BTreeMap::new();
    for node in crate::field_locations::operations(&facts.body, &["expr_unary", "expr_assign_op"]) {
        let key = (node.hir.krate, node.hir.owner, node.hir.local);
        if sources.insert(key, node).is_some() {
            return Err("Duplicated mutable raw HIR identity".into());
        }
    }
    evidence
        .accesses
        .iter()
        .map(|access| {
            let hir = if access.kind == AccessKind::Store {
                &access
                    .compound
                    .as_ref()
                    .ok_or("Mutable store lacks compound source")?
                    .hir
            } else {
                &access.hir
            };
            let node = sources
                .get(&(hir.krate, hir.owner, hir.local))
                .ok_or("Mutable source node is absent")?;
            source_site(node, access)
        })
        .collect()
}

fn source_site(
    node: &HirNode,
    access: &crate::field_locations::scalar_formals::Access,
) -> Result<Site, String> {
    let operator = if access.kind == AccessKind::Store {
        if node.kind != "expr_assign_op"
            || !node
                .children
                .first()
                .is_some_and(|target| target.hir == access.hir && target.span == access.span)
        {
            return Err("Mutable store loses its exact compound target".into());
        }
        Some(
            node.dispatch
                .as_ref()
                .ok_or("Mutable store lacks builtin dispatch")?
                .op
                .clone(),
        )
    } else {
        if node.kind != "expr_unary" || node.span != access.span {
            return Err("Mutable load loses source dereference".into());
        }
        None
    };
    Ok(Site {
        parameter: access.parameter,
        span: node.span,
        kind: access.kind,
        operator,
        pointer: (access.kind == AccessKind::Load).then(|| node.children[0].span),
    })
}

fn check_site(node: &PdgNode, site: &Site) -> Result<(), String> {
    let store = site.kind == AccessKind::Store;
    let (category, kind, construct, ty, role) = interpretation(store);
    let actual_type: crate::StringFact = serde_json::from_value(
        node.annotations
            .get("compat.rust.hir-r0:type")
            .ok_or("Missing mutable access type")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;
    check_source_role(node, site, role)?;
    if node.category != category
        || node.original_kind.known_value() != Some(kind)
        || node.attributes.construct.as_deref() != Some(construct)
        || node.attributes.position != Some(u64::from(site.parameter))
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != site.operator.as_deref()
        || node.attributes.write_form.as_deref() != store.then_some("compound_assignment")
        || actual_type.known_value() != Some(ty)
        || node.attributes.operator.is_some() != site.operator.is_some()
        || lexical_effects(node)
    {
        return Err("Mutable access source/root/type or lexical descriptor effects differ".into());
    }

    Ok(())
}

fn check_effects(
    node: &PdgNode,
    site: &Site,
    id: &str,
    definitions: &[&Definition],
) -> Result<(), String> {
    if site.kind == AccessKind::Load {
        if node.attributes.reads != [effect(id, true)]
            || !node.attributes.writes.is_empty()
            || !node.attributes.defines.is_empty()
            || !definitions.is_empty()
        {
            return Err("Mutable load invents writes or loses its exact referent read".into());
        }
    } else {
        let [definition] = definitions else {
            return Err("Mutable store requires exactly one definition".into());
        };
        if !node.attributes.reads.is_empty()
            || node.attributes.writes != [effect(id, false)]
            || node.attributes.defines != [definition.id.clone()]
            || definition.node_id != node.id
            || definition.symbol_id.is_some()
            || definition.location_id.as_deref() != Some(id)
        {
            return Err("Mutable store loses its exact location-bound definition".into());
        }
    }

    Ok(())
}

fn check_location_definitions(
    pair: PairView<'_>,
    locations: &BTreeMap<String, MemoryLocation>,
) -> Result<(), String> {
    let allowed: BTreeSet<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some(crate::mutable_scalars::STORE))
        .flat_map(|node| node.attributes.defines.iter())
        .collect();
    for definition in &pair.pdg.definitions {
        if definition
            .location_id
            .as_ref()
            .is_some_and(|id| locations.contains_key(id))
            && !allowed.contains(&definition.id)
        {
            return Err("Extra mutable referent definition outside source stores".into());
        }
    }

    Ok(())
}

fn check_pointer(
    pair: PairView<'_>,
    node: &PdgNode,
    site: &Site,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let Some(span) = site.pointer else {
        return Ok(());
    };
    let pointer = serialized_operation(pair, &span, |candidate| {
        candidate.category == NodeCategory::Read
            && candidate.original_kind.known_value() == Some("expr_path")
    })?;

    let root = &facts.signature.params[site.parameter as usize].symbol;
    let read = Access {
        symbol_id: Some(root.clone()),
        location_id: None,
        unresolved_reason: None,
        operand_position: Some(0),
    };
    if pointer.attributes.symbol_ids != [root.clone()]
        || pointer.attributes.reads != [read]
        || !pointer.attributes.writes.is_empty()
        || !pointer.attributes.defines.is_empty()
    {
        return Err("Mutable load pointer must remain a separate lexical formal read".into());
    }

    super::verify_saved_operands(pair, node, &[pointer])
}

fn interpretation(
    store: bool,
) -> (
    NodeCategory,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    if store {
        (
            NodeCategory::Assignment,
            "expr_assign_op",
            crate::mutable_scalars::STORE,
            "unit",
            "compound_write",
        )
    } else {
        (
            NodeCategory::Read,
            "expr_unary",
            crate::mutable_scalars::LOAD,
            "usize",
            "source_operation",
        )
    }
}

fn lexical_effects(node: &PdgNode) -> bool {
    !node.attributes.symbol_ids.is_empty()
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.exit_kind.is_some()
        || !node.attributes.outcomes.is_empty()
        || node.annotations.contains_key(crate::data_places::KEY)
}

fn check_source_role(node: &PdgNode, site: &Site, role: &str) -> Result<(), String> {
    let span = node
        .source_span
        .as_ref()
        .ok_or("Mutable access lacks original source span")?;
    let expected = serde_json::to_value(SourceRole {
        construct_span: crate::Span {
            source_id: span.source_id.clone(),
            start: u64::from(site.span.start),
            end: u64::from(site.span.end),
        },
        role,
    })
    .map_err(|error| error.to_string())?;
    if node
        .attributes
        .extensions
        .get("compat.rust.hir-r0:source-operation")
        != Some(&expected)
    {
        return Err("Mutable access construct source/role differs".into());
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct SourceRole<'a> {
    construct_span: crate::Span,
    role: &'a str,
}
