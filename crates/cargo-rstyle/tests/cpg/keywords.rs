//! Runtime bounds over what the Python frontend writes for calls that pass an argument by
//! keyword: `keywords.py` in rstyle-core's test data, with the document rstyle-py wrote for it.
use super::callables::bounds;

/// A keyword argument is an argument: the call evaluates what it passes. A call that takes one
/// is a call, and leaves the loops around it their bound.
#[test]
fn a_keyword_argument_is_lowered_as_what_it_passes() {
    let python = bounds("keywords-py.cpg.json");
    let scanned = "loop body entries <= len(xs)#0 + len(xs)#0 * len(ys)#1;";
    assert!(
        python["padded"].starts_with(scanned),
        "{}",
        python["padded"]
    );
    let printed = "\nbuiltins.print invocations <= 1";
    assert!(python["padded"].contains(printed), "{}", python["padded"]);

    let sorted = "\nbuiltins.sorted invocations <= 1";
    assert!(python["ordered"].contains(sorted), "{}", python["ordered"]);
    // A method takes one as a function does.
    let encoded = "\nbuiltins.str.encode invocations <= 1";
    assert!(python["joined"].contains(encoded), "{}", python["joined"]);
}

/// What a keyword passes is lowered as it would be anywhere: a lambda nothing runs here leaves
/// the function unknown, and a mapping unpacked into keywords isn't one argument.
#[test]
fn what_a_keyword_passes_is_read_as_it_is_elsewhere() {
    let python = bounds("keywords-py.cpg.json");
    assert!(
        python["keyed"].starts_with("unknown: Closure"),
        "{}",
        python["keyed"]
    );
    assert!(
        python["spread"].starts_with("unknown: Expression"),
        "{}",
        python["spread"]
    );
}
