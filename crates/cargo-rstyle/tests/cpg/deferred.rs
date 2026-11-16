//! Runtime bounds over calls run later than where they're written, removals and chained
//! comparisons: `deferred.go` and `statements.py` in rstyle-core's test data, with the documents
//! rstyle-go and rstyle-py wrote for them.
use super::callables::bounds;

/// A deferred call, or one a goroutine runs, is evaluated where it's written and counts once each
/// time its statement runs. A closure run later reads values that can change first, so it stays
/// unknown even when it only reads them.
#[test]
fn a_deferred_call_counts_once_each_time_its_statement_runs() {
    let go = bounds("deferred-go.cpg.json");
    let each = &go["Each"];
    assert!(
        each.starts_with("loop body entries <= len(xs)#1;"),
        "{each}"
    );
    assert!(
        each.contains("\n(*sync.WaitGroup).Done invocations <= len(xs)#1"),
        "{each}"
    );
    let locked = &go["Locked"];
    assert!(
        locked.contains("\n(*sync.Mutex).Unlock invocations <= 1"),
        "{locked}"
    );
    assert!(
        go["Started"].contains("\nwork invocations <= 1"),
        "{}",
        go["Started"]
    );
    let reported = &go["Reported"];
    assert!(
        reported.starts_with("unknown: A closure that runs later"),
        "{reported}"
    );
}

/// del removes a name, an item or an attribute, and the last two are calls; a chain of
/// comparisons of values other than numbers runs one call per comparison.
#[test]
fn removals_and_chained_comparisons_are_calls_where_they_run_methods() {
    let python = bounds("statements-py.cpg.json");
    for function in ["dropped", "forgotten"] {
        let row = &python[function];
        assert!(
            row.contains("\npy:del invocations <= 1"),
            "{function}: {row}"
        );
    }
    let compared = python["ordered"]
        .matches("\npy:compare-chain invocations <= 1")
        .count();
    assert_eq!(compared, 2, "{}", python["ordered"]);
    let between = &python["between"];
    assert!(between.starts_with("loop body entries <= 0;"), "{between}");
    assert!(!between.contains("py:compare-chain"), "{between}");
    let unpacked = &python["unpacked"];
    assert!(
        unpacked.starts_with("unknown: Removing anything"),
        "{unpacked}"
    );
}
