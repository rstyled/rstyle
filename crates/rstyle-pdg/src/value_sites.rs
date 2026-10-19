//! Independent path checks for nonexecuting expression values and guards.
//! This validator reads serialized CFG/PDG evidence, not producer assertions.

use crate::producer::{Atom, Guard};
use crate::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const SITES: &str = "compat.rust.hir-r0:value-sites";
const GUARDS: &str = "compat.rust.hir-r0:guards";
const TYPE: &str = "compat.rust.hir-r0:type";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Annotations {
    #[serde(rename = "compat.rust.hir-r0:guards")]
    guards: Guard,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Alternative {
    value_pdg_node_id: String,
    outcome: String,
    completion_cfg_node_ids: Vec<String>,
    annotations: Annotations,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Site {
    #[serde(default, rename = "rstyle.hir-coverage:selection-kind")]
    selection_kind: Option<String>,
    pdg_node_id: String,
    construct_span: Span,
    controller_pdg_node_id: String,
    cfg_successor_node_id: String,
    annotations: Annotations,
    alternatives: Vec<Alternative>,
}

#[derive(Clone)]
struct Path {
    vertices: Vec<String>,
    operations: Vec<String>,
    outcomes: BTreeMap<String, String>,
}

struct Context<'a> {
    pdg: &'a PdgDocument,
    cfg: &'a CfgDocument,
    sites: Vec<Site>,
    paths: Vec<Path>,
}

struct Availability<'a> {
    path: &'a Path,
    before: &'a BTreeSet<&'a String>,
    visiting: BTreeSet<String>,
}

pub fn validate_value_sites(
    pdg: &PdgDocument,
    cfg: &CfgDocument,
) -> Result<BTreeSet<String>, Vec<String>> {
    if crate::extraction_profile::local_policy(&pdg.profile)
        && crate::extraction_profile::policy_supports(&pdg.profile, "rust.scalar-loops")
        && pdg
            .nodes
            .iter()
            .any(|node| matches!(node.attributes.construct.as_deref(), Some("while" | "loop")))
    {
        return crate::cyclic_values::validate(pdg, cfg);
    }
    let merges: Vec<_> = pdg
        .nodes
        .iter()
        .filter(|n| n.category == NodeCategory::Merge)
        .collect();
    if merges.is_empty() && !pdg.extensions.contains_key(SITES) {
        return Ok(BTreeSet::new());
    }
    let ctx = context(pdg, cfg).map_err(|e| vec![e])?;
    let results: Vec<_> = ctx.sites.iter().map(|site| ctx.check_site(site)).collect();
    let errors: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().err().cloned())
        .chain(ctx.check_value_operands())
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    let ids: BTreeSet<_> = ctx
        .sites
        .iter()
        .map(|site| site.pdg_node_id.clone())
        .collect();
    if ids.len() != ctx.sites.len() || ids != merges.iter().map(|m| m.id.clone()).collect() {
        return Err(vec![
            "Value sites must cover every merge exactly once".to_string(),
        ]);
    }

    Ok(results
        .into_iter()
        .filter_map(Result::ok)
        .flatten()
        .collect())
}

fn context<'a>(pdg: &'a PdgDocument, cfg: &'a CfgDocument) -> Result<Context<'a>, String> {
    let value = pdg
        .extensions
        .get(SITES)
        .ok_or("Missing merge value-site evidence")?;
    if cfg.extensions.get(SITES) != Some(value) {
        return Err("Paired value-site evidence disagrees".to_string());
    }
    let sites: Vec<Site> =
        serde_json::from_value(value.clone()).map_err(|e| format!("Malformed value sites: {e}"))?;
    let map = read_operation_map(&pdg.extensions)?;
    let mapping = map
        .iter()
        .map(|m| (m.cfg_node_id.clone(), m.pdg_node_id.clone()))
        .collect();
    let paths = paths(cfg, &mapping)?;

    Ok(Context {
        pdg,
        cfg,
        sites,
        paths,
    })
}

fn paths(cfg: &CfgDocument, mapping: &BTreeMap<String, String>) -> Result<Vec<Path>, String> {
    let first = Path {
        vertices: vec![cfg.entry_node_id.clone()],
        operations: Vec::new(),
        outcomes: BTreeMap::new(),
    };
    let mut pending = vec![first];
    let mut complete = Vec::new();
    while let Some(path) = pending.pop() {
        let end = path.vertices.last().expect("path entry");
        if cfg.exit_node_ids.contains(end) {
            complete.push(path);
            continue;
        }
        let edges: Vec<_> = cfg.edges.iter().filter(|e| &e.source == end).collect();
        if edges.is_empty() {
            return Err("Reachable normal-flow path does not reach an exit".to_string());
        }
        for edge in edges {
            if path.vertices.contains(&edge.target) {
                return Err("Cyclic flow is outside R0".to_string());
            }
            pending.push(extend_path(&path, edge, mapping)?);
        }
    }

    Ok(complete)
}

fn extend_path(
    path: &Path,
    edge: &CfgEdge,
    mapping: &BTreeMap<String, String>,
) -> Result<Path, String> {
    let mut next = path.clone();
    next.vertices.push(edge.target.clone());
    if let Some(operation) = mapping.get(&edge.target) {
        next.operations.push(operation.clone());
    }
    if matches!(edge.kind, CfgEdgeKind::True | CfgEdgeKind::False) {
        let controller = mapping
            .get(&edge.source)
            .ok_or("Boolean edge has no controller operation")?;
        next.outcomes.insert(
            controller.clone(),
            if edge.kind == CfgEdgeKind::True {
                "true"
            } else {
                "false"
            }
            .to_string(),
        );
    }

    Ok(next)
}

impl Context<'_> {
    fn node(&self, id: &str) -> Result<&PdgNode, String> {
        self.pdg
            .nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or_else(|| format!("Unknown value/controller node {id}"))
    }

    fn check_guard(&self, guard: &Guard) -> Result<(), String> {
        let mut terms = BTreeSet::new();
        for term in guard {
            let atoms: BTreeSet<_> = term.iter().collect();
            let controllers: BTreeSet<_> = term.iter().map(|a| &a.controller_pdg_node_id).collect();
            if atoms.len() != term.len() || controllers.len() != term.len() || !terms.insert(atoms)
            {
                return Err("Duplicate or contradictory guard atoms/terms".to_string());
            }
            for atom in term {
                self.check_atom(atom)?;
            }
        }

        Ok(())
    }

    fn check_atom(&self, atom: &Atom) -> Result<(), String> {
        let controller = self.node(&atom.controller_pdg_node_id)?;
        if controller.category != NodeCategory::Control
            || !controller.attributes.outcomes.contains(&atom.outcome)
            || !matches!(atom.outcome.as_str(), "true" | "false")
        {
            return Err(
                "Guard must name a real predicate and declared boolean outcome".to_string(),
            );
        }

        Ok(())
    }

    fn check_site(&self, site: &Site) -> Result<Option<String>, String> {
        self.check_site_metadata(site)?;
        let node = self.node(&site.pdg_node_id)?;
        self.check_guard(&site.annotations.guards)?;
        if node.annotations.get(GUARDS)
            != Some(&serde_json::to_value(&site.annotations.guards).map_err(|e| e.to_string())?)
        {
            return Err("Merge/site guard disagreement".to_string());
        }
        self.check_site_alternatives(site)?;
        let site_paths: Vec<_> = self
            .paths
            .iter()
            .filter(|p| p.vertices.contains(&site.cfg_successor_node_id))
            .collect();

        for path in &site_paths {
            self.check_site_path(site, path)?;
        }
        if site_paths
            .iter()
            .any(|p| satisfies(&site.annotations.guards, p))
        {
            Ok(Some(site.pdg_node_id.clone()))
        } else {
            Ok(None)
        }
    }

    fn check_site_metadata(&self, site: &Site) -> Result<(), String> {
        if let Some(kind) = &site.selection_kind
            && (kind != "ordered-match/1"
                || !crate::extraction_profile::local_policy(&self.pdg.profile)
                || !crate::extraction_profile::policy_supports(
                    &self.pdg.profile,
                    "rust.scalar-match",
                ))
        {
            return Err("Unregistered value selection policy".into());
        }
        let node = self.node(&site.pdg_node_id)?;
        if node.category != NodeCategory::Merge
            || node
                .annotations
                .get(TYPE)
                .and_then(known_type)
                .is_none_or(|ty| ty == "unit")
        {
            return Err("Value site must name a nonunit merge".to_string());
        }
        if site.construct_span.start > site.construct_span.end {
            return Err("Reversed construct span".to_string());
        }
        if node
            .source_span
            .as_ref()
            .is_some_and(|span| span != &site.construct_span)
        {
            return Err("Merge construct/source span disagreement".to_string());
        }

        Ok(())
    }

    fn check_site_alternatives(&self, site: &Site) -> Result<(), String> {
        let alternatives: BTreeSet<_> = site.alternatives.iter().map(|a| &a.outcome).collect();
        if alternatives.len() != site.alternatives.len() || site.alternatives.len() < 2 {
            return Err("Merge requires distinct normal alternatives".to_string());
        }
        for alternative in &site.alternatives {
            self.check_alternative(site, alternative)?;
        }

        if self
            .pdg
            .edges
            .iter()
            .filter(|edge| edge.target == site.pdg_node_id && edge.subkind == "value_alternative")
            .count()
            != if site.selection_kind.is_some()
                && !self
                    .paths
                    .iter()
                    .any(|path| path.vertices.contains(&site.cfg_successor_node_id))
            {
                0
            } else {
                site.alternatives.len()
            }
        {
            return Err("Extra or missing merge alternative edges".to_string());
        }
        Ok(())
    }

    fn check_alternative(&self, site: &Site, alternative: &Alternative) -> Result<(), String> {
        self.check_guard(&alternative.annotations.guards)?;
        let frontier: BTreeSet<_> = alternative.completion_cfg_node_ids.iter().collect();
        if frontier.is_empty() || frontier.len() != alternative.completion_cfg_node_ids.len() {
            return Err("Completion frontier must be a nonempty set".to_string());
        }
        if frontier.iter().any(|id| {
            self.cfg
                .nodes
                .iter()
                .find(|n| &n.id == *id)
                .is_none_or(|n| n.operations.len() != 1)
        }) {
            return Err("Frontier members must be real operations".to_string());
        }
        self.check_alternative_edge(site, alternative)?;
        let paths: Vec<_> = self
            .paths
            .iter()
            .filter(|p| {
                p.vertices.contains(&site.cfg_successor_node_id)
                    && satisfies(&alternative.annotations.guards, p)
            })
            .collect();
        let observed: BTreeSet<_> = paths
            .iter()
            .filter_map(|p| predecessor(p, &site.cfg_successor_node_id))
            .collect();
        if !paths.is_empty() && observed != frontier {
            return Err("Alternative completion frontier is not exact".to_string());
        }

        Ok(())
    }

    fn check_alternative_edge(&self, site: &Site, alternative: &Alternative) -> Result<(), String> {
        let edges: Vec<_> = self
            .pdg
            .edges
            .iter()
            .filter(|e| {
                e.target == site.pdg_node_id
                    && e.source == alternative.value_pdg_node_id
                    && e.subkind == "value_alternative"
                    && e.outcome.as_ref() == Some(&alternative.outcome)
            })
            .collect();
        if site.selection_kind.is_some()
            && edges.is_empty()
            && !self
                .paths
                .iter()
                .any(|path| path.vertices.contains(&site.cfg_successor_node_id))
        {
            return Ok(());
        }

        let [edge] = edges.as_slice() else {
            return Err("One alternative edge per semantic alternative is required".to_string());
        };
        let position = if site.selection_kind.is_some() {
            alternative
                .outcome
                .strip_prefix("arm:")
                .and_then(|index| index.parse::<u64>().ok())
                .filter(|index| *index < 16)
                .ok_or("Invalid match arm ordinal")?
        } else {
            if alternative.outcome == "true" { 0 } else { 1 }
        };
        if edge.kind != EdgeFamily::Data
            || edge.operand_position != Some(position)
            || edge.annotations.get(GUARDS)
                != Some(
                    &serde_json::to_value(&alternative.annotations.guards)
                        .map_err(|e| e.to_string())?,
                )
        {
            return Err("Alternative edge/guard/position disagreement".to_string());
        }
        Ok(())
    }

    fn check_site_path(&self, site: &Site, path: &Path) -> Result<(), String> {
        let successor = path
            .vertices
            .iter()
            .position(|v| v == &site.cfg_successor_node_id)
            .expect("site on path");
        let before = path.vertices[..successor].iter().collect();
        let availability = Availability {
            path,
            before: &before,
            visiting: BTreeSet::new(),
        };
        let available = self.satisfies_before(&site.annotations.guards, &availability);
        if available != path.outcomes.contains_key(&site.controller_pdg_node_id) {
            return Err(
                "Merge availability does not match normally completed controller paths".to_string(),
            );
        }
        let selected: Vec<_> = site
            .alternatives
            .iter()
            .filter(|a| self.satisfies_before(&a.annotations.guards, &availability))
            .collect();
        if selected.len() != usize::from(available) {
            return Err("Merge alternative guards must partition available paths".to_string());
        }
        for alternative in selected {
            if site.selection_kind.is_none()
                && path.outcomes.get(&site.controller_pdg_node_id) != Some(&alternative.outcome)
            {
                return Err("Alternative guard does not imply the controller outcome".to_string());
            }
            if !self.available(&alternative.value_pdg_node_id, &availability)? {
                return Err(
                    "Alternative snapshot was not evaluated or available on its path".to_string(),
                );
            }
        }

        Ok(())
    }

    fn available(&self, id: &str, availability: &Availability<'_>) -> Result<bool, String> {
        if availability.visiting.contains(id) {
            return Err("Cyclic value dependency".to_string());
        }
        let node = self.node(id)?;
        if node.category == NodeCategory::FormalInput {
            return Ok(true);
        }
        if node.category != NodeCategory::Merge {
            return Ok(availability
                .before
                .iter()
                .any(|cfg_id| self.operation_at(cfg_id) == Some(id)));
        }
        let site = self
            .sites
            .iter()
            .find(|s| s.pdg_node_id == id)
            .ok_or("Missing nested merge site")?;
        if !self.satisfies_before(&site.annotations.guards, availability) {
            return Ok(false);
        }
        let selected: Vec<_> = site
            .alternatives
            .iter()
            .filter(|a| self.satisfies_before(&a.annotations.guards, availability))
            .collect();
        let [alternative] = selected.as_slice() else {
            return Ok(false);
        };
        let visiting = availability
            .visiting
            .iter()
            .cloned()
            .chain(std::iter::once(id.to_string()))
            .collect();

        let nested = Availability {
            visiting,
            ..*availability
        };

        self.available(&alternative.value_pdg_node_id, &nested)
    }

    fn satisfies_before(&self, guard: &Guard, availability: &Availability<'_>) -> bool {
        guard.iter().any(|term| {
            term.iter().all(|atom| {
                availability.path.outcomes.get(&atom.controller_pdg_node_id) == Some(&atom.outcome)
                    && availability.before.iter().any(|cfg_id| {
                        self.operation_at(cfg_id) == Some(&atom.controller_pdg_node_id)
                    })
            })
        })
    }

    fn operation_at(&self, cfg_id: &str) -> Option<&str> {
        let map = self.pdg.extensions.get(OPERATION_MAP_KEY)?.as_array()?;
        map.iter()
            .find(|m| m.get("cfg_node_id").and_then(serde_json::Value::as_str) == Some(cfg_id))?
            .get("pdg_node_id")?
            .as_str()
    }

    fn check_value_operands(&self) -> Vec<String> {
        self.pdg
            .edges
            .iter()
            .filter(|e| matches!(e.subkind.as_str(), "value_operand" | "value_alternative"))
            .filter_map(|edge| self.value_operand(edge).err())
            .collect()
    }

    fn value_operand(&self, edge: &PdgEdge) -> Result<(), String> {
        let producer = self.node(&edge.source)?;
        if producer.annotations.get(TYPE).and_then(known_type) == Some("unit")
            && !crate::data_aggregates::unit_slot(self.pdg, edge)
        {
            return Err("Unit value edges are forbidden".to_string());
        }
        if edge.subkind != "value_operand" {
            return Ok(());
        }
        for path in self
            .paths
            .iter()
            .filter(|p| p.operations.contains(&edge.target))
        {
            let target_cfg = path
                .vertices
                .iter()
                .position(|v| self.operation_at(v) == Some(&edge.target))
                .expect("mapped target");
            let before = path.vertices[..target_cfg].iter().collect();
            let availability = Availability {
                path,
                before: &before,
                visiting: BTreeSet::new(),
            };
            if !self.available(&edge.source, &availability)? {
                return Err("Operand snapshot unavailable before consumer".to_string());
            }
        }

        Ok(())
    }
}

fn known_type(value: &serde_json::Value) -> Option<&str> {
    (value.get("status")?.as_str()? == "known")
        .then(|| value.get("value")?.as_str())
        .flatten()
}

fn satisfies(guard: &Guard, path: &Path) -> bool {
    guard.iter().any(|term| {
        term.iter()
            .all(|atom| path.outcomes.get(&atom.controller_pdg_node_id) == Some(&atom.outcome))
    })
}

fn predecessor<'a>(path: &'a Path, successor: &str) -> Option<&'a String> {
    let index = path.vertices.iter().position(|v| v == successor)?;
    index.checked_sub(1).map(|index| &path.vertices[index])
}
