//! Runtime bounds over Python comprehensions: `comprehensions.py` in rstyle-core's test data, with
//! the document rstyle-py wrote for it.
use super::callables::bounds;

/// A comprehension runs the loops it desugars to where it's written, then builds its result, an
/// unresolved call since a set's or a dict's keys are hashed.
#[test]
fn a_comprehension_runs_its_loops_where_it_is_written() {
    let python = bounds("comprehensions-py.cpg.json");
    for (function, bound) in [
        ("squares", "len(xs)#0;"),
        ("unique", "len(xs)#0;"),
        ("table", "max(n, 0) + max(n, 0) * max(m, 0);"),
    ] {
        let row = &python[function];
        let expected = format!("loop body entries <= {bound}");
        assert!(row.starts_with(&expected), "{function}: {row}");
        assert!(
            row.contains("\npy:comprehension invocations <= 1"),
            "{function}: {row}"
        );
    }
}

/// A generator expression runs where it's consumed: in place as an argument of the call that
/// consumes it, directly or through a starred argument, and unknown when it's kept for later.
#[test]
fn a_generator_runs_where_the_call_it_is_handed_to_consumes_it() {
    let python = bounds("comprehensions-py.cpg.json");
    for function in ["summed", "spread"] {
        let row = &python[function];
        assert!(
            row.starts_with("loop body entries <= len(xs)#0;"),
            "{function}: {row}"
        );
    }
    let stored = &python["stored"];
    assert!(stored.starts_with("unknown: A lazy value"), "{stored}");
}
