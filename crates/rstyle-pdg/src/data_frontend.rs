//! Independently checked source/type/origin boundary for local data extraction.

use crate::{HirNode, concrete_types::*, data_types, field_locations};
use source_proof::PathResolution;

struct Unresolved;
impl PathResolution for Unresolved {
    fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
        Err("Named data types require selected-unit nominal resolution/effect evidence".into())
    }
}

pub fn verify(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    crate::character_scalars::verify_function(facts)?;
    let interface = interface(facts)?;
    let source: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    verify_lifetimes(&source)?;
    crate::size_constants::verify_function(facts)?;
    crate::bool_constants::verify_function(facts)?;
    let resolution = nominal_resolution(facts, &interface)?;
    verify_interface(facts, &source, &interface, resolution.as_deref())?;

    for parameter in &facts.signature.params {
        crate::data_origins::parameter(parameter)?;
    }

    verify_fields(facts)?;
    let places = crate::data_places::Places::verify(facts)?;
    verify_nodes(&facts.body, facts, &places)
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let capabilities = [
        "rust.scoped-nominal-imports",
        "rust.copy-array-fields",
        "rust.scalar-enum-values",
        "rust.borrowed-array-views",
        "rust.accelerator-packing",
        "rust.borrowed-copy-child-fields",
        crate::fieldless_enums::CAPABILITY,
    ];
    if capabilities
        .iter()
        .all(|capability| crate::extraction_profile::policy_supports(profile, capability))
    {
        return Ok(());
    }

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
        verify_nominal_policy(profile, &resolution)?;
    }
    Ok(())
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

fn verify_nominal_policy(
    profile: &crate::ProfileId,
    resolution: &crate::nominal_boundary::resolution::Resolution,
) -> Result<(), String> {
    for (capability, required, message) in [
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

    fn scalar_option(&self, path: &syn::TypePath, payload: &TypeTree) -> Result<(), String> {
        let TypeTree::Scalar(scalar) = payload else {
            return Err("Core Option source proof requires a scalar payload".into());
        };
        self.facts.verify_option_type(path, scalar)
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
                if receiver.reference.is_none()
                    && !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
                        .supports("rust.owned-copy-self")
                {
                    return Err("Owned self requires its registered Copy boundary".into());
                }
                verify_receiver(receiver, observation, parameter, resolution)?
            }
        }
        verify_parameter_origins(parameter)?;
    }

    verify_output(facts, &source.sig.output, &interface.result, resolution)?;
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
        data_types::parse_for(
            &token,
            crate::extraction_profile::ExtractionProfile::from_facts(self.facts)?,
        )?;
        if normalized != Some(token.as_str()) {
            return Err("Resolved data interface token disagrees with captured tree".into());
        }

        source_proof::verify_top_spelling(
            spelling(self.bytes, source)?,
            ty,
            &Resolver {
                nominal: self.resolution,
                facts: self.facts,
            },
        )?;

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
    if crate::named_callbacks::candidate(node, facts) {
        let checked = crate::named_callbacks::verify(node, facts)?;
        verify_nodes(checked.receiver, facts, places)?;
        return verify_nodes(checked.default, facts, places);
    }
    if crate::data_discards::candidate(node, facts) {
        return crate::data_discards::verify(node, facts);
    }
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
    verify_value_operations(node, facts)?;
    if let Some(token) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) {
        if token.contains("\"kind\":\"option\"") && !crate::checked_options::enabled(facts) {
            return Err("Checked Option types require their registered capability".into());
        }
        data_types::parse_for(
            token,
            crate::extraction_profile::ExtractionProfile::from_facts(facts)?,
        )?;
        if node.kind == "expr_path" && !crate::checked_options::constructor_candidate(node, facts) {
            verify_path(node, facts, token)?;
        }
    }
    verify_reference_origins(node, facts, places)?;

    for child in &node.children {
        verify_nodes(child, facts, places)?;
    }
    Ok(())
}

fn verify_value_operations(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if crate::data_aggregates::constructor(node)
        && crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.aggregate-literal-values")
    {
        crate::data_aggregates::verify(node, facts)?;
    }

    crate::byte_literals::verify_node(node, facts)?;
    crate::character_scalars::verify_node(node, facts)?;
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

fn verify_reference_origins(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
) -> Result<(), String> {
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
        return Err("Reference-valued expression lacks complete formal origins".into());
    }

    Ok(())
}

fn verify_path(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    token: &str,
) -> Result<(), String> {
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
        return crate::scalar_constants::ScalarConstant::verify(
            node,
            facts.build.pointer_width.value,
        )
        .map(|_| ());
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
        && token != "char"
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
