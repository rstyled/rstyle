use super::*;
use crate::cpg::read_records;
use crate::cpg::records::Demand;
use crate::sarif;
use serde_json::{Value, json};

const DEMAND: Demand = Demand {
    graphs: false,
    bounds: false,
    flow: false,
    defects: true,
};

/// `example.cpg.json`'s first function with a store nothing reads, on its first statement's line.
fn document_with_dead_store() -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/example.cpg.json")).unwrap();
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    functions.truncate(1);
    let function = &mut functions[0];
    let span = function["span"].clone();
    function["nodes"] = json!([
        {"id": "root", "kind": "block", "original_kind": "block", "children": ["statement"],
         "span": span, "provenance": {"origin": "source"}},
        {"id": "statement", "kind": "local_declaration", "original_kind": "d",
         "children": ["pattern", "value"], "span": span, "provenance": {"origin": "source"}},
        {"id": "pattern", "kind": "pattern_binding", "original_kind": "p", "children": [],
         "span": span, "provenance": {"origin": "source"}},
        {"id": "value", "kind": "call", "original_kind": "c", "children": [],
         "span": span, "provenance": {"origin": "source"}},
    ]);
    function["root"] = json!("root");
    let integer = integer_type();
    function["symbols"] = json!([{"id": "s:v", "name": "v", "kind": "local", "scope": "f",
                                  "node": "pattern", "mutable": true, "type": integer}]);
    function["accesses"] = json!([{"node": "pattern", "symbol": "s:v", "mode": "write"}]);
    function["refs"] = json!([]);
    function["places"] = json!([]);
    function["signature"]["parameters"] = json!([]);
    function["signature"]
        .as_object_mut()
        .unwrap()
        .remove("receiver");
    function["cfg"] = json!({"entry": "entry", "exits": [{"id": "exit", "kind": "normal"}], "edges": [
        {"id": "e0", "source": "entry", "target": "value", "kind": "normal"},
        {"id": "e1", "source": "value", "target": "pattern", "kind": "normal"},
        {"id": "e2", "source": "pattern", "target": "statement", "kind": "normal"},
        {"id": "e3", "source": "statement", "target": "exit", "kind": "normal"},
    ]});
    document
}

/// The ID of the example unit's first integer type.
fn integer_type() -> Value {
    let document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/example.cpg.json")).unwrap();
    let types = document["unit"]["types"].as_array().unwrap();
    let integer = types.iter().find(|ty| ty["kind"] == "integer").unwrap();
    integer["id"].clone()
}

fn records(documents: Vec<Value>) -> Vec<Record> {
    read_records(documents.into_iter().map(Ok), DEMAND).unwrap()
}

fn report_of(documents: Vec<Value>, config: &Config) -> DefectsReport {
    from_records(&mut records(documents), config).unwrap()
}

#[test]
fn a_defect_says_where_it_is_and_what_it_is_in() {
    let report = report_of(vec![document_with_dead_store()], &Config::default());
    let [row] = report.defects.as_slice() else {
        panic!("{report:?}");
    };
    assert_eq!(row.rule, "dead_store");
    assert!(row.message.contains("`v`"), "{}", row.message);
    assert!(!row.path.is_empty() && row.line > 0, "{row:?}");
    assert!(!row.function.is_empty());
    assert_eq!(report.functions, 1);
    let text = report.text();
    assert!(text.contains(": dead_store: "), "{text}");
    assert!(text.contains("Defects: 1 defect in 1 functions"), "{text}");
}

#[test]
fn nothing_found_is_reported_as_nothing_in_so_many_functions() {
    let document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/example.cpg.json")).unwrap();
    let report = report_of(vec![document], &Config::default());
    assert!(report.defects.is_empty(), "{report:?}");
    assert_eq!(report.functions, 3);
}

#[test]
fn a_function_read_twice_is_reported_once() {
    let mut records = records(vec![document_with_dead_store()]);
    records.extend(self::records(vec![document_with_dead_store()]));
    assert_eq!(records.len(), 2);
    let report = from_records(&mut records, &Config::default()).unwrap();
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.functions, 1);
}

#[test]
fn an_allow_comment_with_a_reason_suppresses_the_rule_and_counts_it() {
    let mut records = records(vec![document_with_dead_store()]);
    let Measured::Known(facts) = &mut records[0].measured else {
        panic!("not measured");
    };
    facts.preceding_line = "// rstyle-allow(dead_store): the store documents the protocol".into();
    let report = from_records(&mut records, &Config::default()).unwrap();
    assert!(report.defects.is_empty());
    assert_eq!(report.suppressed, 1);
}

#[test]
fn an_excluded_path_is_not_read() {
    let mut config = Config::default();
    config.exclude.0 = vec!["**/*".into(), "*".into()];
    let report = report_of(vec![document_with_dead_store()], &config);
    assert!(report.defects.is_empty(), "{report:?}");
    assert_eq!(report.functions, 0);
}

#[test]
fn a_function_a_macro_writes_has_no_author_to_tell() {
    let mut document = document_with_dead_store();
    let function = &mut document["unit"]["functions"][0];
    let span = function["span"].clone();
    let provenance = json!({"origin": "macro_expansion", "call_site": span});
    function["signature"]["provenance"] = provenance.clone();
    function["nodes"][0]["provenance"] = provenance;
    let report = report_of(vec![document], &Config::default());
    assert!(report.defects.is_empty(), "{report:?}");
    assert_eq!(report.functions, 0);
}

#[test]
fn a_function_the_rules_cant_read_is_listed_with_why() {
    let mut document = document_with_dead_store();
    document["coverage"]["layers"]["cfg"]["status"] = json!("unsupported");
    let report = report_of(vec![document], &Config::default());
    assert!(report.defects.is_empty());
    let [unknown] = report.unknown.as_slice() else {
        panic!("{report:?}");
    };
    assert!(unknown.reason.contains("control flow"), "{unknown:?}");
    assert!(report.text().contains("1 unknown"));
}

#[test]
fn sarif_holds_one_warning_per_defect_under_its_own_category() {
    let report = report_of(vec![document_with_dead_store()], &Config::default());
    let root = std::path::Path::new(if cfg!(windows) { r"C:\w" } else { "/w" });
    let log = serde_json::to_value(sarif::defects(&report, root).unwrap()).unwrap();
    let run = &log["runs"][0];
    assert_eq!(run["automationDetails"]["id"], "rstyle/defects/");
    let results = run["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["ruleId"], "dead_store");
    assert_eq!(results[0]["level"], "warning");
    assert!(results[0]["locations"][0]["physicalLocation"]["region"]["startLine"].is_number());
}
