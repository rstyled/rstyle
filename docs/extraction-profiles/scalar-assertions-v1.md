# Standard scalar assertion macros

Data-v3 extraction 3.6.0 adds `rust.scalar-assertions`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Unformatted `assert!(condition)` and
`debug_assert!(condition)` invocations, including `core::`/`std::` qualification
and a trailing comma, compose with admitted bool expressions and unit results.
Existing scalar local writes and read-only data/helper operations in conditions
retain their ordinary admission boundaries. Formatting/comparison macros, renamed
macro imports and arbitrary user macros remain outside this F14 slice.

The driver recognizes compiler-resolved nonlocal `core` assertion definitions and
their standard HIR expansion shape. It captures one `expr_assert` containing the
original condition, existing expansion callsite, canonical macro resolution and
active setting. Debug expansion's cfg literal must agree with the captured
debug-assertions setting. A custom macro merely named `assert` is not recognized.
No separate source snapshot, expansion provenance table, executable fingerprint or
cache is added; rustc owns standard macro expansion.

Independent admission parses the original invocation, checks macro resolution,
unit/bool types, original condition byte span, callsite/source correspondence and
active setting. Original-source inventory visits the condition as well as the
invocation. It retains disabled debug conditions and does not infer that arbitrary
cfg-removed operations are inactive. Nested unsupported macros/source constructs
still fail their existing checks. Earlier Cargo selections keep excluding assertions.

An enabled assertion evaluates its condition once, in normal source order, and
emits one unit computation with operator `assert` and the resolved macro construct.
Successful normal continuation is modeled; exceptional transfer, panic payload,
unwinding and cleanup are omitted. The existing exceptional-flow assessment marks
evidence Partial unless the operand is a directly known true bool literal or
resolved constant. An unproved or false condition retains an explicit limitation;
this is not a panic-free or rewrite-safety claim. The current helper boundary
continues to require Complete nonpanicking callee evidence.

A disabled debug assertion emits neither the assertion nor condition operations;
its original resolved source remains in frontend evidence. Existing matching build
compatibility includes the debug-assertions setting and panic strategy. Disabling
debug assertions therefore changes actual execution without silently sharing
incompatible build evidence.

Validation covers debug/release builds, condition mutation evaluated once,
read-only method receivers, original parentheses/Unicode positions, known-true
and false assertions, helper admission, custom/formatted/comparison exclusion,
earlier-profile exclusion and modified build/resolution/source evidence rejection.
Natural coverage is measured separately with the unchanged frozen inventory.
