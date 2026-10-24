//! Data location transport and independent serialized semantic checks.

mod aggregates;
mod assertions;
mod bitset_cardinality;
mod bool_constants;
mod bool_usize;
mod borrowed_result_errors;
pub(crate) mod borrowed_result_propagation;
mod borrows;
mod byte_calls;
mod byte_scalars;
mod callbacks;
mod character_ascii;
mod checked_instances;
mod checked_lengths;
mod checked_options;
mod checked_slice_views;
mod checked_suffix_views;
mod conditional_payloads;
pub(crate) mod copy_tagged_values;
mod fieldless_enums;
mod float_scalars;
mod helper_option_callbacks;
mod index_reads;
mod indirect_reads;
mod iterator_position;
mod locals;
mod locations;
mod memory_flow;
mod mutable_scalars;
pub(crate) mod named_callbacks;
mod option_if_let;
pub(crate) mod option_propagation;
pub(crate) mod origins;
mod owned_product_values;
mod product_instances;
pub(crate) mod product_result_propagation;
mod propagation;
mod read_accesses;
mod reference_instances;
mod reference_products;
pub(crate) mod result_maps;
pub(crate) mod result_propagation;
mod scalar_enums;
mod scalar_from;
pub(crate) mod scalar_result_propagation;
mod size_calls;
mod subslice_views;
mod summary_records;
mod value_consumers;
mod value_fields;
pub(crate) use locations::{LocationContext, correspondence as location_correspondence};

use crate::{
    NodeCategory, PdgNode,
    field_locations::*,
    producer::{Pair, PairView},
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub const LOCATION_KEY: &str = "rstyle.data:formal-location";
pub const POLICY_KEY: &str = "rstyle.data:matching-profile";

pub fn apply(
    pair: Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &crate::producer::Graph,
) -> Result<Pair, String> {
    let pair = install_locations(pair, facts, graph)?;
    let mut pair = install_value_contracts(pair, facts)?;
    install_evidence(&mut pair.pdg, facts)?;
    origins::install(&mut pair, facts)?;

    let pair = install_annotations(
        pair,
        size_calls::sources(facts),
        crate::size_calls::KEY,
        |source| crate::size_calls::semantics(source, facts),
    )?;
    let pair = install_annotations(
        pair,
        checked_lengths::sources(facts),
        crate::checked_lengths::KEY,
        |source| checked_lengths::semantics(source, facts),
    )?;
    let pair = install_annotations(
        pair,
        bool_usize::sources(facts),
        crate::bool_usize::KEY,
        |source| crate::bool_usize::semantics(source, facts),
    )?;
    install_annotations(
        pair,
        float_scalars::sources(facts),
        crate::float_scalars::KEY,
        |source| crate::float_scalars::semantics(source, facts),
    )
}

fn install_value_contracts(
    mut pair: Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Pair, String> {
    install_policy(&mut pair, facts)?;
    crate::concrete_results::documents::install(&mut pair);
    install_propagation_contracts(&mut pair);

    install_borrowed_contracts(pair, facts)
}

fn install_borrowed_contracts(
    mut pair: Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Pair, String> {
    install_result_contracts(&mut pair, facts)?;
    checked_slice_views::install(&mut pair, facts)?;
    checked_suffix_views::install(&mut pair, facts)?;

    Ok(pair)
}

fn install_result_contracts(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    type Installer = fn(&mut Pair, &dyn crate::unit_store::EvidenceFacts) -> Result<(), String>;
    let installers: [Installer; 5] = [
        borrowed_result_propagation::install,
        scalar_result_propagation::install,
        product_result_propagation::install,
        result_maps::install,
        borrowed_result_errors::install,
    ];

    for install in installers {
        install(pair, facts)?;
    }
    Ok(())
}

fn install_propagation_contracts(pair: &mut Pair) {
    result_propagation::install(pair);
    option_propagation::install(pair);
}

fn install_annotations<'a>(
    mut pair: Pair,
    sources: impl IntoIterator<Item = &'a crate::HirNode>,
    key: &str,
    semantics: impl Fn(&crate::HirNode) -> Result<serde_json::Value, String>,
) -> Result<Pair, String> {
    for source in sources {
        let node = serialized_operation_mut(&mut pair, source)?;
        node.annotations.insert(key.into(), semantics(source)?);
    }
    Ok(pair)
}

fn install_locations(
    mut pair: Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &crate::producer::Graph,
) -> Result<Pair, String> {
    let fields = observations(facts)?;
    let mut context = LocationContext::new(facts);
    let locations = fields
        .operations
        .iter()
        .filter(|field| matches!(field.location, Location::Captured { .. }))
        .map(|field| install_field(&mut pair.pdg, field, &mut context))
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let derived = install_derived_reads(&mut pair, &mut context)?;
    let memory = install_memory(&mut pair, &mut context, graph)?;
    finish_locations(
        pair,
        crate::helper_reads::merge_locations(
            crate::helper_reads::merge_locations(locations, derived)?,
            memory,
        )?,
    )
}

fn install_memory(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
    graph: &crate::producer::Graph,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let accesses = mutable_scalars::install(pair, context)?;
    let roots = memory_flow::install(pair, context, graph)?;

    crate::helper_reads::merge_locations(accesses, roots)
}

fn verify_memory(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    verify_location_passes(
        pair,
        context,
        &[mutable_scalars::verify, memory_flow::verify],
    )
}

type LocationVerifier = fn(
    PairView<'_>,
    &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String>;

fn verify_location_passes(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    passes: &[LocationVerifier],
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    passes
        .iter()
        .try_fold(BTreeMap::new(), |locations, verify| {
            crate::helper_reads::merge_locations(locations, verify(pair, context)?)
        })
}

type ReadInstaller = fn(
    &mut Pair,
    &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String>;

fn install_derived_reads(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let installers: [ReadInstaller; 6] = [
        indirect_reads::install,
        index_reads::install,
        crate::helper_reads::install,
        byte_calls::install,
        checked_options::install,
        iterator_position::install,
    ];

    installers
        .into_iter()
        .try_fold(BTreeMap::new(), |locations, install| {
            crate::helper_reads::merge_locations(locations, install(pair, context)?)
        })
}

fn finish_locations(
    mut pair: Pair,
    locations: BTreeMap<String, crate::MemoryLocation>,
) -> Result<Pair, String> {
    pair.pdg.memory_locations = locations.into_values().collect();
    pair.pdg.function.reference_count.value = Some(read_count(pair.as_view())?);

    Ok(pair)
}

fn install_field(
    pdg: &mut crate::PdgDocument,
    field: &FieldOperation,
    context: &mut LocationContext<'_>,
) -> Result<(String, crate::MemoryLocation), String> {
    let (id, location) = location(field, context)?;
    let node = pdg
        .nodes
        .iter_mut()
        .find(|node| {
            matches!(
                node.category,
                NodeCategory::Read | NodeCategory::Computation
            ) && node.original_kind.known_value() == Some("expr_field")
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(field.span.start)
                        && span.end == u64::from(field.span.end)
                })
        })
        .ok_or("Missing lowered field read")?;
    if node.category == NodeCategory::Computation {
        node.annotations
            .insert(crate::data_places::KEY.into(), json!(id));
    } else {
        node.attributes.reads[0].location_id = Some(id.clone());
    }

    Ok((id, location))
}

fn install_policy(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let policy = json!(matching_identity(facts)?);
    pair.pdg
        .extensions
        .insert(POLICY_KEY.into(), policy.clone());
    pair.cfg.extensions.insert(POLICY_KEY.into(), policy);
    Ok(())
}

fn matching_identity(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<&'static str, String> {
    crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .data_matching()
        .ok_or_else(|| "Missing registered Data matching identity".into())
}

fn install_evidence(
    pdg: &mut crate::PdgDocument,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let aliases = json!(crate::data_origins::aliases(facts)?);
    let capability = memory_evidence(facts)?;
    pdg.extensions
        .insert("rstyle.data:possible-formal-aliases".into(), aliases);
    pdg.extraction_evidence
        .capabilities
        .insert("memory_aliases".into(), capability);

    Ok(())
}

fn memory_evidence(
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<crate::Capability, String> {
    Ok(crate::Capability {
        policy: Some(crate::extraction_profile::ExtractionProfile::from_facts(facts)?.policy()),
        status: crate::CapabilityStatus::Supported,
        description: if crate::mutable_scalars::enabled(facts) {
            "Verified formal read/write locations; conservative reference aliases and explicit normal/failure post-state".into()
        } else {
            "Verified formal read locations; shared inputs may alias; no disjointness or memory writes".into()
        },
    })
}

pub fn read_keys(node: &crate::PdgNode, locations: &[crate::MemoryLocation]) -> Vec<String> {
    node.attributes
        .reads
        .iter()
        .filter_map(|access| {
            access
                .location_id
                .as_ref()
                .map(|id| {
                    if matches!(
                        node.attributes.construct.as_deref(),
                        Some(
                            "data.index"
                                | "data.bytes.copy-result"
                                | crate::iterator_position::PREFIX
                        )
                    ) || node.attributes.construct.as_deref() == Some("data.indirect-deref")
                        && locations.iter().any(|location| {
                            location.id == *id
                                && location.abstraction == "rstyle.formal-byte-window/1"
                        })
                    {
                        format!("location:{id}:index-read:{}", node.id)
                    } else {
                        format!("location:{id}")
                    }
                })
                .or_else(|| access.symbol_id.as_ref().map(|id| format!("binding:{id}")))
        })
        .collect()
}

fn observations(
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<std::borrow::Cow<'_, FieldObservations>, String> {
    facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()
}

fn location(
    field: &FieldOperation,
    context: &mut LocationContext<'_>,
) -> Result<(String, crate::MemoryLocation), String> {
    let Location::Captured {
        parameter,
        path,
        value,
    } = &field.location
    else {
        return Err("Missing field origin".into());
    };

    context.formal_location(
        *parameter,
        path,
        crate::data_types::TypeToken::from_tree(value),
    )
}

fn verify_call_operations(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
    calls: &[&crate::HirNode],
) -> Result<(), String> {
    checked_instances::verify::<reference_instances::Operations>(pair, facts)?;
    checked_instances::verify::<product_instances::Operations>(pair, facts)?;
    checked_instances::verify::<product_instances::RangeOperations>(pair, facts)?;
    reference_products::verify(pair, facts)?;
    scalar_from::verify(pair, facts, calls)?;
    character_ascii::verify(pair, facts, calls)?;
    float_scalars::verify(pair, facts)?;
    bool_usize::verify(pair, facts)?;
    bitset_cardinality::verify(pair, facts, calls)?;
    crate::local_value_trait_calls::verify_document_policy(pair, facts, calls)?;
    crate::product_constant_values::verify_documents(pair, facts)?;
    checked_lengths::verify(pair, facts)?;

    borrowed_result_propagation::verify(pair, facts)?;
    checked_slice_views::verify(pair, facts)?;
    checked_suffix_views::verify(pair, facts)?;
    scalar_result_propagation::verify(pair, facts)?;
    product_result_propagation::verify(pair, facts)?;
    result_maps::verify(pair, facts)?;
    owned_product_values::verify(pair, facts)?;
    borrowed_result_errors::verify(pair, facts)?;
    option_if_let::verify(pair, facts)?;
    result_propagation::verify(pair, facts)?;
    option_propagation::verify(pair, facts)?;
    assertions::verify(pair, facts)?;
    size_calls::verify(pair, facts)?;
    crate::data_library::verify_calls(pair, facts, calls)?;
    Ok(())
}

/// Check semantic locations directly against independently verified source
/// operations, without calling the lowerer or document emitter.
pub fn verify<'a>(
    pair: impl Into<crate::producer::PairView<'a>>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let pair = pair.into();
    verify_source_boundary(pair, facts)?;
    let calls = crate::field_locations::operations(&facts.body, &["expr_call", "expr_method_call"]);
    verify_call_operations(pair, facts, &calls)?;
    crate::data_frontend::verify(facts)?;
    verify_registration(pair, facts)?;
    aggregates::verify(pair, facts)?;
    byte_scalars::verify(pair, facts)?;
    bool_constants::verify(pair, facts)?;
    verify_enum_values(pair, facts)?;
    locals::verify(pair, facts)?;
    verify_interface(pair, facts)?;

    let fields = observations(facts)?;
    let mut expected = BTreeMap::new();
    let mut context = LocationContext::from_pair(pair, facts)?;
    for field in &fields.operations {
        verify_field(pair, &mut context, field, &mut expected)?;
    }
    expected.extend(verify_derived_reads(pair, &mut context, &calls)?);
    expected.extend(verify_memory(pair, &mut context)?);

    let actual: BTreeMap<_, _> = pair
        .pdg
        .memory_locations
        .iter()
        .map(|location| (&location.id, location))
        .collect();
    if actual.len() != pair.pdg.memory_locations.len() || !actual.into_iter().eq(expected.iter()) {
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
    borrows::verify(pair, facts)?;
    origins::verify(pair, facts)
}

fn verify_enum_values(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if crate::copy_tagged_values::candidate(facts) {
        let resolution = facts.nominal_resolution()?;
        let receipt = resolution
            .copy_conversion_source(facts.facts())?
            .ok_or("Missing checked original conversion graph authority")?;
        copy_tagged_values::verify(pair, facts, receipt)?;
    } else {
        scalar_enums::verify(pair, facts)?;
    }
    Ok(())
}

fn verify_registration(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::producer::verify_range_policy(pair)?;
    crate::data_frontend::verify_document_policy(&pair.pdg.profile, facts)?;
    if pair.pdg.extensions.get(POLICY_KEY) != Some(&json!(matching_identity(facts)?))
        || pair.cfg.extensions.get(POLICY_KEY) != pair.pdg.extensions.get(POLICY_KEY)
    {
        return Err("Data matching registration disagrees".into());
    }
    Ok(())
}

fn verify_scalar_option_policy(pair: crate::producer::PairView<'_>) -> Result<(), String> {
    if crate::extraction_profile::policy_supports(&pair.pdg.profile, "rust.scalar-option-values") {
        return Ok(());
    }

    let nodes = pair.pdg.nodes.iter().filter_map(|node| {
        node.annotations
            .get("compat.rust.hir-r0:type")?
            .get("value")?
            .as_str()
    });
    let interface = pair
        .pdg
        .function
        .inputs
        .iter()
        .chain(&pair.pdg.function.outputs)
        .filter_map(|parameter| {
            parameter
                .extensions
                .get("compat.rust.hir-r0:resolved-type")?
                .get("value")?
                .as_str()
        });
    for token in nodes
        .chain(interface)
        .filter(|token| token.contains("\"kind\":\"option\""))
    {
        if crate::data_types::scalar_option(&crate::data_types::parse(token)?) {
            return Err("Scalar Option document requires its registered policy".into());
        }
    }
    Ok(())
}

fn verify_derived_reads(
    pair: crate::producer::PairView<'_>,
    context: &mut LocationContext<'_>,
    calls: &[&crate::HirNode],
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let mut expected = verify_projection_reads(pair, context)?;
    expected.extend(verify_call_reads(pair, context, calls)?);
    crate::helper_reads::verify_locations(pair, context, &mut expected)?;

    Ok(expected)
}

fn verify_call_reads(
    pair: crate::producer::PairView<'_>,
    context: &mut LocationContext<'_>,
    calls: &[&crate::HirNode],
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    callbacks::verify(pair, context.facts)?;
    named_callbacks::verify(pair, context.facts)?;
    let bytes = byte_calls::verify(pair, context, calls)?;
    let options = verify_option_reads(pair, context, calls)?;
    let iterators = iterator_position::verify(pair, context)?;

    crate::helper_reads::merge_locations(
        crate::helper_reads::merge_locations(bytes, options)?,
        iterators,
    )
}

fn verify_option_reads(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    calls: &[&crate::HirNode],
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    crate::concrete_results::documents::verify(pair, context.facts)?;
    let options = checked_options::verify(pair, context, calls)?;
    let callbacks = helper_option_callbacks::verify(pair, context)?;
    crate::helper_reads::merge_locations(options, callbacks)
}

fn verify_projection_reads(
    pair: crate::producer::PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    verify_location_passes(
        pair,
        context,
        &[indirect_reads::verify_dereferences, index_reads::verify],
    )
}

fn verify_field(
    pair: crate::producer::PairView<'_>,
    context: &mut LocationContext<'_>,
    field: &FieldOperation,
    expected: &mut BTreeMap<String, crate::MemoryLocation>,
) -> Result<(), String> {
    if owned_product_values::field_role(field, context.facts)? {
        // The complete update validator consumes this exact field observation
        // as a write role; it is not an independent field-value read.
        return Ok(());
    }
    let node = serialized_field(pair, field)?;

    let facts = context.facts;
    if matches!(field.location, Location::Unsupported { .. }) {
        let source = origins::source_node(node, facts)?;
        if crate::field_locations::derived::candidate(source, facts) {
            expected.extend(indirect_reads::verify(node, source, pair, context)?);
            return Ok(());
        }
        return value_fields::verify(node, source, pair, facts);
    }
    let (id, location) = location(field, context)?;
    let places = crate::data_places::Places::verify(facts)?;
    let source = origins::source_node(node, facts)?;
    if places.contains(source) {
        borrows::verify_place(node, &id)?;
    } else {
        if node.category != NodeCategory::Read
            || node.attributes.reads.len() != 1
            || node.attributes.reads[0].location_id.as_ref() != Some(&id)
            || !node.attributes.writes.is_empty()
        {
            return Err("Serialized field read has incorrect location/effects".into());
        }
    }

    verify_field_operands(pair, facts, field, node)?;
    expected.insert(id, location);
    Ok(())
}

fn serialized_field<'a>(
    pair: crate::producer::PairView<'a>,
    field: &FieldOperation,
) -> Result<&'a crate::PdgNode, String> {
    serialized_operation(pair, &field.span, |node| {
        matches!(
            node.category,
            NodeCategory::Read | NodeCategory::Computation
        ) && node.original_kind.known_value() == Some("expr_field")
    })
}

fn serialized_operation_mut<'a>(
    pair: &'a mut Pair,
    source: &crate::HirNode,
) -> Result<&'a mut crate::PdgNode, String> {
    let node_id = serialized_operation(pair.as_view(), &source.span, |node| {
        matches!(
            node.category,
            NodeCategory::Read | NodeCategory::Computation | NodeCategory::Call
        ) && node.original_kind.known_value() == Some(source.kind.as_str())
            && node.attributes.construct.as_deref() != Some("data.mutable-index-check")
    })?
    .id
    .clone();
    pair.pdg
        .nodes
        .iter_mut()
        .find(|node| node.id == node_id)
        .ok_or_else(|| "Missing serialized source operation".into())
}

pub(crate) fn serialized_operation<'a>(
    pair: crate::producer::PairView<'a>,
    span: &crate::ByteSpan,
    accepts: impl Fn(&crate::PdgNode) -> bool,
) -> Result<&'a crate::PdgNode, String> {
    let mut candidates = pair.pdg.nodes.iter().filter(|node| {
        accepts(node)
            && node.source_span.as_ref().is_some_and(|source| {
                source.start == u64::from(span.start) && source.end == u64::from(span.end)
            })
    });
    let node = candidates
        .next()
        .ok_or("Missing serialized source operation")?;
    if candidates.next().is_some() {
        return Err("Ambiguous serialized source operation".into());
    }
    Ok(node)
}

pub(crate) fn typed_operation<'a>(
    pair: PairView<'a>,
    source: &crate::HirNode,
    construct: &str,
    ty: &str,
) -> Result<&'a PdgNode, String> {
    let node = serialized_operation(pair, &source.span, |node| {
        node.attributes.construct.as_deref() == Some(construct)
    })?;
    if node.original_kind.known_value() != Some(source.kind.as_str())
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some(ty)
    {
        return Err("Serialized operation type/source correspondence disagrees".into());
    }
    Ok(node)
}

fn verify_reads(pair: crate::producer::PairView<'_>, fields: usize) -> Result<(), String> {
    if pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.category,
                NodeCategory::Read | NodeCategory::Computation
            ) && (node.original_kind.known_value() == Some("expr_field")
                || node.attributes.construct.as_deref()
                    == Some(crate::owned_product_values::UPDATE))
        })
        .count()
        != fields
        || pair
            .pdg
            .nodes
            .iter()
            .any(|node| invalid_memory_access(node, pair))
    {
        return Err("Data operation inventory or read-only effects disagree".into());
    }
    if pair.pdg.function.reference_count.value != Some(read_count(pair)?) {
        return Err("Data reference count disagrees with verified read locations".into());
    }
    Ok(())
}

fn invalid_memory_access(node: &crate::PdgNode, pair: crate::producer::PairView<'_>) -> bool {
    if mutable_scalars::is_access(node)
        && crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            crate::mutable_scalars::CAPABILITY,
        )
    {
        return false;
    }
    let field = node.category == NodeCategory::Read
        && (node.original_kind.known_value() == Some("expr_field")
            || node.attributes.construct.as_deref() == Some("data.indirect-deref")
                && crate::extraction_profile::policy_supports(
                    &pair.pdg.profile,
                    "rust.shared-dereference-reads",
                ));
    let indexed = node.category == NodeCategory::Read
        && node.original_kind.known_value() == Some("expr_index")
        && crate::extraction_profile::policy_supports(&pair.pdg.profile, "rust.scalar-index-reads");
    let call = read_only_call(node, pair);
    let payload = node.attributes.construct.as_deref() == Some("data.option-payload")
        && crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            "rust.checked-slice-option",
        );
    let iterator = node.attributes.construct.as_deref() == Some(crate::iterator_position::PREFIX)
        && crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            crate::iterator_position::CAPABILITY,
        );

    node.attributes
        .writes
        .iter()
        .any(|access| access.location_id.is_some())
        || !field
            && !payload
            && !iterator
            && !indexed
            && !call
            && node
                .attributes
                .reads
                .iter()
                .any(|access| access.location_id.is_some())
}

fn read_only_call(node: &crate::PdgNode, pair: crate::producer::PairView<'_>) -> bool {
    node.category == NodeCategory::Call
        && (node.original_kind.known_value() == Some("expr_call")
            || node.original_kind.known_value() == Some("expr_method_call")
                && crate::extraction_profile::policy_supports(
                    &pair.pdg.profile,
                    "rust.verified-inherent-helper-calls",
                ))
        && crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            "rust.verified-read-only-helper-calls",
        )
}

fn read_count(pair: crate::producer::PairView<'_>) -> Result<u64, String> {
    let calls = crate::extraction_profile::policy_supports(
        &pair.pdg.profile,
        "rust.verified-read-only-helper-calls",
    );
    let reachable = crate::reachable_vertices(pair.cfg);
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
        .filter(|node| {
            (node.category == NodeCategory::Read || calls && node.category == NodeCategory::Call)
                && nodes.contains(&node.id)
        })
        .flat_map(|node| read_keys(node, &pair.pdg.memory_locations))
        .collect();
    Ok(keys.len() as u64)
}

fn verify_field_operands(
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
    field: &FieldOperation,
    node: &crate::PdgNode,
) -> Result<(), String> {
    verify_field_root(facts, field, node)?;
    let base = field_base(pair, facts, node)?;

    verify_field_value_edge(pair, node, base)
}

fn verify_field_root(
    facts: &dyn crate::unit_store::EvidenceFacts,
    field: &FieldOperation,
    node: &crate::PdgNode,
) -> Result<(), String> {
    let Location::Captured { parameter, .. } = field.location else {
        return Err("Missing formal field origin".into());
    };
    let root = &facts.signature.params[parameter as usize].symbol;
    if node.category == NodeCategory::Computation {
        return if node.attributes.symbol_ids.is_empty() {
            Ok(())
        } else {
            Err("Place projection invents a binding read".into())
        };
    }

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
    pair: crate::producer::PairView<'a>,
    facts: &dyn crate::unit_store::EvidenceFacts,
    node: &crate::PdgNode,
) -> Result<&'a crate::PdgNode, String> {
    let source = origins::source_node(node, facts)?;
    let [base] = source.children.as_slice() else {
        return Err("Field source lacks exact base".into());
    };

    value_operation(pair, base)
}

pub(crate) fn value_operation<'a>(
    pair: crate::producer::PairView<'a>,
    base: &crate::HirNode,
) -> Result<&'a crate::PdgNode, String> {
    let mut base = base;
    while base.kind == "expr_block"
        && let Some(tail) = base.children.last()
        && tail.kind.starts_with("expr_")
    {
        base = tail;
    }

    let candidates: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|candidate| {
            method_value(pair, base, candidate)
                && matches!(
                    candidate.category,
                    NodeCategory::Read
                        | NodeCategory::Computation
                        | NodeCategory::Call
                        | NodeCategory::Merge
                )
                && aggregates::source_span(candidate).is_ok_and(|span| {
                    span.start == u64::from(base.span.start) && span.end == u64::from(base.span.end)
                })
                && aggregates::check_value(candidate, base).is_ok()
        })
        .collect();
    let [base] = candidates.as_slice() else {
        return Err("Field base has no unique serialized operation".into());
    };
    Ok(base)
}

fn method_value(pair: PairView<'_>, source: &crate::HirNode, node: &PdgNode) -> bool {
    let mapped = source.kind == "expr_method_call"
        && crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            crate::result_maps::CAPABILITY,
        )
        && source.res.as_ref().is_some_and(|resolution| {
            serde_json::from_str::<crate::data_library::Callee>(&resolution.description).is_ok_and(
                |callee| {
                    matches!(
                        callee.path.as_str(),
                        "core::result::Result::<T, E>::map"
                            | "core::result::Result::<T, E>::map_err"
                    )
                },
            )
        });
    !mapped
        || node.category == NodeCategory::Merge
            && node.attributes.construct.as_deref() == Some(crate::result_maps::RESULT)
}

fn verify_field_value_edge(
    pair: crate::producer::PairView<'_>,
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

fn verify_interface(
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
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

fn verify_value_effects(node: &crate::PdgNode, literal: bool, reason: &str) -> Result<(), String> {
    if node.attributes.operator.is_some()
        || !literal && node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
    {
        return Err(reason.into());
    }
    Ok(())
}

fn live_operations(pair: crate::producer::PairView<'_>) -> Result<BTreeSet<String>, String> {
    let reached = crate::reachable_vertices(pair.cfg);
    let mapping = crate::read_operation_map(&pair.pdg.extensions)?;

    Ok(mapping
        .into_iter()
        .filter(|entry| reached.contains(&entry.cfg_node_id))
        .map(|entry| entry.pdg_node_id)
        .collect())
}

pub(crate) fn verify_saved_operands(
    pair: PairView<'_>,
    node: &PdgNode,
    values: &[&PdgNode],
) -> Result<(), String> {
    let mut actual: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .map(|edge| (edge.operand_position, edge.source.as_str()))
        .collect();

    actual.sort();
    let expected: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(position, value)| (Some(position as u64), value.id.as_str()))
        .collect();
    if actual != expected
        || pair.pdg.edges.iter().any(|edge| {
            edge.target == node.id
                && edge.subkind == "reaching_definition"
                && !(crate::extraction_profile::policy_supports(
                    &pair.pdg.profile,
                    crate::mutable_scalars::CAPABILITY,
                ) && edge.location_id.is_some()
                    && edge.symbol_id.is_none())
        })
    {
        return Err("Operation loses its exact ordered saved value operands".into());
    }
    Ok(())
}

fn verify_construct_inventory<const N: usize>(
    pair: PairView<'_>,
    constructs: [&str; N],
    expected: [usize; N],
) -> Result<(), String> {
    for (construct, count) in constructs.into_iter().zip(expected) {
        if pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some(construct))
            .count()
            != count
        {
            return Err(format!(
                "Operation inventory for {construct} disagrees with original source"
            ));
        }
    }
    Ok(())
}

fn verify_source_boundary(
    pair: PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::character_scalars::verify_document_policy(&pair.pdg.profile, facts)?;
    crate::character_ascii::verify_document_policy(&pair.pdg.profile, facts)?;
    verify_scalar_option_policy(pair)?;

    verify_source_owner(facts)?;
    crate::data_receivers::verify_documents(pair, facts)
}

fn verify_source_owner(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    if facts
        .body
        .any(&|node| crate::result_maps::candidate(node, facts))
    {
        let resolution = facts.nominal_resolution()?;
        let receipt = resolution
            .transport_source()
            .ok_or("Generated Result map lacks its exact checked source owner")?;
        return receipt.verify(facts.facts());
    }
    crate::inherent_calls::definition(facts).map(|_| ())
}
