//! A call names the library function it runs in the dependence graph, so a `[[risk.calls]]` rule
//! for one fires, though an overloaded Python function's callee has no one function type. The
//! documents are rstyle-py's for:
//!
//! ```python
//! import os
//!
//!
//! def size(xs: list[int], name: str) -> int:
//!     if isinstance(name, str):
//!         print(name)
//!     xs.append(len(xs))
//!     return len(xs) + len(os.getenv("PAD", ""))
//! ```
//!
//! and for the same `size` with `subprocess.run(["true"], check=False)` before its `return`. In
//! typeshed `print`, `os.getenv` and `subprocess.run` are overloaded and `len` isn't.

use super::records::Demand;
use super::run_records;
use crate::config::Config;
use crate::facts::NodeKind;
use crate::merge_risk::rating::Level;
use crate::merge_risk::{self, Snapshot, SourceFiles};
use serde_json::Value;

const SIZES: &str = include_str!("testdata/effects-py.cpg.json");
const RUNS: &str = include_str!("testdata/effects-run-py.cpg.json");

const DEMAND: Demand = Demand {
    graphs: true,
    bounds: false,
    flow: false,
    defects: false,
};

fn document(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn an_overloaded_library_function_s_call_names_it_in_the_graph() {
    let records = run_records(vec![document(SIZES)], DEMAND).unwrap();
    let size = records.iter().find(|record| record.name == "size").unwrap();
    let graph = size.graph.as_ref().unwrap();
    let mut callees: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Call)
        .filter_map(|node| node.callee_id.as_deref())
        .collect();
    callees.sort_unstable();
    callees.dedup();

    for callee in ["py::builtins.len", "py::builtins.print", "py::os.getenv"] {
        assert!(callees.contains(&callee), "{callee} in {callees:?}");
    }
}

#[test]
fn a_rule_for_a_library_function_fires_when_a_call_of_it_is_added() {
    let policy = "[risk]\npdg_change = \"low\"\n\n\
                  [[risk.calls]]\ncallee = \"py::subprocess.run\"\nlevel = \"high\"\n";
    let config = Config::parse(policy).unwrap();
    let snapshot = |text| {
        let records = run_records(vec![document(text)], DEMAND).unwrap();
        Snapshot::of(records, SourceFiles::default(), &config).unwrap()
    };

    let report = merge_risk::compare(&snapshot(SIZES), &snapshot(RUNS), &config);

    let size = report
        .changes
        .iter()
        .find(|change| change.name == "size")
        .unwrap();
    assert!(
        size.calls_added.contains(&"py::subprocess.run".to_owned()),
        "{:?}",
        size.calls_added
    );
    assert_eq!(report.rating.risk, Level::High);
    assert!(report.rating.reasons.iter().any(|reason| {
        reason.level == Level::High && reason.callee.as_deref() == Some("py::subprocess.run")
    }));
}
