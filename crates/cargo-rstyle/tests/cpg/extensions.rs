//! Runtime bounds over nodes a frontend registers as its own: `extensions.py` and `extensions.go`
//! in rstyle-core's test data, with the documents rstyle-py and rstyle-go wrote for them.
use super::callables::bounds;

/// A Python extension is lowered by its role: it evaluates its children, and one that runs an
/// operation the document names no callee for is a call at the node, so the scan around it keeps
/// its bound and counts that call once per item.
#[test]
fn a_python_extension_keeps_the_bound_of_the_scan_around_it() {
    let python = bounds("extensions-py.cpg.json");
    let scanned = "loop body entries <= len(xs)#0;";
    for (function, call) in [
        ("labels", Some("py:format")),
        ("rows", Some("py:dict")),
        ("spread", Some("py:star")),
        ("constant", None),
        ("tails", None),
    ] {
        let row = &python[function];
        assert!(row.starts_with(scanned), "{function}: {row}");
        let counted = call.map(|call| format!("\n{call} invocations <= len(xs)#0"));
        match counted {
            Some(counted) => assert!(row.contains(&counted), "{function}: {row}"),
            // Joining a string and building a slice call nothing.
            None => assert!(!row.contains("\npy:"), "{function}: {row}"),
        }
    }
}

/// What the document names no callee for stays opaque in the composed result, and a keyword
/// argument with a name is only what it passes.
#[test]
fn an_extension_operation_is_an_unresolved_call() {
    let python = bounds("extensions-py.cpg.json");
    for (function, call) in [
        ("waits", "py:await"),
        ("imports", "py:import"),
        ("named", "py:keyword"),
    ] {
        let row = &python[function];
        assert!(
            row.starts_with("loop body entries <= 0;"),
            "{function}: {row}"
        );
        let opaque = format!("opaque {call}: Unresolved callee {call}");
        assert!(row.contains(&opaque), "{function}: {row}");
        assert!(
            row.contains(&format!("\n{call} invocations <= 1")),
            "{function}: {row}"
        );
    }
    // `strict=True` passes a value; `**options` unpacks a mapping, and is the one call.
    assert_eq!(python["named"].matches("py:keyword invocations").count(), 1);
}

/// Go's slice expression reads its operand and bounds and calls nothing.
#[test]
fn a_go_slice_expression_keeps_the_bound_of_the_scan_around_it() {
    let go = bounds("extensions-go.cpg.json");
    for function in ["Tails", "Heads"] {
        let row = &go[function];
        assert!(
            row.starts_with("loop body entries <= len(xs)#0;"),
            "{function}: {row}"
        );
        assert!(!row.contains("go:slice"), "{function}: {row}");
    }
}
