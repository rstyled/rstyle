# Borrowed accelerator views: source-first target

This target is specified against completed Data-v3 3.49 (`7ebf718`) before
implementation. It activates no capability. Preserve the frozen sixty identities
and denominators, historical reports, and previous development selections.

## Original functions and context

Select regex-automata 0.4.18, complete default-feature debug Cargo library under
nightly-2026-09-30 / rustc 5c543b0b8c73c7b72bc8284ced4fb22ead15734d,
aarch64-unknown-linux-gnu. Retain original modules, imports, neighbors, derives
and cfgs. Require pinned x86_64 Linux and aarch64 macOS capture before activation;
cross-target capture is distinct from native runtime execution.

All three selected methods are in `src/dfa/accel.rs`:

| Function | Byte range | SHA-256 |
| --- | --- | --- |
| Accel::len | 18618..18687 | 42d2bcaa66823a8ad482f6d8abd981f0d91cd19b3e4c0e8486de74d7ee3f5f3f |
| Accel::is_empty | 18803..18865 | 78d4124cda0a33ad8ad19db5c2b488457c1bb820179c97266b811a0bcc3d2d42 |
| Accel::needles | 19001..19074 | adc9be3867dec592d18a4ff056241b1a08d57d3629b54482c4dc011525cae7c1 |

The original owner is `#[derive(Clone)] struct Accel { bytes: [u8; ACCEL_CAP] }`,
where the same module declares `const ACCEL_CAP: usize = 8`. It does not implement
Copy. The getters read byte zero, test that length for zero, or borrow
`self.bytes[1..1 + self.len()]`. This is a complete read-only getter chain, not
support for all Accel operations.

## Complete blocker chain and dependencies

The unchanged 3.49 roster excludes all three. The first rejection is the named
array length; is_empty also depends on the unsupported len helper. Needles also
rejects its array-place base because existing subslices require a shared slice.
Source inspection establishes additional restrictions hidden behind those errors:

1. Source/type correspondence currently proves only literal array lengths.
   Resolve the named length in the original field declaration's lexical module,
   with cfg/import/shadowing and exact declaration checks. Match the bounded usize
   literal to the compiler-observed length; do not trust matching names or the
   resolved length alone. Initial support is a single same-module nongeneric
   constant with a literal usize initializer, not arbitrary const evaluation.
2. Existing borrowed-only mixed-content owners permit scalar fields, while
   fixed-array fields require builtin Copy evidence. Extend the borrowed-only
   ownership contract to bounded primitive arrays without granting Copy, owned
   construction, consumption, moves or drop permission. Preserve checks on every
   parameter, result and body value. Original Clone and neighboring methods are
   retained; they supply no Copy or drop proof.
3. A field array place is not a copied array value or an independently allocated
   buffer. Borrowing a subrange must retain the original formal field location,
   including through saved locals and helper results. Existing slice-reference
   forwarding cannot manufacture origins for an array-valued field.
4. Retain exact pinned core range/index authority, builtin array element/type and
   exclusive bounds, source correspondence and panic behavior. Evaluate the base
   place before both endpoints, and each operand once. Do not read or copy every
   array element merely to form the view. The len helper's byte-zero read remains
   a real read even though the enclosing operation creates a shared view.
5. Compose len into is_empty and the needles endpoint; then compose the borrowed
   result into callers, local saved views and scalar reads. Callee read effects,
   saved argument values, returned origin and subsequent bounds checks must all
   survive independent serialized verification and strict bundle import.

Existing checked source census/module environments, field locations, scalar
constant observations and helper-result machinery are the starting dependencies.
Inspect their evidence before adding fields. No new global provenance table,
duplicated source census, generic const evaluator or cache is justified yet.
Unknowns are the exact array Index dispatch/adjustments, which existing field
origin checks can be reused without weakening, and any later helper-summary
restriction after these first blockers are removed.

The retained original HIR now confirms an overloaded `[]` on `[u8; 8]` and
`core::ops::Range<usize>`, yielding `[u8]`. Its callee is the pinned core
`Index::index` trait definition already used by slice views. The base is an
array-valued `expr_field`, not a shared slice. This resolves the dispatch-shape
unknown; it does not establish a location, an implicit adjustment contract or
permission to replace the place with a copied value.

The owned new/from_bytes_unchecked constructors were considered with these
getters. They additionally require non-Copy ownership/drop semantics; changing
their derive to Copy or borrowing away their result would substitute a different
function. They remain negative expectations here. Unit::is_word_byte instead
requires a function callback, nested const-function ownership and loop-written
constant-table proof. Unit::eoi combines formatted assertion and Result/conversion
chains. Those broader chains remain recorded separately rather than being
mistaken for one missing callback or constant row.

## Independent semantic expectations

For every possible byte-zero value, len equals its mathematical unsigned value
and is_empty is true exactly at zero. Do not constrain metadata to 0..=3 merely
because normal construction prefers that range.

For metadata n in 0..=7, needles returns exactly the original object's bytes at
indices 1 through n, with length n and the corresponding address, including the
empty n=0 view. For n in 8..=255 it panics on bounds. No overflow occurs in
`1 + usize::from(n)` on the supported targets. A caller indexing a valid view
returns byte `1 + index` precisely when index < n, and otherwise panics. Saved
views retain their originating object's location; selection between two owners
retains the selected origin even if the input borrows overlap.

The separate fixed corpus is
`eval/coverage-development/borrowed-accel-views.rs`. Before admission evaluation,
the expected in-scope functions are Accel::len, Accel::is_empty, Accel::needles,
getter_len, getter_empty, getter_view, first, saved, selected and slice_window.
The latter is an existing slice-only control. Negative expectations are the
owned constructors, make, owned, mutate, suffix (unregistered RangeFrom),
arithmetic (nonliteral constant expression), and generated Clone. Do not delete
or rewrite these functions to obtain admission. Runtime oracle tests may construct
objects to test getters; that does not admit those constructors to extraction.

Activation requires complete Cargo capture, independent checking, strict import
and matching of the composed positives; adversarial source/binding/bounds/origin
cases; original-source runtime expectations; unchanged natural/frozen selection
deltas; and performance against exact 7ebf718. Preserve scoring and admission
thresholds, distinct unsupported/unknown/inactive/incomplete outcomes, and all
existing assertion/subslice/callback regressions. Run rstyle and patterns with
zero violations/candidates. No abstraction activation, rewrite safety or held-out
detector-quality claim follows from this target.

## Source-first baseline validation

The complete 18-function development Cargo capture passes at 3.49: **one admitted
slice control /seventeen excluded**, complete capture, assessment, enumeration
and matching, with independent verification of the admitted control. The future
contract expects ten admitted /eight excluded; those classifications above were
specified before this baseline was evaluated.

The complete-original-crate oracle passes both tests: every valid metadata value
and every seed byte preserve contents/address, while all 8..=255 metadata values
retain getter behavior and panic on view creation. Run it with:

```sh
python3 eval/coverage-development/tagged-units-oracle.py /path/to/regex-automata-0.4.18 --family borrowed-accel
```

The runner checks all three original source identities, copies the complete
original crate and only appends tests. Runtime construction supplies inputs and
does not certify extraction of non-Copy constructors. No new natural admission
gain, activated capability or frozen replay result is claimed by this audit.

Validation also passes the existing six tagged-unit original-source oracle tests,
CLI integration-test Clippy with warnings denied, formatting, Python compilation
and whitespace checks. Rstyle reports 2827 functions /zero violations; patterns
reports zero candidates. The preceding 884-test workspace run remains the last
full implementation validation; this source-first audit does not replace it.
