//! Runtime bounds over loops over dicts and sets: `containers.py` and `containers.cpp` in
//! rstyle-core's test data, with the documents rstyle-py and rstyle-cxx wrote for them.
use super::callables::bounds;

/// A dict's items or values, and a set, count their entries while nothing in the function can
/// change them, which the frontend says by giving them a `map` or `set` type.
#[test]
fn a_dict_or_a_set_nothing_changes_counts_its_entries() {
    let python = bounds("containers-py.cpg.json");
    for (function, bound) in [
        ("pairs", "len(counts)#0;"),
        ("values", "len(counts)#0;"),
        ("members", "len(seen)#0;"),
    ] {
        let row = &python[function];
        let expected = format!("loop body entries <= {bound}");
        assert!(row.starts_with(&expected), "{function}: {row}");
    }
    let cxx = bounds("containers-cxx.cpg.json");
    let members = &cxx["members"];
    assert!(
        members.starts_with("loop body entries <= len(seen)#0;"),
        "{members}"
    );
}

/// A dict the function changes before the scan is the object it is, whose entries the scan has no
/// count for.
#[test]
fn a_dict_the_function_changes_has_no_count() {
    let python = bounds("containers-py.cpg.json");
    let grown = &python["grown"];
    assert!(grown.starts_with("unknown: Only resolved"), "{grown}");
}
