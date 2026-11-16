//! Runtime bounds over code that raises and handles exceptions: `exceptions.py` and
//! `exceptions.cpp` in rstyle-core's test data, with the documents rstyle-py and rstyle-cxx wrote
//! for them.
use super::callables::bounds;

/// Raising leaves a scan early, an assertion that fails raises, and a handler runs where the body
/// raised: none of them makes the scans around them unknown.
#[test]
fn raising_and_handling_keep_the_bounds_around_them() {
    let python = bounds("exceptions-py.cpg.json");
    for (function, bound) in [
        ("checked", "len(xs)#0;"),
        ("asserted", "len(xs)#0;"),
        ("handled", "len(xs)#0 + len(xs)#0 * len(ys)#1;"),
        ("caught", "len(xs)#0 + len(xs)#0 * len(ys)#1;"),
        ("skipped", "len(xs)#0 + len(xs)#0 * len(ys)#1;"),
    ] {
        let row = &python[function];
        let expected = format!("loop body entries <= {bound}");
        assert!(row.starts_with(&expected), "{function}: {row}");
    }
}

/// A finally block runs once on every way out, a return among them, so its scan counts once.
#[test]
fn a_finally_block_runs_once_on_every_way_out() {
    let python = bounds("exceptions-py.cpg.json");
    let row = &python["finished"];
    let once = "loop body entries <= len(ys)#1 + len(xs)#0;";
    assert!(row.starts_with(once), "{row}");
}

/// What a body sets before it raises reaches its handler, and the loop after it, unknown.
#[test]
fn a_counter_a_raising_body_sets_is_unknown_after_it() {
    let python = bounds("exceptions-py.cpg.json");
    let cxx = bounds("exceptions-cxx.cpg.json");
    for row in [&python["interrupted"], &cxx["interrupted"]] {
        assert!(row.starts_with("partial: "), "{row}");
        assert!(row.contains("Limit lacks an invariant"), "{row}");
    }
}

/// A with enters each context and exits it once, binding a pattern or a name, and one that binds
/// an attribute stays unknown.
#[test]
fn a_with_enters_and_exits_its_context_once() {
    let python = bounds("exceptions-py.cpg.json");
    for function in ["managed", "bound", "rebound"] {
        let row = &python[function];
        assert!(
            row.starts_with("loop body entries <= len(xs)#1;"),
            "{function}: {row}"
        );
        let calls = row.matches("\npy:with invocations <= 1").count();
        assert_eq!(calls, 2, "{function}: {row}");
    }
    let kept = &python["Holder.kept"];
    assert!(kept.starts_with("unknown: A with target"), "{kept}");
}

/// C++ throws and catches the same way, and a throw counts what it constructs.
#[test]
fn cxx_throws_and_catches_keep_the_bounds_around_them() {
    let cxx = bounds("exceptions-cxx.cpg.json");
    let checked = &cxx["checked"];
    assert!(
        checked.starts_with("loop body entries <= len(xs)#0;"),
        "{checked}"
    );
    let constructed = "\nstd::invalid_argument::invalid_argument invocations <= len(xs)#0";
    assert!(checked.contains(constructed), "{checked}");
    let handled = &cxx["handled"];
    let scanned = "loop body entries <= len(xs)#0 + len(xs)#0 * len(ys)#1;";
    assert!(handled.starts_with(scanned), "{handled}");
    // A try in a lambda called where it's written, whose handler returns from the lambda.
    let inlined = &cxx["inlined"];
    assert!(inlined.starts_with("loop body entries <= 3;"), "{inlined}");
}
