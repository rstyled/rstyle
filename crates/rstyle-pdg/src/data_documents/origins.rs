//! Independently compare source origins with serialized value-dependence paths.

mod borrowed_results;

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
    if install_reader_origins(pair, facts)? || install_guarded_origins(pair, facts)? {
        return Ok(());
    }
    install_standard_origins(pair, facts)
}

fn install_guarded_origins(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    if super::named_error_payloads::install(pair, facts)? {
        return Ok(true);
    }
    if super::nonzero_digits::install(pair, facts)? {
        return Ok(true);
    }
    install_product_origins(pair, facts)
}

fn install_reader_origins(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    type Installer = fn(&mut Pair, &dyn crate::unit_store::EvidenceFacts) -> Result<bool, String>;
    let installers: [Installer; 7] = [
        super::widening_readers::install,
        super::id_reader_checks::install,
        super::id_readers::install,
        super::reader_products::install,
        super::wire_validators::install,
        super::slice_lengths::install,
        super::fallible_readers::install,
    ];

    for install in installers {
        if install(pair, facts)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn install_product_origins(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    if super::digit_calls::install(pair, facts)? {
        return Ok(true);
    }
    if crate::scalar_product_instances::activation::when_checked(facts, |checked| {
        crate::scalar_product_instances::origins::documents::install(pair, &checked, facts)
    })?
    .is_some()
    {
        return Ok(true);
    }
    if super::tagged_calls::install(pair, facts)? {
        return Ok(true);
    }
    Ok(false)
}

fn install_standard_origins(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if crate::copy_tagged_values::candidate(facts) {
        let resolution = facts.nominal_resolution()?;
        let receipt = resolution
            .copy_conversion_source(facts.facts())?
            .ok_or("Missing checked original conversion origins authority")?;
        return super::copy_tagged_values::origins::install(pair, facts, receipt);
    }
    install_reference_origins(pair, facts)
}

fn install_reference_origins(
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
    if verify_guarded_origins(pair, facts)? {
        return Ok(());
    }
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
        if actual != expected
            || graph != expected
            || expected.is_empty() && !borrowed_results::verified_empty(node, facts)?
        {
            return Err(
                "Reference origins disagree with original source or serialized value dependencies"
                    .into(),
            );
        }
    }

    Ok(())
}

fn verify_guarded_origins(
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    if super::widening_readers::verify(pair, facts)?
        || super::id_reader_checks::verify(pair, facts)?
        || super::id_readers::verify(pair, facts)?
        || super::reader_products::verify(pair, facts)?
        || super::wire_validators::verify(pair, facts)?
    {
        return Ok(true);
    }
    super::slice_lengths::verify(pair, facts)?;
    super::fallible_readers::verify(pair, facts)?;
    if crate::helper_calls::slice_lengths::candidate(facts)
        || super::fallible_readers::verify_source(facts)?
    {
        return Ok(true);
    }
    if super::named_error_payloads::verify_source(facts)? {
        super::named_error_payloads::verify(pair, facts)?;
        return Ok(true);
    }
    super::named_error_payloads::verify(pair, facts)?;
    if super::nonzero_digits::verify(pair, facts)? {
        return Ok(true);
    }
    if super::digit_calls::verify(pair, facts)? {
        return Ok(true);
    }
    if crate::scalar_product_instances::activation::when_checked(facts, |checked| {
        crate::scalar_product_instances::origins::documents::verify(pair, &checked, facts)
    })?
    .is_some()
    {
        return Ok(true);
    }
    if super::tagged_calls::verify(pair, facts)? {
        return Ok(true);
    }

    if crate::copy_tagged_values::candidate(facts) {
        let resolution = facts.nominal_resolution()?;
        let receipt = resolution
            .copy_conversion_source(facts.facts())?
            .ok_or("Missing checked original conversion origins authority")?;
        return super::copy_tagged_values::origins::verify(pair, facts, receipt).map(|()| true);
    }
    Ok(false)
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
    if node.attributes.construct.as_deref() == Some(crate::owned_product_values::UPDATE) {
        return crate::owned_product_values::source(source, &[], facts)?
            .ok_or_else(|| "Owned product graph transition has no checked source value".into());
    }
    if borrowed_results::verified_empty(node, facts)? {
        return Ok(Vec::new());
    }
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

pub(crate) fn selected_return(
    node: &PdgNode,
    selectors: &[usize],
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    if node.category != NodeCategory::Return {
        return Err("Component summary requires an exact RETURN".into());
    }
    let source = source_node(node, facts)?;
    let mut expected = crate::data_component_origins::source(source, selectors, facts)?;
    let mut actual = Graph { pair, facts }.origins(node, selectors, &mut BTreeSet::new())?;

    expected.sort();
    expected.dedup();
    actual.sort();
    actual.dedup();

    if expected.is_empty() || actual != expected {
        return Err(format!(
            "Selected RETURN origins disagree in {} at {:?}: source={expected:?}, graph={actual:?}",
            facts.name, selectors,
        ));
    }

    Ok(expected)
}

pub(crate) fn selected_product_return(
    node: &PdgNode,
    selectors: &[usize],
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    crate::concrete_results::fresh_products::product_boundary(facts)?;
    if node.category != NodeCategory::Return {
        return Err("Product component summary requires an exact RETURN".into());
    }
    if borrowed_results::verified_empty(node, facts)? {
        let actual = Graph { pair, facts }.origins(node, &[], &mut BTreeSet::new())?;
        if !actual.is_empty() {
            return Err("Proved product Err return unexpectedly carries graph origins".into());
        }
        return Ok(actual);
    }
    selected_return(node, selectors, pair, facts)
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
        if let Some(origins) = self.value_origins(node, selectors, visiting) {
            return origins;
        }
        if node.category == NodeCategory::FormalInput {
            return self.formal(node, selectors);
        }

        if (node.original_kind.known_value() == Some("expr_field")
            || node.attributes.construct.as_deref() == Some("data.indirect-deref"))
            && node.category == NodeCategory::Read
        {
            return loaded_origins(node, selectors, self.pair);
        }

        self.traversed_origins(node, selectors, visiting)
    }

    fn value_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Option<Result<Vec<Origin>, String>> {
        if node.attributes.construct.as_deref() == Some("data.owned-product-update") {
            return Some(self.owned_update_origins(node, selectors, visiting));
        }
        if let Some(origins) = self.address_origins(node, selectors, visiting) {
            return Some(origins);
        }
        self.call_origins(node, selectors, visiting)
    }

    fn owned_update_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        if !visiting.insert(node.id.clone()) {
            return Err("Cyclic owned product update lacks linear value evidence".into());
        }

        let roots = self.origins(self.operand(node, 0)?, selectors, visiting);
        visiting.remove(&node.id);
        roots
    }

    fn call_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Option<Result<Vec<Origin>, String>> {
        let construct = node.attributes.construct.as_deref();
        if matches!(
            construct,
            Some(
                "data.option-get-prefix"
                    | "data.option-get-suffix"
                    | "data.option-ok-or"
                    | "data.result-ok"
                    | "data.result-err"
                    | "try.borrowed-result-ok-reference"
                    | "try.borrowed-result-error"
                    | "try.scalar-result-error"
                    | "try.product-result-error"
                    | "try.product-result-ok-product"
            )
        ) {
            return Some(self.borrowed_result_operation(node, selectors, visiting));
        }
        if let Some(origins) = self.helper_call_origins(node, selectors, visiting) {
            return Some(origins);
        }
        match construct {
            Some("data.bytes.borrow-result" | "data.bytes.unwrap-borrow" | "data.option-get") => {
                Some(self.byte_call_operation(node, selectors, visiting))
            }
            _ => None,
        }
    }

    fn helper_call_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Option<Result<Vec<Origin>, String>> {
        if node.category == NodeCategory::Call {
            let source = match source_node(node, self.facts) {
                Ok(source) => source,
                Err(error) => return Some(Err(error)),
            };
            if source
                .dispatch
                .as_ref()
                .is_some_and(|dispatch| dispatch.op == "core::convert::AsRef::as_ref")
                && self.facts.reference_instance().is_some()
            {
                return Some(self.reference_instance_origins(node, selectors, visiting));
            }
            if crate::helper_calls::selected_resolution(source, self.facts).is_some() {
                if crate::helper_component_outputs::call_used(source, self.facts) {
                    return Some(crate::helper_component_outputs::map(
                        source,
                        selectors,
                        self.facts,
                        |position| {
                            self.origins(self.operand(node, u64::from(position))?, &[], visiting)
                        },
                    ));
                }
                if !selectors.is_empty() {
                    return Some(Err("Selected helper result lacks component evidence".into()));
                }
                return Some(crate::helper_references::map_outputs(
                    source,
                    self.facts,
                    |position| {
                        self.origins(self.operand(node, u64::from(position))?, &[], visiting)
                    },
                ));
            }
        }
        None
    }

    fn reference_instance_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        let source = source_node(node, self.facts)?;
        let operation = crate::reference_instances::activation::operation(source, self.facts)?
            .ok_or("Unproved AsRef site in checked constructor graph")?;
        if !selectors.is_empty() {
            return Err("Checked byte AsRef has no aggregate selector".into());
        }

        let roots = self.origins(self.operand(node, 0)?, &[], visiting)?;
        operation.project(roots)
    }

    fn byte_call_operation(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        let source = source_node(node, self.facts)?;
        let roots = self.origins(self.operand(node, 0)?, &[], visiting)?;

        if node.attributes.construct.as_deref() == Some("data.option-get") {
            crate::checked_options::verify_get(source, self.facts)?;
            return Ok(crate::checked_options::project(roots));
        }
        if !selectors.is_empty() {
            return Err("Byte payload has no aggregate selector".into());
        }
        let kind = crate::data_bytes::verify(source, self.facts)?;
        Ok(crate::data_bytes::project(kind, roots))
    }

    fn traversed_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
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

    fn address_origins(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Option<Result<Vec<Origin>, String>> {
        if node.category == NodeCategory::Computation
            && node.original_kind.known_value() == Some("expr_addr_of")
        {
            let source = match source_node(node, self.facts) {
                Ok(source) => source,
                Err(error) => return Some(Err(error)),
            };
            if crate::data_subslices::borrowed(source, self.facts) {
                return Some(
                    self.operand(node, 0)
                        .and_then(|base| self.origins(base, selectors, visiting)),
                );
            }
            return Some(super::borrows::graph_origin(node, self.pair).map(|origin| vec![origin]));
        }
        if node.attributes.construct.as_deref() == Some("data.subslice-place") {
            let checked = match source_node(node, self.facts)
                .and_then(|source| crate::data_subslices::verify(source, self.facts))
            {
                Ok(checked) => checked,
                Err(error) => return Some(Err(error)),
            };
            if checked.array_length.is_some() {
                return Some(self.operand(node, 0).and_then(|place| {
                    super::borrows::place_origin(place, self.pair).map(|origin| vec![origin])
                }));
            }
            return Some(
                self.operand(node, 0)
                    .and_then(|base| self.origins(base, selectors, visiting)),
            );
        }
        None
    }

    fn formal(&self, node: &PdgNode, selectors: &[usize]) -> Result<Vec<Origin>, String> {
        if selectors.is_empty() {
            return source_origins(node, self.facts);
        }
        crate::data_component_origins::selected(
            &crate::data_types::parse(&wire_type(node)?)?,
            selectors,
            u32::try_from(
                node.attributes
                    .position
                    .ok_or("Missing selected formal position")?,
            )
            .map_err(|_| "Selected formal position is out of range")?,
            Vec::new(),
        )
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
            && node.category == NodeCategory::Computation
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

pub(crate) fn verify_type(
    node: &PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if let Some(result) = super::widening_readers::verify_type(node, facts) {
        return result;
    }
    if let Some(result) = super::id_error_maps::verify_type(node, facts) {
        return result;
    }
    if let Some(result) = super::result_maps::verify_type(node, facts) {
        return result;
    }
    if node.attributes.construct.as_deref() == Some("data.owned-product-update") {
        let boundary = crate::concrete_results::fresh_products::product_boundary(facts)?;
        if wire_type(node)? != boundary.ok.text()? {
            return Err("Owned product transition loses its exact complete product type".into());
        }
        return Ok(());
    }

    let expected = source_type(node, facts)?;
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

fn source_type<'a>(
    node: &PdgNode,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
) -> Result<Option<&'a str>, String> {
    if propagation_return(node, facts)? {
        return Ok(facts.signature.return_ty.normalized.as_deref());
    }
    if let Some(ty) = event_type(node) {
        return Ok(Some(ty));
    }

    if mutable_bounds_check(node, facts)? {
        return Ok(Some("unit"));
    }

    if callback_environment(node, facts)? {
        return Ok(Some("unit"));
    }

    Ok(match node.category {
        NodeCategory::FormalInput => input_type(node, facts)?,
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
        NodeCategory::Computation
            if node.attributes.construct.as_deref() == Some("data.copy-discard") =>
        {
            crate::data_discards::verify(source_node(node, facts)?, facts)?;
            Some("unit")
        }
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
    })
}

fn mutable_bounds_check(
    node: &PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    if node.attributes.construct.as_deref() != Some("data.mutable-index-check") {
        return Ok(false);
    }
    if !crate::mutable_scalars::enabled(facts) || node.category != NodeCategory::Computation {
        return Err("Mutable bounds check requires its checked memory family".into());
    }
    crate::mutable_scalars::index_root(source_node(node, facts)?, facts)?;
    Ok(true)
}

fn propagation_return(
    node: &PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    match node.attributes.construct.as_deref() {
        Some("try.product-result-error") => {
            crate::product_result_propagation::verify(source_node(node, facts)?, facts)?;
        }
        Some("try.reader-result-error") => {
            crate::named_error_result_propagation::verify(source_node(node, facts)?, facts)?;
        }
        Some("try.scalar-result-error") => {
            crate::scalar_result_propagation::verify(source_node(node, facts)?, facts)?;
        }
        Some("try.borrowed-result-error") => {
            crate::borrowed_result_propagation::verify(source_node(node, facts)?, facts)?;
        }
        Some("try.option-none") => {
            crate::option_propagation::verify(source_node(node, facts)?, facts)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

fn input_type<'a>(
    node: &PdgNode,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
) -> Result<Option<&'a str>, String> {
    let position = node.attributes.position.ok_or("Missing input position")? as usize;
    Ok(facts
        .signature
        .params
        .get(position)
        .and_then(|parameter| parameter.ty.normalized.as_deref()))
}

fn event_type(node: &PdgNode) -> Option<&'static str> {
    match node.category {
        NodeCategory::Computation
            if node.attributes.construct.as_deref() == Some(crate::named_callbacks::ITEM) =>
        {
            Some("unit")
        }
        NodeCategory::Computation
            if node.attributes.construct.as_deref() == Some(crate::named_callbacks::PAYLOAD) =>
        {
            Some("u8")
        }
        NodeCategory::Call
            if node.attributes.construct.as_deref() == Some(crate::named_callbacks::INVOKE) =>
        {
            Some("bool")
        }
        NodeCategory::Read
            if node.attributes.construct.as_deref() == Some(crate::iterator_position::PREFIX) =>
        {
            Some(crate::iterator_position::RESULT_TOKEN)
        }
        NodeCategory::Computation
            if matches!(
                node.attributes.construct.as_deref(),
                Some("iterator.fresh-shared-bytes" | "iterator.shared-byte-environment")
            ) =>
        {
            Some("unit")
        }
        NodeCategory::Computation
            if node.attributes.construct.as_deref() == Some("iterator.empty-position") =>
        {
            Some(crate::iterator_position::RESULT_TOKEN)
        }
        _ => None,
    }
}

fn source_kind(node: &PdgNode) -> Result<&str, String> {
    Ok(if node.category == NodeCategory::Merge {
        if super::aggregates::method_result(node) {
            "expr_method_call"
        } else {
            ""
        }
    } else {
        let kind = node
            .original_kind
            .known_value()
            .ok_or("Unknown data source kind")?;
        if kind == "rstyle.iterator-prefix" {
            "expr_method_call"
        } else {
            kind
        }
    })
}

pub(super) fn source_node<'a>(
    node: &PdgNode,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
) -> Result<&'a HirNode, String> {
    if let Some(source) = super::widening_readers::source_node(node, facts) {
        return source;
    }
    if let Some(source) = super::id_error_maps::source_node(node, facts) {
        return source;
    }
    if let Some(source) = super::result_maps::source_node(node, facts) {
        return source;
    }
    let kind = source_kind(node)?;
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
        return Err(format!(
            "No unique source value operation for {kind} at {}..{}",
            span.start, span.end
        ));
    };
    Ok(source)
}

pub(crate) fn wire_type(node: &PdgNode) -> Result<String, String> {
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

fn callback_environment(
    node: &PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    if node.category != NodeCategory::Computation {
        return Ok(false);
    }
    match node.attributes.construct.as_deref() {
        Some("callback.shared-byte-environment") => {
            crate::option_callbacks::verify_environment_event(source_node(node, facts)?, facts)?;
        }
        Some(crate::helper_option_callbacks::ENVIRONMENT) => {
            crate::helper_option_callbacks::verify_environment_event(
                source_node(node, facts)?,
                facts,
            )?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
