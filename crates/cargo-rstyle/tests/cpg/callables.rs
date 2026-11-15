//! Runtime bounds over what the Go and C++ frontends write for functions that hand a callable to
//! the standard library, or call one where it's written: `callables.go` and `callables.cpp` in
//! rstyle-core's test data, with the document each frontend wrote for it.
use super::*;
use std::collections::BTreeMap;

/// Each function's bound in a frontend's document, by name: its row, then the lines under it.
fn bounds(fixture: &str) -> BTreeMap<String, String> {
    let testdata = workspace_root().join("crates/rstyle-core/src/cpg/testdata");
    let output = Command::new(cargo_bin("cargo-rstyle"))
        .args(["rstyle", "cpg", "analyze", "--filter", "bounds"])
        .arg(testdata.join(fixture))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();

    let mut rows: BTreeMap<String, String> = BTreeMap::new();
    let mut function = String::new();
    for line in text.lines() {
        let row = line
            .split_once(": ")
            .and_then(|(_, row)| row.split_once(": "));
        match (line.strip_prefix("  "), row) {
            (Some(detail), _) => rows
                .entry(function.clone())
                .or_default()
                .push_str(&format!("\n{detail}")),
            (None, Some((name, bound))) => {
                function = name.to_string();
                rows.insert(name.to_string(), bound.to_string());
            }
            (None, None) => {}
        }
    }
    rows
}

/// A function or member that runs what it's handed at most once is a branch around it in any
/// language whose contracts say so: Go's `sync.Once.Do`, and C++'s `std::call_once` and the
/// members of `std::optional`. A loop in the closure counts once, beside the function's own,
/// and a function it's handed by name is called there.
#[test]
fn a_callable_run_at_most_once_is_a_branch_in_go_and_cxx() {
    let go = bounds("callables-go.cpg.json");
    let scanned = "loop body entries <= len(xs)#0 + len(xs)#0 + len(xs)#0 * len(ys)#1;";
    assert!(go["Once"].starts_with(scanned), "{}", go["Once"]);
    assert!(
        go["Named"].contains("\nclean invocations <= 1"),
        "{}",
        go["Named"]
    );

    let cxx = bounds("callables-cxx.cpg.json");
    let scanned = "loop body entries <= len(xs)#0 + len(xs)#0 * len(ys)#1;";
    assert!(cxx["mapped"].starts_with(scanned), "{}", cxx["mapped"]);
    for name in ["chained", "once"] {
        let bound = &cxx[name];
        assert!(
            bound.starts_with("loop body entries <= len(xs)#0;"),
            "{name}: {bound}"
        );
        assert!(
            bound.contains("\ncount invocations <= len(xs)#0"),
            "{name}: {bound}"
        );
    }
    assert!(
        cxx["named"].contains("\ncount invocations <= 1"),
        "{}",
        cxx["named"]
    );
}

/// What the lowering can't resolve stays as it was. A value only the caller knows is a call of
/// the member, which may run anything, and a closure that writes what it captures leaves the
/// function unknown, as a callback's does.
#[test]
fn a_callable_nothing_resolves_keeps_ordinary_call_lowering() {
    let go = bounds("callables-go.cpg.json");
    assert!(
        go["Value"].contains("\n(*sync.Once).Do invocations <= 1"),
        "{}",
        go["Value"]
    );
    assert!(
        go["Written"].starts_with("unknown: A closure that can write"),
        "{}",
        go["Written"]
    );

    let cxx = bounds("callables-cxx.cpg.json");
    let member = "\nstd::optional::transform invocations <= 1";
    assert!(cxx["value"].contains(member), "{}", cxx["value"]);
    assert!(
        cxx["written"].starts_with("unknown: Lambda capture"),
        "{}",
        cxx["written"]
    );
}

/// A closure called where it's written runs there, once for each time the call is reached.
#[test]
fn a_closure_called_where_it_s_written_is_inlined() {
    let go = bounds("callables-go.cpg.json");
    let cxx = bounds("callables-cxx.cpg.json");
    for bound in [&go["Immediate"], &cxx["immediate"]] {
        assert!(
            bound.starts_with("loop body entries <= len(xs)#0;"),
            "{bound}"
        );
        assert!(
            bound.contains("\ncount invocations <= len(xs)#0"),
            "{bound}"
        );
    }
}
