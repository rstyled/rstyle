//! Independently compare source origins with serialized value-dependence paths.

use crate::{
    HirNode, NodeCategory, PdgNode,
    data_origins::{self, Origin},
    producer::Pair,
};
use std::collections::BTreeSet;

pub(super) fn install(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    for node in &mut pair.pdg.nodes {
        if reference(node)? {
            let origins = source_origins(node, facts)?;
            node.annotations.insert(
                data_origins::KEY.into(),
                serde_json::to_value(origins).map_err(|error| error.to_string())?,
            );
        }
    }
    Ok(())
}

pub(super) fn verify(
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    for node in &pair.pdg.nodes {
        verify_type(node, facts)?;
        if !reference(node)? {
            if node.annotations.contains_key(data_origins::KEY) {
                return Err("Scalar operation claims reference origins".into());
            }
            continue;
        }
        let expected = source_origins(node, facts)?;
        let actual: Vec<Origin> = serde_json::from_value(
            node.annotations
                .get(data_origins::KEY)
                .ok_or("Missing serialized reference origins")?
                .clone(),
        )
        .map_err(|error| error.to_string())?;
        let graph = graph_origins(node, pair, facts, &mut BTreeSet::new())?;
        if actual != expected || graph != expected || expected.is_empty() {
            return Err(
                "Reference origins disagree with original source or serialized value dependencies"
                    .into(),
            );
        }
    }
    Ok(())
}

fn reference(node: &PdgNode) -> Result<bool, String> {
    if node.annotations.contains_key(crate::data_places::KEY)
        || node.attributes.construct.as_deref() == Some("data.indirect-place")
    {
        return Ok(false);
    }
    let token = wire_type(node)?;
    Ok(data_origins::shared(&crate::data_types::parse(&token)?))
}

fn source_origins(
    node: &PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    if node.category == NodeCategory::FormalInput {
        let position = node
            .attributes
            .position
            .ok_or("Missing reference input position")? as usize;
        let parameter = facts
            .signature
            .params
            .get(position)
            .ok_or("Unknown reference input")?;
        if node.annotations["compat.rust.hir-r0:type"]["value"].as_str()
            != parameter.ty.normalized.as_deref()
        {
            return Err("Serialized reference formal type disagrees with source".into());
        }
        return data_origins::parameter(
            facts
                .signature
                .params
                .get(position)
                .ok_or("Unknown reference input")?,
        );
    }

    let source = source_node(node, facts)?;
    let actual = node.annotations["compat.rust.hir-r0:type"]["value"].as_str();
    if actual != source.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) {
        return Err("Serialized reference value type disagrees with source".into());
    }
    data_origins::origins(source, facts)
}

fn find<'a>(node: &'a HirNode, kind: &str, span: &crate::Span, matches: &mut Vec<&'a HirNode>) {
    if (node.kind == kind
        || kind.is_empty()
            && matches!(node.kind.as_str(), "expr_if" | "expr_binary" | "expr_match"))
        && u64::from(node.span.start) == span.start
        && u64::from(node.span.end) == span.end
    {
        matches.push(node);
    }
    for child in &node.children {
        find(child, kind, span, matches);
    }
}

fn graph_origins(
    node: &PdgNode,
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
    visiting: &mut BTreeSet<String>,
) -> Result<Vec<Origin>, String> {
    Graph { pair, facts }.origins(node, &[], visiting)
}

struct Graph<'a> {
    pair: crate::producer::PairView<'a>,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
}

impl Graph<'_> {
    fn origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        if node.category == NodeCategory::Computation
            && node.original_kind.known_value() == Some("expr_addr_of")
        {
            return super::borrows::graph_origin(node, self.pair).map(|origin| vec![origin]);
        }
        if node.category == NodeCategory::FormalInput {
            if selectors.is_empty() {
                return source_origins(node, self.facts);
            }
            return crate::data_component_origins::selected(
                &crate::data_types::parse(&wire_type(node)?)?,
                selectors,
                u32::try_from(
                    node.attributes
                        .position
                        .ok_or("Missing selected formal position")?,
                )
                .map_err(|_| "Selected formal position is out of range")?,
                Vec::new(),
            );
        }
        if node.original_kind.known_value() == Some("expr_field")
            && node.category == NodeCategory::Read
        {
            return loaded_origins(node, selectors, self.pair);
        }

        let key = if selectors.is_empty() {
            node.id.clone()
        } else {
            format!("{}:{selectors:?}", node.id)
        };
        if !visiting.insert(key.clone()) {
            return if crate::data_reference_locals::flow_enabled(self.facts) {
                Ok(Vec::new())
            } else {
                Err("Cyclic reference value proof is unavailable".into())
            };
        }
        let roots = self.selected_inputs(node, selectors, visiting)?;
        if !crate::data_reference_locals::flow_enabled(self.facts) {
            visiting.remove(&key);
        }
        Ok(roots)
    }

    fn selected_inputs(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        if node.attributes.construct.as_deref() == Some("data.value-field") {
            let position = node
                .attributes
                .literal
                .as_ref()
                .and_then(|value| value.known_value())
                .ok_or("Missing selected graph component")?
                .parse::<usize>()
                .map_err(|error| error.to_string())?;
            let path: Vec<_> = [position]
                .into_iter()
                .chain(selectors.iter().copied())
                .collect();
            return self.origins(self.operand(node, 0)?, &path, visiting);
        }
        if !selectors.is_empty()
            && matches!(
                node.original_kind.known_value(),
                Some("expr_tup" | "expr_struct")
            )
        {
            let (operand, rest) = constructor_operand(node, selectors)?;
            return self.origins(self.operand(node, operand as u64)?, rest, visiting);
        }

        self.forwarded_inputs(node, selectors, visiting)
    }

    fn forwarded_inputs(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        if selectors.is_empty()
            && let Some(slots) = update_slots(node)?
        {
            return self.updated(node, &slots, visiting);
        }
        if !selectors.is_empty() && node.category == NodeCategory::Call {
            return Err("Selected helper aggregate result is unsupported".into());
        }
        self.input_origins(node, selectors, visiting)
    }

    fn updated(
        &self,
        node: &PdgNode,
        slots: &[Option<usize>],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        let crate::data_types::TypeToken::Struct { fields, .. } =
            crate::data_types::parse(&wire_type(node)?)?
        else {
            return Err("Serialized update requires a struct type".into());
        };
        let mut roots = BTreeSet::new();
        for (position, _) in fields
            .iter()
            .enumerate()
            .filter(|(_, (_, ty))| data_origins::shared(ty))
        {
            let selectors = [position];
            let (operand, remaining) = crate::data_component_origins::slot(slots, &selectors)?;
            roots.extend(self.origins(self.operand(node, operand as u64)?, remaining, visiting)?);
        }
        Ok(roots.into_iter().collect())
    }

    fn operand(&self, node: &PdgNode, position: u64) -> Result<&PdgNode, String> {
        let edges: Vec<_> = self
            .pair
            .pdg
            .edges
            .iter()
            .filter(|edge| {
                edge.target == node.id
                    && edge.kind == crate::EdgeFamily::Data
                    && edge.subkind == "value_operand"
                    && edge.operand_position == Some(position)
            })
            .collect();
        let [edge] = edges.as_slice() else {
            return Err("Selected component requires one exact operand".into());
        };
        self.pair
            .pdg
            .nodes
            .iter()
            .find(|value| value.id == edge.source)
            .ok_or("Missing selected component predecessor".into())
    }

    fn input_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        let selected = returned_operands(node, self.facts)?;
        let mut roots = BTreeSet::new();
        for edge in self.pair.pdg.edges.iter().filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && matches!(
                    edge.subkind.as_str(),
                    "value_operand" | "value_alternative" | "reaching_definition"
                )
                && selected.as_ref().is_none_or(|positions| {
                    edge.subkind == "value_operand"
                        && edge
                            .operand_position
                            .is_some_and(|position| positions.contains(&position))
                })
        }) {
            let predecessor = self
                .pair
                .pdg
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.source)
                .ok_or("Missing reference value predecessor")?;
            if reference(predecessor)?
                || matches!(
                    predecessor.category,
                    NodeCategory::Declaration | NodeCategory::Assignment
                )
            {
                roots.extend(self.origins(predecessor, selectors, visiting)?);
            }
        }
        Ok(roots.into_iter().collect())
    }
}

fn constructor_operand<'a>(
    node: &PdgNode,
    selectors: &'a [usize],
) -> Result<(usize, &'a [usize]), String> {
    let (&position, rest) = selectors
        .split_first()
        .ok_or("Missing graph component selector")?;
    if node.original_kind.known_value() == Some("expr_tup") {
        return Ok((position, rest));
    }
    let slots: Vec<Option<usize>> = serde_json::from_str(
        node.attributes
            .literal
            .as_ref()
            .and_then(|literal| literal.known_value())
            .ok_or("Missing serialized struct slots")?,
    )
    .map_err(|error| error.to_string())?;
    crate::data_component_origins::slot(&slots, selectors)
}

fn update_slots(node: &PdgNode) -> Result<Option<Vec<Option<usize>>>, String> {
    if node.attributes.construct.as_deref() != Some("data.struct") {
        return Ok(None);
    }
    let text = node
        .attributes
        .literal
        .as_ref()
        .and_then(|literal| literal.known_value())
        .ok_or("Missing serialized struct slots")?;
    // Descriptors are independently checked against canonical source-derived slots.
    if !text.contains("null") {
        return Ok(None);
    }
    serde_json::from_str(text)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn returned_operands(
    node: &PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Option<BTreeSet<u64>>, String> {
    if node.category != NodeCategory::Call {
        return Ok(None);
    }
    let source = source_node(node, facts)?;
    Ok(Some(
        crate::helper_references::parameters(source, facts)?
            .into_iter()
            .map(u64::from)
            .collect(),
    ))
}

fn loaded_origins(
    node: &PdgNode,
    selectors: &[usize],
    pair: crate::producer::PairView<'_>,
) -> Result<Vec<Origin>, String> {
    let mut roots = BTreeSet::new();
    for access in &node.attributes.reads {
        roots.extend(access_origins(access, node, selectors, pair)?);
    }
    Ok(roots.into_iter().collect())
}

fn access_origins(
    access: &crate::Access,
    node: &PdgNode,
    selectors: &[usize],
    pair: crate::producer::PairView<'_>,
) -> Result<Vec<Origin>, String> {
    let id = access
        .location_id
        .as_ref()
        .ok_or("Missing stored reference location")?;
    let location = pair
        .pdg
        .memory_locations
        .iter()
        .find(|location| &location.id == id)
        .ok_or("Unknown stored reference location")?;
    let evidence = location
        .extensions
        .get(super::LOCATION_KEY)
        .ok_or("Missing stored reference formal location")?;

    let origin: Origin = serde_json::from_value(serde_json::json!({
        "parameter": evidence["parameter"], "path": evidence["path"]
    }))
    .map_err(|error| error.to_string())?;
    let ty: crate::data_types::TypeToken =
        serde_json::from_value(evidence["type"].clone()).map_err(|error| error.to_string())?;
    if ty != crate::data_types::parse(&wire_type(node)?)? {
        return Err("Stored reference read/location value types disagree".into());
    }
    crate::data_component_origins::selected(&ty, selectors, origin.parameter, origin.path)
}

fn verify_type(node: &PdgNode, facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    let expected = match node.category {
        NodeCategory::FormalInput => {
            let position = node.attributes.position.ok_or("Missing input position")? as usize;
            facts
                .signature
                .params
                .get(position)
                .and_then(|parameter| parameter.ty.normalized.as_deref())
        }
        NodeCategory::Return => facts.signature.return_ty.normalized.as_deref(),
        NodeCategory::Declaration | NodeCategory::Assignment => Some("unit"),
        NodeCategory::Control
            if matches!(
                node.original_kind.known_value(),
                Some("expr_break" | "expr_continue")
            ) =>
        {
            Some("unit")
        }
        NodeCategory::Control => Some("bool"),
        NodeCategory::Computation if node.original_kind.known_value() == Some("expr_assign_op") => {
            source_node(node, facts)?
                .dispatch
                .as_ref()
                .and_then(|dispatch| dispatch.operand_tys.first().map(String::as_str))
        }
        _ => source_node(node, facts)?
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref()),
    };
    if expected.is_none() || wire_type(node)?.as_str() != expected.unwrap_or_default() {
        return Err(format!(
            "Serialized value type disagrees with independently checked source role in {} at {}: expected {expected:?}, actual {}",
            facts.name,
            node.id,
            wire_type(node)?
        ));
    }
    Ok(())
}

pub(super) fn source_node<'a>(
    node: &PdgNode,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
) -> Result<&'a HirNode, String> {
    let kind = if node.category == NodeCategory::Merge {
        ""
    } else {
        node.original_kind
            .known_value()
            .ok_or("Unknown data source kind")?
    };
    let span: crate::Span = if let Some(span) = &node.source_span {
        span.clone()
    } else {
        serde_json::from_value(
            node.attributes
                .extensions
                .get("compat.rust.hir-r0:source-operation")
                .and_then(|value| value.get("construct_span"))
                .ok_or("Missing source operation span")?
                .clone(),
        )
        .map_err(|error| error.to_string())?
    };

    let mut matches = Vec::new();
    find(&facts.body, kind, &span, &mut matches);
    let [source] = matches.as_slice() else {
        return Err("No unique source value operation".into());
    };
    Ok(source)
}

pub(super) fn wire_type(node: &PdgNode) -> Result<String, String> {
    let value = node
        .annotations
        .get("compat.rust.hir-r0:type")
        .ok_or("Missing serialized data value type")?;
    let fact: crate::StringFact =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    fact.known_value()
        .map(str::to_owned)
        .ok_or_else(|| "Unknown serialized data value type".into())
}
