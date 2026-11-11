//! A Python library call is classified by `py.toml`, and an entry that only holds for a call that
//! calls nothing back holds where the document says so. The document is rstyle-py's for:
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
//! rstyle-py says the `isinstance` call calls nothing back, since `str` has no workspace methods, and
//! says nothing of `len`'s, which runs `__len__`.

use super::records::{Demand, Measured};
use super::run_records;
use crate::facts::{EffectKind, LibraryClass};
use serde_json::{Value, json};

const SIZES: &str = include_str!("testdata/effects-py.cpg.json");

const PURE: Option<LibraryClass> = Some(LibraryClass::Pure);

/// Each library call `size` makes, by callee, with its class.
fn classes(document: Value) -> Vec<(String, Option<LibraryClass>)> {
    let records = run_records(vec![document], Demand::default()).unwrap();
    let record = records.iter().find(|record| record.name == "size").unwrap();
    let Measured::Known(facts) = &record.measured else {
        panic!("size is not measured");
    };
    facts
        .calls
        .iter()
        .map(|call| (call.callee.clone(), call.library))
        .collect()
}

fn class_of(classes: &[(String, Option<LibraryClass>)], callee: &str) -> Vec<Option<LibraryClass>> {
    classes
        .iter()
        .filter(|(name, _)| name == callee)
        .map(|(_, class)| *class)
        .collect()
}

#[test]
fn a_python_call_is_classified_by_what_it_does() {
    let classes = classes(serde_json::from_str(SIZES).unwrap());

    assert_eq!(class_of(&classes, "builtins.isinstance"), [PURE]);
    assert_eq!(
        class_of(&classes, "builtins.print"),
        [Some(LibraryClass::Effect(EffectKind::Io))]
    );
    assert_eq!(class_of(&classes, "builtins.list.append"), [PURE]);
    assert_eq!(
        class_of(&classes, "os.getenv"),
        [Some(LibraryClass::Effect(EffectKind::Global))]
    );
    // It runs `__len__`, and nothing says that's the library's.
    assert_eq!(class_of(&classes, "builtins.len"), [None, None, None]);
}

#[test]
fn a_call_that_calls_nothing_back_is_pure_by_an_entry_that_needs_it() {
    let mut document: Value = serde_json::from_str(SIZES).unwrap();
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    let size = functions.iter_mut().find(|f| f["name"] == "size").unwrap();
    // The call `len(xs)` that `xs.append` is handed.
    let call = "f1.n14";
    assert_eq!(
        size["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == call)
            .unwrap()["kind"],
        "call"
    );
    for reference in size["refs"].as_array_mut().unwrap() {
        if reference["node"] == call {
            reference["calls_back"] = json!({"value": false, "reason": "a list's length"});
        }
    }

    let classes = classes(document);
    let lens = class_of(&classes, "builtins.len");
    assert_eq!(
        lens.iter().filter(|class| **class == PURE).count(),
        1,
        "{lens:?}"
    );
    assert_eq!(
        lens.iter().filter(|class| class.is_none()).count(),
        2,
        "{lens:?}"
    );
}
