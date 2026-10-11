//! Validation tests: malformed inputs, unknown evidence, policy identities.

use rstyle_pdg::*;
use std::collections::BTreeMap;

fn profile() -> ProfileId {
    ProfileId {
        id: "fixture-only".to_string(),
        version: "0.1.0".to_string(),
    }
}

fn counted_policy() -> ProfileId {
    ProfileId {
        id: REFERENCE_COUNT_POLICY_ID.to_string(),
        version: REFERENCE_COUNT_POLICY_VERSION.to_string(),
    }
}

fn supported_capability() -> Capability {
    Capability {
        status: CapabilityStatus::Supported,
        description: "d".to_string(),
        policy: None,
    }
}

fn na_capability() -> Capability {
    Capability {
        status: CapabilityStatus::NotApplicable,
        description: "absent".to_string(),
        policy: None,
    }
}

fn capabilities() -> BTreeMap<String, Capability> {
    let mut caps = BTreeMap::new();
    for key in REQUIRED_CAPABILITIES {
        caps.insert(key.to_string(), na_capability());
    }
    for key in REQUIRED_NORMAL_FLOW {
        caps.insert(key.to_string(), supported_capability());
    }
    caps.insert("reference_counting".to_string(), supported_capability());
    caps.insert(
        "exceptional_flow".to_string(),
        Capability {
            status: CapabilityStatus::Unsupported,
            description: "normal-flow only".to_string(),
            policy: None,
        },
    );
    caps
}

fn span() -> Span {
    Span {
        source_id: "s0".to_string(),
        start: 0,
        end: 1,
    }
}

fn provenance() -> Provenance {
    Provenance {
        origin: ProvenanceOrigin::Source,
        description: "d".to_string(),
        origin_node_ids: Vec::new(),
        origin_spans: Vec::new(),
        policy: None,
    }
}

fn source_node(id: &str, category: NodeCategory, attrs: NodeAttributes) -> PdgNode {
    PdgNode {
        id: id.to_string(),
        category,
        original_kind: StringFact::known("hir-kind"),
        synthetic: false,
        source_span: Some(span()),
        attributes: attrs,
        provenance: provenance(),
        evidence: supported_capability(),
        annotations: BTreeMap::new(),
    }
}

fn formal_input() -> PdgNode {
    let mut node = source_node(
        "n_p",
        NodeCategory::FormalInput,
        NodeAttributes {
            symbol_ids: vec!["s_x".to_string()],
            defines: vec!["d0".to_string()],
            position: Some(0),
            ..empty_attrs()
        },
    );
    node.synthetic = true;
    node.source_span = None;
    node
}

fn empty_attrs() -> NodeAttributes {
    NodeAttributes {
        symbol_ids: Vec::new(),
        defines: Vec::new(),
        reads: Vec::new(),
        writes: Vec::new(),
        operator: None,
        literal: None,
        construct: None,
        write_form: None,
        position: None,
        exit_kind: None,
        callee: None,
        call_form: None,
        outcomes: Vec::new(),
        description: None,
        extensions: BTreeMap::new(),
    }
}

fn edge(id: &str, source: &str, target: &str) -> PdgEdge {
    PdgEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        kind: EdgeFamily::Data,
        subkind: "value_operand".to_string(),
        policy: profile(),
        evidence: supported_capability(),
        provenance: provenance(),
        operand_position: None,
        symbol_id: None,
        definition_id: None,
        location_id: None,
        outcome: None,
        loop_carried: None,
        annotations: BTreeMap::new(),
    }
}

fn base_doc() -> PdgDocument {
    PdgDocument {
        schema_version: PDG_SCHEMA_VERSION.to_string(),
        profile: profile(),
        producer: base_producer(),
        sources: vec![Source {
            id: "s0".to_string(),
            path: "a.rs".to_string(),
            content_identity: StringFact::missing(FactStatus::Unknown, "hand-authored"),
        }],
        function: base_function(),
        symbols: vec![Symbol {
            id: "s_x".to_string(),
            name: StringFact::known("x"),
            scope_id: "scope".to_string(),
            role: SymbolRole::Parameter,
            type_fact: StringFact::known("i32"),
            extensions: BTreeMap::new(),
        }],
        definitions: vec![Definition {
            id: "d0".to_string(),
            node_id: "n_p".to_string(),
            symbol_id: Some("s_x".to_string()),
            location_id: None,
        }],
        memory_locations: Vec::new(),
        nodes: vec![
            formal_input(),
            read_node(),
            source_node("n_ret", NodeCategory::Return, empty_attrs()),
        ],
        edges: vec![edge("e0", "n_p", "n_r"), edge("e1", "n_r", "n_ret")],
        extraction_evidence: ExtractionEvidence {
            capabilities: capabilities(),
            diagnostics: Vec::new(),
            conformance: Vec::new(),
        },
        extensions: BTreeMap::new(),
    }
}

fn base_producer() -> Producer {
    Producer {
        name: "fixture-author".to_string(),
        version: "0".to_string(),
        language: "Rust".to_string(),
        language_version: StringFact::known("1.0"),
        configuration: BTreeMap::new(),
    }
}

fn read_node() -> PdgNode {
    let mut read_attrs = empty_attrs();
    read_attrs.reads = vec![Access {
        symbol_id: Some("s_x".to_string()),
        location_id: None,
        unresolved_reason: None,
        operand_position: Some(0),
    }];
    source_node("n_r", NodeCategory::Read, read_attrs)
}

fn base_function() -> FunctionInfo {
    FunctionInfo {
        id: "f".to_string(),
        name: StringFact::known("alpha"),
        qualified_name: StringFact::missing(FactStatus::NotApplicable, "no module"),
        source_span: span(),
        inputs: vec![base_param(ParamRole::Parameter, Some("s_x"), "x", "i32")],
        outputs: vec![base_param(ParamRole::Return, None, "", "unit")],
        interface_completeness: supported_capability(),
        reference_count: CountFact {
            status: FactStatus::Known,
            value: Some(1),
            policy: Some(counted_policy()),
            reason: None,
        },
    }
}

fn base_param(role: ParamRole, symbol_id: Option<&str>, name: &str, type_name: &str) -> Parameter {
    Parameter {
        position: 0,
        role,
        symbol_id: symbol_id.map(str::to_string),
        name: match role {
            ParamRole::Return => StringFact::missing(FactStatus::NotApplicable, "unnamed return"),
            _ => StringFact::known(name),
        },
        type_fact: StringFact::known(type_name),
        variadic: BoolFact::known(false),
        extensions: BTreeMap::new(),
    }
}

fn fixture_ctx() -> ValidationContext {
    ValidationContext::FixtureOnly {
        note: "test".to_string(),
    }
}

fn check(doc: &PdgDocument) -> ValidationReport {
    validate(doc, &fixture_ctx(), &BTreeMap::new())
}

#[test]
fn valid_fixture_doc_is_eligible_but_never_conformant() {
    let report = check(&base_doc());
    assert_eq!(
        report.matching,
        Eligibility::Eligible,
        "{:?}",
        report.issues
    );
    assert_eq!(
        report.producer_conformance,
        ProducerConformance::NotEvaluated
    );
}

#[test]
fn duplicate_node_ids_are_ineligible() {
    let mut doc = base_doc();
    doc.nodes.push(doc.nodes[1].clone());
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(report.issues.iter().any(|i| i.code == "pdg.duplicate-id"));
}

#[test]
fn dangling_edge_endpoint_is_ineligible() {
    let mut doc = base_doc();
    doc.edges.push(edge("e9", "n_r", "ghost"));
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn unknown_read_symbol_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes[1].attributes.reads[0].symbol_id = Some("ghost".to_string());
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn reversed_span_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes[1].source_span = Some(Span {
        source_id: "s0".to_string(),
        start: 5,
        end: 2,
    });
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn out_of_bounds_span_is_ineligible_when_bytes_known() {
    let doc = base_doc();
    let bytes = BTreeMap::from([("s0".to_string(), b"fn".to_vec())]);
    let report = validate(&doc, &fixture_ctx(), &bytes);
    assert_eq!(report.matching, Eligibility::Eligible);
    let mut far = base_doc();
    far.nodes[1].source_span = Some(Span {
        source_id: "s0".to_string(),
        start: 0,
        end: 99,
    });
    let report = validate(&far, &fixture_ctx(), &bytes);
    assert_eq!(report.matching, Eligibility::Ineligible);
}

#[test]
fn noncontiguous_interface_positions_are_ineligible() {
    let mut doc = base_doc();
    doc.function.inputs[0].position = 3;
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn unknown_reference_count_is_ineligible_and_never_zero() {
    let mut doc = base_doc();
    doc.function.reference_count = CountFact {
        status: FactStatus::Unknown,
        value: None,
        policy: None,
        reason: Some("unavailable".to_string()),
    };
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "pdg.reference-count")
    );
}

#[test]
fn unknown_policy_is_ineligible_under_compiler_evidence() {
    let doc = base_doc();
    let ctx = ValidationContext::CompilerEvidence {
        pointer_width_bits: 64,
    };
    let report = validate(&doc, &ctx, &BTreeMap::new());
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(report.issues.iter().any(|i| i.code == "pdg.unknown-policy"));
}

#[test]
fn compiler_evidence_requires_source_digests() {
    let mut doc = base_doc();
    let r0 = ProfileId {
        id: R0_POLICY_ID.to_string(),
        version: R0_POLICY_VERSION.to_string(),
    };
    doc.profile = r0.clone();

    for edge in &mut doc.edges {
        edge.policy = r0.clone();
    }

    let ctx = ValidationContext::CompilerEvidence {
        pointer_width_bits: 64,
    };
    let report = validate(&doc, &ctx, &BTreeMap::new());
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "pdg.source-identity")
    );
    doc.sources[0].content_identity = StringFact::known(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    );
    let report = validate(&doc, &ctx, &BTreeMap::new());
    assert_eq!(
        report.matching,
        Eligibility::Eligible,
        "{:?}",
        report.issues
    );
}

#[test]
fn other_without_meaning_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes
        .push(source_node("n_o", NodeCategory::Other, empty_attrs()));
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn memory_summary_is_ineligible_without_separate_profile() {
    let mut doc = base_doc();
    doc.nodes.push(source_node(
        "n_m",
        NodeCategory::MemorySummary,
        empty_attrs(),
    ));
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(report.issues.iter().any(|i| i.code == "pdg.memory-summary"));
}

#[test]
fn approximated_normal_flow_is_ineligible() {
    let mut doc = base_doc();
    doc.extraction_evidence.capabilities.insert(
        "returns".to_string(),
        Capability {
            status: CapabilityStatus::Approximated,
            description: "d".to_string(),
            policy: None,
        },
    );
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn unknown_function_name_is_ineligible() {
    let mut doc = base_doc();
    doc.function.name = StringFact::missing(FactStatus::Unknown, "no name");
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn definition_back_link_mismatch_is_ineligible() {
    let mut doc = base_doc();
    doc.definitions[0].node_id = "n_r".to_string();
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn unknown_evidence_survives_a_serde_round_trip() {
    let doc = base_doc();
    let text = serde_json::to_string(&doc).expect("serialize");
    let back: PdgDocument = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(back.sources[0].content_identity.status, FactStatus::Unknown);
    assert_eq!(back.sources[0].content_identity.value, None);
}

#[test]
fn parallel_edges_preserve_multiplicity() {
    let mut doc = base_doc();
    doc.edges.push(edge("e2", "n_r", "n_ret"));
    let text = serde_json::to_string(&doc).expect("serialize");
    let back: PdgDocument = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(back.edges.len(), 3);
}

#[test]
fn cfg_entry_with_incoming_edge_is_flagged() {
    let doc = cfg_doc();
    assert!(validate_cfg(&doc).is_empty());
    let mut bad = cfg_doc();
    bad.edges.push(CfgEdge {
        id: "x".to_string(),
        source: "b".to_string(),
        target: "E".to_string(),
        kind: CfgEdgeKind::Normal,
        case_value: None,
        loop_role: None,
        reason: None,
    });
    assert!(
        validate_cfg(&bad)
            .iter()
            .any(|i| i.code == "cfg.entry-incoming")
    );
}

fn cfg_producer() -> CfgProducer {
    CfgProducer {
        name: "fixture-author".to_string(),
        version: "0".to_string(),
        language: "Rust".to_string(),
        language_version: StringFact::known("1.0"),
        configuration: BTreeMap::new(),
    }
}

fn cfg_nodes() -> Vec<CfgNode> {
    vec![
        CfgNode {
            id: "E".to_string(),
            kind: CfgNodeKind::Entry,
            origin: "synthetic entry".to_string(),
            exit_kind: None,
            operations: Vec::new(),
        },
        CfgNode {
            id: "b".to_string(),
            kind: CfgNodeKind::Block,
            origin: "test operation".to_string(),
            exit_kind: None,
            operations: vec![CfgOperation {
                id: "op0".to_string(),
                original_kind: "k".to_string(),
                text: "t".to_string(),
                source_span: span(),
            }],
        },
        CfgNode {
            id: "X".to_string(),
            kind: CfgNodeKind::Exit,
            origin: "synthetic exit".to_string(),
            exit_kind: Some(CfgExitKind::Normal),
            operations: Vec::new(),
        },
    ]
}

fn cfg_edges() -> Vec<CfgEdge> {
    vec![
        CfgEdge {
            id: "c0".to_string(),
            source: "E".to_string(),
            target: "b".to_string(),
            kind: CfgEdgeKind::Normal,
            case_value: None,
            loop_role: None,
            reason: None,
        },
        CfgEdge {
            id: "c1".to_string(),
            source: "b".to_string(),
            target: "X".to_string(),
            kind: CfgEdgeKind::Return,
            case_value: None,
            loop_role: None,
            reason: None,
        },
    ]
}

fn cfg_doc() -> CfgDocument {
    CfgDocument {
        schema_version: CFG_SCHEMA_VERSION.to_string(),
        profile: r0_profile(),
        producer: cfg_producer(),
        sources: vec![CfgSource {
            id: "s0".to_string(),
            path: "a.rs".to_string(),
            content_identity: StringFact::missing(FactStatus::Unknown, "hand-authored"),
            content: None,
        }],
        function: CfgFunction {
            id: "f".to_string(),
            name: StringFact::known("alpha"),
            source_span: span(),
        },
        entry_node_id: "E".to_string(),
        exit_node_ids: vec!["X".to_string()],
        nodes: cfg_nodes(),
        edges: cfg_edges(),
        extraction_evidence: CfgExtractionEvidence {
            status: ExtractionStatus::Complete,
            limitations: Vec::new(),
        },
        extensions: BTreeMap::new(),
    }
}

#[test]
fn cfg_reachability_comes_from_entry() {
    let reached = reachable_vertices(&cfg_doc());
    assert!(reached.contains("b"));
    assert!(reached.contains("X"));
}

#[test]
fn correspondence_rejects_unknown_pdg_nodes_and_shared_vertices() {
    let cfg = cfg_doc();
    let pdg = singleton_ids(["n0"]);
    let map = vec![OperationMapEntry {
        cfg_node_id: "b".to_string(),
        cfg_operation_id: "op0".to_string(),
        pdg_node_id: "ghost".to_string(),
    }];
    assert!(
        validate_correspondence(&map, &cfg, &pdg)
            .iter()
            .any(|i| i.code == "cfg.operation-map")
    );
}

fn singleton_ids(ids: [&str; 1]) -> std::collections::BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

fn has_code(report: &ValidationReport, code: &str) -> bool {
    report.issues.iter().any(|i| i.code == code)
}

#[test]
fn unclaimed_definition_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes[0].attributes.defines.clear();
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.definition-ownership"));
}

#[test]
fn edge_policy_mismatch_is_ineligible() {
    let mut doc = base_doc();
    doc.edges[0].policy = counted_policy();
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.edge-policy"));
}

#[test]
fn empty_edge_subkind_is_ineligible() {
    let mut doc = base_doc();
    doc.edges[0].subkind.clear();
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn read_without_access_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes[1].attributes.reads.clear();
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.read-access"));
}

#[test]
fn call_without_callee_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes
        .push(source_node("n_c", NodeCategory::Call, empty_attrs()));
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.call-target"));
}

#[test]
fn control_without_construct_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes
        .push(source_node("n_p2", NodeCategory::Control, empty_attrs()));
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.control-construct"));
}

#[test]
fn bad_extension_key_is_ineligible() {
    let mut doc = base_doc();
    doc.extensions
        .insert("bare".to_string(), serde_json::json!(1));
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.extension-namespace"));
}

#[test]
fn unknown_provenance_node_is_ineligible() {
    let mut doc = base_doc();
    doc.nodes[1].provenance.origin_node_ids = vec!["ghost".to_string()];
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.dangling-node"));
}

#[test]
fn unknown_edge_definition_is_ineligible() {
    let mut doc = base_doc();
    doc.edges[0].definition_id = Some("ghost".to_string());
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert!(has_code(&report, "pdg.dangling-definition"));
}

#[test]
fn unknown_interface_symbol_is_ineligible() {
    let mut doc = base_doc();
    doc.function.inputs[0].symbol_id = Some("ghost".to_string());
    assert_eq!(check(&doc).matching, Eligibility::Ineligible);
}

#[test]
fn conformant_claim_does_not_establish_eligibility() {
    let mut doc = base_doc();
    doc.extraction_evidence.conformance.push(ConformanceRecord {
        profile: profile(),
        status: ConformanceStatus::Conformant,
        reasons: Vec::new(),
    });
    doc.edges.push(edge("e9", "n_r", "ghost"));
    let report = check(&doc);
    assert_eq!(report.matching, Eligibility::Ineligible);
    assert_eq!(
        report.producer_conformance,
        ProducerConformance::NotEvaluated
    );
}
