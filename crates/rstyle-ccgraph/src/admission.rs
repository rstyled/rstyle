//! Strict production admission: validated PDG + CFG documents into matching inputs.
//!
//! Production matching must go through [`admit_strict`] or
//! [`evaluate_strict_documents`]. Those derive reachability, administrative
//! status, resolved types, reference policy, and merge availability from
//! validated evidence, never from caller booleans. Direct use of
//! [`evaluate_all`](crate::pipeline::evaluate_all) on hand-built
//! [`ImportedFunction`](crate::graph::ImportedFunction) remains available for
//! compact stage fixtures but is explicitly non-certifying: it never
//! establishes extraction eligibility or producer conformance. Producer conformance requires separately verified frontend evidence.

use crate::config::CcgraphConfig;
use crate::graph::{ImportedEdge, ImportedFunction, ImportedNode, InterfaceElement};
use crate::pipeline::{
    Decision, FunctionRecord, PairRecord, RunReport, StageName, StageOutcome, StageRecord,
};
use rstyle_pdg::unit_store::EvidenceFacts;
use rstyle_pdg::{
    CfgDocument, NodeCategory, PdgDocument, ValidationContext, reachable_vertices,
    read_operation_map, validate_cfg, validate_correspondence, validate_paired_agreement,
};
use std::collections::{BTreeMap, BTreeSet};

/// Resolved-type extension carrying the normalized primitive.
pub const RESOLVED_TYPE_KEY: &str = "compat.rust.hir-r0:resolved-type";

/// One function as validated wire documents plus the bytes and build facts
/// needed to check them independently.
pub struct StrictBundle<'a> {
    pub pdg: &'a PdgDocument,
    pub cfg: &'a CfgDocument,
    pub source_bytes: &'a BTreeMap<String, Vec<u8>>,
    pub ctx: &'a ValidationContext,
}

/// Narrow producer handoff: verify the captured HIR/source/build evidence,
/// then use the unchanged strict imported-document admission boundary.
pub fn admit_produced(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn EvidenceFacts,
) -> Result<ImportedFunction, Vec<String>> {
    rstyle_pdg::producer::verify(pair, facts)?;
    let source_bytes = rstyle_pdg::validate::CapturedSources::new(
        &pair.pdg.sources,
        facts.source.bytes.as_bytes(),
    );
    let width = facts
        .build
        .pointer_width
        .value
        .and_then(|v| u8::try_from(v).ok())
        .ok_or_else(|| vec!["Missing captured target width".to_string()])?;
    let ctx = ValidationContext::CompilerEvidence {
        pointer_width_bits: width,
    };

    admit_checked(
        &AdmissionInput {
            pdg: &pair.pdg,
            cfg: &pair.cfg,
            source_bytes: &source_bytes,
            ctx: &ctx,
        },
        Some(facts),
    )
}

/// Admit one validated document pair into a matching input.
///
/// Every check must pass: PDG eligibility, clean CFG validation, identical
/// operation maps and paired-graph links in both documents, and complete
/// correspondence. Interface types, roles, variadic facts, reference counts,
/// names, and keys come from the documents; reachability comes from the paired
/// CFG entry; administrative status comes from boundary cleanliness. Failures
/// are returned as reason strings and never coerce missing evidence.
pub fn admit_strict(bundle: &StrictBundle) -> Result<ImportedFunction, Vec<String>> {
    admit_checked(
        &AdmissionInput {
            pdg: bundle.pdg,
            cfg: bundle.cfg,
            source_bytes: bundle.source_bytes,
            ctx: bundle.ctx,
        },
        None,
    )
}

struct AdmissionInput<'a> {
    pdg: &'a PdgDocument,
    cfg: &'a CfgDocument,
    source_bytes: &'a dyn rstyle_pdg::validate::SourceBytes,
    ctx: &'a ValidationContext,
}

struct AdmissionContext<'a> {
    validation: &'a ValidationContext,
    frontend: Option<&'a dyn EvidenceFacts>,
}

fn admit_checked(
    bundle: &AdmissionInput,
    frontend: Option<&dyn EvidenceFacts>,
) -> Result<ImportedFunction, Vec<String>> {
    let AdmissionInput {
        pdg,
        cfg,
        source_bytes,
        ctx,
    } = bundle;

    require_compiler_evidence(ctx)?;
    let imported = if frontend.is_some() {
        check_local_profile(pdg, cfg)?;
        None
    } else {
        check_local_producer(pdg, cfg)?
    };
    let frontend = frontend.or_else(|| imported.as_ref().map(|facts| facts as &dyn EvidenceFacts));
    check_pdg_eligibility(pdg, ctx, *source_bytes)?;

    let admitted = AdmittedPair { pdg, cfg };
    admitted.check_cfg()?;
    let map = admitted.check_agreement()?;
    admitted.check_correspondence(&map)?;

    assemble_function(
        pdg,
        cfg,
        &map,
        &AdmissionContext {
            validation: ctx,
            frontend,
        },
    )
}

fn check_local_profile(pdg: &PdgDocument, cfg: &CfgDocument) -> Result<bool, Vec<String>> {
    use rstyle_pdg::extraction_profile::LOCAL_ID;
    if pdg.profile.id != LOCAL_ID
        && pdg.profile.id != "rstyle.rust.hir-helpers"
        && pdg.profile.id != rstyle_pdg::extraction_profile::DATA_ID
    {
        return Ok(false);
    }
    if cfg.profile.name != pdg.profile.id || cfg.profile.version != pdg.profile.version {
        return Err(vec!["Paired extraction profiles disagree".into()]);
    }
    Ok(true)
}

fn check_local_producer(
    pdg: &PdgDocument,
    cfg: &CfgDocument,
) -> Result<Option<rstyle_pdg::HirFnFacts>, Vec<String>> {
    use rstyle_pdg::extraction_profile::EVIDENCE_KEY;
    if !check_local_profile(pdg, cfg)? {
        return Ok(None);
    }
    let evidence = pdg
        .extensions
        .get(EVIDENCE_KEY)
        .ok_or_else(|| vec!["Missing local producer frontend evidence".into()])?;
    if cfg.extensions.get(EVIDENCE_KEY) != Some(evidence) {
        return Err(vec!["Paired local frontend evidence disagrees".into()]);
    }
    let facts: rstyle_pdg::HirFnFacts = serde::Deserialize::deserialize(&evidence["frontend"])
        .map_err(|e| vec![format!("Invalid local frontend evidence: {e}")])?;
    let pair = rstyle_pdg::producer::PairView { pdg, cfg };

    rstyle_pdg::producer::verify_view(pair, &facts)?;
    Ok(Some(facts))
}

struct AdmittedPair<'a> {
    pdg: &'a PdgDocument,
    cfg: &'a CfgDocument,
}

fn require_compiler_evidence(ctx: &ValidationContext) -> Result<(), Vec<String>> {
    match ctx {
        ValidationContext::CompilerEvidence { .. } => Ok(()),
        ValidationContext::FixtureOnly { .. } => Err(vec![
            "fixture-only context cannot establish strict admission".to_string(),
        ]),
    }
}

fn check_pdg_eligibility(
    pdg: &PdgDocument,
    ctx: &ValidationContext,
    source_bytes: &dyn rstyle_pdg::validate::SourceBytes,
) -> Result<(), Vec<String>> {
    let report = rstyle_pdg::validate::validate_sources(pdg, ctx, source_bytes);

    if report.matching == rstyle_pdg::Eligibility::Eligible {
        return Ok(());
    }

    Err(report.issues.into_iter().map(|i| i.message).collect())
}

impl AdmittedPair<'_> {
    fn check_cfg(&self) -> Result<(), Vec<String>> {
        let issues = validate_cfg(self.cfg);

        if issues.is_empty() {
            return Ok(());
        }

        Err(issues.into_iter().map(|i| i.message).collect())
    }

    fn check_agreement(&self) -> Result<Vec<rstyle_pdg::OperationMapEntry>, Vec<String>> {
        let issues = validate_paired_agreement(&self.pdg.extensions, &self.cfg.extensions);

        if !issues.is_empty() {
            return Err(issues.into_iter().map(|i| i.message).collect());
        }

        read_operation_map(&self.pdg.extensions).map_err(|e| vec![e])
    }

    fn check_correspondence(
        &self,
        map: &[rstyle_pdg::OperationMapEntry],
    ) -> Result<(), Vec<String>> {
        let pdg_nodes: BTreeSet<String> = self.pdg.nodes.iter().map(|n| n.id.clone()).collect();
        let issues = validate_correspondence(map, self.cfg, &pdg_nodes);

        if issues.is_empty() {
            return Ok(());
        }

        Err(issues.into_iter().map(|i| i.message).collect())
    }
}

fn assemble_function(
    pdg: &PdgDocument,
    cfg: &CfgDocument,
    map: &[rstyle_pdg::OperationMapEntry],
    ctx: &AdmissionContext,
) -> Result<ImportedFunction, Vec<String>> {
    let reachability = node_reachability(cfg, map);
    let value_sites = rstyle_pdg::validate_value_sites(pdg, cfg)?;
    let nodes = import_nodes(pdg, &reachability, &value_sites);
    let data = pdg.profile.id == rstyle_pdg::extraction_profile::DATA_ID;
    let inputs = import_interface(&pdg.function.inputs, data);
    let outputs = import_interface(&pdg.function.outputs, data);
    let name = import_name(pdg);
    let reference = import_reference(pdg);

    let errors = [
        nodes.as_ref().err().cloned().unwrap_or_default(),
        inputs.as_ref().err().cloned().unwrap_or_default(),
        outputs.as_ref().err().cloned().unwrap_or_default(),
        name.as_ref()
            .err()
            .map(|single| vec![single.clone()])
            .unwrap_or_default(),
        reference.as_ref().err().cloned().unwrap_or_default(),
    ]
    .concat();

    if !errors.is_empty() {
        return Err(errors);
    }

    let key = import_key(pdg);
    let (width, native) = import_widths(pdg, ctx)?;

    Ok(ImportedFunction {
        matching_profile: imported_matching_profile(pdg),
        extraction_context: local_context(pdg)?,
        key,
        nodes: nodes.unwrap_or_default(),
        edges: import_edges(pdg),
        inputs: inputs.unwrap_or_default(),
        outputs: outputs.unwrap_or_default(),
        name: name.ok(),
        reference_count: reference.as_ref().ok().copied(),
        target_pointer_width: width,
        native_width_involved: native,
    })
}

fn imported_matching_profile(pdg: &PdgDocument) -> crate::graph::MatchingProfile {
    use crate::graph::MatchingProfile;
    match (pdg.profile.id.as_str(), pdg.profile.version.as_str()) {
        (rstyle_pdg::extraction_profile::DATA_ID, "2.0.0" | "3.0.0" | "3.1.0" | "3.2.0") => {
            MatchingProfile::DataCallsV1
        }
        (rstyle_pdg::extraction_profile::DATA_ID, _) => MatchingProfile::DataV1,
        _ => MatchingProfile::ScalarV1,
    }
}

fn local_context(pdg: &PdgDocument) -> Result<Option<String>, Vec<String>> {
    if pdg.profile.id != rstyle_pdg::extraction_profile::LOCAL_ID
        && pdg.profile.id != "rstyle.rust.hir-helpers"
        && pdg.profile.id != rstyle_pdg::extraction_profile::DATA_ID
    {
        return Ok(None);
    }
    let build = pdg
        .producer
        .configuration
        .get(rstyle_pdg::producer::matching_build::KEY)
        .ok_or_else(|| vec!["Missing local matching build".into()])?;
    let settings =
        rstyle_pdg::producer::matching_build::settings(build).map_err(|error| vec![error])?;
    // Compiler/target identities remain in the owning frontend. Native widths
    // are checked separately only when the function uses native types.
    let context = if pdg.profile.id == rstyle_pdg::extraction_profile::DATA_ID {
        serde_json::json!([
            pdg.profile,
            pdg.extensions.get(rstyle_pdg::data_documents::POLICY_KEY),
            settings
        ])
    } else {
        serde_json::json!([pdg.profile, settings])
    };

    Ok(Some(context.to_string()))
}

/// PDG node id to CFG reachability, derived from the paired CFG entry through
/// the operation map. Unmapped boundary and value entities are unreachable by
/// construction; unmapped operations are an admission error recorded later.
fn node_reachability(
    cfg: &CfgDocument,
    map: &[rstyle_pdg::OperationMapEntry],
) -> BTreeMap<String, bool> {
    let reachable = reachable_vertices(cfg);
    let node_of_operation: BTreeMap<&str, &str> = map
        .iter()
        .map(|entry| (entry.cfg_operation_id.as_str(), entry.cfg_node_id.as_str()))
        .collect();
    let mut out = BTreeMap::new();

    for entry in map {
        let cfg_node = node_of_operation[entry.cfg_operation_id.as_str()];
        out.insert(entry.pdg_node_id.clone(), reachable.contains(cfg_node));
    }

    out
}

/// Partition fallible imports into values or the collected reasons.
/// Shared by node and interface import; missing evidence is never dropped.
fn collect_results<T, E>(results: impl Iterator<Item = Result<T, E>>) -> Result<Vec<T>, Vec<E>> {
    let mut values = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok(value) => values.push(value),
            Err(message) => errors.push(message),
        }
    }

    if errors.is_empty() {
        Ok(values)
    } else {
        Err(errors)
    }
}

fn import_nodes(
    pdg: &PdgDocument,
    reachability: &BTreeMap<String, bool>,
    value_sites: &BTreeSet<String>,
) -> Result<Vec<ImportedNode>, Vec<String>> {
    collect_results(pdg.nodes.iter().map(|node| {
        import_node(
            node,
            reachability,
            value_sites,
            pdg.profile.id == rstyle_pdg::extraction_profile::DATA_ID,
        )
    }))
}

fn import_node(
    node: &rstyle_pdg::PdgNode,
    reachability: &BTreeMap<String, bool>,
    value_sites: &BTreeSet<String>,
    data: bool,
) -> Result<ImportedNode, String> {
    let procedural = matches!(
        node.category,
        NodeCategory::FormalInput
            | NodeCategory::Merge
            | NodeCategory::Entry
            | NodeCategory::Exit
            | NodeCategory::FormalOutput
    );

    let reachable = match reachability.get(&node.id) {
        Some(reached) => *reached,
        None if procedural => false,
        None => {
            return Err(format!(
                "PDG node {:?} lacks an operation-map triple",
                node.id
            ));
        }
    };

    Ok(ImportedNode {
        id: node.id.clone(),
        category: node.category,
        administrative: is_administrative(node),
        reachable,
        defines_binding: !node.attributes.defines.is_empty(),
        reads_or_writes: !node.attributes.reads.is_empty() || !node.attributes.writes.is_empty(),
        value_site_reachable: value_sites.contains(&node.id),
        read_symbols: if data {
            rstyle_pdg::data_documents::read_keys(node)
        } else {
            read_symbols(node)
        },
        registered_meaning: node.attributes.description.clone(),
    })
}

/// Administrative boundaries are proven from the absence of binding semantics,
/// never from a caller flag.
fn is_administrative(node: &rstyle_pdg::PdgNode) -> bool {
    matches!(node.category, NodeCategory::Entry | NodeCategory::Exit)
        && node.attributes.symbol_ids.is_empty()
        && node.attributes.defines.is_empty()
        && node.attributes.reads.is_empty()
        && node.attributes.writes.is_empty()
}

fn read_symbols(node: &rstyle_pdg::PdgNode) -> Vec<String> {
    rstyle_pdg::collection::collect_present(&node.attributes.reads, |access| {
        access.symbol_id.clone()
    })
}

fn import_edges(pdg: &PdgDocument) -> Vec<ImportedEdge> {
    pdg.edges
        .iter()
        .map(|edge| ImportedEdge {
            source: edge.source.clone(),
            target: edge.target.clone(),
            family: edge.kind,
            origin: edge.id.clone(),
        })
        .collect()
}

fn import_interface(
    params: &[rstyle_pdg::Parameter],
    data: bool,
) -> Result<Vec<InterfaceElement>, Vec<String>> {
    convert_params(&sorted_params(params), data)
}

fn sorted_params(params: &[rstyle_pdg::Parameter]) -> Vec<rstyle_pdg::Parameter> {
    let mut ordered = params.to_vec();
    ordered.sort_by_key(|param| param.position);
    ordered
}

fn convert_params(
    ordered: &[rstyle_pdg::Parameter],
    data: bool,
) -> Result<Vec<InterfaceElement>, Vec<String>> {
    collect_results(
        ordered
            .iter()
            .map(|parameter| import_element(parameter, data)),
    )
}

fn import_element(param: &rstyle_pdg::Parameter, data: bool) -> Result<InterfaceElement, String> {
    let normalized = resolve_type(param, data)?;

    if param.variadic.value != Some(false) || param.variadic.status != rstyle_pdg::FactStatus::Known
    {
        return Err(format!(
            "interface position {}: variadic must be known false",
            param.position
        ));
    }

    Ok(InterfaceElement {
        role: param_role_token(param.role).to_string(),
        normalized_type: normalized,
        variadic: false,
    })
}

fn param_role_token(role: rstyle_pdg::ParamRole) -> &'static str {
    match role {
        rstyle_pdg::ParamRole::Parameter => "parameter",
        rstyle_pdg::ParamRole::Receiver => "receiver",
        rstyle_pdg::ParamRole::Capture => "capture",
        rstyle_pdg::ParamRole::Return => "return",
        rstyle_pdg::ParamRole::Output => "output",
    }
}

/// Normalized types come from the resolved-type extension or the known type
/// fact with `()` normalized to `unit`. Alias spellings are never rewritten.
fn resolve_type(param: &rstyle_pdg::Parameter, data: bool) -> Result<String, String> {
    if let Some(extension) = param.extensions.get(RESOLVED_TYPE_KEY) {
        match resolved_extension_value(extension).as_deref() {
            Some(seen)
                if is_accepted_type(seen)
                    || data && rstyle_pdg::data_types::parse(seen).is_ok() =>
            {
                return Ok(seen.to_string());
            }
            Some(seen) => {
                return Err(format!(
                    "interface position {}: unsupported resolved type {seen:?}",
                    param.position
                ));
            }
            None => {
                return Err(format!(
                    "interface position {}: resolved-type extension must be a known string fact",
                    param.position
                ));
            }
        }
    }

    match param.type_fact.known_value() {
        Some("()") => Ok("unit".to_string()),
        Some(seen)
            if is_accepted_type(seen) || data && rstyle_pdg::data_types::parse(seen).is_ok() =>
        {
            Ok(seen.to_string())
        }
        Some(seen) => Err(format!(
            "interface position {}: unsupported type {seen:?}",
            param.position
        )),
        None => Err(format!(
            "interface position {}: type must be known",
            param.position
        )),
    }
}

fn resolved_extension_value(value: &serde_json::Value) -> Option<String> {
    let fact: rstyle_pdg::StringFact = serde_json::from_value(value.clone()).ok()?;

    fact.known_value().map(str::to_string)
}

fn is_accepted_type(seen: &str) -> bool {
    matches!(
        seen,
        "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "unit"
    )
}

fn import_name(pdg: &PdgDocument) -> Result<String, String> {
    match pdg.function.name.known_value() {
        Some(name) if !name.is_empty() => Ok(name.to_string()),
        _ => Err("function name must be known and nonempty".to_string()),
    }
}

fn import_reference(pdg: &PdgDocument) -> Result<u64, Vec<String>> {
    let count = &pdg.function.reference_count;

    match (count.status, count.value) {
        (rstyle_pdg::FactStatus::Known, Some(seen)) => Ok(seen),
        _ => Err(vec!["reference count must be known".to_string()]),
    }
}

fn import_key(pdg: &PdgDocument) -> String {
    pdg.extensions
        .get(rstyle_pdg::PAIRED_GRAPH_KEY)
        .and_then(|link| link.get("pdg_document_key"))
        .and_then(|key| key.as_str())
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| pdg.function.id.clone())
}

fn import_widths(
    pdg: &PdgDocument,
    ctx: &AdmissionContext,
) -> Result<(Option<u16>, bool), Vec<String>> {
    let width = match ctx.validation {
        ValidationContext::CompilerEvidence { pointer_width_bits } => {
            Some(*pointer_width_bits as u16)
        }
        ValidationContext::FixtureOnly { .. } => None,
    };
    let native = pdg
        .function
        .inputs
        .iter()
        .chain(pdg.function.outputs.iter())
        .filter_map(|param| param.type_fact.known_value())
        .any(|seen| seen == "isize" || seen == "usize");

    let local_native = (pdg.profile.id == rstyle_pdg::extraction_profile::LOCAL_ID
        || pdg.profile.id == "rstyle.rust.hir-helpers")
        && pdg.nodes.iter().any(|n| {
            n.annotations
                .get("compat.rust.hir-r0:type")
                .and_then(|v| v.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|ty| matches!(ty, "isize" | "usize"))
        });
    let data_native = pdg.profile.id == rstyle_pdg::extraction_profile::DATA_ID
        && pdg.nodes.iter().any(|node| {
            node.annotations
                .get("compat.rust.hir-r0:type")
                .and_then(|value| value.get("value"))
                .and_then(serde_json::Value::as_str)
                .and_then(|token| rstyle_pdg::data_types::parse(token).ok())
                .is_some_and(|token| token.native_width())
        });
    let transitive_native = helper_native_width(pdg, ctx.frontend)?;

    Ok((
        width,
        native || local_native || data_native || transitive_native,
    ))
}

fn helper_native_width(
    pdg: &PdgDocument,
    frontend: Option<&dyn EvidenceFacts>,
) -> Result<bool, Vec<String>> {
    if !rstyle_pdg::extraction_profile::policy_supports(&pdg.profile, "rust.verified-helper-calls")
    {
        return Ok(false);
    }
    if let Some(facts) = frontend {
        return rstyle_pdg::helper_calls::native_width(facts).map_err(|error| vec![error]);
    }
    let evidence = pdg
        .extensions
        .get(rstyle_pdg::extraction_profile::EVIDENCE_KEY)
        .ok_or_else(|| vec!["Missing helper frontend evidence for target compatibility".into()])?;
    let facts: rstyle_pdg::HirFnFacts = serde::Deserialize::deserialize(&evidence["frontend"])
        .map_err(|error| vec![error.to_string()])?;

    rstyle_pdg::helper_calls::native_width(&facts).map_err(|error| vec![error])
}

/// Evaluate validated document pairs through the real admission path.
///
/// Admitted functions run through the standard gate pipeline. Pairs touching
/// a document that fails admission are reported as validation-ineligible with
/// the admission reasons; later stages stay `not_evaluated`. The run is
/// marked incomplete only for arithmetic errors, never for admission
/// ineligibility. Producer conformance is never claimed.
pub fn evaluate_strict_documents(bundles: &[StrictBundle], config: &CcgraphConfig) -> RunReport {
    use crate::pipeline::evaluate_all;

    let (admitted, refused) = partition_bundles(bundles);
    let mut report = evaluate_all(&admitted, config);
    merge_refused(&mut report, refused);

    report
}

fn partition_bundles(
    bundles: &[StrictBundle],
) -> (Vec<ImportedFunction>, Vec<(String, Vec<String>)>) {
    let mut admitted = Vec::new();
    let mut refused: Vec<(String, Vec<String>)> = Vec::new();

    for bundle in bundles {
        let fallback = bundle.pdg.function.id.clone();

        match admit_strict(bundle) {
            Ok(function) => admitted.push(function),
            Err(reasons) => refused.push((import_key(bundle.pdg).if_empty(fallback), reasons)),
        }
    }

    (admitted, refused)
}

fn merge_refused(report: &mut RunReport, refused: Vec<(String, Vec<String>)>) {
    let refused_set = refused_keys(&refused);

    for (key, reasons) in refused {
        report.functions.push(FunctionRecord {
            key: key.clone(),
            matching_profile: None,
            eligible: false,
            reasons,
            projection: None,
        });
    }

    add_refused_pairs(report, &refused_set);
}

trait IfEmpty {
    fn if_empty(self, fallback: String) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, fallback: String) -> String {
        if self.is_empty() { fallback } else { self }
    }
}

fn refused_keys(refused: &[(String, Vec<String>)]) -> BTreeSet<String> {
    refused.iter().map(|(key, _)| key.clone()).collect()
}

fn add_refused_pairs(report: &mut RunReport, refused: &BTreeSet<String>) {
    if refused.is_empty() {
        return;
    }

    let mut keys: Vec<String> = report
        .functions
        .iter()
        .map(|function| function.key.clone())
        .collect();
    keys.sort();

    let reasons_of = |key: &str| {
        report
            .functions
            .iter()
            .find(|function| function.key == key)
            .map(|function| function.reasons.join("; "))
            .unwrap_or_default()
    };

    for (first_index, first) in keys.iter().enumerate() {
        for second in keys.iter().skip(first_index + 1) {
            if !refused.contains(first) && !refused.contains(second) {
                continue;
            }

            if report
                .pairs
                .iter()
                .any(|pair| pair.first == *first && pair.second == *second)
            {
                continue;
            }

            report.pairs.push(refused_pair(
                first,
                second,
                &reasons_of(first),
                &reasons_of(second),
            ));
        }
    }

    report.pairs.sort_by(|a, b| {
        (a.first.clone(), a.second.clone()).cmp(&(b.first.clone(), b.second.clone()))
    });
}

fn refused_pair(
    first: &str,
    second: &str,
    first_reasons: &str,
    second_reasons: &str,
) -> PairRecord {
    let mut reasons = Vec::new();

    if !first_reasons.is_empty() {
        reasons.push(format!("{first}: {first_reasons}"));
    }

    if !second_reasons.is_empty() {
        reasons.push(format!("{second}: {second_reasons}"));
    }

    let mut stages = vec![StageRecord {
        stage: StageName::Validation,
        outcome: StageOutcome::Ineligible {
            reason: reasons.join("; "),
        },
        detail: BTreeMap::new(),
    }];

    for stage in [
        StageName::Projection,
        StageName::Nonempty,
        StageName::Numerical,
        StageName::Strings,
        StageName::WlFinal,
    ] {
        stages.push(StageRecord {
            stage,
            outcome: StageOutcome::NotEvaluated,
            detail: BTreeMap::new(),
        });
    }

    PairRecord {
        first: first.to_string(),
        second: second.to_string(),
        decision: Decision::Rejected,
        displayed_score: None,
        stages,
    }
}
