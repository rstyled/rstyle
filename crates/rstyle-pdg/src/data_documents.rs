//! Data location transport and independent serialized semantic checks.

mod origins;

use crate::{HirFnFacts, NodeCategory, field_locations::*, producer::Pair};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub const LOCATION_KEY: &str = "rstyle.data:formal-location";
pub const POLICY_KEY: &str = "rstyle.data:matching-profile";

pub fn apply(pair: Pair, facts: &HirFnFacts) -> Result<Pair, String> {
    let mut pair = install_locations(pair, facts)?;
    install_policy(&mut pair);
    install_evidence(&mut pair.pdg, facts)?;
    origins::install(&mut pair, facts)?;

    Ok(pair)
}

fn install_locations(mut pair: Pair, facts: &HirFnFacts) -> Result<Pair, String> {
    let fields = observations(facts)?;
    let locations = fields
        .operations
        .iter()
        .map(|field| install_field(&mut pair.pdg, field, facts))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    pair.pdg.memory_locations = locations.into_values().collect();
    pair.pdg.function.reference_count.value = Some(read_count(&pair)?);

    Ok(pair)
}

fn install_field(
    pdg: &mut crate::PdgDocument,
    field: &FieldOperation,
    facts: &HirFnFacts,
) -> Result<(String, crate::MemoryLocation), String> {
    let (id, location) = location(field, facts)?;
    let node = pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.category == NodeCategory::Read
                && node.original_kind.known_value() == Some("expr_field")
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(field.span.start)
                        && span.end == u64::from(field.span.end)
                })
        })
        .ok_or("Missing lowered field read")?;
    node.attributes.reads[0].location_id = Some(id.clone());

    Ok((id, location))
}

fn install_policy(pair: &mut Pair) {
    let policy = json!(crate::extraction_profile::DATA_MATCHING_ID);
    pair.pdg
        .extensions
        .insert(POLICY_KEY.into(), policy.clone());
    pair.cfg.extensions.insert(POLICY_KEY.into(), policy);
}

fn install_evidence(pdg: &mut crate::PdgDocument, facts: &HirFnFacts) -> Result<(), String> {
    let aliases = json!(crate::data_origins::aliases(facts)?);
    let capability = memory_evidence();
    pdg.extensions
        .insert("rstyle.data:possible-formal-aliases".into(), aliases);
    pdg.extraction_evidence
        .capabilities
        .insert("memory_aliases".into(), capability);

    Ok(())
}

fn memory_evidence() -> crate::Capability {
    crate::Capability {
        policy: Some(crate::extraction_profile::ExtractionProfile::DataV1.policy()),
        status: crate::CapabilityStatus::Supported,
        description: "Verified formal read locations; shared inputs may alias; no disjointness or memory writes".into(),
    }
}

pub fn read_keys(node: &crate::PdgNode) -> Vec<String> {
    node.attributes
        .reads
        .iter()
        .filter_map(|access| {
            access
                .location_id
                .as_ref()
                .map(|id| format!("location:{id}"))
                .or_else(|| access.symbol_id.as_ref().map(|id| format!("binding:{id}")))
        })
        .collect()
}

fn observations(facts: &HirFnFacts) -> Result<FieldObservations, String> {
    crate::data_frontend::evidence(facts, KEY)
}

#[derive(serde::Serialize)]
struct LocationEvidence<'a> {
    parameter: u32,
    root_symbol: &'a str,
    path: &'a [Projection],
    #[serde(rename = "type")]
    value: crate::data_types::TypeToken,
}

fn location(
    field: &FieldOperation,
    facts: &HirFnFacts,
) -> Result<(String, crate::MemoryLocation), String> {
    let Location::Captured {
        parameter,
        path,
        value,
    } = &field.location
    else {
        return Err("Missing field origin".into());
    };

    let root = &facts.signature.params[*parameter as usize].symbol;
    let identity = json!([root, path]);
    let bytes = serde_json::to_vec(&identity).map_err(|error| error.to_string())?;
    let id = format!("location:{}", crate::concrete_types::digest(&bytes));
    Ok((
        id.clone(),
        crate::MemoryLocation {
            id,
            abstraction: "rstyle.formal-field/1".into(),
            description: "Read-only formal projection; shared roots may alias".into(),
            evidence: crate::Capability {
                policy: Some(crate::extraction_profile::ExtractionProfile::DataV1.policy()),
                status: crate::CapabilityStatus::Supported,
                description: "Independent source/resolved field and immutable concrete type proof"
                    .into(),
            },
            extensions: BTreeMap::from([(
                LOCATION_KEY.into(),
                serde_json::to_value(LocationEvidence {
                    parameter: *parameter,
                    root_symbol: root,
                    path,
                    value: crate::data_types::TypeToken::from_tree(value),
                })
                .map_err(|error| error.to_string())?,
            )]),
        },
    ))
}

/// Check semantic locations directly against independently verified source
/// operations, without calling the lowerer or document emitter.
pub fn verify(pair: &Pair, facts: &HirFnFacts) -> Result<(), String> {
    crate::data_frontend::verify(facts)?;
    verify_interface(pair, facts)?;
    if pair.pdg.extensions.get(POLICY_KEY)
        != Some(&json!(crate::extraction_profile::DATA_MATCHING_ID))
        || pair.cfg.extensions.get(POLICY_KEY) != pair.pdg.extensions.get(POLICY_KEY)
    {
        return Err("Data matching registration disagrees".into());
    }

    let fields = observations(facts)?;
    let mut expected = BTreeMap::new();
    for field in &fields.operations {
        verify_field(pair, facts, field, &mut expected)?;
    }

    let actual: BTreeMap<_, _> = pair
        .pdg
        .memory_locations
        .iter()
        .map(|location| (location.id.clone(), location.clone()))
        .collect();
    if actual != expected || actual.len() != pair.pdg.memory_locations.len() {
        return Err("Serialized data location inventory disagrees with source".into());
    }
    if pair
        .pdg
        .extensions
        .get("rstyle.data:possible-formal-aliases")
        != Some(&json!(crate::data_origins::aliases(facts)?))
    {
        return Err("Serialized shared-input alias uncertainty disagrees".into());
    }

    verify_reads(pair, fields.operations.len())?;
    origins::verify(pair, facts)
}

fn verify_field(
    pair: &Pair,
    facts: &HirFnFacts,
    field: &FieldOperation,
    expected: &mut BTreeMap<String, crate::MemoryLocation>,
) -> Result<(), String> {
    let candidates: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == NodeCategory::Read
                && node.original_kind.known_value() == Some("expr_field")
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(field.span.start)
                        && span.end == u64::from(field.span.end)
                })
        })
        .collect();
    let [node] = candidates.as_slice() else {
        return Err("Ambiguous or missing serialized field correspondence".into());
    };

    let (id, location) = location(field, facts)?;
    if node.category != NodeCategory::Read
        || node.attributes.reads.len() != 1
        || node.attributes.reads[0].location_id.as_ref() != Some(&id)
        || !node.attributes.writes.is_empty()
    {
        return Err("Serialized field read has incorrect location/effects".into());
    }

    verify_field_operands(pair, facts, field, node)?;
    expected.insert(id, location);
    Ok(())
}

fn verify_reads(pair: &Pair, fields: usize) -> Result<(), String> {
    if pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == NodeCategory::Read
                && node.original_kind.known_value() == Some("expr_field")
        })
        .count()
        != fields
        || pair.pdg.nodes.iter().any(|node| {
            node.attributes
                .writes
                .iter()
                .any(|access| access.location_id.is_some())
        })
    {
        return Err("Data operation inventory or read-only effects disagree".into());
    }
    if pair.pdg.function.reference_count.value != Some(read_count(pair)?) {
        return Err("Data reference count disagrees with verified read locations".into());
    }
    Ok(())
}

fn read_count(pair: &Pair) -> Result<u64, String> {
    let reachable = crate::reachable_vertices(&pair.cfg);
    let map = crate::read_operation_map(&pair.pdg.extensions)?;
    let nodes: BTreeSet<_> = map
        .iter()
        .filter(|entry| reachable.contains(&entry.cfg_node_id))
        .map(|entry| &entry.pdg_node_id)
        .collect();

    let keys: BTreeSet<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Read && nodes.contains(&node.id))
        .flat_map(read_keys)
        .collect();
    Ok(keys.len() as u64)
}

fn verify_field_operands(
    pair: &Pair,
    facts: &HirFnFacts,
    field: &FieldOperation,
    node: &crate::PdgNode,
) -> Result<(), String> {
    verify_field_root(facts, field, node)?;
    let base = field_base(pair, facts, node)?;

    verify_field_value_edge(pair, node, base)
}

fn verify_field_root(
    facts: &HirFnFacts,
    field: &FieldOperation,
    node: &crate::PdgNode,
) -> Result<(), String> {
    let Location::Captured { parameter, .. } = field.location else {
        return Err("Missing formal field origin".into());
    };
    let root = &facts.signature.params[parameter as usize].symbol;
    if node.attributes.reads[0].symbol_id.as_ref() != Some(root)
        || node.attributes.symbol_ids != [root.clone()]
        || node.attributes.construct.as_deref() != Some("data.field")
        || node.attributes.reads[0].unresolved_reason.is_some()
        || node.attributes.reads[0].operand_position != Some(0)
        || !node.attributes.defines.is_empty()
    {
        return Err("Serialized field read has incorrect root/value interpretation".into());
    }
    Ok(())
}

fn field_base<'a>(
    pair: &'a Pair,
    facts: &HirFnFacts,
    node: &crate::PdgNode,
) -> Result<&'a crate::PdgNode, String> {
    let source = origins::source_node(node, facts)?;
    let [base] = source.children.as_slice() else {
        return Err("Field source lacks exact base".into());
    };

    let candidates: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|candidate| {
            candidate.category == NodeCategory::Read
                && candidate.original_kind.known_value() == Some(base.kind.as_str())
                && candidate.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(base.span.start) && span.end == u64::from(base.span.end)
                })
        })
        .collect();
    let [base] = candidates.as_slice() else {
        return Err("Field base has no unique serialized operation".into());
    };
    Ok(base)
}

fn verify_field_value_edge(
    pair: &Pair,
    node: &crate::PdgNode,
    base: &crate::PdgNode,
) -> Result<(), String> {
    let operands: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .collect();
    if operands.len() != 1
        || operands[0].source != base.id
        || operands[0].operand_position != Some(0)
    {
        return Err("Serialized field read loses single base evaluation/dependence".into());
    }
    Ok(())
}

fn verify_interface(pair: &Pair, facts: &HirFnFacts) -> Result<(), String> {
    if pair.pdg.function.inputs.len() != facts.signature.params.len() {
        return Err("Serialized data input arity disagrees with source".into());
    }
    for (input, parameter) in pair.pdg.function.inputs.iter().zip(&facts.signature.params) {
        let role = if facts.signature.has_self && parameter.position == 0 {
            crate::ParamRole::Receiver
        } else {
            crate::ParamRole::Parameter
        };
        verify_parameter_type(input, &parameter.ty)?;
        if input.role != role
            || input.position != u64::from(parameter.position)
            || input.symbol_id.as_deref() != Some(&parameter.symbol)
        {
            return Err(
                "Serialized data input identity, role or type disagrees with source".into(),
            );
        }
    }

    let [output] = pair.pdg.function.outputs.as_slice() else {
        return Err("Serialized data output arity disagrees with source".into());
    };
    if output.role != crate::ParamRole::Return || output.position != 0 || output.symbol_id.is_some()
    {
        return Err("Serialized data output identity or role disagrees with source".into());
    }
    verify_parameter_type(output, &facts.signature.return_ty)
}

fn verify_parameter_type(
    parameter: &crate::Parameter,
    ty: &crate::ResolvedTy,
) -> Result<(), String> {
    let token: crate::StringFact = serde_json::from_value(
        parameter
            .extensions
            .get("compat.rust.hir-r0:resolved-type")
            .ok_or("Missing serialized data interface type")?
            .clone(),
    )
    .map_err(|e| e.to_string())?;
    if token.known_value() != ty.normalized.as_deref()
        || parameter.type_fact.known_value() != Some(&ty.spelling)
    {
        return Err("Serialized data interface type disagrees with source".into());
    }
    Ok(())
}
