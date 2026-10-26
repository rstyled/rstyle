# Original checked arithmetic error chain

Source-selected before activation: regex-automata0.4.18 `util::wire::{add,mul}`
→ `DeserializeError::arithmetic_overflow`. Original source spans, complete files,
all ten `DeserializeErrorKind` variants, generated PatternID/StateID error macro
and invocations are frozen in `selection.json`. Historical inventories are untouched.

Contract: checked usize addition/multiplication return the exact mathematical
value when representable, otherwise the original `ArithmeticOverflow` variant
retaining the exact static `what` reference. No panic is permitted. The error is
non-Copy; its complete ownership/no-drop proof must cover every variant, including
the generated error members. An unused variant is not permission to omit it.

Initial Data3.118 capture lost the complete type (`Scalar enum requires single
tuple payloads`). The pending observation increment retains all ten variants,
15 fields and both generated error children for all three originals; it grants
no value or cleanup permission. Remaining blockers: precise imports ending at
independently checked generated declarations; recursive whole-error no-drop proof;
static-string/reference component transport; guarded checked-Option payload →
Result construction; constructor/helper summary linking.
Checked_add/mul operations are already registered. Unresolved: which existing
aggregate-origin and guarded payload machinery can be reused without new transport.
`shl` additionally needs conversion, captured map_err and Result `?`; it remains
outside this first selection. Formatting, generic enum support and cleanup are
separate work.

Independent oracle uses u128 mathematical sum/product, all256² small pairs,
15² boundary pairs and8,192 deterministic full-width pairs, each against three
static messages and both operations; plus three direct helper checks:
**443,721 checks/context**. Errors are matched against the private original variant
and string pointer/length. Any original panic fails the run. No production result
or capture output supplies expected values.

Required adversaries: wrong/missing error variant; dropped variant inventory;
changed generated child/declaration; custom Drop; altered lifetime/static source;
wrong saved operands or arithmetic kind; missing payload guard; changed helper or
build identity. These are future semantic-verifier requirements, not tests already
executed by the runtime oracle.

```sh
python3 eval/coverage-development/wire-arithmetic-errors/oracle.py
python3 eval/coverage-development/wire-arithmetic-errors/oracle.py --run-native --output /tmp/wire-arithmetic-oracle.json
```

Source checks and all four native contexts pass443,721 checks each, zero
mismatches. Native no-default extraction retains131/590 admissions unchanged;
all three targets remain excluded. This is a prerequisite, not connected support.
Native runner copies the entire original package, verifies every original byte,
appends a private-module oracle and root bridge, and executes a separate runner.
Original functions, manifests and module paths remain byte-identical. Default and
no-default × debug assertions on/off use pinned compiler/64-bit hosts; opt-level0,
overflow checks and unwind remain explicit. This establishes original runtime
expectations, not admission, strict verification, matching or detector quality.

Next proof reuses existing expansion/declaration and generated-error cleanup
checks, detached from the current map wrapper's callable receipt. Imported names
must terminate at a checked emitted declaration; captured names alone are not
source authority. The full ten-variant cleanup closure precedes activation.
Evidence: `/tmp/rstyle-445-{natural-shapes.json,comparison.json,oracle.json}`.
