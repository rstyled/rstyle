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
        check_extension_namespaces(doc),
        check_provenance_references(doc),
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
    let sources = collect_ids(&doc.sources, |s| &s.id);
    let blocking = [
        check_definition_links(doc, &nodes, &symbols),
        check_definition_ownership(doc),
        check_edge_endpoints(doc, &nodes),
        check_edge_references(doc),
        check_node_links(doc, &symbols),
        check_interface_references(doc, &symbols),
        check_function_span_reference(doc, &sources),
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
        .flat_map(|edge| endpoint_issues(edge, nodes, doc))
        .collect()
}

fn endpoint_issues(edge: &PdgEdge, nodes: &BTreeSet<String>, doc: &PdgDocument) -> Vec<Issue> {
    [
        dangling_endpoint_issue(edge, nodes),
        policy_issue(edge, doc),
        subkind_issue(edge),
        meaning_issue(edge),
        evidence_issue(edge),
        outcome_issue(edge),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn dangling_endpoint_issue(edge: &PdgEdge, nodes: &BTreeSet<String>) -> Option<Issue> {
    if nodes.contains(&edge.source) && nodes.contains(&edge.target) {
        return None;
    }

    Some(issue(
        "pdg.dangling-edge",
        format!("edge {:?} names an unknown endpoint", edge.id),
    ))
}

fn policy_issue(edge: &PdgEdge, doc: &PdgDocument) -> Option<Issue> {
    if edge.policy.id.is_empty() || edge.policy.version.is_empty() {
        return Some(issue(
            "pdg.policy-identity",
            format!("edge {:?} lacks a policy identity", edge.id),
        ));
    }

    if edge.policy != doc.profile {
        return Some(issue(
            "pdg.edge-policy",
            format!("edge {:?} policy must match the document profile", edge.id),
        ));
    }

    None
}

fn subkind_issue(edge: &PdgEdge) -> Option<Issue> {
    if edge.subkind.is_empty() {
        return Some(issue(
            "pdg.edge-subkind",
            format!("edge {:?} lacks a subkind", edge.id),
        ));
    }

    None
}

fn meaning_issue(edge: &PdgEdge) -> Option<Issue> {
    match edge.evidence.status {
        CapabilityStatus::Supported | CapabilityStatus::NotApplicable => None,
        _ => Some(issue(
            "pdg.edge-meaning",
            format!("edge {:?} lacks a known meaning", edge.id),
        )),
    }
}

fn evidence_issue(edge: &PdgEdge) -> Option<Issue> {
    if edge.evidence.description.is_empty() {
        return Some(issue(
            "pdg.edge-evidence",
            format!("edge {:?} lacks an evidence description", edge.id),
        ));
    }

    None
}

fn outcome_issue(edge: &PdgEdge) -> Option<Issue> {
    if edge.outcome.as_deref().is_some_and(str::is_empty) {
        return Some(issue(
            "pdg.edge-outcome",
            format!("edge {:?} has an empty outcome", edge.id),
        ));
    }

    None
}

/// Symbol/definition/location references carried by edges must resolve.
fn check_edge_references(doc: &PdgDocument) -> Vec<Issue> {
    let symbols = collect_ids(&doc.symbols, |s| &s.id);
    let definitions: BTreeSet<String> = collect_ids(&doc.definitions, |d| &d.id);
    let locations = collect_ids(&doc.memory_locations, |m| &m.id);
    doc.edges
        .iter()
        .flat_map(|edge| edge_reference_issues(edge, &symbols, &definitions, &locations))
        .collect()
}

fn edge_reference_issues(
    edge: &PdgEdge,
    symbols: &BTreeSet<String>,
    definitions: &BTreeSet<String>,
    locations: &BTreeSet<String>,
) -> Vec<Issue> {
    let mut blocking = Vec::new();

    if let Some(symbol) = edge.symbol_id.as_deref()
        && !symbols.contains(symbol)
    {
        blocking.push(issue(
            "pdg.dangling-symbol",
            format!("edge {:?} names unknown symbol", edge.id),
        ));
    }
    if let Some(definition) = edge.definition_id.as_deref()
        && !definitions.contains(definition)
    {
        blocking.push(issue(
            "pdg.dangling-definition",
            format!("edge {:?} names unknown definition", edge.id),
        ));
    }
    if let Some(location) = edge.location_id.as_deref()
        && !locations.contains(location)
    {
        blocking.push(issue(
            "pdg.dangling-location",
            format!("edge {:?} names unknown location", edge.id),
        ));
    }

    blocking
}

/// Every definition must be claimed by its owning node: bidirectional ownership.
fn check_definition_ownership(doc: &PdgDocument) -> Vec<Issue> {
    let claimed: BTreeSet<&str> = doc
        .nodes
        .iter()
        .flat_map(|node| node.attributes.defines.iter().map(String::as_str))
        .collect();
    doc.definitions
        .iter()
        .filter(|def| !claimed.contains(def.id.as_str()))
        .map(|def| {
            issue(
                "pdg.definition-ownership",
                format!(
                    "definition {:?} is not claimed by node {:?}",
                    def.id, def.node_id
                ),
            )
        })
        .collect()
}

/// Interface symbol references must resolve.
fn check_interface_references(doc: &PdgDocument, symbols: &BTreeSet<String>) -> Vec<Issue> {
    doc.function
        .inputs
        .iter()
        .chain(doc.function.outputs.iter())
        .filter_map(|param| param.symbol_id.as_deref())
        .filter(|symbol| !symbols.contains(*symbol))
        .map(|symbol| {
            issue(
                "pdg.dangling-symbol",
                format!("interface names unknown symbol {symbol:?}"),
            )
        })
        .collect()
}

/// The function span must name a known source.
fn check_function_span_reference(doc: &PdgDocument, sources: &BTreeSet<String>) -> Vec<Issue> {
    if sources.contains(&doc.function.source_span.source_id) {
        Vec::new()
    } else {
        vec![issue(
            "pdg.dangling-source",
            "function span names an unknown source".to_string(),
        )]
    }
}

fn check_node_links(doc: &PdgDocument, symbols: &BTreeSet<String>) -> Vec<Issue> {
    let definitions: BTreeMap<&str, &Definition> = doc
        .definitions
        .iter()
        .map(|def| (def.id.as_str(), def))
        .collect();
    let locations = collect_ids(&doc.memory_locations, |m| &m.id);
    doc.nodes
        .iter()
        .flat_map(|node| node_link_issues(node, symbols, &definitions, &locations))
        .collect()
}

fn node_link_issues(
    node: &PdgNode,
    symbols: &BTreeSet<String>,
    definitions: &BTreeMap<&str, &Definition>,
    locations: &BTreeSet<String>,
) -> Vec<Issue> {
    [
        symbol_issues(node, symbols),
        define_issues(node, definitions),
        access_issues(node, &node.attributes.reads, symbols, locations),
        access_issues(node, &node.attributes.writes, symbols, locations),
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

fn access_issues(
    node: &PdgNode,
    accesses: &[Access],
    symbols: &BTreeSet<String>,
    locations: &BTreeSet<String>,
) -> Vec<Issue> {
    accesses
        .iter()
        .filter_map(|access| access_issue(node, access, symbols, locations))
        .collect()
}

fn access_issue(
    node: &PdgNode,
    access: &Access,
    symbols: &BTreeSet<String>,
    locations: &BTreeSet<String>,
) -> Option<Issue> {
    if let Some(symbol) = access.symbol_id.as_deref()
        && !symbols.contains(symbol)
    {
        return Some(issue(
            "pdg.dangling-symbol",
            format!("node {:?} reads unknown symbol", node.id),
        ));
    }
    if let Some(location) = access.location_id.as_deref()
        && !locations.contains(location)
    {
        return Some(issue(
            "pdg.dangling-location",
            format!("node {:?} accesses unknown location", node.id),
        ));
    }
    match (
        &access.symbol_id,
        &access.location_id,
        &access.unresolved_reason,
    ) {
        (None, None, None) => Some(issue(
            "pdg.unresolved-access",
            format!("node {:?} has an access with no target or reason", node.id),
        )),
        (None, None, Some(reason)) if reason.is_empty() => Some(issue(
            "pdg.unresolved-access",
            format!("node {:?} has an empty unresolved reason", node.id),
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
        NodeCategory::Declaration
            if node.attributes.symbol_ids.is_empty() && node.attributes.defines.is_empty() =>
        {
            vec![issue(
                "pdg.declaration-binding",
                format!("declaration {:?} names no symbol or definition", node.id),
            )]
        }
        NodeCategory::Read if node.attributes.reads.is_empty() => vec![issue(
            "pdg.read-access",
            format!("read {:?} has no read access", node.id),
        )],
        NodeCategory::Assignment
            if node.attributes.defines.is_empty() && node.attributes.writes.is_empty() =>
        {
            vec![issue(
                "pdg.assignment-target",
                format!("assignment {:?} has no definition or write", node.id),
            )]
        }
        NodeCategory::Call => call_issues(node),
        NodeCategory::Control => control_issues(node),
        _ => Vec::new(),
    }
}

fn call_issues(node: &PdgNode) -> Vec<Issue> {
    match node.attributes.callee.as_ref() {
        None => vec![issue(
            "pdg.call-target",
            format!("call {:?} lacks callee evidence", node.id),
        )],
        Some(callee) if !callee.shape_ok() => vec![issue(
            "pdg.fact-shape",
            format!("call {:?} has a malformed callee fact", node.id),
        )],
        _ => Vec::new(),
    }
}

fn control_issues(node: &PdgNode) -> Vec<Issue> {
    if node
        .attributes
        .construct
        .as_deref()
        .is_none_or(str::is_empty)
    {
        return vec![issue(
            "pdg.control-construct",
            format!("control {:?} lacks a construct", node.id),
        )];
    }
    Vec::new()
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
    key_issues(
        &REQUIRED_CAPABILITIES,
        "pdg.capability-missing",
        |key| !caps.contains_key(key),
        |key| format!("capability {key} is not recorded"),
    )
}

fn normal_flow_issues(caps: &BTreeMap<String, Capability>) -> Vec<Issue> {
    key_issues(
        &REQUIRED_NORMAL_FLOW,
        "pdg.normal-flow",
        |key| !is_established(caps.get(key)),
        |key| format!("required normal-flow capability {key} is not established"),
    )
}

fn key_issues(
    keys: &[&str],
    code: &'static str,
    bad: impl Fn(&str) -> bool,
    message: impl Fn(&str) -> String,
) -> Vec<Issue> {
    keys.iter()
        .filter(|key| bad(key))
        .map(|key| issue(code, message(key)))
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

/// Extension keys must use the namespaced `prefix:suffix` form.
fn check_extension_namespaces(doc: &PdgDocument) -> CheckOutput {
    let blocking = [
        document_extension_issues(doc),
        node_extension_issues(doc),
        edge_extension_issues(doc),
        symbol_extension_issues(doc),
    ]
    .concat();

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn document_extension_issues(doc: &PdgDocument) -> Vec<Issue> {
    invalid_extension_keys(&doc.extensions)
        .into_iter()
        .map(|key| {
            issue(
                "pdg.extension-namespace",
                format!("document extension key {key:?} is not namespaced"),
            )
        })
        .collect()
}

fn node_extension_issues(doc: &PdgDocument) -> Vec<Issue> {
    doc.nodes
        .iter()
        .flat_map(|node| {
            invalid_extension_keys(&node.annotations)
                .into_iter()
                .chain(invalid_extension_keys(&node.attributes.extensions))
                .map(|key| {
                    issue(
                        "pdg.extension-namespace",
                        format!("node {:?} extension key {key:?} is not namespaced", node.id),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn edge_extension_issues(doc: &PdgDocument) -> Vec<Issue> {
    doc.edges
        .iter()
        .flat_map(|edge| {
            invalid_extension_keys(&edge.annotations)
                .into_iter()
                .map(|key| {
                    issue(
                        "pdg.extension-namespace",
                        format!("edge {:?} extension key {key:?} is not namespaced", edge.id),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn symbol_extension_issues(doc: &PdgDocument) -> Vec<Issue> {
    doc.symbols
        .iter()
        .flat_map(|symbol| {
            invalid_extension_keys(&symbol.extensions)
                .into_iter()
                .map(|key| {
                    issue(
                        "pdg.extension-namespace",
                        format!(
                            "symbol {:?} extension key {key:?} is not namespaced",
                            symbol.id
                        ),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Provenance origin references must resolve within the document.
fn check_provenance_references(doc: &PdgDocument) -> CheckOutput {
    let nodes = collect_ids(&doc.nodes, |n| &n.id);
    let sources = collect_ids(&doc.sources, |s| &s.id);
    let blocking = [
        node_provenance_issues(doc, &nodes, &sources),
        edge_provenance_issues(doc),
    ]
    .concat();

    CheckOutput {
        blocking,
        notes: Vec::new(),
    }
}

fn node_provenance_issues(
    doc: &PdgDocument,
    nodes: &BTreeSet<String>,
    sources: &BTreeSet<String>,
) -> Vec<Issue> {
    doc.nodes
        .iter()
        .flat_map(|node| provenance_issues(node, nodes, sources, &doc.profile))
        .collect()
}

fn provenance_issues(
    node: &PdgNode,
    nodes: &BTreeSet<String>,
    sources: &BTreeSet<String>,
    profile: &ProfileId,
) -> Vec<Issue> {
    let mut out = Vec::new();
    out.extend(dangling_origin_issues(node, nodes));
    out.extend(dangling_origin_span_issues(node, sources));

    if let Some(complaint) = provenance_policy_issue(&node.provenance.policy, profile, &node.id) {
        out.push(complaint);
    }

    out
}

fn dangling_origin_issues(node: &PdgNode, nodes: &BTreeSet<String>) -> Vec<Issue> {
    node.provenance
        .origin_node_ids
        .iter()
        .filter(|origin| !nodes.contains(origin.as_str()))
        .map(|_| {
            issue(
                "pdg.dangling-node",
                format!("node {:?} provenance names unknown node", node.id),
            )
        })
        .collect()
}

fn dangling_origin_span_issues(node: &PdgNode, sources: &BTreeSet<String>) -> Vec<Issue> {
    node.provenance
        .origin_spans
        .iter()
        .filter(|span| !sources.contains(&span.source_id))
        .map(|_| {
            issue(
                "pdg.dangling-source",
                format!("node {:?} provenance names unknown source", node.id),
            )
        })
        .collect()
}

fn provenance_policy_issue(
    policy: &Option<ProfileId>,
    profile: &ProfileId,
    owner: &str,
) -> Option<Issue> {
    match policy {
        Some(seen) if seen != profile => Some(issue(
            "pdg.provenance-policy",
            format!("{owner:?} provenance policy must match the document profile"),
        )),
        _ => None,
    }
}

fn edge_provenance_issues(doc: &PdgDocument) -> Vec<Issue> {
    doc.edges
        .iter()
        .filter_map(|edge| provenance_policy_issue(&edge.provenance.policy, &doc.profile, &edge.id))
        .collect()
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
