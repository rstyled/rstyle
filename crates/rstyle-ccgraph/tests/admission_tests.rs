//! End-to-end imported-document tests through the strict admission adapter.
//!
//! These exercise [`admit_strict`] and [`evaluate_strict_documents`]: valid
//! PDG + CFG + bytes pairs are admitted from validated evidence (never caller
//! booleans), while malformed or missing evidence fails closed. Compact
//! [`evaluate_all`] fixtures stay available but non-certifying.

use rstyle_ccgraph::*;
use rstyle_pdg::*;
use std::collections::BTreeMap;

fn r0() -> ProfileId {
    ProfileId {
        id: R0_POLICY_ID.to_string(),
        version: R0_POLICY_VERSION.to_string(),
    }
}

fn supported() -> Capability {
    Capability {
        policy: None,
        status: CapabilityStatus::Supported,
        description: "d".to_string(),
    }
}

fn na() -> Capability {
    Capability {
        policy: None,
        status: CapabilityStatus::NotApplicable,
        description: "absent".to_string(),
    }
}

fn capabilities() -> BTreeMap<String, Capability> {
    let mut caps = BTreeMap::new();

    for key in REQUIRED_CAPABILITIES {
        caps.insert(key.to_string(), na());
    }

    for key in REQUIRED_NORMAL_FLOW {
        caps.insert(key.to_string(), supported());
    }

    caps.insert("reference_counting".to_string(), supported());
    caps.insert(
        "exceptional_flow".to_string(),
        Capability {
            policy: None,
            status: CapabilityStatus::Unsupported,
            description: "normal-flow only".to_string(),
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
        policy: None,
        origin: ProvenanceOrigin::Source,
        description: "d".to_string(),
        origin_node_ids: Vec::new(),
        origin_spans: Vec::new(),
    }
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

// SHA-256 of b"a".
const DIGEST_A: &str = "sha256:ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb";

fn bytes() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([("s0".to_string(), b"a".to_vec())])
}

fn ctx() -> ValidationContext {
    ValidationContext::CompilerEvidence {
        pointer_width_bits: 64,
    }
}

fn operation_map() -> serde_json::Value {
    serde_json::json!([
        {"cfg_node_id": "b1", "cfg_operation_id": "op0", "pdg_node_id": "n_r"},
        {"cfg_node_id": "b2", "cfg_operation_id": "op1", "pdg_node_id": "n_ret"},
    ])
}

fn paired_link(pdg_key: &str, cfg_key: &str) -> serde_json::Value {
    serde_json::json!({"cfg_document_key": cfg_key, "pdg_document_key": pdg_key})
}

fn extensions(pdg_key: &str, cfg_key: &str) -> Extensions {
    BTreeMap::from([
        (OPERATION_MAP_KEY.to_string(), operation_map()),
        (PAIRED_GRAPH_KEY.to_string(), paired_link(pdg_key, cfg_key)),
    ])
}

fn formal_attrs() -> NodeAttributes {
    let mut attrs = empty_attrs();
    attrs.symbol_ids = vec!["s_x".to_string()];
    attrs.defines = vec!["d0".to_string()];
    attrs.position = Some(0);
    attrs
}

fn read_attrs() -> NodeAttributes {
    let mut attrs = empty_attrs();
    attrs.reads = vec![Access {
        symbol_id: Some("s_x".to_string()),
        location_id: None,
        unresolved_reason: None,
        operand_position: Some(0),
    }];
    attrs
}

fn pdg_nodes() -> Vec<PdgNode> {
    vec![
        PdgNode {
            id: "n_p".to_string(),
            category: NodeCategory::FormalInput,
            original_kind: StringFact::known("hir-kind"),
            synthetic: true,
            source_span: None,
            attributes: formal_attrs(),
            provenance: provenance(),
            evidence: supported(),
            annotations: BTreeMap::new(),
        },
        PdgNode {
            id: "n_r".to_string(),
            category: NodeCategory::Read,
            original_kind: StringFact::known("hir-kind"),
            synthetic: false,
            source_span: Some(span()),
            attributes: read_attrs(),
            provenance: provenance(),
            evidence: supported(),
            annotations: BTreeMap::new(),
        },
        PdgNode {
            id: "n_ret".to_string(),
            category: NodeCategory::Return,
            original_kind: StringFact::known("hir-kind"),
            synthetic: false,
            source_span: Some(span()),
            attributes: empty_attrs(),
            provenance: provenance(),
            evidence: supported(),
            annotations: BTreeMap::new(),
        },
    ]
}

fn pdg_edges() -> Vec<PdgEdge> {
    vec![
        PdgEdge {
            policy: r0(),
            id: "e0".to_string(),
            source: "n_p".to_string(),
            target: "n_r".to_string(),
            kind: EdgeFamily::Data,
            subkind: "reaching_definition".to_string(),
            evidence: supported(),
            provenance: provenance(),
            operand_position: Some(0),
            symbol_id: Some("s_x".to_string()),
            definition_id: Some("d0".to_string()),
            location_id: None,
            outcome: None,
            loop_carried: Some(false),
            annotations: BTreeMap::new(),
        },
        PdgEdge {
            policy: r0(),
            id: "e1".to_string(),
            source: "n_r".to_string(),
            target: "n_ret".to_string(),
            kind: EdgeFamily::Data,
            subkind: "value_operand".to_string(),
            evidence: supported(),
            provenance: provenance(),
            operand_position: Some(0),
            symbol_id: None,
            definition_id: None,
            location_id: None,
            outcome: None,
            loop_carried: None,
            annotations: BTreeMap::new(),
        },
    ]
}

fn pdg_function() -> FunctionInfo {
    FunctionInfo {
        id: "f".to_string(),
        name: StringFact::known("alpha"),
        qualified_name: StringFact::missing(FactStatus::NotApplicable, "no module"),
        source_span: span(),
        inputs: vec![Parameter {
            position: 0,
            role: ParamRole::Parameter,
            symbol_id: Some("s_x".to_string()),
            name: StringFact::known("x"),
            type_fact: StringFact::known("i32"),
            variadic: BoolFact::known(false),
            extensions: BTreeMap::new(),
        }],
        outputs: vec![Parameter {
            position: 0,
            role: ParamRole::Return,
            symbol_id: None,
            name: StringFact::missing(FactStatus::NotApplicable, "unnamed return"),
            type_fact: StringFact::known("unit"),
            variadic: BoolFact::known(false),
            extensions: BTreeMap::new(),
        }],
        interface_completeness: supported(),
        reference_count: CountFact::known(1, r0()),
    }
}

fn pdg_symbols() -> Vec<Symbol> {
    vec![Symbol {
        id: "s_x".to_string(),
        name: StringFact::known("x"),
        scope_id: "scope".to_string(),
        role: SymbolRole::Parameter,
        type_fact: StringFact::known("i32"),
        extensions: BTreeMap::new(),
    }]
}

fn pdg_definitions() -> Vec<Definition> {
    vec![Definition {
        id: "d0".to_string(),
        node_id: "n_p".to_string(),
        symbol_id: Some("s_x".to_string()),
        location_id: None,
    }]
}

fn pdg_doc(pdg_key: &str, cfg_key: &str) -> PdgDocument {
    PdgDocument {
        schema_version: PDG_SCHEMA_VERSION.to_string(),
        profile: r0(),
        producer: Producer {
            name: "test-producer".to_string(),
            version: "0".to_string(),
            language: "Rust".to_string(),
            language_version: StringFact::known("1.0"),
            configuration: BTreeMap::new(),
        },
        sources: vec![Source {
            id: "s0".to_string(),
            path: "a.rs".to_string(),
            content_identity: StringFact::known(DIGEST_A),
        }],
        function: pdg_function(),
        symbols: pdg_symbols(),
        definitions: pdg_definitions(),
        memory_locations: Vec::new(),
        nodes: pdg_nodes(),
        edges: pdg_edges(),
        extraction_evidence: ExtractionEvidence {
            capabilities: capabilities(),
            diagnostics: Vec::new(),
            conformance: vec![Conformance::not_evaluated(r0(), "test")],
        },
        extensions: extensions(pdg_key, cfg_key),
    }
}

fn cfg_block(id: &str, op: &str) -> CfgNode {
    CfgNode {
        id: id.to_string(),
        kind: CfgNodeKind::Block,
        origin: "test operation".to_string(),
        exit_kind: None,
        operations: vec![CfgOperation {
            id: op.to_string(),
            original_kind: "k".to_string(),
            text: "t".to_string(),
            source_span: span(),
        }],
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
        cfg_block("b1", "op0"),
        cfg_block("b2", "op1"),
        CfgNode {
            id: "X".to_string(),
            kind: CfgNodeKind::Exit,
            origin: "synthetic exit".to_string(),
            exit_kind: Some(CfgExitKind::Normal),
            operations: Vec::new(),
        },
    ]
}

fn cfg_edges_list() -> Vec<CfgEdge> {
    let edge = |id: &str, source: &str, target: &str, kind| CfgEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        kind,
        case_value: None,
        loop_role: None,
        reason: None,
    };

    vec![
        edge("c0", "E", "b1", CfgEdgeKind::Normal),
        edge("c1", "b1", "b2", CfgEdgeKind::Normal),
        edge("c2", "b2", "X", CfgEdgeKind::Return),
    ]
}

fn cfg_doc(pdg_key: &str, cfg_key: &str) -> CfgDocument {
    CfgDocument {
        schema_version: CFG_SCHEMA_VERSION.to_string(),
        profile: r0_profile(),
        producer: CfgProducer {
            name: "test-producer".to_string(),
            version: "0".to_string(),
            language: "Rust".to_string(),
            language_version: StringFact::known("1.0"),
            configuration: BTreeMap::new(),
        },
        sources: vec![CfgSource {
            id: "s0".to_string(),
            path: "a.rs".to_string(),
            content_identity: StringFact::known(DIGEST_A),
            content: Some("a".into()),
        }],
        function: CfgFunction {
            id: "f".to_string(),
            name: StringFact::known("alpha"),
            source_span: span(),
        },
        entry_node_id: "E".to_string(),
        exit_node_ids: vec!["X".to_string()],
        nodes: cfg_nodes(),
        edges: cfg_edges_list(),
        extraction_evidence: CfgExtractionEvidence {
            status: ExtractionStatus::Complete,
            limitations: Vec::new(),
        },
        extensions: extensions(pdg_key, cfg_key),
    }
}

fn open_config() -> CcgraphConfig {
    CcgraphConfig {
        minimum_size_ratio: Frac::parse_decimal("0").expect("t"),
        numerical_threshold: Frac::parse_decimal("0").expect("t"),
        string_threshold: Frac::parse_decimal("0").expect("t"),
        final_threshold: Frac::parse_decimal("0").expect("t"),
        scoring: Scoring::SmallerGraphOverlap,
    }
}

#[test]
fn strict_pair_is_admitted_and_evaluated() {
    let storage = bytes();
    let context = ctx();
    let pdg_a = pdg_doc("p1", "c1");
    let cfg_a = cfg_doc("p1", "c1");
    let pdg_b = pdg_doc("p2", "c2");
    let cfg_b = cfg_doc("p2", "c2");
    let bundles = [
        StrictBundle {
            pdg: &pdg_a,
            cfg: &cfg_a,
            source_bytes: &storage,
            ctx: &context,
        },
        StrictBundle {
            pdg: &pdg_b,
            cfg: &cfg_b,
            source_bytes: &storage,
            ctx: &context,
        },
    ];

    for bundle in &bundles {
        let admitted = admit_strict(bundle).expect("strict admission succeeds");

        assert_eq!(admitted.name.as_deref(), Some("alpha"));
        assert_eq!(admitted.reference_count, Some(1));
    }

    let report = evaluate_strict_documents(&bundles, &open_config());
    assert!(!report.incomplete);

    let pair = report
        .pairs
        .iter()
        .find(|pair| pair.first == "p1" && pair.second == "p2")
        .expect("strict pair exists");
    assert_eq!(pair.decision, Decision::Accepted);
    assert!(
        pair.stages
            .iter()
            .all(|stage| matches!(stage.outcome, StageOutcome::Passed))
    );
}

#[test]
fn missing_operation_map_fails_closed() {
    let storage = bytes();
    let context = ctx();
    let mut pdg = pdg_doc("p1", "c1");
    pdg.extensions.remove(OPERATION_MAP_KEY);
    let cfg = cfg_doc("p1", "c1");
    let bundle = StrictBundle {
        pdg: &pdg,
        cfg: &cfg,
        source_bytes: &storage,
        ctx: &context,
    };

    assert!(admit_strict(&bundle).is_err());

    let report = evaluate_strict_documents(&[bundle], &open_config());
    let function = report
        .functions
        .iter()
        .find(|function| function.key == "p1")
        .expect("refused function is recorded");
    assert!(!function.eligible);
    assert!(report.pairs.is_empty());
}

#[test]
fn fixture_context_cannot_strictly_admit() {
    let storage = BTreeMap::new();
    let fixture = ValidationContext::FixtureOnly {
        note: "stage".to_string(),
    };
    let pdg = pdg_doc("p1", "c1");
    let cfg = cfg_doc("p1", "c1");
    let bundle = StrictBundle {
        pdg: &pdg,
        cfg: &cfg,
        source_bytes: &storage,
        ctx: &fixture,
    };

    assert!(admit_strict(&bundle).is_err());
}

#[test]
fn wrong_digest_never_reaches_matching() {
    let context = ctx();
    let mut pdg = pdg_doc("p1", "c1");
    pdg.sources[0].content_identity = StringFact::known(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    );
    let cfg = cfg_doc("p1", "c1");
    let storage = bytes();
    let bundle = StrictBundle {
        pdg: &pdg,
        cfg: &cfg,
        source_bytes: &storage,
        ctx: &context,
    };

    assert!(admit_strict(&bundle).is_err());
}
