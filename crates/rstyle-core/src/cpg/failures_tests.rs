//! Where a Go function's error comes from. The documents are rstyle-go's for `testdata/errors.go`
//! and for `testdata/errors-edited.go`, where `open` can also return `ErrClosed` and `Each` skips a
//! step that isn't there.

use super::failures::{self, Failure, Source};
use super::flow;
use super::model::Document;
use super::records::{Demand, Record};
use super::run_records;
use super::tree::Tables;
use crate::config::Config;
use crate::merge_risk::{self, ChangeReport, Snapshot, SourceFiles};
use serde_json::{Value, json};

const BASE: &str = include_str!("testdata/errors-go.cpg.json");
const EDITED: &str = include_str!("testdata/errors-edited-go.cpg.json");

const DEMAND: Demand = Demand {
    graphs: true,
    bounds: false,
    flow: true,
    defects: false,
};

fn document(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn records(document: Value) -> Vec<Record> {
    run_records(vec![document], DEMAND).unwrap()
}

fn failures_in(document: Value, name: &str) -> Option<Vec<Failure>> {
    let records = records(document);
    let record = records.iter().find(|record| record.name == name).unwrap();
    record.failures.clone()
}

fn failures(name: &str) -> Option<Vec<Failure>> {
    failures_in(document(BASE), name)
}

/// Where a function's error can come from, however deep each is wrapped.
fn sources(name: &str) -> Vec<Source> {
    let found = failures(name).unwrap();
    found.into_iter().map(|failure| failure.source).collect()
}

fn at_once(source: Source) -> Failure {
    Failure { source, wrapped: 0 }
}

#[test]
fn a_message_is_where_a_constructor_makes_the_error() {
    let open = failures("open").unwrap();
    assert_eq!(open, vec![at_once(Source::Message("no name".into()))]);
}

#[test]
fn a_wrapped_error_keeps_its_source_one_wrapping_deeper() {
    let load = failures("Load").unwrap();
    let call = Failure {
        source: Source::Call("go::example.com/errs.open".into()),
        wrapped: 1,
    };
    assert_eq!(
        load,
        vec![at_once(Source::Message("load %s: %w".into())), call]
    );
}

#[test]
fn only_the_assignment_that_reaches_the_return_counts() {
    let last = failures("Last").unwrap();
    assert_eq!(last, vec![at_once(Source::Message("second".into()))]);
}

#[test]
fn a_function_value_can_t_be_followed() {
    assert_eq!(failures("Each").unwrap(), vec![at_once(Source::Unknown)]);
}

#[test]
fn the_error_a_result_tuple_ends_with_is_the_one_returned() {
    let parse = failures("Parse").unwrap();
    assert_eq!(
        parse,
        vec![at_once(Source::Call("go::strconv.Atoi".into()))]
    );
}

#[test]
fn what_a_closure_returns_is_the_closure_s_own() {
    let later = failures("Later").unwrap();
    assert_eq!(later, vec![at_once(Source::Message("outside".into()))]);
}

#[test]
fn an_error_a_loop_wraps_in_itself_is_followed_once() {
    let messages = ["never tried", "try %d: %w"].map(|message| Source::Message(message.into()));
    assert_eq!(sources("Retry"), messages);
}

#[test]
fn a_value_that_names_no_definition_can_t_be_followed() {
    assert_eq!(failures("First").unwrap(), vec![at_once(Source::Unknown)]);
}

/// Only a value typed `error` is read as one, so far: `Shut` returns a `*closedError`.
#[test]
fn an_error_of_its_own_type_is_not_read_yet() {
    assert_eq!(failures("Shut"), Some(Vec::new()));
}

#[test]
fn without_flow_no_error_can_be_followed() {
    let mut document = document(BASE);
    document["coverage"]["layers"]["cfg"]["status"] = json!("unsupported");
    let load = failures_in(document, "Load").unwrap();
    assert_eq!(load, vec![at_once(Source::Unknown)]);
}

/// Reading a document rejects a definition its unit doesn't hold, so this asks `failures::of`
/// itself, as a caller with a unit of its own would.
#[test]
fn a_definition_the_unit_doesn_t_hold_can_t_be_followed() {
    let mut document = document(BASE);
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    let at = functions
        .iter()
        .position(|function| function["name"] == "open")
        .unwrap();
    for found in functions[at]["refs"].as_array_mut().unwrap() {
        if found["target"]["id"] == "def:go::nil" {
            found["target"]["id"] = json!("def:missing");
        }
    }
    let document: Document = serde_json::from_value(document).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let tables = Tables::new(unit).unwrap();
    let open = &unit.functions.as_deref().unwrap()[at];
    let flow = flow::derive(&document.coverage, open, tables.wording());

    let made = at_once(Source::Message("no name".into()));
    let found = failures::of(open, &flow, &tables);
    assert_eq!(found, Some(vec![made, at_once(Source::Unknown)]));
}

fn compared() -> ChangeReport {
    let config = Config::default();
    let snapshot = |text| Snapshot::of(records(document(text)), SourceFiles::default(), &config);
    merge_risk::compare(
        &snapshot(BASE).unwrap(),
        &snapshot(EDITED).unwrap(),
        &config,
    )
}

#[test]
fn a_change_is_said_where_it_happens_and_where_it_is_passed_on() {
    let report = compared();

    let closed = Source::Variable("go::example.com/errs.ErrClosed".into());
    let changed: Vec<(&str, &[Source], usize)> = report
        .errors
        .changed
        .iter()
        .map(|change| {
            (
                change.name.as_str(),
                change.gained.as_slice(),
                change.wrapped,
            )
        })
        .collect();
    let gained = std::slice::from_ref(&closed);
    assert_eq!(changed, vec![("Load", gained, 1), ("open", gained, 0)]);
}

#[test]
fn a_changed_function_that_can_fail_through_a_function_value_is_untraced() {
    let report = compared();
    let untraced: Vec<&str> = report
        .errors
        .untraced
        .iter()
        .map(|change| change.name.as_str())
        .collect();
    assert_eq!(untraced, ["Each"]);
    assert_eq!(report.errors.followed, 2);
}
