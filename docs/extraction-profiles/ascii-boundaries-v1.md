# ASCII and line boundaries: nested Copy child proof

Selection: `hir-data-v3`, `rstyle.rust.hir-data/3.58.0`.
Matching/scoring are unchanged. This increment supports borrowed nested-field
getters and their callers; the complete ASCII predicate family remains open.

## Fixed target and independent expectations

Original crate: regex-automata 0.4.18, default-feature debug library, edition 2021,
pinned nightly 2026-09-30. Select these unchanged `util::look::LookMatcher`
methods in `src/util/look.rs`: `get_line_terminator`, `is_word_ascii`,
`is_word_ascii_negate`, `is_word_start_ascii`, `is_word_end_ascii`,
`is_word_start_half_ascii`, `is_word_end_half_ascii`. The checked byte spans and
SHA256 identities live in the `ascii-boundaries` selection of
[`tagged-units-oracle.py`](../../eval/coverage-development/tagged-units-oracle.py).
Before evaluating the fresh natural capture, the source audit also selects
`is_start`, `is_end`, `is_start_lf`, `is_end_lf`, `is_start_crlf`, and `is_end_crlf`
as a connected line-boundary family (six intended positives, previously all
excluded). Their full chain is nested-owner proof → existing scalar/length
predicates → guarded direct scalar reads and exact memory-free helper calls.
They require no dynamic-read helper transport. Their identities are checked by
the same runner. An independent oracle tokenizes CRLF as one line-ending token,
and lone CR/LF as separate tokens, specifying boundaries before/after complete
tokens across all byte pairs and five configurable LF-mode terminators.
Capture always uses the complete original crate, including `DebugByte`, its
custom Debug impl, the owner's Clone/Debug derives, imports and neighboring code.
No original source or frozen inventory is replaced by the development source.

Before implementation, the separate
[`ascii-boundaries.rs`](../../eval/coverage-development/ascii-boundaries.rs)
selection contains 19 captured functions (source SHA256
`08c2e2895984e824e0cb713f66541b55b98ae028aab036bec7e3f8e0fee5aecc`).
Its ten intended positives are seven Matcher methods, `utf8::is_word_byte`,
`getter`, and `repeated`. Nine negatives are owned construction/consumption,
mutation, nested non-Copy fields, Drop method/getter, effectful helper/caller,
and generated Clone. Baseline 3.57 admits only the classifier leaf: 1/19.
Classifications come from source semantics, independently of admission results.

The runtime oracle appends tests to a disposable complete original crate after
checking all thirteen identities. A fixed independent ASCII membership bitset
specifies empty, all 256 singleton and all 65,536 two-byte inputs at every valid
position. It tests four line terminators, plus invalid positions `len + 1` and
`usize::MAX`: five predecessor-reading predicates panic; half-end returns true.
No valid-position precondition is added to extraction or helper summaries.

## Complete known blocker chain

| Dependency | Evidence and remaining boundary |
| --- | --- |
| Nested borrowed owner | Non-Copy LookMatcher contains checked-Copy DebugByte. Former scalar/array-only restriction rejects all seven methods. This increment resolves that restriction. |
| Slice metadata | Exact receiver/type/core provenance is already captured. The `.len()` rejection depends on the owner failure; it requires no invented metadata summary. |
| Scalar/index/order | Direct predicate source uses short-circuit guards, saved operands, shared scalar indexing, subtraction and the already supported original const-table classifier. Existing bounds/partial-flow evidence applies. |
| Indexed-read helpers | `helper_calls::check_boundary` excludes `data.index`; helper read locations lack access-relative dynamic index substitution. Negation and repeated callers remain excluded. Retain exact per-read selectors, guards, argument snapshots, aliases and panic propagation before admitting them. |
| Unknown broader callers | Not audited beyond this selection. Loop merges, mutable ownership, Unicode, generic dispatch and iterator families have no new support claim. |

## Bounded support contract

`rust.borrowed-copy-child-fields` permits struct fields of a borrowed-only,
nongeneric local owner when each child has independently checked builtin Copy
trait/derive authority and an exact active original declaration. A first pass
checks every selected declaration, attribute, namespace, field type and Copy
origin; a second pass applies borrowed-only restrictions using the resulting
checked Copy set. Declaration order cannot grant permission. Existing closed
source/cfg/import/derive checks remain mandatory. No new registry/cache/census.
Existing field-path read occurrences and shared-root alias uncertainty remain
unchanged.

The outer owner gains no owned Copy permission. Every interface/body type is
still checked for owned occurrences: construction, consumption, value-self and
implicit drop remain excluded. Shared getters do not execute neighboring Debug
or Clone implementations; those implementations are not certified. Mutable,
reference-bearing, generic, missing-observation and destructor cases retain
their existing exclusions. Nested non-Copy children remain outside this bound.
Historical selections and document policies do not gain this capability.

## Measured increment

Development: **1 → 8 /19**, zero losses. Getter → caller completes; five direct
predicates admit with the unchanged classifier leaf. Negation and repeated-call
composition remain excluded, so the ten-positive family is **not complete**.
Natural: **85 → 97 /2582**, exactly the twelve declared positive methods, zero
losses. The six line-boundary methods complete their existing helper chain;
ASCII negation remains open. All source/build identities agree with 3.57.
Earlier word/byte selections remain **13/20 and 9/11**. Full lz4_flex,
regex-syntax and time-core selections remain **8/150, 23/956 and 11/226**.
Frozen run `run-hotp6ilg`: **23 admitted /30 excluded /7 inactive /60**;
all identities, denominators and historical results remain unchanged.

All **169 admissions** independently strict-verify from their owning units:
97 natural, 30 development and 42 in the three other complete package selections.
All **210 comparisons among 21 selected keys** complete without diagnostics.
Full natural matching remains explicitly limited by max-pairs zero.
**928 workspace tests pass /5 skipped**; six installed target/std contexts,
Copy-authority/derive/ownership forgeries, declaration-order independence,
historical exclusions and the reported assertion/subslice regressions pass.
Four original runtime oracle tests pass. macOS coverage is cross-target capture;
native macOS execution has not been performed. All-target Clippy, doctests,
formatting and diff checks pass; **rstyle 0 /3008 functions; patterns 0**.

Extraction coverage makes no abstraction activation, rewrite-safety or held-out
detector-quality claim.

Performance: clean e56baf6 baseline, one warm-up followed by seven alternating
samples per version; identical control identities/outcomes/matching results.

| Workload | Median seconds, baseline → current | Median peak KiB, baseline → current |
| --- | --- | --- |
| Existing Copy control, 12 admissions | 0.62 → 0.64 | 118704 → 118540 |
| Existing cyclic control, 4 admissions | 0.64 → 0.61 | 108512 → 108672 |
| New family, 1 → 8 admissions /0 → 28 comparisons | 0.55 → 1.07 | 116592 → 121032 |

Control ranges overlap. The new family performs seven additional admissions and
28 comparisons. Full natural capture is a single-run diagnostic: 881.16 → 896.65 s,
6375928 → 6249648 peak KiB. These bounded observations make no general performance
guarantee. Performance runs, large captures and owning-unit verification were
serialized; binaries were kept stable during captures/tests.
