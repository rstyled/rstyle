//! Explicit callback chains select nested calls without changing root selection.
use super::*;
use crate::{
    ByteSpan, HirRef, generated_sources::capture::Definition, id_error_maps::capture::Environment,
};
use std::collections::{BTreeMap, BTreeSet};

const SCOPE: &str =
    "immediate-core-result-reader-callback-owning-body; nested owners separately scoped";
const ENVIRONMENT: &str = "compiler_usize_reader_callback_environment";
const IDENTITY: &str = "rstyle.rust.usize-reader-callback-environment/1";
const MAX_NODES: usize = 4096;
const MAX_HIR_DEPTH: usize = 128;

#[derive(Clone, Copy)]
struct Scope<'record, 'hir> {
    record: &'record Value,
    closure: &'hir HirNode,
    depth: usize,
}

struct Walk<'a> {
    budget: Budget,
    retained: BTreeMap<HirKey, &'a HirNode>,
}

struct Budget {
    remaining: usize,
}

impl Budget {
    fn visit(&mut self, depth: usize, limit: usize) -> Result<(), String> {
        if depth > limit || self.remaining == 0 {
            return Err("Scoped lossless evidence exceeds its node or depth budget".into());
        }
        self.remaining -= 1;
        Ok(())
    }
}

/// The caller must gate the new reader capability and prove the complete source
/// and graph. This join grants only access to an exact raw selected-call record.
pub(crate) fn site<'a>(
    inventory: &'a Value,
    node: &HirNode,
    chain: &[&HirNode],
) -> Result<&'a Value, String> {
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
        || chain.is_empty()
        || chain.len() > crate::concrete_types::MAX_DEPTH
    {
        return Err("Scoped lossless call requires a complete explicit callback chain".into());
    }
    let mut budget = Budget {
        remaining: MAX_NODES,
    };
    if occurrences(inventory, node, &mut budget)? != 1 {
        return Err("Scoped lossless call collides with another root or callback record".into());
    }
    let outer = unique(
        crate::source_evidence::call_inventory::callbacks(
            inventory,
            "Invalid scoped callback roster",
        )?,
        chain[0],
    )?;
    let mut walk = Walk {
        budget,
        retained: BTreeMap::new(),
    };
    walk.scope(
        Scope {
            record: outer,
            closure: chain[0],
            depth: 0,
        },
        None,
    )?;

    let scope = walk.selected_scope(inventory, chain)?;
    let call = unique(calls(scope)?, node)?;
    selected_call(call, node)?;
    Ok(call)
}

impl Walk<'_> {
    fn selected_scope<'record>(
        &self,
        inventory: &'record Value,
        chain: &[&HirNode],
    ) -> Result<&'record Value, String> {
        let mut callbacks = crate::source_evidence::call_inventory::callbacks(
            inventory,
            "Invalid scoped callback roster",
        )?;
        let mut selected = None;

        for closure in chain {
            if self.retained.get(&hir_key(&closure.hir)).copied() != Some(*closure) {
                return Err("Explicit callback chain differs from the retained owned HIR".into());
            }
            let record = unique(callbacks, closure)?;
            selected = Some(record);
            callbacks = crate::source_evidence::call_inventory::callbacks(
                record,
                "Invalid scoped callback roster",
            )?;
        }

        selected.ok_or_else(|| "Missing lossless callback scope".into())
    }
}

impl<'a> Walk<'a> {
    fn scope(&mut self, scope: Scope<'_, 'a>, parent: Option<&HirNode>) -> Result<(), String> {
        let Scope {
            record,
            closure,
            depth,
        } = scope;
        self.remember(closure, depth)?;
        let [_, body] = closure.children.as_slice() else {
            return Err("Lossless callback requires its complete binding and owned body".into());
        };
        let environment = environment(closure)?;
        let definition = definition(record)?;
        if closure.kind != "expr_closure"
            || closure.expansion.is_some()
            || record["scope"] != SCOPE
            || record["complete"] != true
            || crate::source_evidence::call_inventory::decode::<HirRef>(&record["body"])?
                != body.hir
            || environment.body != body.hir
            || environment.definition_identity.is_empty()
            || definition.definition_identity != environment.definition_identity
            || record["capture_count"].as_u64() != Some(environment.captures.len() as u64)
            || body.hir.krate != closure.hir.krate
            || !contains(closure.span, body.span)
            || depth > crate::concrete_types::MAX_DEPTH
        {
            return Err(
                "Lossless callback scope differs from its exact compiler environment".into(),
            );
        }

        parent_join(record, closure, parent)?;
        let nodes = owned_nodes(body, &mut self.budget)?;
        scope_calls(record, body, &nodes)?;
        self.scope_children(scope, &nodes)
    }

    fn remember(&mut self, closure: &'a HirNode, depth: usize) -> Result<(), String> {
        self.budget.visit(depth, crate::concrete_types::MAX_DEPTH)?;
        if self
            .retained
            .insert(hir_key(&closure.hir), closure)
            .is_some()
        {
            return Err("Retained callbacks repeat a compiler HIR identity".into());
        }
        Ok(())
    }
}

fn environment(closure: &HirNode) -> Result<Environment, String> {
    let resolution = closure
        .res
        .as_ref()
        .ok_or("Missing lossless callback environment")?;
    if resolution.kind != ENVIRONMENT || resolution.target.is_some() {
        return Err("Lossless callback requires its own compiler environment kind".into());
    }
    let environment: Environment =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;

    if environment.identity != IDENTITY
        || environment.closure != closure.hir
        || environment.signature_abi != "RustCall"
    {
        return Err("Lossless callback environment identity or owner disagrees".into());
    }
    Ok(environment)
}

fn parent_join(record: &Value, closure: &HirNode, parent: Option<&HirNode>) -> Result<(), String> {
    match parent {
        Some(parent) => {
            let parent_environment = environment(parent)?;
            if crate::source_evidence::call_inventory::decode::<HirRef>(&record["parent_callback"])?
                != parent.hir
                || closure.hir.owner != parent_environment.body.owner
                || !contains(parent.span, closure.span)
            {
                return Err("Lossless callback changes its exact lexical parent scope".into());
            }
        }
        None if record.get("parent_callback").is_some() => {
            return Err("Root callback cannot claim a nested callback parent".into());
        }
        None => {}
    }
    Ok(())
}

type HirKey = (u32, u32, u32);
type SpanKey = (u32, u32);

struct OwnedCalls<'a> {
    hir: BTreeMap<HirKey, Option<&'a HirNode>>,
    span: BTreeMap<SpanKey, Option<&'a HirNode>>,
}

impl<'a> OwnedCalls<'a> {
    fn new(nodes: &[&'a HirNode]) -> Self {
        let mut index = Self {
            hir: BTreeMap::new(),
            span: BTreeMap::new(),
        };
        for &node in nodes {
            if matches!(node.kind.as_str(), "expr_call" | "expr_method_call") {
                index
                    .hir
                    .entry(hir_key(&node.hir))
                    .and_modify(|node| *node = None)
                    .or_insert(Some(node));
                index
                    .span
                    .entry(span_key(node.span))
                    .and_modify(|node| *node = None)
                    .or_insert(Some(node));
            }
        }
        index
    }

    fn exact(&self, hir: &HirRef, span: ByteSpan) -> bool {
        matches!((self.hir.get(&hir_key(hir)), self.span.get(&span_key(span))),
            (Some(Some(by_hir)), Some(Some(by_span)))
                if by_hir.hir == *hir && by_hir.span == span
                    && by_span.hir == *hir && by_span.span == span)
    }
}

fn hir_key(hir: &HirRef) -> HirKey {
    (hir.krate, hir.owner, hir.local)
}

fn span_key(span: ByteSpan) -> SpanKey {
    (span.start, span.end)
}

// Driver inventories include eligible trait/generic calls, not every source call.
// complete:true therefore needs a caller proof of the exact expected source roster.
fn scope_calls(record: &Value, body: &HirNode, nodes: &[&HirNode]) -> Result<(), String> {
    let index = OwnedCalls::new(nodes);
    let mut seen_hir = BTreeSet::new();
    let mut seen_span = BTreeSet::new();

    for call in calls(record)? {
        let hir = crate::source_evidence::call_inventory::decode(&call["hir"])?;
        let span: ByteSpan =
            serde_json::from_value(call["span"].clone()).map_err(|error| error.to_string())?;
        if !index.exact(&hir, span)
            || !seen_hir.insert(hir_key(&hir))
            || !seen_span.insert(span_key(span))
            || hir.owner != body.hir.owner
            || hir.krate != body.hir.krate
            || !contains(body.span, span)
        {
            return Err(
                "Callback call is absent, ambiguous or outside its direct owned HIR".into(),
            );
        }
    }
    Ok(())
}

impl<'a> Walk<'a> {
    fn scope_children(
        &mut self,
        scope: Scope<'_, 'a>,
        nodes: &[&'a HirNode],
    ) -> Result<(), String> {
        let Scope {
            record,
            closure,
            depth,
        } = scope;
        let nested: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|node| node.kind == "expr_closure")
            .collect();
        let records = crate::source_evidence::call_inventory::callbacks(
            record,
            "Invalid scoped callback roster",
        )?;
        if records.len() != nested.len() {
            return Err("Callback child roster omits or invents an owned nested closure".into());
        }

        for child in nested {
            let selected = unique(records, child)?;
            if selected["definition"]["crate"] != record["definition"]["crate"]
                || selected["definition"]["stable_crate_id"]
                    != record["definition"]["stable_crate_id"]
            {
                return Err("Nested callback changes compiled crate ownership".into());
            }
            self.scope(
                Scope {
                    record: selected,
                    closure: child,
                    depth: depth + 1,
                },
                Some(closure),
            )?;
        }
        Ok(())
    }
}

fn definition(record: &Value) -> Result<Definition, String> {
    let definition: Definition =
        serde_json::from_value(record["definition"].clone()).map_err(|error| error.to_string())?;
    if !definition.local
        || definition.definition_identity.is_empty()
        || definition.path.is_empty()
        || definition.crate_name.is_empty()
        || definition.stable_crate_id.is_empty()
    {
        return Err("Callback requires its complete local compiler definition".into());
    }
    Ok(definition)
}

fn owned_nodes<'a>(body: &'a HirNode, budget: &mut Budget) -> Result<Vec<&'a HirNode>, String> {
    let mut nodes = vec![];
    let mut pending = vec![(body, 0)];
    while let Some((node, depth)) = pending.pop() {
        budget.visit(depth, MAX_HIR_DEPTH)?;
        nodes.push(node);
        if node.kind != "expr_closure" {
            if node.children.len().saturating_add(pending.len()) > budget.remaining {
                return Err("Scoped lossless HIR child roster exceeds its node budget".into());
            }
            pending.extend(node.children.iter().map(|child| (child, depth + 1)));
        }
    }
    Ok(nodes)
}

fn unique<'a>(records: &'a [Value], node: &HirNode) -> Result<&'a Value, String> {
    let mut matching = records
        .iter()
        .filter(|record| crate::source_evidence::call_inventory::Site::read(record).collides(node));
    let selected = matching
        .next()
        .ok_or("Missing exact scoped lossless source site")?;
    if matching.next().is_some()
        || crate::source_evidence::call_inventory::decode::<HirRef>(&selected["hir"])? != node.hir
        || serde_json::from_value::<ByteSpan>(selected["span"].clone())
            .map_err(|error| error.to_string())?
            != node.span
    {
        return Err("Scoped lossless source site is ambiguous or mismatched".into());
    }
    Ok(selected)
}

fn selected_call(call: &Value, node: &HirNode) -> Result<(), String> {
    if node.expansion.is_some()
        || call["status"] != "resolved"
        || call["expanded"] != false
        || call
            .get("compiled_dependency")
            .is_some_and(|value| !value.is_null())
    {
        return Err("Scoped lossless selected call is unresolved, expanded or foreign".into());
    }
    Ok(())
}

fn occurrences(inventory: &Value, node: &HirNode, budget: &mut Budget) -> Result<usize, String> {
    let mut count = 0;
    let mut pending = vec![(inventory, 0)];
    while let Some((record, depth)) = pending.pop() {
        budget.visit(depth, crate::concrete_types::MAX_DEPTH + 1)?;
        let calls = calls(record)?;
        let children = crate::source_evidence::call_inventory::callbacks(
            record,
            "Invalid scoped callback roster",
        )?;
        if calls
            .len()
            .saturating_add(children.len())
            .saturating_add(pending.len())
            > budget.remaining
        {
            return Err("Scoped lossless inventory roster exceeds its node budget".into());
        }
        count += call_occurrences(calls, node, budget, depth)?;
        pending.extend(children.iter().map(|child| (child, depth + 1)));
    }
    Ok(count)
}

fn call_occurrences(
    calls: &[Value],
    node: &HirNode,
    budget: &mut Budget,
    depth: usize,
) -> Result<usize, String> {
    let mut count = 0;
    for call in calls {
        budget.visit(depth, crate::concrete_types::MAX_DEPTH + 1)?;
        count +=
            usize::from(crate::source_evidence::call_inventory::Site::read(call).collides(node));
    }
    Ok(count)
}

fn calls(record: &Value) -> Result<&[Value], String> {
    record["calls"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "Missing scoped call roster".into())
}

fn contains(outer: ByteSpan, inner: ByteSpan) -> bool {
    outer.start <= inner.start && inner.start < inner.end && inner.end <= outer.end
}

#[cfg(test)]
mod tests;
