//! Runtime bounds over loops with no guard of their own, and C++'s do statements, whose test is the
//! last statement of their block: `loops.cpp` and `loops.go` in rstyle-core's test data, with the
//! documents rstyle-cxx and rstyle-go wrote for them.
use super::callables::bounds;

/// A loop with no guard of its own, or one whose test the lowering can't bound, may run any number
/// of passes: its own entries are opaque, and the rest of the function keeps its bound.
#[test]
fn a_loop_without_a_guard_it_bounds_is_opaque() {
    let cxx = bounds("loops-cxx.cpg.json");
    let go = bounds("loops-go.cpg.json");
    for row in [
        &cxx["forever"],
        &cxx["repeated"],
        &cxx["skipping"],
        &cxx["spinning"],
        &go["Forever"],
    ] {
        assert!(
            row.starts_with("partial: loop body entries <= ?loop("),
            "{row}"
        );
        assert!(
            row.contains("Loop requires an integer less-than guard"),
            "{row}"
        );
    }
    // Its body's calls count once per pass.
    let repeated = &cxx["repeated"];
    assert!(
        repeated.contains("\ncount invocations <= ?loop("),
        "{repeated}"
    );
}

/// A do statement whose test is false wherever it runs, as a macro's `while (0)` is, runs its body
/// once and isn't a loop, and a continue in it goes to its test.
#[test]
fn a_do_statement_that_tests_false_runs_once() {
    let cxx = bounds("loops-cxx.cpg.json");
    let macros = &cxx["macros"];
    assert!(
        macros.starts_with("loop body entries <= len(xs)#0;"),
        "{macros}"
    );
    let counted = macros.matches("\ncount invocations <= len(xs)#0").count();
    assert_eq!(counted, 3, "{macros}");
    let once = &cxx["once"];
    assert!(once.starts_with("loop body entries <= 0;"), "{once}");
    assert!(once.contains("\ncount invocations <= 1"), "{once}");
}
