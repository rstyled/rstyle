//! Guarded intervals for the original decimal expression, never a family-wide
//! exemption. Serialized verification replays lowering and this assessment;
//! prefix graph/origin verification separately proves the physical read regions.
mod guards;
mod interval;

use crate::{
    HirNode, NodeCategory,
    producer::{Analysis, Graph, Operation},
    unit_store::EvidenceFacts,
};
use interval::Interval;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const METHOD: &str = "original prefix true-edge guards, distinct byte loads, reaching snapshots and resolved-width decimal intervals";

pub(crate) struct Ranges {
    allowed: BTreeSet<String>,
    values: BTreeMap<String, Interval>,
    snapshots: BTreeMap<String, String>,
    width: Option<u32>,
}

impl Ranges {
    pub(crate) fn new(
        facts: &dyn EvidenceFacts,
        graph: &Graph,
        analysis: &Analysis,
    ) -> Result<Option<Self>, String> {
        if !crate::extraction_profile::facts_support(
            facts,
            crate::scalar_product_instances::activation::VALUE_CAPABILITY,
        ) || !graph
            .nodes
            .iter()
            .any(|node| node.construct.as_deref() == Some("slice.prefix-length"))
        {
            return Ok(None);
        }
        let checked = crate::scalar_product_instances::activation::verify(facts)?;
        let guards = guards::Guards::new(graph, &checked)?;
        let mut result = Self {
            allowed: BTreeSet::new(),
            values: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            width: facts
                .build
                .pointer_width
                .value
                .and_then(|width| u32::try_from(width).ok()),
        };
        for source in scalar_nodes(checked.prefix().success()) {
            let node = source_operation(graph, source)?;
            guards.success(node, analysis)?;
            result.allowed.insert(node.id.clone());
            if node.category == NodeCategory::Read && node.original_kind == "expr_path" {
                result
                    .snapshots
                    .insert(node.id.clone(), snapshot(node, graph, analysis)?);
            }
        }
        result.values = digit_ranges(&checked, &guards, graph, analysis)?;

        Ok(Some(result))
    }

    pub(crate) fn safe(&self, node: &Operation) -> bool {
        self.allowed.contains(&node.id)
            && node.category == NodeCategory::Computation
            && matches!(node.operator.as_deref(), Some("+" | "-" | "*"))
            && self
                .expression(node)
                .is_some_and(|value| value.fits(&node.ty, self.width))
    }

    pub(crate) fn observe(&mut self, node: &Operation, constants: &BTreeMap<String, Option<i128>>) {
        if !self.allowed.contains(&node.id) || self.values.contains_key(&node.id) {
            return;
        }
        let value = if node.original_kind == "expr_lit" {
            constants
                .get(&node.id)
                .copied()
                .flatten()
                .and_then(Interval::constant)
        } else {
            self.expression(node)
        };
        if let Some(value) = value.filter(|value| value.fits(&node.ty, self.width)) {
            self.values.insert(node.id.clone(), value);
        }
    }

    fn expression(&self, node: &Operation) -> Option<Interval> {
        if node.category == NodeCategory::Read {
            return self.values.get(self.snapshots.get(&node.id)?).copied();
        }
        let first = *self.values.get(node.operands.first()?)?;
        if node.original_kind == "expr_cast"
            && node.operator.as_deref() == Some("as")
            && node.operands.len() == 1
        {
            return Some(first);
        }

        let [_, second] = node.operands.as_slice() else {
            return None;
        };
        first.binary(node.operator.as_deref()?, *self.values.get(second)?)
    }
}

fn scalar_nodes(root: &HirNode) -> Vec<&HirNode> {
    let mut pending = vec![root];
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        if matches!(
            node.kind.as_str(),
            "expr_lit" | "expr_binary" | "expr_cast" | "expr_unary"
        ) || node.kind == "expr_path"
            && crate::source_evidence::resolved_local_leaf(node).is_some()
        {
            nodes.push(node);
        }
        pending.extend(&node.children);
    }
    nodes
}

fn snapshot(node: &Operation, graph: &Graph, analysis: &Analysis) -> Result<String, String> {
    let definitions = analysis
        .definition_in
        .get(&node.id)
        .and_then(|incoming| incoming.get(node.symbol.as_ref()?))
        .ok_or("Missing original decimal local reaching snapshot")?;
    if definitions.len() != 1 {
        return Err("Decimal local requires a unique original reaching snapshot".into());
    }
    let definition = definitions
        .first()
        .ok_or("Missing original decimal local definition")?;
    let declaration = graph
        .nodes
        .iter()
        .find(|node| node.definition.as_ref() == Some(definition))
        .ok_or("Unknown original decimal local definition")?;
    let [value] = declaration.operands.as_slice() else {
        return Err("Decimal local initializer requires one original value".into());
    };
    if declaration.category != NodeCategory::Declaration
        || declaration.write_form.as_deref() != Some("initialization")
    {
        return Err("Decimal local snapshot must come from original initialization".into());
    }

    Ok(value.clone())
}

fn source_operation<'a>(graph: &'a Graph, source: &HirNode) -> Result<&'a Operation, String> {
    let mut nodes = graph.nodes.iter().filter(|node| {
        node.span == source.span
            && node.original_kind == source.kind
            && matches!(
                node.category,
                NodeCategory::Computation | NodeCategory::Read
            )
    });
    let node = nodes
        .next()
        .ok_or("Missing original decimal scalar operation")?;
    if nodes.next().is_some()
        || source.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(node.ty.as_str())
    {
        return Err("Ambiguous or retyped original decimal scalar operation".into());
    }
    Ok(node)
}

fn digit_ranges(
    checked: &crate::scalar_product_instances::activation::Checked<'_>,
    guards: &guards::Guards<'_>,
    graph: &Graph,
    analysis: &Analysis,
) -> Result<BTreeMap<String, Interval>, String> {
    let mut values = BTreeMap::new();
    for digit in checked.prefix().digits() {
        let read = source_operation(graph, digit.read())?;
        guards.success(read, analysis)?;
        guards::binding(graph, analysis, digit, read)?;
        if read.category != NodeCategory::Read
            || read.construct.as_deref() != Some("data.indirect-deref")
            || read.ty != "u8"
        {
            return Err(
                "Guarded decimal interval requires the original distinct byte dereference".into(),
            );
        }
        values.insert(read.id.clone(), Interval::byte_digit());
    }

    Ok(values)
}
