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

/// What a frontend wrote of `a.go`, `a.cpp` or `a.py`, and each function a value reached a sink in.
fn taint_in(document: &str) -> Vec<(String, usize)> {
    let document: Value = serde_json::from_str(document).unwrap();
    let report = report_of(vec![document], &Config::default());
    assert!(report.unknown.is_empty(), "{:?}", report.unknown);

    let taint = report.defects.iter().filter(|row| row.rule == "taint_flow");
    taint
        .map(|row| {
            let name = row.function.rsplit(['.', ':']).next().unwrap_or_default();
            (name.to_owned(), row.line)
        })
        .collect()
}

#[test]
fn input_is_followed_in_what_a_go_frontend_writes() {
    let found = taint_in(include_str!("../cpg/testdata/taint-go.cpg.json"));
    assert_eq!(
        found,
        [
            ("compound".to_owned(), 14),
            ("builder".to_owned(), 34),
            ("shell".to_owned(), 42),
            ("viaSource".to_owned(), 48),
            ("output".to_owned(), 50),
        ],
        "overwritten, parameterized, onlyDecides, number, argv, viaIgnoring and fields are clean"
    );
}

#[test]
fn input_is_followed_in_what_a_cxx_frontend_writes() {
    let found = taint_in(include_str!("../cpg/testdata/taint-cxx.cpg.json"));
    let lines: Vec<usize> = found.iter().map(|(_, line)| *line).collect();
    assert_eq!(
        lines,
        [5, 18, 21, 22],
        "local, out_parameter, via_source and run; overwritten and only_decides are clean: {found:?}"
    );
}

#[test]
fn input_is_followed_in_what_a_python_frontend_writes() {
    let found = taint_in(include_str!("../cpg/testdata/taint-py.cpg.json"));
    let expected = [
        ("environ", 11),
        ("getenv", 15),
        ("shell", 19),
        ("no_shell", 27),
        ("literal_shell", 35),
        ("evaluated", 47),
        ("path", 51),
        ("compound", 57),
        ("lines", 81),
        ("write", 85),
        ("via_source", 105),
        ("listed_program", 122),
        ("listed_code", 126),
        ("string_program", 134),
        ("keyword_list", 138),
        ("exec_code", 147),
        ("exec_listed_code", 151),
        ("keyword_first", 159),
        ("open_by_name", 167),
        ("popen_after_stdout", 175),
        ("run_in", 180),
        ("save", 192),
        ("run", 201),
    ];
    let expected = expected.map(|(name, line)| (name.to_owned(), line));
    assert_eq!(
        found, expected,
        "argv, not_the_command, program_argument, quoted, parameterized, number, overwritten, \
         only_decides, via_ignoring, environment, fields, listed_data, variable_list, exec_data, \
         keyword_cwd, open_encoding and helper_directory are clean"
    );
}

#[test]
fn without_bindings_a_python_keyword_is_numbered_where_its_written() {
    let mut document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/taint-py.cpg.json")).unwrap();
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    let refs = functions.iter_mut().flat_map(|function| {
        let refs = function["refs"].as_array_mut().unwrap();
        refs.iter_mut()
    });
    for reference in refs {
        reference.as_object_mut().unwrap().remove("arguments");
    }

    let report = report_of(vec![document], &Config::default());
    let taint = report.defects.iter().filter(|row| row.rule == "taint_flow");
    let found: Vec<(usize, &str)> = taint
        .map(|row| (row.line, row.message.split(" reaches").next().unwrap()))
        .collect();
    // `encoding=input()` reads as the path, and `directory=input()` as the command.
    assert!(found.contains(&(171, "input from `builtins.input` on line 171")));
    assert!(found.contains(&(180, "input from `builtins.input` on line 188")));
    // `shell=True` names no parameter, and `args=[..]` isn't looked into.
    for missed in [138, 159, 167, 175, 192, 201] {
        assert!(found.iter().all(|(line, _)| *line != missed), "{missed}");
    }
}

#[test]
fn a_function_compiled_twice_reports_its_flow_once() {
    let document = || -> Value {
        serde_json::from_str(include_str!("../cpg/testdata/taint-go.cpg.json")).unwrap()
    };
    let once = report_of(vec![document()], &Config::default());
    let mut compiled_twice = records(vec![document()]);
    compiled_twice.extend(records(vec![document()]));
    let twice = from_records(&mut compiled_twice, &Config::default()).unwrap();
    assert_eq!(once.defects, twice.defects);
}

#[test]
fn an_argument_its_sibling_methods_disagree_with_is_found_in_what_a_go_frontend_writes() {
    let document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/deviant-argument-go.cpg.json")).unwrap();
    let report = report_of(vec![document], &Config::default());
    assert!(report.unknown.is_empty(), "{:?}", report.unknown);

    let found: Vec<(usize, &str)> = report
        .defects
        .iter()
        .filter(|row| row.rule == "deviant_argument")
        .map(|row| (row.line, row.message.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            (
                64,
                "`NewContext` is passed `w.Log` here, where 5 of its 6 calls in sibling methods \
                 pass `log`"
            ),
            (
                66,
                "`(*Writer).record` is passed `name` here, where 6 of its 7 calls in sibling \
                 methods pass `options.Kind`"
            ),
            (
                79,
                "`(*Writer).record` is passed `startLegacy` here, where 6 of its 7 calls in \
                 sibling methods pass `startStorage`"
            ),
        ],
        "Store's and Auth's differences are deliberate; `options.Kind` holds for a `DeleteOptions` only through its embedded `TypeMeta`"
    );
}

#[test]
fn shared_testify_suites_in_regular_go_files_do_not_supply_argument_patterns() {
    let mut document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/deviant-argument-go.cpg.json")).unwrap();
    let types = document["unit"]["types"].as_array_mut().unwrap();
    let writer = types.iter_mut().find(|ty| ty["id"] == "t36").unwrap();
    writer["fields"].as_array_mut().unwrap().push(json!({
        "name": "Suite", "type": "type:test-suite", "embedded": true
    }));
    types.push(json!({"id": "type:test-suite", "kind": "record",
        "spelling": "github.com/stretchr/testify/suite.Suite", "fields": []}));
    let report = report_of(vec![document], &Config::default());
    assert!(report.unknown.is_empty(), "{:?}", report.unknown);
    assert!(
        report
            .defects
            .iter()
            .all(|row| row.rule != "deviant_argument")
    );
}

#[test]
fn a_parser_branch_can_deliberately_use_a_different_tokens_literal() {
    let document: Value = serde_json::from_str(include_str!(
        "../cpg/testdata/deviant-argument-branches.cpg.json"
    ))
    .unwrap();
    let report = report_of(vec![document], &Config::default());
    assert!(report.unknown.is_empty(), "{:?}", report.unknown);
    assert!(
        report
            .defects
            .iter()
            .all(|row| row.rule != "deviant_argument")
    );
}

#[test]
fn a_named_suite_field_does_not_make_a_production_type_a_test_suite() {
    let mut document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/deviant-argument-go.cpg.json")).unwrap();
    let types = document["unit"]["types"].as_array_mut().unwrap();
    let writer = types.iter_mut().find(|ty| ty["id"] == "t36").unwrap();
    writer["fields"].as_array_mut().unwrap().push(json!({
        "name": "Suite", "type": "type:test-suite"
    }));
    types.push(json!({"id": "type:test-suite", "kind": "record",
        "spelling": "github.com/stretchr/testify/suite.Suite", "fields": []}));
    let report = report_of(vec![document], &Config::default());
    assert_eq!(
        report
            .defects
            .iter()
            .filter(|row| row.rule == "deviant_argument")
            .count(),
        3
    );
}

#[test]
fn an_allowed_argument_difference_still_counts_its_siblings_and_counts_once() {
    let document: Value =
        serde_json::from_str(include_str!("../cpg/testdata/deviant-argument-go.cpg.json")).unwrap();
    let mut records = records(vec![document]);
    for record in &mut records {
        let Measured::Known(facts) = &mut record.measured else {
            panic!("not measured");
        };
        facts.preceding_line =
            "// rstyle-allow(deviant_argument): deliberate protocol difference".into();
    }
    records.extend(records.clone());
    let report = from_records(&mut records, &Config::default()).unwrap();
    assert!(report.unknown.is_empty(), "{report:?}");
    assert!(
        report
            .defects
            .iter()
            .all(|row| row.rule != "deviant_argument")
    );
    assert_eq!(
        report.suppressed, 3,
        "each of the three argument differences counts once"
    );
}

/// `identical-branches.go`, where each function says whether its condition is reported.
fn identical_branches() -> Value {
    serde_json::from_str(include_str!("../cpg/testdata/identical-branches.cpg.json")).unwrap()
}

fn found_at(report: &DefectsReport) -> Vec<(usize, &str, bool)> {
    let follows = |row: &DefectRow| row.message.contains("what follows it");
    let rows = report.defects.iter();
    rows.map(|row| (row.line, row.rule, follows(row))).collect()
}

#[test]
fn a_condition_that_runs_the_same_code_either_way_is_reported_where_it_is_written() {
    let report = report_of(vec![identical_branches()], &Config::default());
    assert!(report.unknown.is_empty(), "{:?}", report.unknown);
    assert_eq!(
        found_at(&report),
        [
            (11, "identical_branches", true),
            (20, "identical_branches", false),
            (30, "identical_branches", true),
            (41, "identical_branches", true),
        ],
        "the other six functions' conditions decide something"
    );
}

#[test]
fn a_branch_a_macro_wrote_is_not_compared() {
    let mut document = identical_branches();
    let always_false = &mut document["unit"]["functions"][1];
    let nodes = always_false["nodes"].as_array_mut().unwrap();
    let returned = nodes.iter_mut().find(|node| node["kind"] == "return");
    returned.unwrap()["provenance"]["origin"] = json!("macro_expansion");
    let report = report_of(vec![document], &Config::default());
    let lines: Vec<usize> = report.defects.iter().map(|row| row.line).collect();
    assert_eq!(lines, [20, 30, 41]);
}

#[test]
fn an_allow_comment_suppresses_a_condition_that_changes_nothing() {
    let mut records = records(vec![identical_branches()]);
    for record in &mut records {
        let Measured::Known(facts) = &mut record.measured else {
            panic!("not measured");
        };
        facts.preceding_line = "// rstyle-allow(identical_branches): kept for the next flag".into();
    }
    let report = from_records(&mut records, &Config::default()).unwrap();
    assert!(report.defects.is_empty(), "{:?}", report.defects);
    assert_eq!(report.suppressed, 4);
}
