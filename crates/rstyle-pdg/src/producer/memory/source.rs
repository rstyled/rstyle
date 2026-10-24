//! Join verified original accesses to actual lowered operations, never annotations.
use super::*;
use crate::field_locations::scalar_formals::{self, FormalObservation, ScalarFormalAccesses};
use crate::{ByteSpan, HirRef, NodeCategory, mutable_scalars, unit_store::EvidenceFacts};

type Key = (u8, u32, u32, u32);

struct Expected {
    root: u32,
    span: ByteSpan,
    kind: AccessKind,
    original: &'static str,
    role: &'static str,
    operator: Option<String>,
}

/// Index identities from the already verified HIR; never build serialized keys.
struct SourceIndex {
    compounds: BTreeMap<(u32, u32, u32), ByteSpan>,
    indexes: BTreeMap<(u32, u32), u32>,
}

impl SourceIndex {
    fn new(facts: &dyn EvidenceFacts) -> Result<Self, String> {
        let mut result = Self {
            compounds: BTreeMap::new(),
            indexes: BTreeMap::new(),
        };
        for node in
            crate::field_locations::operations(&facts.body, &["expr_assign_op", "expr_index"])
        {
            let duplicate = if node.kind == "expr_assign_op" {
                result
                    .compounds
                    .insert(hir_key(&node.hir), node.span)
                    .is_some()
            } else {
                let root = mutable_scalars::index_root(node, facts)?;
                result
                    .indexes
                    .insert((node.span.start, node.span.end), root)
                    .is_some()
            };
            if duplicate {
                return Err("Mutable memory source identities are ambiguous".into());
            }
        }
        Ok(result)
    }
}

/// Activate only the checked mutable-scalar family. All memory roots and alias
/// alternatives come from the complete source/compiler inventory.
pub fn from_graph(
    graph: &Graph,
    facts: &dyn EvidenceFacts,
) -> Result<Option<MemoryAnalysis>, String> {
    if !mutable_scalars::enabled(facts) {
        return Ok(None);
    }

    graph_budget(graph)?;
    mutable_scalars::verify(facts)?;
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()?;
    let inventory = fields
        .scalar_formal_accesses
        .as_ref()
        .ok_or("Mutable memory graph lacks scalar formal access evidence")?;

    let roots = inventory
        .formals
        .iter()
        .filter(|formal| {
            matches!(
                formal.observation,
                FormalObservation::Shared { .. } | FormalObservation::Mutable { .. }
            )
        })
        .map(|formal| Location(formal.parameter))
        .collect::<Vec<_>>();
    let aliases = inventory
        .possibly_aliasing_formals
        .iter()
        .map(|pair| (Location(pair[0]), Location(pair[1])))
        .collect::<Vec<_>>();

    let source = SourceIndex::new(facts)?;
    let expected = scalar_sites(&source, inventory)?;
    let accesses = graph_accesses(graph, source.indexes, expected)?;
    let exits = failure_exits(graph)?;

    analyze(
        graph,
        AccessInventory {
            roots: &roots,
            accesses: &accesses,
            aliases: &aliases,
            exits: &exits,
        },
    )
    .map(Some)
}

fn scalar_sites(
    source: &SourceIndex,
    inventory: &ScalarFormalAccesses,
) -> Result<BTreeMap<Key, Expected>, String> {
    let mut expected = BTreeMap::new();
    for access in &inventory.accesses {
        let store = access.kind == scalar_formals::AccessKind::Store;
        let span = if store {
            let compound = access
                .compound
                .as_ref()
                .ok_or("Mutable store lacks its compound source")?;
            *source
                .compounds
                .get(&hir_key(&compound.hir))
                .ok_or("Mutable store compound source is missing")?
        } else {
            access.span
        };
        let site = Expected {
            root: access.parameter,
            span,
            kind: if store {
                AccessKind::Store
            } else {
                AccessKind::Load
            },
            original: if store {
                "expr_assign_op"
            } else {
                "expr_unary"
            },
            role: if store {
                "compound_write"
            } else {
                "source_operation"
            },
            operator: store.then(|| access.compound.as_ref().unwrap().operator.clone()),
        };
        let key = key(if store { 1 } else { 0 }, span, access.parameter);
        if expected.insert(key, site).is_some() {
            return Err("Mutable source access repeats a location site".into());
        }
    }
    Ok(expected)
}

struct SourceSites {
    indexes: BTreeMap<(u32, u32), u32>,
    expected: BTreeMap<Key, Expected>,
}

impl SourceSites {
    fn access(
        &mut self,
        operation: &super::super::Operation,
    ) -> Result<Option<MemoryAccess>, String> {
        match operation.construct.as_deref() {
            Some(mutable_scalars::LOAD | mutable_scalars::STORE) => {
                self.scalar_access(operation).map(Some)
            }
            Some("data.index") => index_access(operation, &mut self.indexes).map(Some),
            _ => Ok(None),
        }
    }

    fn scalar_access(
        &mut self,
        operation: &super::super::Operation,
    ) -> Result<MemoryAccess, String> {
        let store = operation.construct.as_deref() == Some(mutable_scalars::STORE);
        let root = operation
            .position
            .and_then(|position| u32::try_from(position).ok())
            .ok_or("Mutable graph access lacks its formal root ordinal")?;
        let site = self
            .expected
            .remove(&key(if store { 1 } else { 0 }, operation.span, root))
            .ok_or("Mutable graph access has no unique original source site")?;
        verify_site(operation, &site)?;

        Ok(MemoryAccess {
            operation: operation.id.clone(),
            location: Location(root),
            kind: site.kind,
        })
    }
}

fn graph_accesses(
    graph: &Graph,
    indexes: BTreeMap<(u32, u32), u32>,
    expected: BTreeMap<Key, Expected>,
) -> Result<Vec<MemoryAccess>, String> {
    let mut sites = SourceSites { indexes, expected };
    let accesses = graph
        .nodes
        .iter()
        .map(|operation| sites.access(operation))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    if !sites.expected.is_empty() || !sites.indexes.is_empty() {
        return Err("Mutable graph omits original scalar or indexed memory accesses".into());
    }
    Ok(accesses)
}

fn verify_site(operation: &super::super::Operation, site: &Expected) -> Result<(), String> {
    let store = site.kind == AccessKind::Store;
    let operator = operation
        .operator
        .as_deref()
        .map(|operator| operator.trim_end_matches('='));
    let expected_operator = site
        .operator
        .as_deref()
        .map(|operator| operator.trim_end_matches('='));

    if operation.span != site.span
        || operation.construct_span != site.span
        || operation.position != Some(u64::from(site.root))
        || operation.original_kind != site.original
        || operation.role != site.role
        || operation.symbol.is_some()
        || operation.definition.is_some()
        || operation.category
            != if store {
                NodeCategory::Assignment
            } else {
                NodeCategory::Read
            }
        || operation.ty != if store { "unit" } else { "usize" }
        || operation.write_form.as_deref() != store.then_some("compound_assignment")
        || operator != expected_operator
        || operation.operands.len() != 1
    {
        return Err("Mutable graph access changes its source/root/effect interpretation".into());
    }
    Ok(())
}

fn index_access(
    operation: &super::super::Operation,
    indexes: &mut BTreeMap<(u32, u32), u32>,
) -> Result<MemoryAccess, String> {
    let root = indexes
        .remove(&(operation.span.start, operation.span.end))
        .ok_or("Indexed graph load lacks one unused exact HIR source")?;
    if operation.category != NodeCategory::Read
        || operation.original_kind != "expr_index"
        || operation.construct_span != operation.span
        || operation.role != "source_operation"
        || operation.ty != "u8"
        || operation.symbol.is_some()
        || operation.definition.is_some()
        || operation.write_form.is_some()
        || operation.operands.len() != 2
    {
        return Err("Indexed memory graph changes or duplicates its original shared read".into());
    }
    Ok(MemoryAccess {
        operation: operation.id.clone(),
        location: Location(root),
        kind: AccessKind::Load,
    })
}

fn failure_exits(graph: &Graph) -> Result<Vec<MemoryExit>, String> {
    let nodes: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();

    if nodes.keys().any(|id| id.starts_with("P:")) {
        return Err("Mutable failure boundaries cannot contain graph operations".into());
    }

    let mut failures = BTreeSet::new();
    for transfer in &graph.transfers {
        if transfer.kind == "exception" {
            let source = nodes
                .get(transfer.source.as_str())
                .ok_or("Exception source operation is missing")?;
            if transfer.target != format!("P:{}", transfer.source)
                || source.category != NodeCategory::Computation
                || source.construct.as_deref() == Some(mutable_scalars::STORE)
                || transfer.loop_role.is_some()
                || !failures.insert(transfer.target.clone())
            {
                return Err("Mutable failure is not a distinct checked pre-store exit".into());
            }
        } else if transfer.target.starts_with("P:") || transfer.source.starts_with("P:") {
            return Err("Mutable failure boundary has a nonexception continuation".into());
        }
    }

    let mut exits = vec![MemoryExit {
        operation: "X".into(),
        kind: ExitKind::Normal,
    }];
    exits.extend(failures.into_iter().map(|operation| MemoryExit {
        operation,
        kind: ExitKind::Failure,
    }));
    Ok(exits)
}

fn key(kind: u8, span: ByteSpan, root: u32) -> Key {
    (kind, span.start, span.end, root)
}

fn hir_key(hir: &HirRef) -> (u32, u32, u32) {
    (hir.krate, hir.owner, hir.local)
}
