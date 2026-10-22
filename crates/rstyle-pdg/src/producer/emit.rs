//! Schema-faithful paired documents and an external byte-identity manifest.

use super::{Analysis, Dependence, Graph, Operation};
use crate::unit_store::EvidenceFacts;
use crate::{cfg::*, frontend::*, model::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod identities;

pub const PREFIX: &str = "compat.rust.hir-r0";
pub const TYPE_KEY: &str = "compat.rust.hir-r0:type";
pub const GUARDS_KEY: &str = "compat.rust.hir-r0:guards";
pub const SITES_KEY: &str = "compat.rust.hir-r0:value-sites";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentIdentity {
    pub key: String,
    #[serde(alias = "sha256")]
    pub checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub documents: Vec<DocumentIdentity>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub pdg: PdgDocument,
    pub cfg: CfgDocument,
}

/// Borrow paired typed documents without copying their evidence. This view
/// grants no trust; ordinary paired/source/policy checks still apply.
#[derive(Clone, Copy, Serialize)]
pub struct PairView<'a> {
    pub pdg: &'a PdgDocument,
    pub cfg: &'a CfgDocument,
}

impl Pair {
    pub fn as_view(&self) -> PairView<'_> {
        PairView {
            pdg: &self.pdg,
            cfg: &self.cfg,
        }
    }
}

impl<'a> From<&'a Pair> for PairView<'a> {
    fn from(pair: &'a Pair) -> Self {
        pair.as_view()
    }
}

impl<'a> From<&'a mut Pair> for PairView<'a> {
    fn from(pair: &'a mut Pair) -> Self {
        pair.as_view()
    }
}

impl<'a> From<&PairView<'a>> for PairView<'a> {
    fn from(pair: &PairView<'a>) -> Self {
        *pair
    }
}

pub struct SerializedPair {
    pub pdg_bytes: Vec<u8>,
    pub cfg_bytes: Vec<u8>,
    pub manifest: Manifest,
}

/// Exact owned-byte snapshot borrowed for verification. The manifest and both
/// documents must come from the same load; constructing this view grants no trust.
#[derive(Clone, Copy)]
pub struct SerializedPairView<'a> {
    pub pdg_bytes: &'a [u8],
    pub cfg_bytes: &'a [u8],
    pub manifest: &'a Manifest,
}

struct DocumentMetadata {
    producer: Producer,
    extensions: Extensions,
    limitations: Vec<String>,
}

#[derive(Serialize)]
struct OperationProvenance {
    cfg_operation_id: String,
    provenance: Provenance,
    construct_span: Span,
    role: String,
}

#[derive(Serialize)]
struct Reachability {
    entry_reachable_cfg_node_ids: BTreeSet<String>,
}

#[derive(Serialize)]
struct WireAlternative<'a> {
    value_pdg_node_id: &'a str,
    outcome: &'a str,
    completion_cfg_node_ids: &'a [String],
    annotations: Extensions,
}

#[derive(Serialize)]
struct WireSite<'a> {
    #[serde(
        rename = "rstyle.hir-coverage:selection-kind",
        skip_serializing_if = "Option::is_none"
    )]
    selection_kind: &'a Option<String>,
    pdg_node_id: &'a str,
    construct_span: Span,
    controller_pdg_node_id: &'a str,
    cfg_successor_node_id: &'a str,
    annotations: Extensions,
    alternatives: Vec<WireAlternative<'a>>,
}

impl SerializedPair {
    pub fn as_view(&self) -> SerializedPairView<'_> {
        SerializedPairView {
            pdg_bytes: &self.pdg_bytes,
            cfg_bytes: &self.cfg_bytes,
            manifest: &self.manifest,
        }
    }

    pub fn verify_identities(&self, pair: &Pair) -> Result<(), String> {
        self.as_view().verify_identities(pair)
    }
}

impl SerializedPairView<'_> {
    pub fn verify_identities(&self, pair: &Pair) -> Result<(), String> {
        let links = read_paired_graph(&pair.pdg.extensions)?;
        identities::verify_manifest(
            self.manifest,
            [
                (links.pdg_document_key, self.pdg_bytes),
                (links.cfg_document_key, self.cfg_bytes),
            ],
        )?;

        Ok(())
    }
}

impl Pair {
    pub fn serialize(&self) -> Result<SerializedPair, serde_json::Error> {
        let pdg_bytes = serde_json::to_vec(&self.pdg)?;
        let cfg_bytes = serde_json::to_vec(&self.cfg)?;
        let links = read_paired_graph(&self.pdg.extensions).expect("producer paired links");
        let manifest = Manifest {
            documents: vec![
                identities::identity(links.pdg_document_key, &pdg_bytes),
                identities::identity(links.cfg_document_key, &cfg_bytes),
            ],
        };

        Ok(SerializedPair {
            pdg_bytes,
            cfg_bytes,
            manifest,
        })
    }
}

pub fn emit(
    facts: &dyn EvidenceFacts,
    graph: &Graph,
    analysis: &Analysis,
    revision: &str,
) -> Result<Pair, String> {
    let assessment = super::hazards::assess(facts, graph, analysis)?;
    let producer = producer(facts, revision)?;
    let mut extensions = extensions(facts, graph, analysis)?;
    if crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.integer-casts")
    {
        extensions.insert(
            super::casts::KEY.into(),
            super::casts::evidence(facts, graph)?,
        );
    }
    extensions.insert(
        format!("{PREFIX}:exceptional-flow-assessment"),
        serde_json::to_value(&assessment).map_err(|e| e.to_string())?,
    );
    if crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-primitive-summaries")
    {
        extensions.insert(
            crate::call_summaries::KEY.into(),
            super::summaries::evidence(facts, graph)?,
        );
    }
    let metadata = DocumentMetadata {
        producer,
        extensions,
        limitations: assessment.limitations,
    };
    let pdg = pdg_document(facts, graph, analysis, &metadata);
    let cfg = cfg_document(facts, graph, metadata);

    super::profile::apply(Pair { pdg, cfg }, facts)
}

fn pdg_document(
    facts: &dyn EvidenceFacts,
    graph: &Graph,
    analysis: &Analysis,
    metadata: &DocumentMetadata,
) -> PdgDocument {
    PdgDocument {
        schema_version: PDG_SCHEMA_VERSION.to_string(),
        profile: ProfileId {
            id: R0_POLICY_ID.to_string(),
            version: R0_POLICY_VERSION.to_string(),
        },
        producer: metadata.producer.clone(),
        sources: vec![Source {
            id: "source".to_string(),
            path: source_path(facts),
            content_identity: StringFact::known(&facts.source.digest),
        }],
        function: function(facts, graph, analysis),
        symbols: facts.bindings.iter().map(symbol).collect(),
        definitions: definitions(graph),
        memory_locations: Vec::new(),
        nodes: graph.nodes.iter().map(|n| pdg_node(n, graph)).collect(),
        edges: analysis
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| pdg_edge(i, e))
            .collect(),
        extraction_evidence: extraction(graph, &metadata.limitations),
        extensions: metadata.extensions.clone(),
    }
}

fn producer(facts: &dyn EvidenceFacts, revision: &str) -> Result<Producer, String> {
    let build = &facts.build;
    let required = [
        &build.producer_revision,
        &build.rustc_version,
        &build.rustc_commit,
        &build.edition,
        &build.target_triple,
        &build.data_layout,
        &build.panic_strategy,
        &build.opt_level,
    ];
    if revision.is_empty()
        || required
            .iter()
            .any(|f| f.known_value().is_none_or(str::is_empty))
        || build.pointer_width.status != FactStatus::Known
        || !matches!(build.pointer_width.value, Some(32 | 64))
        || build.overflow_checks.status != FactStatus::Known
        || build.overflow_checks.value.is_none()
        || build.debug_assertions.status != FactStatus::Known
        || build.debug_assertions.value.is_none()
        || !frontend_options_complete(build)
    {
        return Err(
            "Required captured compiler/build/producer evidence is unavailable".to_string(),
        );
    }
    let configuration = BTreeMap::from([
        ("producer_revision".to_string(), json!(revision)),
        (
            super::matching_build::KEY.to_string(),
            super::matching_build::capture(build),
        ),
        (
            "normal_flow_scope".to_string(),
            json!("compat.rust.hir-r0/0.1.0; exceptional flow omitted"),
        ),
    ]);

    Ok(Producer {
        name: "rstyle-hir-extraction".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: "Rust".to_string(),
        language_version: build.rustc_version.clone(),
        configuration,
    })
}

fn frontend_options_complete(build: &BuildFacts) -> bool {
    build
        .frontend_options
        .get("edition")
        .and_then(Value::as_str)
        == build.edition.known_value()
        && build
            .frontend_options
            .get("target")
            .and_then(Value::as_str)
            .is_some_and(|target| !target.is_empty())
        && build
            .frontend_options
            .get("invocation")
            .and_then(Value::as_array)
            .is_some_and(|arguments| {
                !arguments.is_empty() && arguments.iter().all(Value::is_string)
            })
}

fn span(span: ByteSpan) -> Span {
    Span {
        source_id: "source".to_string(),
        start: u64::from(span.start),
        end: u64::from(span.end),
    }
}

fn evidence(description: &str) -> Capability {
    Capability {
        policy: Some(ProfileId::hir_r0()),
        status: CapabilityStatus::Supported,
        description: description.to_string(),
    }
}

fn provenance(node: &Operation) -> Provenance {
    let origin = match node.category {
        NodeCategory::FormalInput | NodeCategory::Merge => ProvenanceOrigin::Analysis,
        _ if node.role.starts_with("compound_")
            || node.return_form.as_deref() == Some("implicit")
            || node.category == NodeCategory::Control =>
        {
            ProvenanceOrigin::Desugaring
        }
        _ => ProvenanceOrigin::Source,
    };

    Provenance {
        policy: Some(ProfileId::hir_r0()),
        origin,
        description: format!("R0 {} from resolved {}", node.role, node.original_kind),
        origin_node_ids: Vec::new(),
        origin_spans: vec![span(node.construct_span)],
    }
}

fn analysis_provenance() -> Provenance {
    Provenance {
        policy: Some(ProfileId::hir_r0()),
        origin: ProvenanceOrigin::Analysis,
        description: "Exact R0 normal-flow binding/value/dominance analysis".to_string(),
        origin_node_ids: Vec::new(),
        origin_spans: Vec::new(),
    }
}

fn attributes(node: &Operation) -> NodeAttributes {
    let access = node.symbol.as_ref().map(|symbol| Access {
        symbol_id: Some(symbol.clone()),
        location_id: None,
        unresolved_reason: None,
        operand_position: (node.category == NodeCategory::Read).then_some(0),
    });
    let mut extensions = BTreeMap::from([(
        format!("{PREFIX}:source-operation"),
        json!({"construct_span": span(node.construct_span), "role": node.role}),
    )]);
    if let Some(form) = &node.return_form {
        extensions.insert(format!("{PREFIX}:return-form"), json!(form));
    }

    NodeAttributes {
        symbol_ids: node.symbol.iter().cloned().collect(),
        defines: node.definition.iter().cloned().collect(),
        reads: if node.category == NodeCategory::Read {
            access.clone().into_iter().collect()
        } else {
            Vec::new()
        },
        writes: if node.definition.is_some() && node.category != NodeCategory::FormalInput {
            access.into_iter().collect()
        } else {
            Vec::new()
        },
        operator: node.operator.as_deref().map(StringFact::known),
        literal: node.literal.as_deref().map(StringFact::known),
        construct: node.construct.clone(),
        write_form: node.write_form.clone(),
        position: node.position,
        exit_kind: (node.category == NodeCategory::Return).then_some(ExitKind::Normal),
        callee: if node.category == NodeCategory::Call {
            node.operator.as_deref().map(StringFact::known)
        } else {
            None
        },
        call_form: (node.category == NodeCategory::Call).then_some(CallForm::Direct),
        outcomes: if node.category == NodeCategory::Control {
            vec!["true".to_string(), "false".to_string()]
        } else {
            Vec::new()
        },
        description: None,
        extensions,
    }
}

fn pdg_node(node: &Operation, graph: &Graph) -> PdgNode {
    let mut annotations =
        BTreeMap::from([(TYPE_KEY.to_string(), json!(StringFact::known(&node.ty)))]);
    if let Some(site) = graph.value_sites.iter().find(|s| s.pdg_node_id == node.id) {
        annotations.insert(GUARDS_KEY.to_string(), json!(site.guards));
    }

    PdgNode {
        id: node.id.clone(),
        category: node.category,
        original_kind: StringFact::known(&node.original_kind),
        synthetic: matches!(
            node.category,
            NodeCategory::FormalInput | NodeCategory::Merge
        ),
        source_span: (node.category != NodeCategory::Merge).then(|| span(node.span)),
        attributes: attributes(node),
        provenance: provenance(node),
        evidence: evidence("Resolved R0 operation/value within the bounded normal-flow subset"),
        annotations,
    }
}

fn pdg_edge(index: usize, edge: &Dependence) -> PdgEdge {
    PdgEdge {
        policy: ProfileId::hir_r0(),
        id: format!("e{index}"),
        source: edge.source.clone(),
        target: edge.target.clone(),
        kind: edge.family,
        subkind: edge.subkind.clone(),
        evidence: evidence("Exact R0 normal-flow dependence"),
        provenance: analysis_provenance(),
        operand_position: edge.position,
        symbol_id: edge.symbol.clone(),
        definition_id: edge.definition.clone(),
        location_id: None,
        outcome: edge.outcome.clone(),
        loop_carried: (edge.subkind == "reaching_definition").then_some(false),
        annotations: edge
            .guards
            .as_ref()
            .map(|guards| BTreeMap::from([(GUARDS_KEY.to_string(), json!(guards))]))
            .unwrap_or_default(),
    }
}

fn definitions(graph: &Graph) -> Vec<Definition> {
    crate::collection::collect_present(&graph.nodes, |node| {
        node.definition.as_ref().map(|id| Definition {
            id: id.clone(),
            node_id: node.id.clone(),
            symbol_id: node.symbol.clone(),
            location_id: None,
        })
    })
}

fn resolved_extension(ty: &ResolvedTy) -> Extensions {
    BTreeMap::from([(
        format!("{PREFIX}:resolved-type"),
        json!(StringFact::known(
            ty.normalized.as_deref().expect("eligible normalized type")
        )),
    )])
}

fn symbol(binding: &BindingFact) -> Symbol {
    let mut extensions = resolved_extension(&binding.ty);
    extensions.insert(
        format!("{PREFIX}:binding-key"),
        json!({"declaring_span": span(binding.decl_span), "scope_id": binding.scope}),
    );

    Symbol {
        id: binding.symbol.clone(),
        name: StringFact::known(&binding.name),
        scope_id: binding.scope.clone(),
        role: if binding.is_param {
            SymbolRole::Parameter
        } else {
            SymbolRole::Local
        },
        type_fact: StringFact::known(if binding.ty.spelling.is_empty() {
            &binding.ty.observed
        } else {
            &binding.ty.spelling
        }),
        extensions,
    }
}

fn input_role(facts: &dyn EvidenceFacts, position: u32) -> ParamRole {
    if position == 0
        && facts.signature.has_self
        && crate::extraction_profile::ExtractionProfile::from_facts(facts)
            .is_ok_and(|p| p.is_data())
    {
        ParamRole::Receiver
    } else {
        ParamRole::Parameter
    }
}

fn function(facts: &dyn EvidenceFacts, graph: &Graph, analysis: &Analysis) -> FunctionInfo {
    let inputs = facts
        .signature
        .params
        .iter()
        .map(|p| Parameter {
            position: u64::from(p.position),
            role: input_role(facts, p.position),
            symbol_id: Some(p.symbol.clone()),
            name: StringFact::known(&p.pat.snippet),
            type_fact: StringFact::known(&p.ty.spelling),
            variadic: BoolFact::known(false),
            extensions: resolved_extension(&p.ty),
        })
        .collect();
    let output = Parameter {
        position: 0,
        role: ParamRole::Return,
        symbol_id: None,
        name: StringFact::missing(
            FactStatus::NotApplicable,
            "return metadata has no source binding name",
        ),
        type_fact: StringFact::known(&facts.signature.return_ty.spelling),
        variadic: BoolFact::known(false),
        extensions: resolved_extension(&facts.signature.return_ty),
    };
    let reads: BTreeSet<_> = graph
        .nodes
        .iter()
        .filter(|n| n.category == NodeCategory::Read && analysis.reachable.contains(&n.id))
        .filter_map(|n| n.symbol.as_ref())
        .collect();

    FunctionInfo {
        id: facts.source.digest.clone(),
        name: StringFact::known(source_identifier(facts)),
        qualified_name: StringFact::known(&facts.name),
        source_span: span(ByteSpan {
            start: 0,
            end: facts.source.bytes.len() as u32,
        }),
        inputs,
        outputs: vec![output],
        interface_completeness: evidence(
            "Every resolved named primitive parameter and the return metadata are retained",
        ),
        reference_count: CountFact::known(reads.len() as u64, ProfileId::hir_r0()),
    }
}

fn source_identifier(facts: &dyn EvidenceFacts) -> &str {
    facts
        .build
        .frontend_options
        .get("source_identifier")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| facts.name.rsplit("::").next().unwrap_or(&facts.name))
        .trim_start_matches("r#")
}

fn source_path(facts: &dyn EvidenceFacts) -> String {
    facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&facts.name)
        .to_owned()
}

fn extraction(graph: &Graph, limitations: &[String]) -> ExtractionEvidence {
    let capabilities = REQUIRED_CAPABILITIES.into_iter().map(|key| {
        let status = match key {
            "node_extraction" | "scalar_reaching_definitions" | "control_dependencies" | "execution_dependencies" | "returns" | "parameters" | "reference_counting" => CapabilityStatus::Supported,
            "calls" if graph.nodes.iter().any(|node| node.category == NodeCategory::Call) => CapabilityStatus::Supported,
            "branch_merges" if !graph.value_sites.is_empty() => CapabilityStatus::Supported,
            "exceptional_flow" if !limitations.is_empty() => CapabilityStatus::Unsupported,
            _ => CapabilityStatus::NotApplicable,
        };
        let description = if key == "exceptional_flow" && !limitations.is_empty() { limitations.join("; ") } else { format!("R0 bounded normal-flow {key}: {status:?}; validated only within demonstrated subset") };
        (key.to_string(), Capability { policy: Some(ProfileId::hir_r0()), status, description })
    }).collect();
    let diagnostics = limitations
        .iter()
        .map(|message| Diagnostic {
            code: "rust.omitted-exceptional-flow".to_string(),
            severity: DiagnosticSeverity::Warning,
            message: message.clone(),
            source_span: None,
            node_id: None,
            capability: Some("rust.unwinding-cleanup".to_string()),
        })
        .collect();

    ExtractionEvidence {
        capabilities,
        diagnostics,
        conformance: vec![Conformance::not_evaluated(
            ProfileId::hir_r0(),
            "Await independent normal-flow validation and strict admission; schema acceptance is insufficient",
        )],
    }
}

fn extensions(
    facts: &dyn EvidenceFacts,
    graph: &Graph,
    analysis: &Analysis,
) -> Result<Extensions, String> {
    let key = format!("{}:{}", facts.source.digest, facts.name);
    let operations: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| !matches!(n.category, NodeCategory::Merge | NodeCategory::FormalInput))
        .collect();
    let map: Vec<_> = operations
        .iter()
        .map(|n| OperationMapEntry {
            cfg_node_id: n.id.clone(),
            cfg_operation_id: format!("op:{}", n.id),
            pdg_node_id: n.id.clone(),
        })
        .collect();
    let provenance: Vec<_> = operations.iter().map(|n| operation_provenance(n)).collect();
    let sites: Result<Vec<_>, _> = graph.value_sites.iter().map(value_site).collect();

    Ok(BTreeMap::from([
        (OPERATION_MAP_KEY.to_string(), json!(map)),
        (
            PAIRED_GRAPH_KEY.to_string(),
            json!(PairedGraph {
                cfg_document_key: format!("{key}:cfg"),
                pdg_document_key: format!("{key}:pdg")
            }),
        ),
        (format!("{PREFIX}:operation-provenance"), json!(provenance)),
        (SITES_KEY.to_string(), json!(sites?)),
        (
            format!("{PREFIX}:reachability"),
            json!(Reachability {
                entry_reachable_cfg_node_ids: analysis.reachable.clone()
            }),
        ),
        (
            format!("{PREFIX}:reference-count-policy"),
            json!({"id": R0_POLICY_ID, "version": R0_POLICY_VERSION, "rule": "distinct symbol IDs read by entry-reachable retained reads, including compound target reads"}),
        ),
    ]))
}

fn operation_provenance(node: &Operation) -> OperationProvenance {
    OperationProvenance {
        cfg_operation_id: format!("op:{}", node.id),
        provenance: provenance(node),
        construct_span: span(node.construct_span),
        role: node.role.clone(),
    }
}

fn value_site(site: &super::ValueSite) -> Result<Value, String> {
    let successor = site
        .cfg_successor_node_id
        .as_ref()
        .ok_or_else(|| "Merge has no real continuation site".to_string())?;
    let alternatives = site
        .alternatives
        .iter()
        .map(|a| WireAlternative {
            value_pdg_node_id: &a.value_pdg_node_id,
            outcome: &a.outcome,
            completion_cfg_node_ids: &a.completion_cfg_node_ids,
            annotations: BTreeMap::from([(GUARDS_KEY.to_string(), json!(a.guards))]),
        })
        .collect();
    let wire = WireSite {
        selection_kind: &site.selection_kind,
        pdg_node_id: &site.pdg_node_id,
        construct_span: span(site.construct_span),
        controller_pdg_node_id: &site.controller_pdg_node_id,
        cfg_successor_node_id: successor,
        annotations: BTreeMap::from([(GUARDS_KEY.to_string(), json!(site.guards))]),
        alternatives,
    };

    serde_json::to_value(wire).map_err(|e| e.to_string())
}

fn cfg_document(
    facts: &dyn EvidenceFacts,
    graph: &Graph,
    metadata: DocumentMetadata,
) -> CfgDocument {
    let DocumentMetadata {
        producer,
        extensions,
        limitations,
    } = metadata;
    let nodes = cfg_nodes(graph, facts);
    let edges = cfg_edges(graph);

    CfgDocument {
        schema_version: CFG_SCHEMA_VERSION.to_string(),
        profile: r0_profile(),
        producer: CfgProducer {
            name: producer.name,
            version: producer.version,
            language: producer.language,
            language_version: producer.language_version,
            configuration: producer.configuration,
        },
        sources: vec![cfg_source(facts)],
        function: CfgFunction {
            id: facts.source.digest.clone(),
            name: StringFact::known(&facts.name),
            source_span: span(ByteSpan {
                start: 0,
                end: facts.source.bytes.len() as u32,
            }),
        },
        entry_node_id: "E".to_string(),
        exit_node_ids: vec!["X".to_string()],
        nodes,
        edges,
        extraction_evidence: CfgExtractionEvidence {
            status: if limitations.is_empty() {
                ExtractionStatus::Complete
            } else {
                ExtractionStatus::Partial
            },
            limitations,
        },
        extensions,
    }
}

fn cfg_source(facts: &dyn EvidenceFacts) -> CfgSource {
    CfgSource {
        id: "source".to_string(),
        path: source_path(facts),
        content_identity: StringFact::known(&facts.source.digest),
        content: Some(facts.source.bytes.clone()),
    }
}

fn cfg_nodes(graph: &Graph, facts: &dyn EvidenceFacts) -> Vec<CfgNode> {
    let mut nodes = vec![
        boundary("E", CfgNodeKind::Entry),
        boundary("X", CfgNodeKind::Exit),
    ];
    nodes.extend(
        graph
            .nodes
            .iter()
            .filter(|n| !matches!(n.category, NodeCategory::FormalInput | NodeCategory::Merge))
            .map(|n| cfg_node(n, facts)),
    );
    nodes
}

fn cfg_edges(graph: &Graph) -> Vec<CfgEdge> {
    graph
        .transfers
        .iter()
        .enumerate()
        .map(|(i, t)| CfgEdge {
            id: format!("t{i}"),
            source: t.source.clone(),
            target: t.target.clone(),
            kind: match t.kind.as_str() {
                "true" => CfgEdgeKind::True,
                "false" => CfgEdgeKind::False,
                "return" => CfgEdgeKind::Return,
                "break" => CfgEdgeKind::Break,
                "continue" => CfgEdgeKind::Continue,
                _ => CfgEdgeKind::Normal,
            },
            case_value: None,
            loop_role: match t.loop_role.as_deref() {
                Some("back_edge") => Some(LoopRole::BackEdge),
                Some("loop_exit") => Some(LoopRole::LoopExit),
                _ => None,
            },
            reason: None,
        })
        .collect()
}

fn boundary(id: &str, kind: CfgNodeKind) -> CfgNode {
    CfgNode {
        id: id.to_string(),
        kind,
        origin: "R0 administrative normal-flow boundary".to_string(),
        exit_kind: (kind == CfgNodeKind::Exit).then_some(CfgExitKind::Normal),
        operations: Vec::new(),
    }
}

fn cfg_node(node: &Operation, facts: &dyn EvidenceFacts) -> CfgNode {
    CfgNode {
        id: node.id.clone(),
        kind: match node.category {
            NodeCategory::Control => CfgNodeKind::Condition,
            NodeCategory::Return => CfgNodeKind::Transfer,
            _ => CfgNodeKind::Block,
        },
        origin: provenance(node).description,
        exit_kind: None,
        operations: vec![CfgOperation {
            id: format!("op:{}", node.id),
            original_kind: node.original_kind.clone(),
            text: facts.source.bytes[node.span.start as usize..node.span.end as usize].to_string(),
            source_span: span(node.span),
        }],
    }
}
