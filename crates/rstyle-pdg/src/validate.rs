//! Structural and semantic validation for milestone interchange inputs.
//!
//! Structural checks (unique IDs, resolvable references, well-formed spans,
//! interface ordering, known policy identities) apply to every document.
//! Semantic checks enforce this milestone's supported context: known
//! identifiers, complete interfaces, known reference counts, known
//! node/edge meanings, and explicit evidence instead of silent defaults.
//!
//! Two contexts are distinct on purpose. `FixtureOnly` covers hand-authored
//! test inputs: they can pass structural and milestone semantic checks but are
//! never stamped with producer conformance. `CompilerEvidence` additionally
//! requires pinned build facts such as source digests and pointer width.
//!
//! Every check is a pure function returning its issues, so results compose
//! without shared mutable state.

use crate::model::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Eligibility {
    Eligible,
    Ineligible,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProducerConformance {
    /// This milestone implements no HIR producer; conformance stays unevaluated.
    NotEvaluated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct ValidationReport {
    pub matching: Eligibility,
    pub producer_conformance: ProducerConformance,
    pub producer_note: String,
    pub spans_bound_checked: bool,
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug)]
pub enum ValidationContext {
    FixtureOnly { note: String },
    CompilerEvidence { pointer_width_bits: u8 },
}

impl ValidationContext {
    fn is_fixture_only(&self) -> bool {
        matches!(self, Self::FixtureOnly { .. })
    }
}

fn issue(code: &'static str, message: String) -> Issue {
    Issue { code, message }
}

fn blocked(code: &'static str, message: String) -> CheckOutput {
    CheckOutput {
        blocking: vec![issue(code, message)],
        notes: Vec::new(),
    }
}

fn clear() -> CheckOutput {
    CheckOutput {
        blocking: Vec::new(),
        notes: Vec::new(),
    }
}

#[derive(Clone, Debug)]
struct CheckOutput {
    blocking: Vec<Issue>,
    notes: Vec<Issue>,
}

/// Validate one interchange document for imported-graph matching.
pub fn validate(
    doc: &PdgDocument,
    ctx: &ValidationContext,
    source_bytes: &BTreeMap<String, Vec<u8>>,
) -> ValidationReport {
    let parts = [
        check_schema_version(doc),
        check_unique_ids(doc),
        check_policy_identity(doc, ctx),
        check_function_identity(doc),
        check_interface(doc),
        check_reference_count(doc),
        check_references(doc),
        check_spans(doc, source_bytes),
        check_node_requirements(doc),
        check_capabilities(doc),
        check_source_identities(doc, ctx),
    ];

    let blocking: Vec<Issue> = parts
        .iter()
        .flat_map(|part| part.blocking.clone())
        .collect();
    let mut issues = parts
        .iter()
        .flat_map(|part| part.notes.clone())
        .collect::<Vec<_>>();
    issues.extend(blocking.clone());

    ValidationReport {
        matching: eligibility_of(&blocking),
        producer_conformance: ProducerConformance::NotEvaluated,
        producer_note: producer_note(ctx).to_string(),
        spans_bound_checked: !source_bytes.is_empty(),
        issues,
    }
}

fn eligibility_of(blocking: &[Issue]) -> Eligibility {
    if blocking.is_empty() {
        Eligibility::Eligible
    } else {
        Eligibility::Ineligible
    }
}

fn producer_note(ctx: &ValidationContext) -> &str {
    match ctx {
        ValidationContext::FixtureOnly { .. } => {
            "fixture-only input: producer conformance not_evaluated, never conformant"
        }
        ValidationContext::CompilerEvidence { .. } => {
            "compiler evidence required: no HIR producer implemented, not_evaluated"
        }
    }
}

fn check_schema_version(doc: &PdgDocument) -> CheckOutput {
    if doc.schema_version != PDG_SCHEMA_VERSION {
        return blocked(
            "pdg.schema-version",
            format!(
                "schema_version {:?} is not {}",
                doc.schema_version, PDG_SCHEMA_VERSION
            ),
        );
    }
    clear()
}

fn collect_ids<T>(items: &[T], mut pick: impl FnMut(&T) -> &str) -> BTreeSet<String> {
    items.iter().map(&mut pick).map(str::to_string).collect()
}

fn check_unique_ids(doc: &PdgDocument) -> CheckOutput {
    let blocking = [
        check_unique(&doc.nodes, |n| &n.id, "node"),
        check_unique(&doc.edges, |e| &e.id, "edge"),
        check_unique(&doc.symbols, |s| &s.id, "symbol"),
        check_unique(&doc.definitions, |d| &d.id, "definition"),
        check_unique(&doc.sources, |s| &s.id, "source"),
        check_unique(&doc.memory_locations, |m| &m.id, "memory_location"),
    ]
    .concat();

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn check_unique<T>(items: &[T], mut pick: impl FnMut(&T) -> &str, kind: &str) -> Vec<Issue> {
    let ids: Vec<&str> = items.iter().map(&mut pick).collect();
    let mut lonely = dup_issues(&ids, kind);
    if ids.iter().any(|id| id.is_empty()) {
        lonely.push(issue("pdg.empty-id", format!("empty {kind} id")));
    }
    lonely
}

fn dup_issues(ids: &[&str], kind: &str) -> Vec<Issue> {
    let mut seen = BTreeSet::new();
    ids.iter()
        .filter(|id| !seen.insert(id.to_string()))
        .map(|id| issue("pdg.duplicate-id", format!("duplicate {kind} id {id:?}")))
        .collect()
}

/// Unknown extraction policies are ineligible, never inferred from shape.
fn check_policy_identity(doc: &PdgDocument, ctx: &ValidationContext) -> CheckOutput {
    if doc.profile.id.is_empty() || doc.profile.version.is_empty() {
        return blocked(
            "pdg.policy-identity",
            "empty profile id or version".to_string(),
        );
    }

    let known = (doc.profile.id == R0_POLICY_ID && doc.profile.version == R0_POLICY_VERSION)
        || (doc.profile.id == REFERENCE_COUNT_POLICY_ID
            && doc.profile.version == REFERENCE_COUNT_POLICY_VERSION);
    if known {
        return clear();
    }
    if ctx.is_fixture_only() {
        return CheckOutput {
            blocking: Vec::new(),
            notes: vec![issue(
                "pdg.fixture-policy",
                format!(
                    "fixture-only policy {}:{}; not a conformant producer policy",
                    doc.profile.id, doc.profile.version
                ),
            )],
        };
    }
    blocked(
        "pdg.unknown-policy",
        format!(
            "unknown extraction policy {}:{}",
            doc.profile.id, doc.profile.version
        ),
    )
}

fn check_function_identity(doc: &PdgDocument) -> CheckOutput {
    let mut blocking = Vec::new();

    match doc.function.name.known_value() {
        Some(name) if !name.is_empty() => {}
        _ => blocking.push(issue(
            "pdg.function-name",
            "function name must be known and nonempty".to_string(),
        )),
    }
    if !doc.function.name.shape_ok() {
        blocking.push(issue(
            "pdg.fact-shape",
            "malformed function name fact".to_string(),
        ));
    }
    if !doc.function.qualified_name.shape_ok() {
        blocking.push(issue(
            "pdg.fact-shape",
            "malformed qualified_name fact".to_string(),
        ));
    }

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn check_interface(doc: &PdgDocument) -> CheckOutput {
    let mut blocking = position_issues(&doc.function.inputs, "input");
    blocking.extend(position_issues(&doc.function.outputs, "output"));
    blocking.extend(interface_issues(doc));

    if doc.function.interface_completeness.status != CapabilityStatus::Supported {
        blocking.push(issue(
            "pdg.interface-completeness",
            "interface must be completely known under an explicit policy".to_string(),
        ));
    }

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn position_issues(params: &[Parameter], kind: &str) -> Vec<Issue> {
    params
        .iter()
        .enumerate()
        .filter(|(index, param)| param.position != *index as u64)
        .map(|(index, param)| {
            issue(
                "pdg.interface-order",
                format!(
                    "{kind} position {} is not contiguous index {index}",
                    param.position
                ),
            )
        })
        .collect()
}

fn interface_issues(doc: &PdgDocument) -> Vec<Issue> {
    doc.function
        .inputs
        .iter()
        .chain(doc.function.outputs.iter())
        .flat_map(parameter_issues)
        .collect()
}

fn parameter_issues(param: &Parameter) -> Vec<Issue> {
    let mut blocking = Vec::new();

    if param.type_fact.known_value().is_none() {
        blocking.push(issue(
            "pdg.interface-type",
            param_pos_msg(param, "type must be known"),
        ));
    }
    if !param.type_fact.shape_ok() || !param.name.shape_ok() || !param.variadic.shape_ok() {
        blocking.push(issue(
            "pdg.fact-shape",
            param_pos_msg(param, "malformed fact shape"),
        ));
    }
    if param.variadic.status == FactStatus::Known && param.variadic.value.is_none() {
        blocking.push(issue(
            "pdg.fact-shape",
            param_pos_msg(param, "known variadic needs a value"),
        ));
    }

    blocking
}

fn param_pos_msg(param: &Parameter, what: &str) -> String {
    format!("interface position {}: {what}", param.position)
}

/// Unknown reference counts block eligibility; they never become zero.
fn check_reference_count(doc: &PdgDocument) -> CheckOutput {
    let count = &doc.function.reference_count;

    if !count.shape_ok() {
        return blocked(
            "pdg.fact-shape",
            "malformed reference_count fact".to_string(),
        );
    }
    if count.status != FactStatus::Known {
        return blocked(
            "pdg.reference-count",
            "unknown reference count is ineligible".to_string(),
        );
    }
    clear()
}

fn check_references(doc: &PdgDocument) -> CheckOutput {
    let nodes = collect_ids(&doc.nodes, |n| &n.id);
    let symbols = collect_ids(&doc.symbols, |s| &s.id);
    let blocking = [
        check_definition_links(doc, &nodes, &symbols),
        check_edge_endpoints(doc, &nodes),
        check_node_links(doc, &symbols),
    ]
    .concat();

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn check_definition_links(
    doc: &PdgDocument,
    nodes: &BTreeSet<String>,
    symbols: &BTreeSet<String>,
) -> Vec<Issue> {
    let locations = collect_ids(&doc.memory_locations, |m| &m.id);
    doc.definitions
        .iter()
        .flat_map(|def| definition_issues(def, nodes, symbols, &locations))
        .collect()
}

fn definition_issues(
    def: &Definition,
    nodes: &BTreeSet<String>,
    symbols: &BTreeSet<String>,
    locations: &BTreeSet<String>,
) -> Vec<Issue> {
    if !nodes.contains(&def.node_id) {
        return vec![issue(
            "pdg.dangling-node",
            format!("definition {:?} names unknown node", def.id),
        )];
    }
    match (&def.symbol_id, &def.location_id) {
        (None, None) => vec![issue(
            "pdg.definition-target",
            format!("definition {:?} names no symbol or location", def.id),
        )],
        (Some(symbol), _) if !symbols.contains(symbol) => vec![issue(
            "pdg.dangling-symbol",
            format!("definition {:?} names unknown symbol", def.id),
        )],
        (_, Some(location)) if !locations.contains(location) => vec![issue(
            "pdg.dangling-location",
            format!("definition {:?} names unknown location", def.id),
        )],
        _ => Vec::new(),
    }
}

fn check_edge_endpoints(doc: &PdgDocument, nodes: &BTreeSet<String>) -> Vec<Issue> {
    doc.edges
        .iter()
        .flat_map(|edge| endpoint_issues(edge, nodes))
        .collect()
}

fn endpoint_issues(edge: &PdgEdge, nodes: &BTreeSet<String>) -> Vec<Issue> {
    let mut blocking = Vec::new();

    if !nodes.contains(&edge.source) || !nodes.contains(&edge.target) {
        blocking.push(issue(
            "pdg.dangling-edge",
            format!("edge {:?} names an unknown endpoint", edge.id),
        ));
    }
    if edge.policy.id.is_empty() || edge.policy.version.is_empty() {
        blocking.push(issue(
            "pdg.policy-identity",
            format!("edge {:?} lacks a policy identity", edge.id),
        ));
    }

    blocking
}

fn check_node_links(doc: &PdgDocument, symbols: &BTreeSet<String>) -> Vec<Issue> {
    let definitions: BTreeMap<&str, &Definition> = doc
        .definitions
        .iter()
        .map(|def| (def.id.as_str(), def))
        .collect();
    doc.nodes
        .iter()
        .flat_map(|node| node_link_issues(node, symbols, &definitions))
        .collect()
}

fn node_link_issues(
    node: &PdgNode,
    symbols: &BTreeSet<String>,
    definitions: &BTreeMap<&str, &Definition>,
) -> Vec<Issue> {
    [
        symbol_issues(node, symbols),
        define_issues(node, definitions),
        access_issues(node, &node.attributes.reads, symbols),
        access_issues(node, &node.attributes.writes, symbols),
    ]
    .concat()
}

fn symbol_issues(node: &PdgNode, symbols: &BTreeSet<String>) -> Vec<Issue> {
    node.attributes
        .symbol_ids
        .iter()
        .filter(|symbol| !symbols.contains(symbol.as_str()))
        .map(|symbol| {
            issue(
                "pdg.dangling-symbol",
                format!("node {:?} names unknown symbol {symbol:?}", node.id),
            )
        })
        .collect()
}

fn define_issues(node: &PdgNode, definitions: &BTreeMap<&str, &Definition>) -> Vec<Issue> {
    node.attributes
        .defines
        .iter()
        .filter(|defined| !links_back(definitions, defined, node))
        .map(|defined| {
            issue(
                "pdg.definition-link",
                format!("node {:?} mislinks definition {defined:?}", node.id),
            )
        })
        .collect()
}

fn links_back(definitions: &BTreeMap<&str, &Definition>, defined: &str, node: &PdgNode) -> bool {
    definitions
        .get(defined)
        .is_some_and(|def| def.node_id == node.id)
}

fn access_issues(node: &PdgNode, accesses: &[Access], symbols: &BTreeSet<String>) -> Vec<Issue> {
    accesses
        .iter()
        .filter_map(|access| access_issue(node, access, symbols))
        .collect()
}

fn access_issue(node: &PdgNode, access: &Access, symbols: &BTreeSet<String>) -> Option<Issue> {
    match (&access.symbol_id, &access.unresolved_reason) {
        (Some(symbol), _) if !symbols.contains(symbol.as_str()) => Some(issue(
            "pdg.dangling-symbol",
            format!("node {:?} reads unknown symbol", node.id),
        )),
        (None, None) if access.location_id.is_none() => Some(issue(
            "pdg.unresolved-access",
            format!("node {:?} has an access with no target or reason", node.id),
        )),
        _ => None,
    }
}

fn check_spans(doc: &PdgDocument, source_bytes: &BTreeMap<String, Vec<u8>>) -> CheckOutput {
    let mut blocking = span_issues(&doc.function.source_span, "function", source_bytes);
    blocking.extend(
        doc.nodes
            .iter()
            .filter_map(|node| node.source_span.as_ref().map(|span| (node, span)))
            .flat_map(|(node, span)| {
                span_issues(span, &format!("node {:?}", node.id), source_bytes)
            }),
    );

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn span_issues(span: &Span, owner: &str, source_bytes: &BTreeMap<String, Vec<u8>>) -> Vec<Issue> {
    if span.start > span.end {
        return vec![issue(
            "pdg.span-order",
            format!("{owner} span starts after it ends"),
        )];
    }
    match source_bytes.get(&span.source_id) {
        Some(bytes) if span.end > bytes.len() as u64 => vec![issue(
            "pdg.span-bounds",
            format!("{owner} span exceeds source length {}", bytes.len()),
        )],
        _ => Vec::new(),
    }
}

fn check_node_requirements(doc: &PdgDocument) -> CheckOutput {
    let blocking = doc.nodes.iter().flat_map(node_requirement_issues).collect();

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn node_requirement_issues(node: &PdgNode) -> Vec<Issue> {
    [
        node_shape_issues(node),
        node_evidence_issues(node),
        node_category_issues(node),
    ]
    .concat()
}

fn node_shape_issues(node: &PdgNode) -> Vec<Issue> {
    let mut blocking = Vec::new();

    if !node.synthetic && node.source_span.is_none() {
        blocking.push(issue(
            "pdg.node-span",
            format!("source-backed node {:?} lacks a source span", node.id),
        ));
    }
    if !node.original_kind.shape_ok() {
        blocking.push(issue(
            "pdg.fact-shape",
            format!("node {:?} has a malformed kind fact", node.id),
        ));
    }

    blocking
}

fn node_evidence_issues(node: &PdgNode) -> Vec<Issue> {
    match node.evidence.status {
        CapabilityStatus::Supported | CapabilityStatus::NotApplicable => Vec::new(),
        _ => vec![issue(
            "pdg.node-meaning",
            format!("node {:?} lacks a known meaning", node.id),
        )],
    }
}

fn node_category_issues(node: &PdgNode) -> Vec<Issue> {
    match node.category {
        NodeCategory::MemorySummary => vec![issue(
            "pdg.memory-summary",
            format!(
                "node {:?}: memory_summary needs a separate profile",
                node.id
            ),
        )],
        NodeCategory::Other if node.attributes.description.is_none() => vec![issue(
            "pdg.other-meaning",
            format!("node {:?}: other needs a registered meaning", node.id),
        )],
        NodeCategory::Entry | NodeCategory::Exit => boundary_issues(node),
        NodeCategory::FormalInput => formal_input_issues(node),
        _ => Vec::new(),
    }
}

fn boundary_issues(node: &PdgNode) -> Vec<Issue> {
    let attrs = &node.attributes;
    let clean = attrs.symbol_ids.is_empty()
        && attrs.defines.is_empty()
        && attrs.reads.is_empty()
        && attrs.writes.is_empty();
    if clean {
        Vec::new()
    } else {
        vec![issue(
            "pdg.boundary-semantics",
            format!("boundary node {:?} carries binding semantics", node.id),
        )]
    }
}

fn formal_input_issues(node: &PdgNode) -> Vec<Issue> {
    if node.attributes.position.is_none()
        || node.attributes.symbol_ids.is_empty()
        || node.attributes.defines.is_empty()
    {
        return vec![issue(
            "pdg.formal-input",
            format!(
                "formal_input {:?} needs position, symbols and defines",
                node.id
            ),
        )];
    }
    Vec::new()
}

fn check_capabilities(doc: &PdgDocument) -> CheckOutput {
    let caps = &doc.extraction_evidence.capabilities;
    let blocking = [
        missing_capabilities(caps),
        normal_flow_issues(caps),
        reference_counting_issues(caps),
        exceptional_flow_issues(caps),
    ]
    .concat();

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn missing_capabilities(caps: &BTreeMap<String, Capability>) -> Vec<Issue> {
    REQUIRED_CAPABILITIES
        .iter()
        .filter(|key| !caps.contains_key(**key))
        .map(|key| {
            issue(
                "pdg.capability-missing",
                format!("capability {key} is not recorded"),
            )
        })
        .collect()
}

fn normal_flow_issues(caps: &BTreeMap<String, Capability>) -> Vec<Issue> {
    REQUIRED_NORMAL_FLOW
        .iter()
        .filter(|key| !is_established(caps.get(**key)))
        .map(|key| {
            issue(
                "pdg.normal-flow",
                format!("required normal-flow capability {key} is not established"),
            )
        })
        .collect()
}

fn is_established(capability: Option<&Capability>) -> bool {
    matches!(
        capability.map(|cap| cap.status),
        Some(CapabilityStatus::Supported) | Some(CapabilityStatus::NotApplicable)
    )
}

fn reference_counting_issues(caps: &BTreeMap<String, Capability>) -> Vec<Issue> {
    if is_supported(caps.get("reference_counting")) {
        Vec::new()
    } else {
        vec![issue(
            "pdg.normal-flow",
            "reference counting support is not established".to_string(),
        )]
    }
}

fn is_supported(capability: Option<&Capability>) -> bool {
    capability.is_some_and(|cap| cap.status == CapabilityStatus::Supported)
}

fn exceptional_flow_issues(caps: &BTreeMap<String, Capability>) -> Vec<Issue> {
    match caps.get("exceptional_flow") {
        Some(cap) if cap.status != CapabilityStatus::Unknown => Vec::new(),
        _ => vec![issue(
            "pdg.exceptional-flow",
            "exceptional-flow evidence must be explicit".to_string(),
        )],
    }
}

/// Source byte identities must be checkable; compiler evidence needs digests.
fn check_source_identities(doc: &PdgDocument, ctx: &ValidationContext) -> CheckOutput {
    let blocking = doc
        .sources
        .iter()
        .flat_map(|source| source_identity_issues(source, ctx))
        .collect();

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn source_identity_issues(source: &Source, ctx: &ValidationContext) -> Vec<Issue> {
    if !source.content_identity.shape_ok() {
        return vec![issue(
            "pdg.fact-shape",
            format!("source {:?} has a malformed identity", source.id),
        )];
    }
    if ctx.is_fixture_only() {
        return Vec::new();
    }
    if source.content_identity.known_value().is_some_and(is_sha256) {
        Vec::new()
    } else {
        vec![issue(
            "pdg.source-identity",
            format!("source {:?} lacks a known sha256 identity", source.id),
        )]
    }
}

fn is_sha256(value: &str) -> bool {
    let digest = value.strip_prefix("sha256:").unwrap_or("");
    digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())
}
