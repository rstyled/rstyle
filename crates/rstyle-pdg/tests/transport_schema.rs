//! Schema-faithful transport: PDG 0.1.0 and CFG 0.2.0 wire documents must
//! validate against the vendored authoritative schemas, omit absent
//! optionals (no schema-invalid nulls), enforce fact alternatives, reject
//! forbidden fields, and validate reason/extension namespaces.

use rstyle_pdg::*;
use std::collections::BTreeMap;

fn vendor(name: &str) -> serde_json::Value {
    let directory = if name.starts_with("pdg.") {
        "vendor-compat/rust-r0"
    } else {
        "vendor-compat"
    };
    let path = format!("{}/../../{directory}/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).expect("vendored snapshot is committed");

    serde_json::from_str(&text).expect("vendored snapshot is valid JSON")
}

fn validator_for(schema: &serde_json::Value) -> jsonschema::Validator {
    jsonschema::validator_for(schema).expect("schema compiles")
}

fn assert_valid(validator: &jsonschema::Validator, instance: &serde_json::Value) {
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();

    assert!(errors.is_empty(), "schema errors: {errors:?}");
}

fn profile() -> ProfileId {
    ProfileId {
        id: "fixture-only".to_string(),
        version: "0.1.0".to_string(),
    }
}

fn supported_capability() -> Capability {
    Capability {
        policy: None,
        status: CapabilityStatus::Supported,
        description: "d".to_string(),
    }
}

fn na_capability() -> Capability {
    Capability {
        policy: None,
        status: CapabilityStatus::NotApplicable,
        description: "absent".to_string(),
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

fn read_node() -> PdgNode {
    let mut attrs = empty_attrs();
    attrs.reads = vec![Access {
        symbol_id: Some("s_x".to_string()),
        location_id: None,
        unresolved_reason: None,
        operand_position: Some(0),
    }];

    source_node("n_r", NodeCategory::Read, attrs)
}

fn edge(id: &str, source: &str, target: &str) -> PdgEdge {
    PdgEdge {
        policy: profile(),
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        kind: EdgeFamily::Data,
        subkind: "value_operand".to_string(),
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

fn base_producer() -> Producer {
    Producer {
        name: "fixture-author".to_string(),
        version: "0".to_string(),
        language: "Rust".to_string(),
        language_version: StringFact::known("1.0"),
        configuration: BTreeMap::new(),
    }
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
        reference_count: CountFact::known(1, profile()),
    }
}

fn base_symbols() -> Vec<Symbol> {
    vec![Symbol {
        id: "s_x".to_string(),
        name: StringFact::known("x"),
        scope_id: "scope".to_string(),
        role: SymbolRole::Parameter,
        type_fact: StringFact::known("i32"),
        extensions: BTreeMap::new(),
    }]
}

fn base_nodes() -> Vec<PdgNode> {
    vec![
        formal_input(),
        read_node(),
        source_node("n_ret", NodeCategory::Return, empty_attrs()),
    ]
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
        symbols: base_symbols(),
        definitions: vec![Definition {
            id: "d0".to_string(),
            node_id: "n_p".to_string(),
            symbol_id: Some("s_x".to_string()),
            location_id: None,
        }],
        memory_locations: Vec::new(),
        nodes: base_nodes(),
        edges: vec![edge("e0", "n_p", "n_r"), edge("e1", "n_r", "n_ret")],
        extraction_evidence: ExtractionEvidence {
            capabilities: capabilities(),
            diagnostics: Vec::new(),
            conformance: vec![Conformance::not_evaluated(profile(), "fixture")],
        },
        extensions: BTreeMap::new(),
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
        producer: CfgProducer {
            name: "fixture-author".to_string(),
            version: "0".to_string(),
            language: "Rust".to_string(),
            language_version: StringFact::known("1.0"),
            configuration: BTreeMap::new(),
        },
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
fn pdg_wire_document_validates_against_authoritative_schema() {
    let schema = vendor("pdg.schema.json");
    let validator = validator_for(&schema);
    let doc = base_doc();
    let json = serde_json::to_value(doc.clone()).expect("serialize");

    assert_valid(&validator, &json);

    let back: PdgDocument = serde_json::from_value(json.clone()).expect("deserialize");
    assert_eq!(back, doc);

    let round = serde_json::to_value(&back).expect("reserialize");
    assert_valid(&validator, &round);
}

#[test]
fn cfg_wire_document_validates_against_authoritative_schema() {
    let schema = vendor("cfg.schema.json");
    let validator = validator_for(&schema);
    let doc = cfg_doc();
    let json = serde_json::to_value(doc.clone()).expect("serialize");

    assert_valid(&validator, &json);

    let back: CfgDocument = serde_json::from_value(json.clone()).expect("deserialize");
    assert_eq!(back, doc);
}

#[test]
fn vendored_cfg_example_deserializes_and_reserializes_schema_valid() {
    let schema = vendor("cfg.schema.json");
    let validator = validator_for(&schema);
    let example = vendor("cfg.example.json");

    assert_valid(&validator, &example);

    let doc: CfgDocument = serde_json::from_value(example.clone()).expect("example parses");
    assert!(validate_cfg(&doc).is_empty(), "{:?}", validate_cfg(&doc));

    let round = serde_json::to_value(doc.clone()).expect("reserialize");
    assert_valid(&validator, &round);
}

#[test]
fn absent_optionals_are_omitted_not_nulled() {
    let json = serde_json::to_value(cfg_doc()).expect("serialize");

    fn assert_no_nulls(value: &serde_json::Value, path: String) {
        match value {
            serde_json::Value::Null => panic!("null at {path}"),
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    assert_no_nulls(child, format!("{path}.{key}"));
                }
            }
            serde_json::Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    assert_no_nulls(child, format!("{path}[{index}]"));
                }
            }
            _ => {}
        }
    }

    assert_no_nulls(&json, "$".to_string());

    let pdg = serde_json::to_value(base_doc()).expect("serialize");
    assert_no_nulls(&pdg, "$".to_string());
}

#[test]
fn forbidden_fields_are_rejected_in_closed_objects() {
    let mut json = serde_json::to_value(base_doc()).expect("serialize");
    json.as_object_mut()
        .expect("doc")
        .insert("forbidden_field".to_string(), serde_json::json!(1));
    assert!(serde_json::from_value::<PdgDocument>(json).is_err());

    let mut cfg = serde_json::to_value(cfg_doc()).expect("serialize");
    cfg.as_object_mut()
        .expect("doc")
        .insert("bogus".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<CfgDocument>(cfg).is_err());

    let mut node = serde_json::json!({"id": "n", "unexpected": 1});
    assert!(serde_json::from_value::<CfgNode>(node.clone()).is_err());
    node.as_object_mut().expect("obj").remove("unexpected");
}

#[test]
fn fact_alternatives_reject_mixed_and_empty_reasons() {
    // Known without a value.
    assert!(serde_json::from_value::<StringFact>(serde_json::json!({"status": "known"})).is_err());
    // Known with a reason.
    assert!(
        serde_json::from_value::<StringFact>(
            serde_json::json!({"status": "known", "value": "x", "reason": "r"})
        )
        .is_err()
    );
    // Unknown with a value.
    assert!(
        serde_json::from_value::<StringFact>(
            serde_json::json!({"status": "unknown", "value": "x", "reason": "r"})
        )
        .is_err()
    );
    // Unknown with an empty reason (schema minLength 1).
    assert!(
        serde_json::from_value::<StringFact>(
            serde_json::json!({"status": "unknown", "reason": ""})
        )
        .is_err()
    );
    // Unknown without a reason.
    assert!(
        serde_json::from_value::<StringFact>(serde_json::json!({"status": "unknown"})).is_err()
    );
    // Known counts require their explicit policy.
    assert!(
        serde_json::from_value::<CountFact>(serde_json::json!({"status": "known", "value": 1}))
            .is_err()
    );
    // A record-level policy is represented natively.
    assert!(
        serde_json::from_value::<CountFact>(
            serde_json::json!({"status": "known", "value": 1, "policy": {"id": "x", "version": "y"}})
        )
        .is_ok()
    );
    // Count unknown carrying a value.
    assert!(
        serde_json::from_value::<CountFact>(
            serde_json::json!({"status": "unknown", "value": 1, "reason": "r"})
        )
        .is_err()
    );
    // Boolean known without a value.
    assert!(serde_json::from_value::<BoolFact>(serde_json::json!({"status": "known"})).is_err());
}

#[test]
fn extension_key_namespaces_are_validated() {
    assert!(is_valid_extension_key("compat.rust.hir-r0:operation-map"));
    assert!(is_valid_extension_key("x:y"));
    assert!(!is_valid_extension_key("bare"));
    assert!(!is_valid_extension_key("1bad:key"));
    assert!(!is_valid_extension_key("prefix:"));
    assert!(!is_valid_extension_key(":suffix"));

    let mut extensions: Extensions = BTreeMap::new();
    extensions.insert("bare".to_string(), serde_json::json!(1));
    assert_eq!(
        invalid_extension_keys(&extensions),
        vec!["bare".to_string()]
    );
}

#[test]
fn edge_case_value_policy_is_schema_checked() {
    // Non-case edge carrying case_value is schema-invalid.
    let mut doc = cfg_doc();
    doc.edges[0].case_value = Some("0".to_string());
    let schema = vendor("cfg.schema.json");
    let validator = validator_for(&schema);
    let json = serde_json::to_value(&doc).expect("serialize");

    assert!(!validator.is_valid(&json));
    assert!(validate_cfg(&doc).iter().any(|i| i.code == "cfg.edge-case"));

    // Case edge without case_value is schema-invalid.
    let mut doc = cfg_doc();
    doc.edges[0].kind = CfgEdgeKind::Case;
    doc.edges[0].case_value = None;
    let json = serde_json::to_value(&doc).expect("serialize");

    assert!(!validator.is_valid(&json));
}
