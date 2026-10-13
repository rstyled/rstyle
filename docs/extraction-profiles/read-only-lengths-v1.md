# Resolved read-only length operations, version 1

Implementation baseline: `0bb807e`, clean `ccgraph/read-only-api-coverage`.
This is a prospective local `hir-data-v1` supplement, not authority to admit
length calls before its capture, semantic registry and independent serialized
checks are implemented. R0, expanded/helper admission and strict scoring remain
unchanged. The four discovery fixtures are currently excluded; no coverage gain
is claimed from observing their compiler resolution.

| Summary identity | Exact resolved core definition | Instantiated signature |
| --- | --- | --- |
| `rstyle.data.slice.len/1` | `core::slice::<impl [T]>::len` | `(&[T]) -> usize` |
| `rstyle.data.slice.is-empty/1` | `core::slice::<impl [T]>::is_empty` | `(&[T]) -> bool` |
| `rstyle.data.str.len/1` | `core::str::<impl str>::len` | `(&str) -> usize` |
| `rstyle.data.str.is-empty/1` | `core::str::<impl str>::is_empty` | `(&str) -> bool` |

`T` must be an already verified concrete, drop-free, non-reference-bearing data
type. This finite instantiation rule is not general generic/trait dispatch.
Every nominal element still requires its complete independent source/drop proof.
The receiver must already be a supported shared-reference value with verified
formal origins. Source method syntax has no explicit arguments or turbofish.
Reject arrays coerced to slices, owned Vec/String, overloaded dereference, raw or
mutable references, indexing, subslicing, iterators and unregistered methods.
Unsized field receivers needing a new implicit place/autoref interpretation remain
unavailable; supported explicit field borrows and stored shared-reference field
values may compose only under their separately verified existing contracts.

Source spelling is insufficient: capture exact resolved core crate/definition
identity, instantiated signature and compiler/build provenance, plus actual
receiver adjustments. Independently compare these against a versioned tested
semantic registry for the pinned compiler/core build. Only shared builtin receiver
adjustments retaining the same verified shared referent may be registered. Do not
accept a method because its display name is len/is_empty, because compilation
passed, or because the caller supplied a purity flag. Missing required evidence
does not certify a call. Unsupported library/build variants are explicitly
unavailable; corrupt or mismatched captured/registered artifacts are evidence
failures, never complete empty success. No evidence cache is introduced.

Length reads reference metadata, not element/UTF-8 contents. Slice length counts
elements; str length counts UTF-8 bytes, including multibyte text. Empty means
that corresponding length is zero. Registered operations do not mutate, allocate,
panic or diverge on valid supported references. This library summary does not
authorize unsafe operations in selected caller source. It introduces no buffer
read, memory write, bounds branch or fabricated panic/exit edge.

Retain a source-backed CALL operation. Evaluate the receiver exactly once before
the call, retaining its argument snapshot/value edge and scalar result dependence.
Do not inline length/is-empty into arithmetic or normalize the source. Preserve
original invocation and file/function-relative navigation. Record the summary
identity, exact instantiated interface, core provenance, build settings and
receiver/value-origin evidence on paired documents. Existing CFG, divergence and
reaching definitions remain unchanged. Conditional/reference composition requires
the existing complete proof; this supplement does not lift loop-epoch exclusions.

Independent serialized checks must inspect actual CALL identity, callee, operand
position, argument/result types, value edges, effects and exact summary inventory,
then compare with independently verified original source/type/core evidence.
Producer re-emission alone is insufficient. Shared roots conservatively may alias;
length does not assert disjointness or add a generic memory node. Actual receiver
binding/field reads retain their existing reference-count interpretation. CALL
projection, nine features, string comparison, sequential gates, directed WL,
depth, weights, multiplicity and exact scores remain unchanged. Native usize
results require existing known-width build compatibility. Exhaustive pairing and
separate capture/assessment/enumeration/evaluation completeness still apply.

Before admission, real Cargo tests must compose scalar conditions, stored slice/
str fields, lifetime-only receivers and explicit shared outputs with these calls;
verify empty/nonempty slices and multibyte UTF-8 independently; reject shadowed
names, unsupported receivers/calls and tampered signature/core/adjustment/summary
facts; check serialized operands/effects, fresh warm runs, edits, build-setting
changes, target contexts and trace invariance. Re-run historical profiles and
resource/artifact regressions. Existing exact type/source/node budgets apply with
explicit incomplete execution on exhaustion. This is a correctness slice, not a
natural coverage or detector-quality study. Reference-aware helper/method calls,
index/range bounds, checked Option access and cross-unit summaries remain separate
outstanding capabilities.

Discovery evidence: `target/read-length-capture.json`, generated from an isolated
four-function Cargo library with the pinned compiler. Its resolved callee paths
match the table; all four outcomes are excluded. These observations do not
replace the registered semantic/core provenance or independent tests required
above.
