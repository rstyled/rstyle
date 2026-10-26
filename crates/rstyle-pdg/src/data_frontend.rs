//! Independently checked source/type/origin boundary for local data extraction.

pub(crate) mod borrowed_results;

use crate::{HirNode, concrete_types::*, data_types, field_locations};
use source_proof::PathResolution;

struct Unresolved;
impl PathResolution for Unresolved {
    fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
        Err("Named data types require selected-unit nominal resolution/effect evidence".into())
    }
}

pub fn verify(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    facts.verify_data_frontend()
}

pub(crate) fn check(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    let observation =
        crate::diagnostics::Stage::new("source.data_frontend", &facts.name, None).enter();
    if crate::slice_length_instances::activation::verify(facts)? {
        // The private proof covers every source/HIR/type role. Its bounded
        // dependency contract supplies conditional error components; ordinary
        // reference origins cannot represent the two distinct Result cases.
        crate::helper_calls::slice_lengths::source_contract(facts)?;
        return observation.finish(Ok(()));
    }
    if crate::product_instances::activation::verify(facts)?
        || crate::scalar_instances::activation::verify(facts)?
        || crate::reference_instances::activation::verify(facts)?
        || crate::reference_instances::forwarder_activation::verify(facts)?
    {
        let places = crate::data_places::Places::verify(facts)?;
        return observation.finish(verify_nodes(&facts.body, facts, &places));
    }
    if crate::nonzero_digit_calls::verify(facts)?.is_some()
        || crate::scalar_product_instances::forwarders::verify(facts)?.is_some()
        || crate::copy_tagged_values::call_wrappers::verify_function(facts)?
    {
        verify_fields(facts)?;
        return observation.finish(Ok(()));
    }
    if crate::copy_tagged_values::verify_function(facts)? {
        verify_fields(facts)?;
        return observation.finish(Ok(()));
    }
    if crate::scalar_product_instances::activation::candidate(facts) {
        crate::scalar_product_instances::activation::verify(facts)?;
    }

    observation.finish(check_ordinary_function(facts))
}

fn check_ordinary_function(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    crate::character_scalars::verify_function(facts)?;
    crate::float_scalars::verify_function(facts)?;
    crate::range_values::verify_function(facts)?;
    let guarded_error = crate::data_documents::named_error_payloads::verify_source(facts)?
        || crate::data_documents::fallible_readers::verify_source(facts)?;
    if !guarded_error {
        crate::concrete_results::verify_function(facts)?;
    }
    crate::checked_option_products::verify_function(facts)?;
    crate::mutable_scalars::verify(facts)?;
    let interface = interface(facts)?;
    let resolution = nominal_resolution(facts, &interface)?;
    let source = source_function(facts, resolution.as_deref())?;
    verify_lifetimes(&source)?;
    crate::size_constants::verify_function(facts)?;
    crate::bool_constants::verify_function(facts)?;
    verify_interface(facts, &source, &interface, resolution.as_deref())?;

    for parameter in &facts.signature.params {
        crate::data_origins::parameter(parameter)?;
    }

    verify_fields(facts)?;
    if guarded_error {
        return Ok(());
    }
    let places = crate::data_places::Places::verify(facts)?;
    verify_nodes(&facts.body, facts, &places)
}

pub(crate) fn source_function(
    facts: &dyn crate::unit_store::EvidenceFacts,
    resolution: Option<&crate::nominal_boundary::resolution::Resolution>,
) -> Result<syn::ItemFn, String> {
    if let Some(receipt) = resolution.and_then(|resolution| resolution.transport_source()) {
        receipt.verify(facts.facts())?;
        let member = receipt.member().function();
        return Ok(syn::ItemFn {
            attrs: member.attrs.clone(),
            vis: member.vis.clone(),
            sig: member.sig.clone(),
            block: Box::new(member.block.clone()),
        });
    }
    syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::nonzero_digit_calls::verify_document_policy(profile, facts)?;
    crate::extraction_profile::require_document(
        profile,
        crate::scalar_product_instances::activation::VALUE_CAPABILITY,
        || crate::scalar_product_instances::activation::candidate(facts),
        "Digit product values require their registered document policy",
    )?;
    crate::extraction_profile::require_document(
        profile,
        crate::copy_tagged_values::CAPABILITY,
        || crate::copy_tagged_values::candidate(facts),
        "Tagged Copy conversions require their registered document policy",
    )?;
    if facts.scalar_instance().is_some()
        && !crate::extraction_profile::policy_supports(profile, crate::scalar_instances::CAPABILITY)
    {
        return Err("Scalar instances require their registered document policy".into());
    }
    crate::default_calls::verify_document_policy(profile, facts)?;
    crate::nominal_boundary::resolution::tryfrom_owners::verify_document_policy(profile, facts)?;
    crate::nominal_boundary::resolution::value_traits::verify_document_policy(profile, facts)?;
    verify_value_document_policy(profile, facts)?;
    if complete_nominal_policy(profile) {
        return Ok(());
    }

    crate::helper_stored_slices::verify_document_policy(profile, facts)?;
    verify_copy_receiver_policy(profile, facts)?;
    if !crate::extraction_profile::policy_supports(profile, "rust.accelerator-packing")
        && crate::extraction_profile::facts_support(facts, "rust.accelerator-packing")
        && facts
            .nominal_resolution()
            .is_ok_and(|resolution| resolution.requires_scalar_aliases(&facts.source.bytes))
    {
        return Err("Scalar alias proof requires its registered document policy".into());
    }

    let interface = interface(facts)?;
    if let Some(resolution) = nominal_resolution(facts, &interface)? {
        verify_source_owner_policy(profile, &resolution)?;
        verify_nominal_policy(profile, &resolution)?;
    }
    Ok(())
}

fn complete_nominal_policy(profile: &crate::ProfileId) -> bool {
    let capabilities = [
        crate::helper_stored_slices::CAPABILITY,
        "rust.passive-copy-dependencies",
        "rust.borrowed-mixed-owners",
        "rust.scoped-nominal-imports",
        "rust.copy-array-fields",
        "rust.scalar-enum-values",
        "rust.borrowed-array-views",
        "rust.accelerator-packing",
        "rust.borrowed-copy-child-fields",
        crate::fieldless_enums::CAPABILITY,
        "rust.checked-copy-owner-source",
        "rust.static-string-copy-products",
        "rust.checked-copy-constructor-sites",
    ];
    capabilities
        .iter()
        .all(|capability| crate::extraction_profile::policy_supports(profile, capability))
}

fn verify_value_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        crate::mutable_scalars::CAPABILITY,
        || crate::mutable_scalars::enabled(facts),
        "Mutable scalar memory flow requires its registered document policy",
    )?;

    crate::extraction_profile::require_document(
        profile,
        crate::range_values::CONSUMING_CAPABILITY,
        || crate::range_values::consuming_input(facts),
        "Consuming Range values require their registered document policy",
    )?;
    crate::nominal_boundary::census::policy::verify_documents(profile, facts)?;
    verify_instance_document_policy(profile, facts)?;
    verify_result_document_policy(profile, facts)
}

fn verify_instance_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        crate::product_instances::activation::CAPABILITY,
        || facts.product_instance().is_some(),
        "Product constructor requires its registered document policy",
    )?;
    crate::extraction_profile::require_document(
        profile,
        crate::reference_instances::activation::CAPABILITY,
        || facts.reference_instance().is_some(),
        "Reference constructor requires its registered document policy",
    )?;
    crate::extraction_profile::require_document(
        profile,
        crate::reference_instances::forwarder_activation::CAPABILITY,
        || facts.reference_forwarder().is_some(),
        "Reference forwarder requires its registered document policy",
    )
}

fn verify_result_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::scalar_loop_helpers::verify_document_policy(profile, facts)?;
    crate::primitive_constants::verify_document_policy(profile, facts)?;
    crate::product_constant_values::verify_document_policy(profile, facts)?;
    crate::concrete_results::verify_document_policy(profile, facts)?;
    crate::checked_option_products::verify_document_policy(profile, facts)?;
    crate::data_documents::result_propagation::verify_document_policy(profile, facts)?;
    crate::data_documents::option_propagation::verify_document_policy(profile, facts)?;
    crate::data_documents::borrowed_result_propagation::verify_document_policy(profile, facts)?;
    crate::data_documents::scalar_result_propagation::verify_document_policy(profile, facts)?;
    crate::data_documents::named_error_result_propagation::verify_document_policy(profile, facts)?;
    crate::data_documents::slice_lengths::verify_document_policy(profile, facts)?;
    crate::data_documents::product_result_propagation::verify_document_policy(profile, facts)?;
    crate::data_documents::result_maps::verify_document_policy(profile, facts)?;
    crate::extraction_profile::require_document(
        profile,
        crate::helper_component_outputs::CAPABILITY,
        || crate::helper_component_outputs::used(facts),
        "Component helper results require their registered document policy",
    )?;
    crate::extraction_profile::require_document(
        profile,
        crate::borrowed_result_types::CAPABILITY,
        || borrowed_results::used(&facts.body, facts),
        "Checked borrowed Result operations require their registered document policy",
    )?;

    crate::extraction_profile::require_document(
        profile,
        crate::range_values::CAPABILITY,
        || crate::range_values::used(facts),
        "Fresh Range results require their registered document policy",
    )
}

fn verify_copy_receiver_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if !crate::extraction_profile::policy_supports(profile, crate::fieldless_enums::CAPABILITY) {
        for call in crate::helper_calls::call_nodes(facts) {
            if crate::owned_receiver::shared_copy(call, facts)? {
                return Err(
                    "Shared Copy value receivers require their registered document policy".into(),
                );
            }
        }
    }
    Ok(())
}

fn verify_source_owner_policy(
    profile: &crate::ProfileId,
    resolution: &crate::nominal_boundary::resolution::Resolution,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        "rust.checked-copy-owner-source",
        || resolution.requires_copy_owner_source(),
        "Copy owner source requires its registered document policy",
    )?;
    crate::extraction_profile::require_document(
        profile,
        "rust.static-string-copy-products",
        || resolution.requires_static_string_products(),
        "Static string Copy products require their registered document policy",
    )?;
    crate::extraction_profile::require_document(
        profile,
        "rust.checked-copy-constructor-sites",
        || resolution.requires_copy_constructor_sites(),
        "Copy constructor sites require their registered document policy",
    )
}

fn verify_nominal_policy(
    profile: &crate::ProfileId,
    resolution: &crate::nominal_boundary::resolution::Resolution,
) -> Result<(), String> {
    for (capability, required, message) in [
        (
            "rust.passive-copy-dependencies",
            resolution.requires_passive_copy_dependencies(),
            "Passive Copy dependencies require their registered document policy",
        ),
        (
            "rust.borrowed-mixed-owners",
            resolution.requires_borrowed_mixed_owners(),
            "Borrowed mixed owners require their registered document policy",
        ),
        (
            crate::fieldless_enums::CAPABILITY,
            resolution.requires_enum_values(true),
            "Fieldless enum requires its registered document policy",
        ),
        (
            "rust.borrowed-copy-child-fields",
            resolution.requires_borrowed_copy_child_fields(),
            "Borrowed Copy child fields require their registered document policy",
        ),
        (
            "rust.borrowed-array-views",
            resolution.requires_borrowed_array_views(),
            "Borrowed array views require their registered document policy",
        ),
        (
            "rust.scalar-enum-values",
            resolution.requires_enum_values(false),
            "Scalar enum values require their registered document policy",
        ),
        (
            "rust.scoped-nominal-imports",
            resolution.requires_scoped_imports(),
            "Scoped nominal import proof requires its registered document policy",
        ),
        (
            "rust.copy-array-fields",
            resolution.requires_copy_array_fields(),
            "Copy array fields require their registered document policy",
        ),
    ] {
        crate::extraction_profile::require_document(profile, capability, || required, message)?;
    }
    Ok(())
}

fn nominal_resolution<'a>(
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    interface: &InterfaceObservations,
) -> Result<Option<crate::nominal_boundary::resolution::ResolutionView<'a>>, String> {
    if crate::declaration_origins::declarations(interface)?.is_empty() {
        return Ok(None);
    }
    facts.nominal_resolution().map(Some)
}

struct Resolver<'a> {
    nominal: Option<&'a crate::nominal_boundary::resolution::Resolution>,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
}
impl PathResolution for Resolver<'_> {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
        self.nominal.map_or_else(
            || {
                if crate::extraction_profile::facts_support(self.facts, "rust.accelerator-packing")
                {
                    self.facts.nominal_resolution()?.resolve(path)
                } else {
                    Unresolved.resolve(path)
                }
            },
            |resolution| resolution.resolve(path),
        )
    }

    fn concrete_result(
        &self,
        path: &syn::TypePath,
        _ok: &TypeTree,
        _err: &TypeTree,
    ) -> Result<(), String> {
        self.facts
            .nominal_resolution()?
            .verify_result_type_source(self.facts.facts(), path)
    }

    fn scalar_option(&self, path: &syn::TypePath, payload: &TypeTree) -> Result<(), String> {
        if let TypeTree::Scalar(scalar) = payload {
            return self.facts.verify_option_type(path, scalar);
        }
        if matches!(payload, TypeTree::ScalarProductInstance(_)) {
            return self
                .facts
                .nominal_resolution()?
                .verify_scalar_product_option_type(self.facts.facts(), path, payload);
        }
        self.facts.nominal_resolution()?.verify_product_option_type(
            self.facts.facts(),
            path,
            payload,
        )
    }
}

fn interface(
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<std::borrow::Cow<'_, InterfaceObservations>, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<crate::concrete_types::InterfaceObservations>()?;
    interface.validate()?;
    Ok(interface)
}

fn verify_lifetimes(source: &syn::ItemFn) -> Result<(), String> {
    if source
        .sig
        .generics
        .params
        .iter()
        .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)))
        || source
            .sig
            .generics
            .where_clause
            .as_ref()
            .is_some_and(|clause| {
                clause
                    .predicates
                    .iter()
                    .any(|predicate| !matches!(predicate, syn::WherePredicate::Lifetime(_)))
            })
    {
        return Err("Only lifetime parameters have a concrete data contract".into());
    }
    Ok(())
}

fn verify_interface(
    facts: &dyn crate::unit_store::EvidenceFacts,
    source: &syn::ItemFn,
    interface: &InterfaceObservations,
    resolution: Option<&crate::nominal_boundary::resolution::Resolution>,
) -> Result<(), String> {
    let receiver = source
        .sig
        .inputs
        .first()
        .is_some_and(|arg| matches!(arg, syn::FnArg::Receiver(_)));
    if receiver != facts.signature.has_self || receiver != facts.signature.receiver.is_some() {
        return Err("Source/serialized receiver ownership disagrees".into());
    }
    if source.sig.inputs.len() != interface.parameters.len() {
        return Err("Data source/formal arity disagrees".into());
    }
    for ((source, observation), parameter) in source
        .sig
        .inputs
        .iter()
        .zip(&interface.parameters)
        .zip(&facts.signature.params)
    {
        match source {
            syn::FnArg::Typed(source) => SourceProof {
                bytes: &facts.source.bytes,
                resolution,
                facts,
            }
            .verify_type(
                &source.ty,
                observation,
                parameter.ty.normalized.as_deref(),
            )?,
            syn::FnArg::Receiver(receiver) => {
                verify_receiver_policy(receiver, facts)?;
                if !crate::nominal_boundary::resolution::value_traits::receiver(facts, observation)?
                {
                    verify_receiver(receiver, observation, parameter, resolution)?;
                }
            }
        }
        verify_parameter_origins(parameter)?;
    }

    verify_output(facts, &source.sig.output, &interface.result, resolution)?;
    Ok(())
}

fn verify_receiver_policy(
    receiver: &syn::Receiver,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if receiver.reference.is_none()
        && !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.owned-copy-self")
    {
        return Err("Owned self requires its registered Copy boundary".into());
    }
    Ok(())
}

fn verify_receiver(
    receiver: &syn::Receiver,
    observation: &Observation,
    parameter: &crate::ParamFact,
    resolution: Option<&crate::nominal_boundary::resolution::Resolution>,
) -> Result<(), String> {
    let owning = resolution
        .and_then(|r| r.receiver.as_ref())
        .ok_or("Missing exact owning receiver impl")?;
    let expected = if receiver.reference.is_some() {
        TypeTree::Shared(Box::new(owning.clone()))
    } else if resolution.is_some_and(|r| r.checked_owned_copy(owning)) {
        owning.clone()
    } else {
        return Err("Owned self requires checked Copy/source correspondence".into());
    };
    if receiver.mutability.is_some()
        || receiver.colon_token.is_some()
        || !matches!(observation, Observation::Captured { ty } if ty == &expected)
        || parameter.position != 0
        || parameter.ty.normalized.as_deref() != Some(data_types::token(&expected)?.as_str())
    {
        return Err("Receiver requires exact concrete self correspondence".into());
    }
    Ok(())
}

fn verify_fields(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()?;
    field_locations::verify(facts, &fields)?;
    let nodes = field_locations::fields(&facts.body);

    for field in &fields.operations {
        let field_locations::Location::Captured { value, .. } = &field.location else {
            continue; // The independent field verifier checked the aggregate value projection.
        };
        crate::helper_stored_slices::verify_field(facts, &field.location)?;
        let token = data_types::token(value)?;
        let matched: Vec<_> = nodes.iter().filter(|node| node.hir == field.hir).collect();
        let [node] = matched.as_slice() else {
            return Err("Field type proof lacks one exact source/HIR operation".into());
        };
        if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(&token) {
            return Err(
                "Field value type contradicts independently verified source location".into(),
            );
        }
    }
    Ok(())
}

struct SourceProof<'a> {
    bytes: &'a str,
    resolution: Option<&'a crate::nominal_boundary::resolution::Resolution>,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
}
impl SourceProof<'_> {
    fn verify_type(
        &self,
        source: &syn::Type,
        observation: &Observation,
        normalized: Option<&str>,
    ) -> Result<(), String> {
        let Observation::Captured { ty } = observation else {
            return Err("Unsupported concrete data type".into());
        };
        let token = data_types::token(ty)?;
        data_types::parse_in(&token, self.facts)?;
        if normalized != Some(token.as_str()) {
            return Err("Resolved data interface token disagrees with captured tree".into());
        }

        let resolver = Resolver {
            nominal: self.resolution,
            facts: self.facts,
        };
        if let Some(resolution) = self.resolution
            && let Some(receipt) = resolution.transport_source()
        {
            receipt.verify(self.facts.facts())?;
            resolution.transport_type_source(source)?;
            return source_proof::verify_top_syntax(source, ty, &resolver);
        }
        source_proof::verify_top_spelling(spelling(self.bytes, source)?, ty, &resolver)?;

        Ok(())
    }
}

fn spelling<'a>(bytes: &'a str, source: &syn::Type) -> Result<&'a str, String> {
    use syn::spanned::Spanned;
    let span = source.span();
    let start = crate::source_evidence::byte_offset(bytes, span.start());
    let end = crate::source_evidence::byte_offset(bytes, span.end());

    bytes
        .get(start..end)
        .ok_or_else(|| "Missing exact data type source bytes".into())
}

fn verify_output(
    facts: &dyn crate::unit_store::EvidenceFacts,
    source: &syn::ReturnType,
    observation: &Observation,
    resolution: Option<&crate::nominal_boundary::resolution::Resolution>,
) -> Result<(), String> {
    let output = match source {
        syn::ReturnType::Default => syn::parse_str("()").expect("unit type"),
        syn::ReturnType::Type(_, output) => *output.clone(),
    };
    let bytes = if matches!(source, syn::ReturnType::Default) {
        "()"
    } else {
        &facts.source.bytes
    };
    SourceProof {
        bytes,
        resolution,
        facts,
    }
    .verify_type(
        &output,
        observation,
        facts.signature.return_ty.normalized.as_deref(),
    )
}

fn verify_nodes(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
) -> Result<(), String> {
    if let Some(operands) = crate::scalar_product_instances::activation::operands(node, facts)? {
        return verify_node_operands(node, facts, places, operands);
    }
    if let Some(update) = crate::owned_product_locals::update_for(node, facts)? {
        verify_nodes(update.receiver(), facts, places)?;
        return verify_nodes(update.value(), facts, places);
    }

    if let Some(operands) = borrowed_results::operands(node, facts) {
        return verify_node_operands(node, facts, places, operands?);
    }
    if crate::option_propagation::candidate(node, facts) {
        let checked = crate::option_propagation::verify(node, facts)?;
        return verify_nodes(checked.operand, facts, places);
    }
    if crate::result_propagation::candidate(node, facts) {
        let checked = crate::result_propagation::verify(node, facts)?;
        return verify_nodes(checked.operand, facts, places);
    }
    if crate::result_maps::candidate(node, facts) {
        let checked = crate::result_maps::verify(node, facts)?;
        verify_node_type(node, facts)?;
        return verify_nodes(checked.receiver, facts, places);
    }

    if crate::helper_option_callbacks::candidate(node, facts) {
        return verify_helper_callback(node, facts, places);
    }
    if crate::named_callbacks::candidate(node, facts) {
        let checked = crate::named_callbacks::verify(node, facts)?;
        verify_nodes(checked.receiver, facts, places)?;
        return verify_nodes(checked.default, facts, places);
    }
    if crate::data_discards::candidate(node, facts) {
        return crate::data_discards::verify(node, facts);
    }
    verify_sequence_nodes(node, facts, places)
}

fn verify_sequence_nodes(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
) -> Result<(), String> {
    if crate::iterator_position::candidate(node, facts) {
        let checked = crate::iterator_position::verify(node, facts)?;
        verify_nodes(checked.receiver, facts, places)?;
        return verify_nodes(checked.predicate.captured_value, facts, places);
    }
    if crate::data_subslices::candidate(node, facts) {
        let checked = crate::data_subslices::verify(node, facts)?;
        for operand in checked.operands() {
            verify_nodes(operand, facts, places)?;
        }
        return Ok(());
    }
    verify_plain_nodes(node, facts, places)
}

fn verify_plain_nodes(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
) -> Result<(), String> {
    verify_value_operations(node, facts)?;
    verify_node_operands(node, facts, places, &node.children)
}

fn verify_node_operands<'a>(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
    operands: impl IntoIterator<Item = &'a HirNode>,
) -> Result<(), String> {
    verify_node_type(node, facts)?;
    verify_reference_origins(node, facts, places)?;
    for operand in operands {
        verify_nodes(operand, facts, places)?;
    }
    Ok(())
}

fn verify_node_type(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if let Some(token) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) {
        if token.contains("\"kind\":\"option\"") && !crate::checked_options::enabled(facts) {
            return Err("Checked Option types require their registered capability".into());
        }
        data_types::parse_in(token, facts)?;
        if node.kind == "expr_path"
            && crate::scalar_product_instances::activation::construct(node, facts)?.is_none()
            && !crate::checked_options::constructor_candidate(node, facts)
            && !crate::borrowed_result_errors::error_candidate(node, facts)
        {
            verify_path(node, facts, token)?;
        }
    }
    Ok(())
}

fn verify_value_operations(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if crate::reference_instances::products::unit_candidate(node, facts) {
        return crate::reference_instances::products::unit(node, facts).map(|_| ());
    }
    if crate::checked_options::if_let_candidate(node, facts) {
        crate::checked_options::verify_if_let(node, facts)?;
    }
    if crate::data_aggregates::constructor(node)
        && crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.aggregate-literal-values")
    {
        crate::data_aggregates::verify(node, facts)?;
    }

    if crate::bool_usize::candidate(node) {
        crate::bool_usize::verify(node, facts)?;
    }
    crate::byte_literals::verify_node(node, facts)?;
    crate::character_scalars::verify_node(node, facts)?;
    crate::float_scalars::verify_node(node, facts)?;
    if node.kind == "expr_binary"
        && node
            .children
            .iter()
            .any(|child| child.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) == Some("char"))
    {
        crate::character_scalars::comparison(node)?;
    }
    verify_enum_node(node, facts)?;
    crate::data_locals::verify_node(node, facts)?;
    if crate::checked_options::constructor_candidate(node, facts) {
        crate::checked_options::verify_constructor(node, facts)?;
    }
    if crate::concrete_results::constructor_candidate(node, facts) {
        crate::concrete_results::verify_constructor(node, facts)?;
    }
    if crate::data_index_reads::candidate(node, facts) {
        crate::data_index_reads::verify(node, facts)?.locations(facts)?;
    }
    if crate::data_dereferences::candidate(node, facts) {
        crate::field_locations::derived::locations(node, facts)?;
    }
    Ok(())
}

fn verify_enum_node(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if crate::fieldless_enums::value_candidate(node, facts) {
        crate::fieldless_enums::value(node, facts)?;
    }

    if crate::fieldless_enums::cast_candidate(node, facts) {
        crate::fieldless_enums::cast(node, facts)?;
    }

    if crate::data_enums::constructor_candidate(node, facts) {
        crate::data_enums::constructor(node, facts)?;
    }
    if crate::data_enums::match_candidate(node, facts) {
        crate::data_enums::matched(node, facts)?;
    }
    Ok(())
}

fn digit_product_reference(node: &HirNode, facts: &dyn crate::unit_store::EvidenceFacts) -> bool {
    crate::scalar_product_instances::activation::candidate(facts)
        && node
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            .and_then(|text| data_types::parse(text).ok())
            .is_some_and(|ty| {
                crate::data_origins::shared(&ty) && !matches!(ty, data_types::TypeToken::Shared(_))
            })
}

fn verify_reference_origins(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
) -> Result<(), String> {
    if digit_product_reference(node, facts) {
        return crate::scalar_product_instances::origins::verify_source_value(node, facts);
    }
    if crate::lossless_integer_calls::message::verify(node, facts)? {
        return Ok(());
    }
    if crate::concrete_results::constructor_candidate(node, facts) {
        let checked = crate::concrete_results::verify_constructor(node, facts)?;
        if checked.construct == "data.result-err" {
            return Ok(());
        }
    }
    // These are Option pattern containers, not evaluated reference values.
    // Their exact shape and nested binding are checked by verify_match.
    if matches!(
        node.kind.as_str(),
        "pat_option_some" | "pat_option_none" | "pat_shared"
    ) {
        return Ok(());
    }
    let origins = if places.contains(node) {
        Vec::new()
    } else {
        crate::data_origins::origins(node, facts)?
    };
    if !places.contains(node)
        && node.kind.starts_with("expr_")
        && node
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            .and_then(|token| data_types::parse(token).ok())
            .is_some_and(|ty| crate::data_origins::shared(&ty))
        && origins.is_empty()
    {
        if crate::concrete_results::fresh_products::interface_used(facts)?
            && crate::concrete_results::fresh_products::empty_error_branch(node, facts)?
        {
            return Ok(());
        }
        return Err("Reference-valued expression lacks complete formal origins".into());
    }

    Ok(())
}

fn verify_path(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    token: &str,
) -> Result<(), String> {
    if crate::product_constant_values::candidate(node) {
        return crate::product_constant_values::verify(node, facts).map(|_| ());
    }
    if crate::reference_instances::products::unit_candidate(node, facts) {
        return crate::reference_instances::products::unit(node, facts).map(|_| ());
    }
    if node
        .res
        .as_ref()
        .is_some_and(|res| res.kind == crate::primitive_constants::RESOLUTION)
    {
        return crate::primitive_constants::verify(node, facts).map(|_| ());
    }

    if node
        .res
        .as_ref()
        .is_some_and(|res| res.kind == crate::bool_constants::RESOLUTION)
    {
        return crate::bool_constants::verify_use(node, facts).map(|_| ());
    }
    if node
        .res
        .as_ref()
        .is_some_and(|resolution| resolution.kind == crate::scalar_constants::RESOLUTION)
    {
        if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.resolved-scalar-constants")
        {
            return Err("Resolved scalar constants require their registered capability".into());
        }
        let constant =
            crate::scalar_constants::ScalarConstant::verify(node, facts.build.pointer_width.value)?;
        if crate::float_scalars::is_float(&constant.ty) {
            facts
                .nominal_resolution()?
                .verify_float_constant(node, &constant, facts)?;
        }
        return Ok(());
    }
    if crate::fieldless_enums::value_candidate(node, facts) {
        return crate::fieldless_enums::value(node, facts).map(|_| ());
    }
    verify_binding_path(node, facts, token)
}

fn verify_binding_path(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    token: &str,
) -> Result<(), String> {
    let target = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Unresolved data binding read")?;
    let binding = facts
        .bindings
        .iter()
        .find(|binding| binding.hir == *target)
        .ok_or("Missing data binding")?;
    if binding.ty.normalized.as_deref() != Some(token) {
        return Err("Data read type disagrees with its binding".into());
    }

    if !scalar(token)
        && !matches!(token, "char" | "f32" | "f64")
        && !binding.is_param
        && !crate::data_locals::supported(
            token,
            crate::extraction_profile::ExtractionProfile::from_facts(facts)?,
        )
    {
        return Err("Data local origin/value correspondence is not yet available".into());
    }
    Ok(())
}

pub fn accepted(token: &str) -> bool {
    data_types::parse(token).is_ok()
}

pub(crate) fn evidence<T: serde::de::DeserializeOwned>(
    facts: &dyn crate::unit_store::EvidenceFacts,
    key: &str,
) -> Result<T, String> {
    let value = facts
        .build
        .frontend_options
        .get(key)
        .ok_or_else(|| format!("Missing data evidence {key}"))?;
    serde::Deserialize::deserialize(value).map_err(|error: serde_json::Error| error.to_string())
}

fn verify_parameter_origins(parameter: &crate::ParamFact) -> Result<(), String> {
    let token = data_types::parse(
        parameter
            .ty
            .normalized
            .as_deref()
            .ok_or("Missing parameter type")?,
    )?;
    if crate::data_origins::shared(&token) && crate::data_origins::parameter(parameter)?.is_empty()
    {
        return Err("Reference-bearing formal requires unavailable element origin evidence".into());
    }
    Ok(())
}

fn verify_helper_callback(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
) -> Result<(), String> {
    let checked = crate::helper_option_callbacks::verify(node, facts)?;
    for operand in [
        checked.arguments.receiver,
        checked.arguments.default,
        checked.binding,
        checked.body,
    ] {
        verify_nodes(operand, facts, places)?;
    }
    Ok(())
}
