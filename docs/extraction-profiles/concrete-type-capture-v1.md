# Concrete type capture prerequisite, version 1

Observation identity: `rstyle.rust.concrete-type-capture/1`. This is not an
extraction or matching admission contract. Under `hir-helpers-v1`, the frontend
may attach `concrete_type_observations` to rejected non-scalar interfaces. R0 and
expanded capture remain unchanged. No normalized scalar token is replaced, and
this evidence cannot make a function eligible.

The finite observation grammar is scalar, tuple, fixed array, immutable shared
reference, shared slice, str, and local concrete struct. Primitive leaves preserve
width and sign. Shared reference trees omit lifetime names, retaining immutable
mutability and exact referent type. Struct tokens bind Cargo package name/version,
manifest bytes digest, resolved definition path and exact declaration digest.
Fields retain declaration order, resolved type and original name. Display names
and session-local compiler IDs are not nominal identity. Declaration source bytes
are retained for independent parsing and digest validation.

The compiler rejects destructor-bearing structs, mutable references, UnsafeCell,
nonlocal declarations without captured source, unresolved generic instances,
nonstruct ADTs, raw pointers and other types. Shared referents are checked
recursively; shared syntax never proves an immutable referent. This structural
capture boundary has no interior mutable leaf. It does not prove effects of a
function or callees. Recursive type expansion is unsupported. Limits are depth
16, 4096 visited type entries, 128 fields per struct, array length 4096 and 64 KiB
per declaration. Exceeding limits fails capture explicitly; unsupported types
retain a reason. Neither case produces a truncated type certificate.

Independent validation checks the closed tree grammar, finite budgets, nominal
source digests, declaration kind/name, field count/order/names and declaration
spelling. Package-context binding, resolved alias/type correspondence, complete
definition/build correspondence, field operations, shared origins and possible
aliases still need their own verification. Object layouts must be captured if a
future operation depends on them; this observation makes no layout claim.

Admission is explicitly unavailable until those prerequisites, source-backed
lowering, location/reference-value verification and the separately registered
data matching extension cross the actual serialized source-to-pair handoff. This
module is retained implementation progress and must not be reported as read-only
API coverage or as completion of the broader PR.

## Validation of this prerequisite

Four independent descriptor/source tests check nominal and field tampering,
direct primitive-width contradictions, shared referent serialization, Unicode/CRLF
offsets and finite budgets. Two real Cargo tests capture nested concrete structs,
tuples, arrays, shared receivers, shared slices and str from BOM/CRLF source;
destructor-bearing and interior-mutable inputs remain unsupported. Every data
function remains excluded. An oversized array makes capture, assessment and pair
evaluation incomplete with the explicit budget diagnostic preserved.

`cargo test --workspace --locked` passed 511 tests in 25 suites, none ignored.
`cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` passed. Inventory
tests passed 11 tests under the documented pinned Python environment; scripting
and study-harness discovery passed 43 tests. Repository-built style/patterns
commands use the PATH setup in [source-to-pair](../source-to-pair.md).
`cargo rstyle --format json` reported zero violations;
`cargo rstyle patterns --format json` reported zero candidates. No thresholds,
suppressions or ignore entries changed.

Remaining obligations include binding nominal packages/declarations to the exact
selected build, complete resolved field/value/source correspondence, location
paths and possible aliases, reference origins and returned borrows, effect proofs,
data lowering, independent serialized verification and a separate matching
extension. Cross-package calls, assertions, broader patterns/constants, slice
operations, loop/value composition and the preregistered broader applicability
inventory also remain unfinished. These are implementation prerequisites, not a
claim that external authority prevents their implementation.
