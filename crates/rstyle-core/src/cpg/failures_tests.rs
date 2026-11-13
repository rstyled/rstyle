//! Where a Go function's error comes from. The documents are rstyle-go's for `testdata/errors.go`
//! and for `testdata/errors-edited.go`, where `open` can also return `ErrClosed`.

use super::failures::{Failure, Source};
use super::records::{Demand, Record};
use super::run_records;
use crate::config::Config;
use crate::merge_risk::{self, Snapshot, SourceFiles};

const BASE: &str = include_str!("testdata/errors-go.cpg.json");
const EDITED: &str = include_str!("testdata/errors-edited-go.cpg.json");

const DEMAND: Demand = Demand {
    graphs: true,
    bounds: false,
    flow: true,
    defects: false,
};

fn records(text: &str) -> Vec<Record> {
    run_records(vec![serde_json::from_str(text).unwrap()], DEMAND).unwrap()
}

fn failures(name: &str) -> Option<Vec<Failure>> {
    let records = records(BASE);
    let record = records.iter().find(|record| record.name == name).unwrap();
    record.failures.clone()
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
fn a_change_is_said_where_it_happens_and_where_it_is_passed_on() {
    let config = Config::default();
    let snapshot = |text| Snapshot::of(records(text), SourceFiles::default(), &config).unwrap();
    let report = merge_risk::compare(&snapshot(BASE), &snapshot(EDITED), &config);

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
    assert!(report.errors.untraced.is_empty());
}
