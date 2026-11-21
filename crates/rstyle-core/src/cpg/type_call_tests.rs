//! A call whose callee names a type runs the function its own reference names, when that
//! reference is resolved and dispatched statically. The document is rstyle-py's for:
//!
//! ```python
//! class Made:
//!     def __init__(self, first: list[int], second: list[int]) -> None:
//!         for value in second:
//!             pass
//!
//!
//! def make(xs: list[int], ys: list[int]) -> Made:
//!     return Made(xs, ys)
//! ```
//!
//! The callee `Made` refers to the class, and the call's reference names `Made.__init__`.

use super::bounds::report::from_records;
use super::records::Demand;
use super::run_records;
use serde_json::{Value, json};

const MADE: &str = include_str!("testdata/type-call-py.cpg.json");

const DEMAND: Demand = Demand {
    graphs: false,
    bounds: true,
    flow: false,
    defects: false,
};

/// `make`'s composed bound, after `change` edits the reference of its call of `Made`.
fn make_composed(change: impl FnOnce(&mut Value)) -> Value {
    composed(serde_json::from_str(MADE).unwrap(), change)
}

/// The document with `__init__`'s loop over `first` instead of `second`.
fn looping_over_first() -> Value {
    let mut document: Value = serde_json::from_str(MADE).unwrap();
    let init = document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "Made.__init__")
        .unwrap();
    for (table, field) in [("refs", "/target/id"), ("accesses", "/symbol")] {
        for row in init[table].as_array_mut().unwrap() {
            if let Some(symbol) = row.pointer_mut(field).filter(|symbol| **symbol == "f3.s2") {
                *symbol = json!("f3.s1");
            }
        }
    }
    document
}

/// `make`'s composed bound in `document`, after `change` edits the reference of its call of
/// `Made`.
fn composed(mut document: Value, change: impl FnOnce(&mut Value)) -> Value {
    let make = document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "make")
        .unwrap();
    let nodes = make["nodes"].as_array().unwrap();
    let call = nodes.iter().find(|node| node["kind"] == "call").unwrap()["id"].clone();
    let refs = make["refs"].as_array_mut().unwrap();
    change(
        refs.iter_mut()
            .find(|reference| reference["node"] == call)
            .unwrap(),
    );

    let records = run_records(vec![document], DEMAND).unwrap();
    let report = serde_json::to_value(from_records(&records, None)).unwrap();
    let rows = report["rows"].as_array().unwrap();
    rows.iter().find(|row| row["name"] == "make").unwrap()["composed"].clone()
}

/// Whether a bound leaves a call it couldn't resolve as an opaque factor.
fn leaves_the_call_opaque(composed: &Value) -> bool {
    composed["Partial"]["opaque_factors"]
        .as_array()
        .is_some_and(|factors| {
            factors.iter().any(|factor| {
                factor["kind"] == "Call"
                    && factor["reason"]
                        .as_str()
                        .is_some_and(|reason| reason.starts_with("Unresolved callee"))
            })
        })
}

#[test]
fn a_call_of_a_type_runs_the_function_its_reference_names() {
    // The value the call makes is `__init__`'s receiver, so `ys` binds `second` at 2, whose loop
    // this is.
    let composed = make_composed(|_| {});
    assert_eq!(
        composed["Proven"]["loop_body_entries"],
        json!({"Input": "len(ys)#1"}),
        "{composed}"
    );
}

#[test]
fn a_call_of_a_type_numbered_as_if_it_passed_the_receiver_binds_no_parameter() {
    // A document from before rstyle-py numbered by the callee's parameters puts `xs` at 0, where
    // `self` is. The value the call makes is `self`, so that numbering doesn't fit the call.
    for arguments in [
        json!([{"position": 0, "parameter": "first"}, {"position": 1, "parameter": "second"}]),
        Value::Null,
    ] {
        let composed = make_composed(|reference| match arguments {
            Value::Null => drop(reference.as_object_mut().unwrap().remove("arguments")),
            arguments => reference["arguments"] = arguments,
        });
        assert!(composed["Proven"].is_null(), "{composed}");
        assert!(!composed.to_string().contains("len(ys)"), "{composed}");
    }
}

#[test]
fn a_call_of_a_type_stays_opaque_when_its_reference_is_unresolved() {
    let composed = make_composed(|reference| {
        reference["status"] = json!("unresolved");
        reference["reason"] = json!("a `__new__` runs before `__init__` would");
    });
    assert!(leaves_the_call_opaque(&composed), "{composed}");
}

#[test]
fn a_call_of_a_type_stays_opaque_when_its_dispatch_is_dynamic() {
    // As rstyle-py writes a class's call when a metaclass's `__call__` runs instead: the class is
    // the receiver, and with no signature the arguments are in written order, so it binds none.
    let composed = make_composed(|reference| {
        reference["status"] = json!("unresolved");
        reference["dispatch"] = json!("dynamic");
        reference["reason"] = json!("a metaclass's `__call__` runs before `__init__` would");
        reference.as_object_mut().unwrap().remove("target");
        reference.as_object_mut().unwrap().remove("arguments");
    });
    assert!(leaves_the_call_opaque(&composed), "{composed}");
}

#[test]
fn a_call_of_a_type_stays_opaque_unless_its_dispatch_is_static() {
    let composed = make_composed(|reference| reference["dispatch"] = json!("function_value"));
    assert!(leaves_the_call_opaque(&composed), "{composed}");
}

#[test]
fn a_call_of_a_type_stays_opaque_when_its_reference_names_the_type() {
    // As rstyle-py writes a call of a class with no `__init__` of its own.
    let composed = make_composed(|reference| {
        reference["target"] = json!({"kind": "definition", "id": "def:py::made.Made"});
    });
    assert!(leaves_the_call_opaque(&composed), "{composed}");
}

#[test]
fn a_call_in_method_form_without_a_dispatch_binds_no_argument_to_a_formal() {
    // Read in method form, the call's first child is its receiver, which its written arguments
    // leave out: `xs` mustn't bind `self`, nor `ys` bind `first`.
    let written = composed(looping_over_first(), |_| {});
    assert_eq!(
        written["Proven"]["loop_body_entries"],
        json!({"Input": "len(xs)#0"}),
        "{written}"
    );
    let composed = composed(looping_over_first(), |reference| {
        reference.as_object_mut().unwrap().remove("dispatch");
        reference["reason"] = json!("a call in method form records no dispatch");
    });
    assert!(composed["Proven"].is_null(), "{composed}");
    assert!(!composed.to_string().contains("len(ys)"), "{composed}");
}
