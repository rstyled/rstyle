//! What a function or member with a callable role runs, read from the argument the role names
//! where the document doesn't say. The documents are rstyle-go's for `testdata/callables.go` and
//! rstyle-cxx's for `testdata/callables.cpp`. Neither frontend names what a library call runs:
//! Go's says nothing of it, and C++'s says a call with a callable argument may call back.

use super::records::{Demand, Measured};
use super::run_records;
use crate::facts::{EffectKind, FnFacts, LibraryClass};

const GO: &str = include_str!("testdata/callables-go.cpg.json");
const CXX: &str = include_str!("testdata/callables-cxx.cpg.json");

const PURE: Option<LibraryClass> = Some(LibraryClass::Pure);
const UNKNOWN: Option<LibraryClass> = Some(LibraryClass::Effect(EffectKind::Unknown));

/// The facts of the function named `name` in a frontend's document.
fn facts(document: &str, name: &str) -> FnFacts {
    let document = serde_json::from_str(document).unwrap();
    let records = run_records(vec![document], Demand::default()).unwrap();
    let record = records.into_iter().find(|record| record.name == name);
    let Measured::Known(facts) = record.unwrap().measured else {
        panic!("{name} is not measured");
    };
    *facts
}

/// The class of each library call whose callee's name ends with `member`.
fn classes(facts: &FnFacts, member: &str) -> Vec<Option<LibraryClass>> {
    let named = |call: &&crate::facts::CallEvent| call.callee.ends_with(member);
    let calls = facts.calls.iter().filter(named);
    calls.map(|call| call.library).collect()
}

/// Whether the function calls a workspace function whose identity holds `callee`, as
/// `go::example.com/callables.clean` and `cxx::count(int)` hold theirs.
fn calls(facts: &FnFacts, callee: &str) -> bool {
    facts.edges.iter().any(|edge| edge.callee.contains(callee))
}

#[test]
fn a_closure_a_callable_role_is_handed_is_the_function_s_own_code() {
    assert_eq!(classes(&facts(GO, "Once"), "Do"), [PURE]);
    assert_eq!(classes(&facts(GO, "Written"), "Do"), [PURE]);
    assert_eq!(classes(&facts(CXX, "mapped"), "transform"), [PURE]);
    assert_eq!(classes(&facts(CXX, "once"), "call_once"), [PURE]);
    let chained = facts(CXX, "chained");
    assert_eq!(classes(&chained, "and_then"), [PURE]);
    assert_eq!(classes(&chained, "or_else"), [PURE]);
}

#[test]
fn a_function_a_callable_role_is_handed_is_called_there() {
    let named = facts(GO, "Named");
    assert_eq!(classes(&named, "Do"), [PURE]);
    assert!(calls(&named, "clean"), "{:?}", named.edges);

    // Go's `c.load` is a method by its value.
    let method = facts(GO, "(*Cache).Get");
    assert_eq!(classes(&method, "Do"), [PURE]);
    assert!(calls(&method, "load"), "{:?}", method.edges);

    let named = facts(CXX, "named");
    assert_eq!(classes(&named, "transform"), [PURE]);
    assert!(calls(&named, "count"), "{:?}", named.edges);
}

#[test]
fn a_value_only_its_caller_knows_is_an_unlisted_call() {
    let value = facts(GO, "Value");
    assert_eq!(classes(&value, "Do"), [UNKNOWN]);
    assert!(value.edges.is_empty(), "{:?}", value.edges);

    let value = facts(CXX, "value");
    assert_eq!(classes(&value, "transform"), [UNKNOWN]);
    assert!(value.edges.is_empty(), "{:?}", value.edges);
}
