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

/// A function's body calling `g` once a line, using its result where `false` and dropping it
/// where `true`.
struct Calling {
    nodes: Vec<Value>,
    refs: Vec<Value>,
    edges: Vec<Value>,
    statements: Vec<String>,
}

fn calling(dropped: &[bool], integer: &Value, span: impl Fn(usize) -> Value) -> Calling {
    let (mut nodes, mut refs, mut edges, mut statements) = (vec![], vec![], vec![], vec![]);
    let mut last = "entry".to_owned();
    for (line, dropped) in dropped.iter().enumerate() {
        let (call, statement) = (format!("call{line}"), format!("statement{line}"));
        let parent = if *dropped {
            "expression_statement"
        } else {
            "return"
        };
        nodes.push(
            json!({"id": call, "kind": "call", "original_kind": "c", "children": [],
                          "type": integer, "span": span(line + 1),
                          "provenance": {"origin": "source"}}),
        );
        nodes.push(
            json!({"id": statement, "kind": parent, "original_kind": "s",
                          "children": [call], "span": span(line + 1),
                          "provenance": {"origin": "source"}}),
        );
        refs.push(
            json!({"node": call, "status": "resolved", "dispatch": "static",
                         "target": {"kind": "definition", "id": "def:g"}}),
        );
        for (source, target) in [(&last, &call), (&call, &statement)] {
            edges.push(json!({"id": format!("e{}", edges.len()), "source": source,
                              "target": target, "kind": "normal"}));
        }
        last.clone_from(&statement);
        statements.push(statement);
    }
    edges.push(json!({"id": "out", "source": last, "target": "exit", "kind": "normal"}));
    Calling {
        nodes,
        refs,
        edges,
        statements,
    }
}

fn document_with_calls(dropped: &[bool]) -> Value {
    let mut document = document_with_dead_store();
    let file = document["unit"]["functions"][0]["span"]["file"].clone();
    let span = |line: usize| json!({"file": file, "start": line * 10, "end": line * 10 + 5});
    let Calling {
        mut nodes,
        refs,
        edges,
        statements,
    } = calling(dropped, &integer_type(), span);
    nodes.push(
        json!({"id": "root", "kind": "block", "original_kind": "block",
                      "children": statements, "span": span(0),
                      "provenance": {"origin": "source"}}),
    );

    let unit = &mut document["unit"];
    unit["definitions"].as_array_mut().unwrap().push(json!(
        {"id": "def:g", "kind": "function", "name": "g", "stable_identity": "id:g", "scope": "unit"}
    ));
    unit["files"][0]["content"] = json!("ok .....\n ".repeat(40));
    let function = &mut unit["functions"][0];
    function["span"] = span(0);
    function["signature"]["span"] = span(0);
    function["nodes"] = json!(nodes);
    function["refs"] = json!(refs);
    function["symbols"] = json!([]);
    function["accesses"] = json!([]);
    function["cfg"] =
        json!({"entry": "entry", "exits": [{"id": "exit", "kind": "normal"}], "edges": edges});
    document
}

fn nine_uses_and_a_drop() -> Vec<bool> {
    let mut calls = vec![false; 9];
    calls.push(true);
    calls
}

#[test]
fn a_call_that_drops_what_the_rest_use_says_how_many_do() {
    let report = report_of(
        vec![document_with_calls(&nine_uses_and_a_drop())],
        &Config::default(),
    );
    let [row] = report.defects.as_slice() else {
        panic!("{report:?}");
    };
    assert_eq!(row.rule, "ignored_result");
    assert_eq!(row.line, 11);
    assert!(
        row.message.contains("`g`") && row.message.contains("9 of its 10"),
        "{}",
        row.message
    );
}

#[test]
fn a_result_dropped_as_often_as_not_says_nothing() {
    let report = report_of(
        vec![document_with_calls(&[
            true, false, true, false, true, false,
        ])],
        &Config::default(),
    );
    assert!(report.defects.is_empty(), "{report:?}");
}

#[test]
fn too_few_calls_say_nothing() {
    let report = report_of(
        vec![document_with_calls(&[false, false, false, true])],
        &Config::default(),
    );
    assert!(report.defects.is_empty(), "{report:?}");
}

#[test]
fn a_project_can_ask_for_fewer_calls_and_a_smaller_share() {
    let calls = document_with_calls(&[false, false, false, true]);
    let mut config = Config::default();
    assert!(report_of(vec![calls.clone()], &config).defects.is_empty());

    config.ignored_result.min_calls = 4;
    config.ignored_result.min_used_percent = crate::config::Percent(75);
    let report = report_of(vec![calls], &config);
    assert_eq!(report.defects.len(), 1, "{report:?}");
    assert!(report.defects[0].message.contains("3 of its 4"));
}

#[test]
fn asking_for_every_call_reports_nothing() {
    let mut config = Config::default();
    config.ignored_result.min_used_percent = crate::config::Percent(100);
    let report = report_of(vec![document_with_calls(&nine_uses_and_a_drop())], &config);
    assert!(report.defects.is_empty(), "{report:?}");
}

#[test]
fn a_call_read_in_two_compilations_counts_once() {
    let document = document_with_calls(&[false, false, false, true]);
    let mut records = records(vec![document.clone()]);
    records.extend(self::records(vec![document]));
    let report = from_records(&mut records, &Config::default()).unwrap();
    assert!(report.defects.is_empty(), "4 calls, not 8: {report:?}");
}

#[test]
fn an_allow_comment_covers_a_dropped_result_and_it_still_counts_the_rest() {
    let mut records = records(vec![document_with_calls(&nine_uses_and_a_drop())]);
    let Measured::Known(facts) = &mut records[0].measured else {
        panic!("not measured");
    };
    facts.preceding_line = "// rstyle-allow(ignored_result): best effort".into();
    let report = from_records(&mut records, &Config::default()).unwrap();
    assert!(report.defects.is_empty(), "{report:?}");
    assert_eq!(report.suppressed, 1);
}

#[test]
fn an_excluded_call_neither_counts_nor_is_reported() {
    let mut config = Config::default();
    config.exclude.0 = vec!["**/*".into(), "*".into()];
    let report = report_of(vec![document_with_calls(&nine_uses_and_a_drop())], &config);
    assert!(report.defects.is_empty(), "{report:?}");
}

#[test]
fn a_test_file_has_no_defects_and_its_calls_do_not_count() {
    let mut document = document_with_dead_store();
    document["unit"]["files"][0]["test"] = json!(true);
    let report = report_of(vec![document], &Config::default());
    assert!(report.defects.is_empty(), "{report:?}");
    assert_eq!(report.functions, 0);

    let mut document = document_with_calls(&nine_uses_and_a_drop());
    document["unit"]["files"][0]["test"] = json!(true);
    let report = report_of(vec![document], &Config::default());
    assert!(report.defects.is_empty(), "{report:?}");
}
