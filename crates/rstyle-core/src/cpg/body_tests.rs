//! A body, what a module or a type runs when it's loaded or defined, is its author's code but wasn't
//! written as a function. The documents are rstyle-py's for:
//!
//! ```python
//! import functools
//! import logging
//!
//! LOG = logging.getLogger(__name__)
//!
//!
//! class Store:
//!     LIMIT = 3
//!
//!     def put(self, value: int) -> int:
//!         return value
//!
//!
//! @functools.cache
//! def load(value: int) -> int:
//!     return value
//! ```
//!
//! and for the same module with `put` and `load` returning `value + 1`.

use super::records::{Demand, Measured, Record};
use super::run_records;
use crate::config::Config;
use crate::merge_risk::{self, ChangeKind, ChangeReport, Snapshot, SourceFiles};
use serde_json::{Value, json};

const BASE: &str = include_str!("testdata/bodies-py.cpg.json");
const EDITED: &str = include_str!("testdata/bodies-edited-py.cpg.json");

const DEMAND: Demand = Demand {
    graphs: true,
    bounds: false,
    flow: false,
    defects: false,
};

fn document(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn function<'d>(document: &'d mut Value, name: &str) -> &'d mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == name)
        .unwrap()
}

fn records(document: Value) -> Vec<Record> {
    run_records(vec![document], DEMAND).unwrap()
}

fn record<'r>(records: &'r [Record], name: &str) -> &'r Record {
    records.iter().find(|record| record.name == name).unwrap()
}

fn compare(base: Value, head: Value) -> ChangeReport {
    let config = Config::default();
    let snapshot =
        |document| Snapshot::of(records(document), SourceFiles::default(), &config).unwrap();
    merge_risk::compare(&snapshot(base), &snapshot(head), &config)
}

fn changed(report: &ChangeReport) -> Vec<(&str, ChangeKind)> {
    let mut changed: Vec<_> = report
        .changes
        .iter()
        .map(|change| (change.name.as_str(), change.kind))
        .collect();
    changed.sort();
    changed
}

#[test]
fn a_module_or_class_body_is_a_body_and_its_functions_are_the_author_s() {
    let records = records(document(BASE));
    for name in ["<module>", "Store.<body>"] {
        let record = record(&records, name);
        assert!(matches!(record.measured, Measured::Body(_)), "{name}");
        // Nothing that judges how a function is written reads it.
        assert!(super::authored(record.clone()).is_none(), "{name}");
    }
    for name in ["Store.put", "load"] {
        assert!(matches!(
            record(&records, name).measured,
            Measured::Known(_)
        ));
    }
}

#[test]
fn a_body_reports_its_own_events_where_it_s_written() {
    let records = records(document(BASE));
    let module = record(&records, "<module>");
    let Measured::Body(facts) = &module.measured else {
        panic!("<module> is not a body");
    };

    let observation = module.measured.observation().unwrap();
    assert_eq!(observation.name, "<module>");
    assert_eq!((observation.line, observation.column), (1, 1));
    assert!(!observation.generated_file);
    // What it calls, such as `logging.getLogger`, is what its effects are read from.
    assert_eq!(observation.calls.len(), facts.calls.len());
    assert_eq!(
        observation.connections.map(|connections| connections.id),
        Some(facts.id.clone())
    );
    // A function a rule reads has its own facts, not an observation.
    assert!(record(&records, "load").measured.observation().is_none());
}

#[test]
fn a_run_reads_a_body_s_effects_but_no_rule_reads_the_body() {
    let inputs = super::run::run_inputs(records(document(BASE)), &Config::default()).unwrap();

    let mut authored: Vec<_> = inputs
        .facts
        .iter()
        .map(|facts| facts.name.as_str())
        .collect();
    authored.sort_unstable();
    assert_eq!(authored, ["Store.put", "load"]);
    assert_eq!(inputs.effects.len(), 2);
}

#[test]
fn a_body_s_text_leaves_out_the_functions_written_in_it() {
    let records = records(document(BASE));
    let own = |name| {
        let record = record(&records, name);
        let Measured::Body(facts) = &record.measured else {
            panic!("{name} is not a body");
        };
        assert_eq!(facts.source, record.source);
        record.source.clone()
    };

    // A decorator runs in the module, so it's the module's.
    assert_eq!(
        own("<module>"),
        "import functools\nimport logging\n\nLOG = logging.getLogger(__name__)\n\n\nclass Store:\n    \
         \n\n\n@functools.cache\n"
    );
    assert_eq!(own("Store.<body>"), "LIMIT = 3\n\n    ");
}

#[test]
fn a_desugaring_of_anything_else_leaves_the_function_the_author_s() {
    let mut base = document(BASE);
    let module = function(&mut base, "<module>");
    let root = module["root"].clone();
    let provenance = json!({"origin": "desugaring", "construct": "rust:await"});
    module["signature"]["provenance"] = provenance.clone();
    for node in module["nodes"].as_array_mut().unwrap() {
        if node["id"] == root {
            node["provenance"] = provenance.clone();
        }
    }

    let records = records(base);
    assert!(matches!(
        record(&records, "<module>").measured,
        Measured::Known(_)
    ));
}

#[test]
fn a_change_in_a_function_a_body_holds_is_that_function_s_alone() {
    let report = compare(document(BASE), document(EDITED));

    assert_eq!(
        changed(&report),
        [
            ("Store.put", ChangeKind::Changed),
            ("load", ChangeKind::Changed)
        ]
    );
    assert_eq!(report.unchanged, 2);
    assert_eq!(report.not_compared, 0);
}

#[test]
fn a_change_in_what_a_body_runs_is_the_body_s() {
    let mut head = document(BASE);
    for node in function(&mut head, "Store.<body>")["nodes"]
        .as_array_mut()
        .unwrap()
    {
        if node["literal"] == "3" {
            node["value"]["value"] = json!("4");
        }
    }

    let report = compare(document(BASE), head);

    assert_eq!(changed(&report), [("Store.<body>", ChangeKind::Changed)]);
    assert!(report.changes[0].values_changed);
}
