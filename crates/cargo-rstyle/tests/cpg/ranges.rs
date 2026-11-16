//! Runtime bounds over loops that count their passes from what the document says of their source:
//! `ranges.py` and `ranges.go` in rstyle-core's test data, with the documents rstyle-py and
//! rstyle-go wrote for them.
use super::callables::bounds;

/// A range a library call makes counts from its start up to its stop, and an integer from zero up
/// to it. A signed count bounds only from a start of at least zero, so `range(a, b)` and one that
/// counts down stay opaque.
#[test]
fn a_range_counts_up_to_its_stop() {
    let python = bounds("ranges-py.cpg.json");
    for (function, bound) in [
        ("counted", "max(n, 0);"),
        ("offset", "max(n, 0);"),
        ("stepped", "max(n, 0);"),
        ("nested", "max(n, 0) + max(n, 0) * max(m, 0);"),
        ("pairs", "3;"),
        ("indexed", "max(n, 0);"),
    ] {
        let row = &python[function];
        let expected = format!("loop body entries <= {bound}");
        assert!(row.starts_with(&expected), "{function}: {row}");
    }
    for function in ["between", "backward"] {
        let row = &python[function];
        assert!(
            row.contains("Signed counter may start below zero"),
            "{function}: {row}"
        );
    }
    let go = bounds("ranges-go.cpg.json");
    let counted = &go["Counted"];
    assert!(
        counted.starts_with("loop body entries <= max(n, 0);"),
        "{counted}"
    );
}

/// enumerate, zip and reversed traverse their first argument, and are calls of their own. A call
/// the contracts give no role, or a stop of no known integer type, leaves the loop unknown, never
/// counted by the length of an argument.
#[test]
fn a_traversal_counts_what_it_traverses() {
    let python = bounds("ranges-py.cpg.json");
    for (function, call) in [
        ("numbered", "builtins.enumerate"),
        ("paired", "builtins.zip"),
        ("backwards", "builtins.reversed"),
    ] {
        let row = &python[function];
        assert!(
            row.starts_with("loop body entries <= len(xs)#0;"),
            "{function}: {row}"
        );
        let invoked = format!("\n{call} invocations <= 1");
        assert!(row.contains(&invoked), "{function}: {row}");
    }
    for function in ["ordered", "alone", "untyped"] {
        let row = &python[function];
        assert!(
            row.starts_with("unknown: Only resolved"),
            "{function}: {row}"
        );
    }
}
