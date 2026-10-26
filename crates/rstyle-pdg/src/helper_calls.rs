//! Verified same-compilation-unit helper dependencies. Names never prove effects.

use crate::unit_store::EvidenceFacts;
use crate::{HirFnFacts, HirNode, producer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) mod digit_products;
mod external;
pub(crate) mod guarded_effects;
pub(crate) mod instances;
pub(crate) mod named_errors;
mod product_conversions;
mod reference_forwarders;
pub(crate) mod slice_lengths;
pub(crate) mod tagged_conversions;
pub(crate) mod tagged_resolution;
pub(crate) mod transport;
mod transport_lexical;

pub const KEY: &str = "verified_helper";
pub(crate) const MAX_DEPTH: usize = 8;
pub(crate) const MAX_BYTES: usize = 8 * 1024 * 1024;

/// Route a source candidate only when its helper capability is registered.
fn capability_candidate(
    facts: &dyn EvidenceFacts,
    capability: &str,
    source_candidate: fn(&dyn EvidenceFacts) -> bool,
) -> bool {
    crate::extraction_profile::facts_support(facts, capability) && source_candidate(facts)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub summary: Summary,
    pub frontend: HirFnFacts,
    pub pdg: String,
    pub cfg: String,
    pub manifest: producer::Manifest,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub identity: String,
    pub argument_types: Vec<String>,
    pub result_type: String,
    pub mutates_arguments: bool,
    pub memory_effects: bool,
    pub may_panic: bool,
    pub always_returns: bool,
    pub argument_evaluation: String,
    pub result_dependence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounded_loop: Option<LoopTermination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_outputs: Option<Vec<crate::data_origins::Origin>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_outputs: Option<Vec<crate::helper_component_outputs::Output>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_reads: Option<Vec<crate::helper_reads::Read>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) tagged_output: Option<crate::copy_tagged_values::origins::WireContract>,
}

/// Normal execution is bounded independently of the preserved panic outcome.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopTermination {
    pub may_diverge: bool,
    pub positive_max_backedges: u32,
}

impl From<crate::scalar_loop_helpers::Checked> for LoopTermination {
    fn from(proof: crate::scalar_loop_helpers::Checked) -> Self {
        Self {
            may_diverge: proof.may_diverge,
            positive_max_backedges: proof.positive_max_backedges,
        }
    }
}

type ReferenceOutputs = Option<Vec<crate::data_origins::Origin>>;

struct Boundary {
    reference_outputs: ReferenceOutputs,
    tagged_output: Option<crate::copy_tagged_values::origins::WireContract>,
}

fn summary(
    facts: &dyn EvidenceFacts,
    pair: &producer::Pair,
    boundary: Boundary,
) -> Result<Summary, String> {
    match boundary.tagged_output {
        Some(output) if guarded_effects::candidate(facts) => {
            guarded_effects::summary(facts, pair, output)
        }
        Some(output) if slice_lengths::candidate(facts) => slice_lengths::summary(facts, output),
        Some(output) if named_errors::candidate(facts) => named_errors::summary(facts, output),
        Some(output) => tagged_conversions::summary(facts, output),
        None => legacy_summary(facts, pair, boundary.reference_outputs),
    }
}

fn legacy_summary(
    facts: &dyn EvidenceFacts,
    pair: &producer::Pair,
    reference_outputs: ReferenceOutputs,
) -> Result<Summary, String> {
    let loop_proof = crate::scalar_loop_helpers::verify_if_loop(facts, pair.as_view())?;
    let component_outputs = crate::helper_component_outputs::derive(facts, pair.as_view())?;
    let guarded =
        component_outputs.is_some() && crate::helper_component_outputs::guarded_product(facts);
    let references = reference_outputs.is_some() || component_outputs.is_some();

    let memory_reads = crate::helper_reads::derive(facts, pair.as_view())?;
    let view = crate::helper_views::projected(facts, &reference_outputs);
    let normal = crate::helper_normal_flow::supported(facts, pair.as_view());

    let may_panic = summary_may_panic(loop_proof.as_ref(), pair.as_view(), view || normal);
    Ok(Summary {
        identity: if guarded {
            crate::helper_component_outputs::PRODUCT_SUMMARY
        } else if component_outputs.is_some() {
            crate::helper_component_outputs::SUMMARY
        } else if loop_proof.is_some() {
            crate::scalar_loop_helpers::SUMMARY
        } else if view {
            crate::helper_views::SUMMARY
        } else if crate::helper_stored_slices::projected(facts, &reference_outputs) {
            crate::helper_stored_slices::SUMMARY
        } else if normal {
            crate::helper_normal_flow::SUMMARY
        } else if memory_reads.is_some() {
            "rstyle.helper.read-only-body/1"
        } else if references {
            "rstyle.helper.shared-return/1"
        } else {
            "rstyle.helper.source-body/1"
        }
        .into(),
        argument_types: argument_types(facts)?,
        result_type: result_type(facts)?,
        mutates_arguments: false,
        memory_effects: memory_reads.is_some(),
        may_panic,
        always_returns: !may_panic && loop_proof.as_ref().is_none_or(|proof| !proof.may_diverge),
        bounded_loop: loop_proof.map(Into::into),
        argument_evaluation: "source-order-once/1".into(),
        result_dependence: guarded_result_dependence(guarded, references).into(),
        reference_outputs,
        component_outputs,
        memory_reads,
        tagged_output: None,
    })
}

fn guarded_result_dependence(guarded: bool, references: bool) -> &'static str {
    if guarded {
        crate::helper_component_outputs::RESULT_DEPENDENCE
    } else {
        result_dependence(references)
    }
}

fn summary_may_panic(
    loop_proof: Option<&crate::scalar_loop_helpers::Checked>,
    pair: producer::PairView<'_>,
    checked_normal: bool,
) -> bool {
    loop_proof.map_or_else(
        || {
            checked_normal
                && pair.cfg.extraction_evidence.status == crate::ExtractionStatus::Partial
        },
        |proof| proof.may_panic,
    )
}

fn result_type(facts: &dyn EvidenceFacts) -> Result<String, String> {
    facts
        .signature
        .return_ty
        .normalized
        .clone()
        .ok_or_else(|| "Missing verified helper result type".into())
}

fn result_dependence(references: bool) -> &'static str {
    if references {
        "verified-formal-referent-substitution/1"
    } else {
        "call-input-snapshots-with-verified-callee-return-graph/1"
    }
}

fn argument_types(facts: &dyn EvidenceFacts) -> Result<Vec<String>, String> {
    facts
        .signature
        .params
        .iter()
        .map(|parameter| {
            parameter
                .ty
                .normalized
                .clone()
                .ok_or_else(|| "Missing verified helper argument type".to_owned())
        })
        .collect()
}

fn identity(facts: &HirFnFacts) -> Option<&str> {
    facts
        .build
        .frontend_options
        .get("definition_identity")?
        .as_str()
}

/// Preserve the original local definition identity after contextual selection.
fn local_resolution(callee: &HirFnFacts) -> Option<Value> {
    Some(serde_json::json!({
        "local": true,
        "path": callee.name,
        "definition_identity": identity(callee)?,
    }))
}

/// Guarded named-error results require their complete family-specific source proof.
pub(crate) fn guarded_error_supported(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if guarded_effects::supported(facts)? {
        return Ok(true);
    }
    if slice_lengths::candidate(facts) {
        slice_lengths::source_contract(facts)?;
        return Ok(true);
    }
    if named_errors::candidate(facts) {
        crate::named_error_payloads::verify_function(facts)?;
        return Ok(true);
    }
    Ok(false)
}

/// Native-width semantics propagate through verified dependencies, even when
/// a caller's own interface and operations have only fixed-width scalar types.
pub fn native_width(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    facts.native_width()
}

pub(crate) fn compute_native_width(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let data = crate::extraction_profile::ExtractionProfile::from_facts(facts)?.is_data();
    let interface = facts
        .signature
        .params
        .iter()
        .try_fold(false, |seen, parameter| {
            native_type(&parameter.ty, data).map(|native| seen | native)
        })?
        | native_type(&facts.signature.return_ty, data)?;

    Ok(interface | node_native_width(&facts.body, data, facts)?)
}

fn native_type(ty: &crate::ResolvedTy, data: bool) -> Result<bool, String> {
    let Some(token) = ty.normalized.as_deref() else {
        return Ok(false);
    };
    if data {
        return crate::data_types::parse(token).map(|token| token.native_width());
    }
    Ok(matches!(token, "isize" | "usize"))
}

fn node_native_width(
    node: &HirNode,
    data: bool,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if crate::option_propagation::candidate(node, facts) {
        let checked = crate::option_propagation::verify(node, facts)?;
        return node_native_width(checked.operand, data, facts);
    }
    if crate::result_propagation::candidate(node, facts) {
        let checked = crate::result_propagation::verify(node, facts)?;
        return node_native_width(checked.operand, data, facts);
    }
    if crate::borrowed_result_propagation::candidate(node, facts) {
        let checked = crate::borrowed_result_propagation::verify(node, facts)?;
        return node_native_width(checked.operand, data, facts);
    }
    if crate::product_result_propagation::candidate(node, facts) {
        let checked = crate::product_result_propagation::verify(node, facts)?;
        return node_native_width(checked.operand, data, facts);
    }
    if crate::named_error_result_propagation::candidate(node, facts) {
        let checked = crate::named_error_result_propagation::verify(node, facts)?;
        return node_native_width(checked.operand, data, facts);
    }
    if crate::scalar_result_propagation::candidate(node, facts) {
        let checked = crate::scalar_result_propagation::verify(node, facts)?;
        return node_native_width(checked.operand, data, facts);
    }

    let own = node
        .ty
        .as_ref()
        .map(|ty| native_type(ty, data))
        .transpose()?
        .unwrap_or(false);
    let children = node.children.iter().try_fold(false, |native, child| {
        node_native_width(child, data, facts).map(|seen| native | seen)
    })?;
    let callee = if selected_resolution(node, facts).is_some() {
        native_dependency(node, facts)?
    } else {
        false
    };

    Ok(own | children | callee)
}

fn native_dependency(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if let Some(helper) = facts.helper(node)? {
        return native_width(&helper.frontend());
    }
    let resolution = direct_resolution(node).ok_or("Missing direct helper resolution")?;
    let dependency: Dependency = transport::decode(&resolution[KEY])
        .map_err(|_| "Missing verified dependency for transitive target-width evidence")?;
    native_width(&dependency.frontend)
}

pub fn direct_resolution(node: &HirNode) -> Option<Value> {
    let direct = node.kind == "expr_call"
        && ((node.res.as_ref()?.kind == "resolved_direct_callee"
            && node.dispatch.as_ref()?.kind == "resolved_direct")
            || (node.res.as_ref()?.kind == "resolved_scalar_instance_callee"
                && node.dispatch.as_ref()?.kind == "resolved_scalar_instance"));
    let inherent = node.kind == "expr_method_call"
        && node.res.as_ref()?.kind == "resolved_inherent_callee"
        && node.dispatch.as_ref()?.kind == "resolved_method";
    let item = crate::named_callbacks::function_item(node)
        && node.dispatch.as_ref()?.kind == "resolved_function_item";
    if (!direct && !inherent && !item) || node.res.as_ref()?.target.is_some() {
        return None;
    }

    serde_json::from_str(&node.res.as_ref()?.description).ok()
}

/// Ordinary compiler resolution or the exact call retained by a private From proof.
pub fn selected_resolution(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Value> {
    if let Some(resolution) = tagged_resolution::selected_resolution(node, facts) {
        return Some(resolution);
    }

    crate::local_value_trait_calls::resolution(node, facts)
        .or_else(|| {
            crate::nominal_boundary::resolution::tryfrom_owners::call_resolution(node, facts)
        })
        .or_else(|| direct_resolution(node))
        .or_else(|| reference_forwarders::resolution(node, facts))
        .or_else(|| product_conversions::resolution(node, facts))
        .or_else(|| slice_lengths::resolution(node, facts))
}

pub(crate) fn callee(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<String> {
    node.dispatch
        .as_ref()
        .map(|dispatch| dispatch.op.clone())
        .or_else(|| reference_forwarders::callee(node, facts))
        .or_else(|| slice_lengths::callee(node, facts))
}

pub(crate) fn call_nodes(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    let mut calls = Vec::new();
    collect_source_kind(&facts.body, "expr_call", facts, &mut calls);
    calls.retain(|node| {
        !crate::call_summaries::static_intrinsic(node, facts)
            && crate::scalar_product_instances::activation::construct(node, facts)
                .ok()
                .flatten()
                .is_none()
            && !crate::checked_options::constructor_candidate(node, facts)
            && !crate::concrete_results::constructor_candidate(node, facts)
            && !crate::data_enums::constructor_candidate(node, facts)
            && !crate::copy_tagged_values::body_operation(node, facts)
    });

    let mut methods = Vec::new();
    collect_source_kind(&facts.body, "expr_method_call", facts, &mut methods);
    calls
        .into_iter()
        .chain(
            methods
                .into_iter()
                .filter(|node| selected_resolution(node, facts).is_some()),
        )
        .chain(
            crate::option_callbacks::invocations(
                facts,
                crate::option_callbacks::Kind::FunctionItem,
            )
            .into_iter()
            .filter_map(|call| call.children.last()),
        )
        .collect()
}

/// Generated Try calls are represented by their independently verified contract;
/// original operand calls still require ordinary helper links and summaries.
fn collect_source_kind<'a>(
    node: &'a HirNode,
    kind: &str,
    facts: &dyn EvidenceFacts,
    calls: &mut Vec<&'a HirNode>,
) {
    if let Some(operand) = propagation_operand(node, facts) {
        collect_source_kind(operand, kind, facts, calls);
        return;
    }
    if node.kind == kind {
        calls.push(node);
    }
    for child in &node.children {
        collect_source_kind(child, kind, facts, calls);
    }
}

fn propagation_operand<'a>(node: &'a HirNode, facts: &dyn EvidenceFacts) -> Option<&'a HirNode> {
    if crate::product_result_propagation::candidate(node, facts) {
        return crate::product_result_propagation::verify(node, facts)
            .ok()
            .map(|checked| checked.operand);
    }
    if crate::named_error_result_propagation::candidate(node, facts) {
        return crate::named_error_result_propagation::verify(node, facts)
            .ok()
            .map(|checked| checked.operand);
    }
    if crate::scalar_result_propagation::candidate(node, facts) {
        return crate::scalar_result_propagation::verify(node, facts)
            .ok()
            .map(|checked| checked.operand);
    }

    if crate::borrowed_result_propagation::candidate(node, facts) {
        return crate::borrowed_result_propagation::verify(node, facts)
            .ok()
            .map(|checked| checked.operand);
    }

    if crate::option_propagation::candidate(node, facts) {
        return crate::option_propagation::verify(node, facts)
            .ok()
            .map(|checked| checked.operand);
    }

    if crate::result_propagation::candidate(node, facts) {
        return crate::result_propagation::verify(node, facts)
            .ok()
            .map(|checked| checked.operand);
    }
    None
}

pub(crate) fn certify_view(
    facts: &dyn EvidenceFacts,
) -> Result<Option<(Summary, producer::Pair)>, String> {
    if digit_products::candidate(facts) {
        return digit_products::certify(facts);
    }

    if !crate::helper_references::supported_result(facts)? {
        return Ok(None);
    }
    let pair = producer::produce(facts).map_err(|errors| errors.join("; "))?;
    let Ok(outputs) = check_boundary(&pair, facts) else {
        return Ok(None);
    };
    Ok(Some((summary(facts, &pair, outputs)?, pair)))
}

pub(crate) enum VerifiedDependency<'a> {
    Shared(crate::unit_store::HelperView<'a>),
    Legacy(Box<Dependency>),
}

impl VerifiedDependency<'_> {
    pub(crate) fn summary(&self) -> &Summary {
        match self {
            Self::Shared(view) => view.summary(),
            Self::Legacy(dependency) => &dependency.summary,
        }
    }

    pub(crate) fn wire(&self) -> Result<Value, String> {
        match self {
            Self::Shared(view) => Ok(view.reference()),
            Self::Legacy(dependency) => transport::encode(dependency),
        }
    }
}

pub(crate) fn verify_use<'a>(
    node: &HirNode,
    caller: &'a dyn EvidenceFacts,
) -> Result<VerifiedDependency<'a>, String> {
    let Some(view) = caller.helper(node)? else {
        return verify(node, caller.facts())
            .map(|dependency| VerifiedDependency::Legacy(Box::new(dependency)));
    };
    verify_shared(node, caller, &view)?;
    Ok(VerifiedDependency::Shared(view))
}

fn verify_shared(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    helper: &crate::unit_store::HelperView<'_>,
) -> Result<(), String> {
    let callee = helper.frontend();
    let selected = guarded_effects::verify_dependency(node, caller, &callee, helper.summary())?
        || slice_lengths::verify_dependency(node, caller, &callee, helper.summary())?
        || named_errors::verify_dependency(node, caller, &callee, helper.summary())?
        || tagged_conversions::verify_dependency(node, caller, &callee, helper.summary())?
        || reference_forwarders::verify(node, caller, &callee)?
        || product_conversions::verify(node, caller, &callee)?;
    let resolution = selected_resolution(node, caller).ok_or("Unresolved direct helper call")?;

    if helper.depth() + 1 > MAX_DEPTH {
        return Err("Helper dependencies exceed explicit depth budget 8".into());
    }
    if !selected {
        check_signature(node, caller, &resolution, &callee)?;
        if resolution["local"] == false {
            external::summary(helper.summary())?;
        }
        crate::inherent_calls::verify(node, caller, &callee, &resolution)?;
    }
    if !helper.verified() {
        verify_shared_callee(helper, &callee)?;
        helper.remember();
    }
    crate::helper_reads::verify_arguments(helper.summary(), node, caller)
}

pub(crate) fn verify_shared_callee(
    helper: &crate::unit_store::HelperView<'_>,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    producer::verify(helper.pair(), callee).map_err(|errors| errors.join("; "))?;
    if digit_products::candidate(callee) {
        return digit_products::verify(callee, helper.pair(), helper.summary());
    }

    let outputs = check_boundary(helper.pair(), callee).map_err(|errors| errors.join("; "))?;
    if (!crate::verification_policy::is_generated()
        || crate::scalar_loop_helpers::has_loop(helper.pair().as_view())
        || tagged_conversions::candidate(callee)
        || helper.summary().tagged_output.is_some())
        && helper.summary() != &summary(callee, helper.pair(), outputs)?
    {
        return Err("Helper summary disagrees with verified callee evidence".into());
    }
    Ok(())
}

fn check_boundary(
    pair: &producer::Pair,
    facts: &dyn EvidenceFacts,
) -> Result<Boundary, Vec<String>> {
    if let Some(output) =
        guarded_effects::derive(facts, pair.as_view()).map_err(|error| vec![error])?
    {
        check_read_only_boundary(pair, facts, &None)?;

        return Ok(Boundary {
            reference_outputs: None,
            tagged_output: Some(output),
        });
    }
    if let Some(output) =
        slice_lengths::derive(facts, pair.as_view()).map_err(|error| vec![error])?
    {
        return Ok(Boundary {
            reference_outputs: None,
            tagged_output: Some(output),
        });
    }
    if let Some(output) =
        named_errors::derive(facts, pair.as_view()).map_err(|error| vec![error])?
    {
        return Ok(Boundary {
            reference_outputs: None,
            tagged_output: Some(output),
        });
    }
    if let Some(output) =
        tagged_conversions::derive(facts, pair.as_view()).map_err(|error| vec![error])?
    {
        return Ok(Boundary {
            reference_outputs: None,
            tagged_output: Some(output),
        });
    }
    Ok(Boundary {
        reference_outputs: check_legacy_boundary(pair, facts)?,
        tagged_output: None,
    })
}

fn check_legacy_boundary(
    pair: &producer::Pair,
    facts: &dyn EvidenceFacts,
) -> Result<ReferenceOutputs, Vec<String>> {
    let outputs = crate::helper_references::returned(facts, pair).map_err(|error| vec![error])?;
    check_read_only_boundary(pair, facts, &outputs)?;
    Ok(outputs)
}

fn check_read_only_boundary(
    pair: &producer::Pair,
    facts: &dyn EvidenceFacts,
    outputs: &ReferenceOutputs,
) -> Result<(), Vec<String>> {
    let loop_proof = crate::scalar_loop_helpers::verify_if_loop(facts, pair.as_view())
        .map_err(|error| vec![error])?;
    let view = crate::helper_views::projected(facts, outputs);
    let byte_reads = crate::extraction_profile::facts_support(facts, "rust.accelerator-packing")
        && crate::helper_normal_flow::supported(facts, pair.as_view());
    let component = crate::helper_component_outputs::supported(facts);
    if component
        && (pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
            || !pair.pdg.memory_locations.is_empty())
        || pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
            && !(pair.cfg.extraction_evidence.status == crate::ExtractionStatus::Partial
                && (view || crate::helper_normal_flow::supported(facts, pair.as_view())))
        || !pair.pdg.memory_locations.is_empty()
            && !crate::helper_reads::supported(facts, pair.as_view())
        || pair.pdg.nodes.iter().any(|node| {
            matches!(node.attributes.construct.as_deref(), Some("loop" | "while"))
                && loop_proof.is_none()
                || matches!(
                    node.attributes.construct.as_deref(),
                    Some("data.bytes.borrow-result" | "data.bytes.unwrap-borrow")
                )
                || matches!(
                    node.attributes.construct.as_deref(),
                    Some("data.bytes.copy-result" | "data.bytes.unwrap")
                ) && !byte_reads
                || node.attributes.construct.as_deref() == Some("data.subslice-place")
                    && !(byte_reads || view && crate::helper_views::array_operation(node))
                || node.attributes.construct.as_deref() == Some("data.index")
                    && !(crate::extraction_profile::facts_support(
                        facts,
                        "rust.indexed-read-helper-calls",
                    ) && crate::helper_normal_flow::copy_context(facts))
                || node.attributes.construct.as_deref() == Some("data.value-index")
                    && !crate::producer::bounded_value_index(pair.as_view(), node, facts)
        })
    {
        return Err(vec![
            "Helper summary requires verified memory-free or registered read-only effects and acyclic normal flow with checked panic/drop boundaries"
                .into(),
        ]);
    }

    Ok(())
}

pub fn verify(node: &HirNode, caller: &HirFnFacts) -> Result<Dependency, String> {
    let evidence = node
        .res
        .as_ref()
        .map(|resolution| resolution.description.as_bytes());
    crate::diagnostics::Stage::new("helper.verify", &caller.name, evidence)
        .observe(|| crate::verification_session::within(|| verify_dependency(node, caller)))
}

fn verify_dependency(node: &HirNode, caller: &HirFnFacts) -> Result<Dependency, String> {
    let description = &node
        .res
        .as_ref()
        .ok_or("Missing helper resolution")?
        .description;
    if description.len() > MAX_BYTES {
        return Err("Helper evidence exceeds explicit 8 MiB per-call budget".into());
    }
    let resolution = direct_resolution(node).ok_or("Unresolved direct helper call")?;
    let dependency: Dependency = transport::decode(&resolution[KEY])
        .map_err(|_| "Missing or invalid verified callee dependency")?;
    if matches!(
        dependency.summary.identity.as_str(),
        slice_lengths::SUMMARY | guarded_effects::SUMMARY
    ) {
        return Err(
            "Checked slice-length summary requires its retained original-owner instance".into(),
        );
    }
    check_depth(&dependency.frontend.body, 1)?;
    if !named_errors::verify_dependency(node, caller, &dependency.frontend, &dependency.summary)?
        && !tagged_conversions::verify_dependency(
            node,
            caller,
            &dependency.frontend,
            &dependency.summary,
        )?
    {
        check_signature(node, caller, &resolution, &dependency.frontend)?;
        crate::inherent_calls::verify(node, caller, &dependency.frontend, &resolution)?;
    }
    if !crate::verification_session::contains(description) {
        verify_callee(&dependency)?;
        crate::verification_session::remember(description);
    } else {
        crate::diagnostics::Stage::new("helper.proof_reused", &dependency.frontend.name, None)
            .observe(|| Ok::<(), String>(()))?;
    }
    crate::helper_reads::verify_arguments(&dependency.summary, node, caller)?;

    Ok(dependency)
}

fn verify_callee(dependency: &Dependency) -> Result<(), String> {
    let serialized = producer::SerializedPairView {
        pdg_bytes: dependency.pdg.as_bytes(),
        cfg_bytes: dependency.cfg.as_bytes(),
        manifest: &dependency.manifest,
    };
    let pair = producer::verify_serialized_view(serialized, &dependency.frontend)
        .map_err(|errors| errors.join("; "))?;
    if digit_products::candidate(&dependency.frontend) {
        return digit_products::verify(&dependency.frontend, &pair, &dependency.summary);
    }
    let outputs =
        check_boundary(&pair, &dependency.frontend).map_err(|errors| errors.join("; "))?;
    if (!crate::verification_policy::is_generated()
        || crate::scalar_loop_helpers::has_loop(pair.as_view())
        || tagged_conversions::candidate(&dependency.frontend)
        || dependency.summary.tagged_output.is_some())
        && dependency.summary != summary(&dependency.frontend, &pair, outputs)?
    {
        return Err("Helper summary signature, effects or returning behavior contradict verified callee evidence".into());
    }

    Ok(())
}

fn check_signature(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    resolution: &Value,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::default_calls::verify_selection(node, caller, callee, resolution)?;
    if callee.scalar_instance().is_some() {
        instances::signature(node, caller, callee)?;
    }
    let signature = if crate::named_callbacks::function_item(node) {
        crate::named_callbacks::signature(node, caller, callee)?;
        true
    } else {
        ordinary_signature(node, caller, callee)?
    };
    if resolution["local"] == false {
        if !signature
            || !external::path(node, resolution, callee)
            || resolution["definition_identity"].as_str() != identity(callee)
        {
            return Err("External helper signature or definition identity disagrees".into());
        }
        return external::signature(node, caller, resolution, callee);
    }
    let build = same_build(caller, callee)?;
    let requested = requested_member(node, caller, resolution, callee)?;
    if !signature
        || !build
        || resolution["local"] != true
        || resolution["path"] != callee.name
        || !requested
        || resolution["definition_identity"].as_str() != identity(callee)
    {
        return Err(
            "Resolved helper identity, signature or semantic build evidence disagrees".into(),
        );
    }

    Ok(())
}

fn requested_member(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    resolution: &Value,
    callee: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing resolved helper signature")?;
    Ok(if resolution.get("generated_tryfrom_requested").is_some() {
        crate::result_maps::trait_leaf::verify_selection(node, caller, callee, resolution)?;
        resolution["generated_tryfrom_requested"].as_str() == Some(dispatch.op.as_str())
    } else if resolution.get("tryfrom_requested").is_some() {
        crate::tryfrom_calls::verify_selection(
            crate::tryfrom_calls::SelectionInput {
                node,
                caller,
                callee,
                resolution,
            },
            &crate::tryfrom_calls::RegistryAuthority,
        )?;
        resolution["tryfrom_requested"].as_str() == Some(dispatch.op.as_str())
    } else if resolution.get("value_trait_requested").is_some() {
        crate::local_value_trait_calls::verify_selection(node, caller, callee, resolution)?;
        resolution["value_trait_requested"].as_str() == Some(dispatch.op.as_str())
    } else {
        dispatch.op == callee.name
    })
}

fn ordinary_signature(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing resolved helper signature")?;
    let arguments: Option<Vec<_>> = callee
        .signature
        .params
        .iter()
        .map(|param| param.ty.normalized.clone())
        .collect();
    let result = callee.signature.return_ty.normalized.as_ref();
    let actual = if arguments.as_deref() == Some(dispatch.operand_tys.as_slice()) {
        std::borrow::Cow::Borrowed(dispatch.operand_tys.as_slice())
    } else {
        crate::owned_receiver::signature_arguments(node, caller, &dispatch.operand_tys)?
    };

    Ok(arguments.as_deref() == Some(actual.as_ref())
        && !dispatch.operand_tys.iter().any(|ty| ty == "unit")
        && result == Some(&dispatch.result_ty)
        && node.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) == result
        && node.children.len() == dispatch.operand_tys.len()
        && node
            .children
            .iter()
            .zip(&dispatch.operand_tys)
            .all(|(child, expected)| {
                child.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) == Some(expected)
            }))
}

pub(crate) fn same_build(
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let profile = crate::extraction_profile::ExtractionProfile::from_facts(caller)?;
    let compatible = caller.build.overflow_checks == callee.build.overflow_checks
        && caller.build.debug_assertions == callee.build.debug_assertions
        && caller.build.panic_strategy == callee.build.panic_strategy
        && caller.build.pointer_width == callee.build.pointer_width
        && profile == crate::extraction_profile::ExtractionProfile::from_facts(callee)?;
    if !profile.is_data() {
        return Ok(compatible);
    }
    Ok(compatible && crate::helper_references::same_unit(caller, callee)?)
}

fn check_depth(node: &HirNode, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("Helper dependencies exceed explicit depth budget 8".into());
    }
    for child in &node.children {
        check_depth(child, depth)?;
    }
    if let Some(resolution) = direct_resolution(node)
        && !resolution[KEY].is_null()
    {
        let dependency: Dependency = transport::decode(&resolution[KEY])
            .map_err(|_| "Corrupt transitive helper dependency")?;
        check_depth(&dependency.frontend.body, depth + 1)?;
    }

    Ok(())
}

/// Independent serialized inventory validation; never invokes caller lowering.
pub fn verify_documents<'a>(
    pair: impl Into<producer::PairView<'a>>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let pair = pair.into();
    crate::verification_session::within(|| verify_inventory(pair, facts))
}

fn verify_inventory(pair: producer::PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    crate::inherent_calls::definition(facts)?;
    let calls = call_nodes(facts);
    let records = pair
        .pdg
        .extensions
        .get(crate::call_summaries::KEY)
        .and_then(Value::as_array)
        .ok_or("Missing serialized call inventory")?;
    if pair.cfg.extensions.get(crate::call_summaries::KEY)
        != pair.pdg.extensions.get(crate::call_summaries::KEY)
    {
        return Err("Paired serialized call inventories disagree".into());
    }
    let mut locations = crate::data_documents::LocationContext::from_pair(pair, facts)?;
    for call in &calls {
        verify_document_call(pair, &mut locations, call, records)?;
    }
    let nodes = serialized_helper_count(pair, records);
    let dependencies = records
        .iter()
        .filter(|record| record.get("dependency").is_some())
        .count();
    if nodes != calls.len() || dependencies != calls.len() {
        return Err(
            "Serialized helper call/dependency inventories are incomplete or duplicated".into(),
        );
    }

    Ok(())
}

fn serialized_helper_count(pair: producer::PairView<'_>, records: &[Value]) -> usize {
    pair.pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == crate::NodeCategory::Call
                && (node.original_kind.known_value() == Some("expr_call")
                    && !matches!(
                        node.attributes.construct.as_deref(),
                        Some(
                            "data.bytes.native-u16"
                                | "data.bytes.native-u128"
                                | "data.bytes.native-u32"
                                | "data.bytes.little-u32"
                                | "data.bytes.native-usize"
                                | "data.scalar-from"
                                | "data.scalar-size"
                                | "data.integer.try-u32-usize"
                                | "data.integer.try-usize-u16"
                                | "data.integer.try-usize-u64"
                                | "data.integer.lossless-try-u32-usize"
                                | "data.integer.lossless-try-u64-usize"
                        )
                    )
                    || records.iter().any(|record| {
                        record["pdg_node_id"] == node.id && record.get("dependency").is_some()
                    }))
        })
        .count()
}

fn verify_document_call(
    pair: producer::PairView<'_>,
    locations: &mut crate::data_documents::LocationContext<'_>,
    call: &HirNode,
    records: &[Value],
) -> Result<(), String> {
    let nodes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == crate::NodeCategory::Call
                && node.original_kind.known_value() == Some(call.kind.as_str())
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(call.span.start) && span.end == u64::from(call.span.end)
                })
        })
        .collect();
    let [node] = nodes.as_slice() else {
        return Err("No unique serialized source-backed helper CALL".into());
    };

    let dependency = verify_use(call, locations.facts)?;
    let inventory: Vec<_> = records
        .iter()
        .filter(|record| record["pdg_node_id"] == node.id)
        .collect();
    let [record] = inventory.as_slice() else {
        return Err("Missing or duplicate serialized helper summary".into());
    };

    crate::helper_reads::verify_effects(node, call, dependency.summary(), locations)?;
    verify_call_metadata(
        SummaryOperands { node, call, record },
        dependency,
        locations.facts,
    )?;
    verify_call_operands(pair, node, call, record)
}

fn verify_call_metadata(
    operands: SummaryOperands<'_>,
    dependency: VerifiedDependency<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let SummaryOperands { node, call, record } = operands;

    if record["dependency"] != dependency.wire()?
        || record["source_span"]
            != serde_json::to_value(call.span).map_err(|error| error.to_string())?
        || node
            .attributes
            .callee
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != callee(call, facts).as_deref()
        || node.attributes.call_form != Some(crate::CallForm::Direct)
    {
        return Err(
            "Serialized helper summary or resolved callee disagrees with captured evidence".into(),
        );
    }

    Ok(())
}

pub(crate) fn verify_call_operands(
    pair: producer::PairView<'_>,
    node: &crate::PdgNode,
    call: &HirNode,
    record: &Value,
) -> Result<(), String> {
    if crate::named_callbacks::function_item(call) {
        return crate::data_documents::named_callbacks::operands(pair, call, node, record);
    }
    verify_argument_values(
        pair,
        node,
        &call.children.iter().collect::<Vec<_>>(),
        record,
    )
}

/// A RangeTo is a checked structural input, represented by its saved endpoint.
pub(crate) struct SummaryOperands<'a> {
    pub node: &'a crate::PdgNode,
    pub call: &'a HirNode,
    pub record: &'a Value,
}

pub(crate) fn verify_summary_operands(
    pair: producer::PairView<'_>,
    operands: SummaryOperands<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if crate::checked_slice_views::suffix::candidate(operands.call, facts) {
        return verify_suffix_summary_operands(pair, operands, facts);
    }
    let SummaryOperands { node, call, record } = operands;

    if !crate::checked_slice_views::candidate(call, facts) {
        return verify_call_operands(pair, node, call, record);
    }
    let checked = crate::checked_slice_views::verify(call, facts)?;

    if node.attributes.construct.as_deref() != Some(crate::checked_slice_views::CONSTRUCT) {
        return Err("Checked prefix summary lacks its precise range-to-end lowering".into());
    }
    crate::data_documents::verify_saved_operands(
        pair,
        node,
        &[
            crate::data_documents::value_operation(pair, checked.receiver)?,
            crate::data_documents::value_operation(pair, checked.end)?,
        ],
    )?;

    verify_argument_values(pair, node, &[checked.receiver, checked.end], record)
}

fn verify_suffix_summary_operands(
    pair: producer::PairView<'_>,
    operands: SummaryOperands<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let SummaryOperands { node, call, record } = operands;
    let checked = crate::checked_slice_views::suffix::verify(call, facts)?;
    if node.attributes.construct.as_deref() != Some(crate::checked_slice_views::suffix::CONSTRUCT) {
        return Err("Checked suffix summary lacks its precise range-to-start lowering".into());
    }

    crate::data_documents::verify_saved_operands(
        pair,
        node,
        &[
            crate::data_documents::value_operation(pair, checked.receiver)?,
            crate::data_documents::value_operation(pair, checked.start)?,
        ],
    )?;

    verify_argument_values(pair, node, &[checked.receiver, checked.start], record)
}

fn verify_argument_values(
    pair: producer::PairView<'_>,
    node: &crate::PdgNode,
    arguments: &[&HirNode],
    record: &Value,
) -> Result<(), String> {
    let operands = record["operand_pdg_node_ids"]
        .as_array()
        .ok_or("Missing serialized helper operands")?;
    if operands.len() != arguments.len() {
        return Err("Serialized helper argument arity disagrees".into());
    }
    let data_count = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .count();
    if data_count != operands.len() {
        return Err("Extra serialized helper argument dependence".into());
    }
    for (position, operand) in operands.iter().enumerate() {
        let edges: Vec<_> = pair
            .pdg
            .edges
            .iter()
            .filter(|edge| {
                edge.target == node.id
                    && edge.kind == crate::EdgeFamily::Data
                    && edge.subkind == "value_operand"
                    && edge.operand_position == Some(position as u64)
            })
            .collect();
        if edges.len() != 1 || operand.as_str() != Some(edges[0].source.as_str()) {
            return Err("Serialized helper argument evaluation/dependence disagrees".into());
        }
        verify_operand(pair, &edges[0].source, arguments[position])?;
    }

    Ok(())
}

pub(crate) fn verify_operand(
    pair: producer::PairView<'_>,
    operand: &str,
    argument: &HirNode,
) -> Result<(), String> {
    let value = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.id == operand)
        .ok_or("Missing helper argument value node")?;
    let span = value
        .source_span
        .clone()
        .or_else(|| {
            serde_json::from_value(
                value
                    .attributes
                    .extensions
                    .get("compat.rust.hir-r0:source-operation")?
                    .get("construct_span")?
                    .clone(),
            )
            .ok()
        })
        .ok_or("Missing source correspondence for helper argument value")?;
    let ty = value
        .annotations
        .get("compat.rust.hir-r0:type")
        .and_then(|fact| fact.get("value"))
        .and_then(Value::as_str);
    if span.start < u64::from(argument.span.start)
        || span.end > u64::from(argument.span.end)
        || ty != argument.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
    {
        return Err("Helper argument value lies outside its source/type correspondence".into());
    }

    Ok(())
}

#[cfg(test)]
mod width_tests {
    use super::*;
    use crate::data_types::TypeToken;

    fn resolved(token: &TypeToken) -> crate::ResolvedTy {
        crate::ResolvedTy {
            normalized: Some(token.text().unwrap()),
            observed: "forensics only".into(),
            spelling: String::new(),
        }
    }

    #[test]
    fn data_native_width_follows_nested_referents_and_elements() {
        let native = TypeToken::Shared(Box::new(TypeToken::Tuple(vec![
            TypeToken::Scalar("u8".into()),
            TypeToken::Array {
                element: Box::new(TypeToken::Scalar("usize".into())),
                length: 2,
            },
        ])));
        assert!(native_type(&resolved(&native), true).unwrap());
        assert!(!native_type(&resolved(&native), false).unwrap());
        let fixed = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
            "u64".into(),
        )))));
        assert!(!native_type(&resolved(&fixed), true).unwrap());
    }

    #[test]
    fn malformed_data_width_evidence_is_an_error() {
        let ty = crate::ResolvedTy {
            normalized: Some("{broken data token".into()),
            observed: "usize".into(),
            spelling: "usize".into(),
        };
        assert!(native_type(&ty, true).is_err());
        assert!(!native_type(&ty, false).unwrap());
    }
}
