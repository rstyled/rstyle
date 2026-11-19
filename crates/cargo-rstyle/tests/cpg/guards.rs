//! Runtime bounds over `while` loops whose condition runs something: `guards.py` and `guards.cpp`
//! in rstyle-core's test data, with the documents rstyle-py and rstyle-cxx wrote for them.
use super::callables::bounds;

/// A condition that calls something runs before every pass, where a `continue` goes, and once
/// more, so the loop keeps its one guard: its entries are opaque rather than the function
/// unknown, and the call counts once per pass and once more.
#[test]
fn a_condition_that_calls_is_tested_again_after_each_pass() {
    let python = bounds("guards-py.cpg.json");
    let polled = &python["polled"];
    assert!(
        polled.starts_with("partial: loop body entries <= ?loop("),
        "{polled}"
    );
    assert!(polled.contains("\nready invocations <= ?loop("), "{polled}");
    assert!(polled.contains("\nready invocations <= 1"), "{polled}");
    let counted = &python["counted"];
    assert!(
        counted.starts_with("loop body entries <= max(n, 0);"),
        "{counted}"
    );
}

/// A condition that runs something before the comparison it tests, as C++'s comma does, keeps that
/// comparison as the loop's guard.
#[test]
fn a_comparison_after_a_call_still_guards_the_loop() {
    let cxx = bounds("guards-cxx.cpg.json");
    let sequenced = &cxx["sequenced"];
    assert!(
        sequenced.starts_with("loop body entries <= limit;"),
        "{sequenced}"
    );
    assert!(
        sequenced.contains("\ncount invocations <= limit"),
        "{sequenced}"
    );
    assert!(
        sequenced.contains("\ncount invocations <= 1"),
        "{sequenced}"
    );
}
