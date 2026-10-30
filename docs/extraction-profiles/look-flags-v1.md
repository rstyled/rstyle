# Fieldless flags through verified helpers

Source-selected target: regex-automata 0.4.18 `util::look`, default features,
lib/non-test, edition 2021, pinned nightly; std/no_std and the existing three
64-bit target contexts are independent validation contexts. The thirteen target
functions are `Look::as_repr` and `LookSet::{contains, contains_anchor,
contains_anchor_haystack, contains_anchor_line, contains_anchor_lf,
contains_anchor_crlf, contains_word, contains_word_unicode, contains_word_ascii,
singleton, insert, remove}`. `LookSet::empty` is a prerequisite.

Before admission evaluation, source specifies all thirteen as in scope: each
variant represents its explicit single bit; membership tests that bit, grouped
queries test the documented masks, and insertion/removal return changed values
without modifying the input. A separate fixed development copy is
`tests/fixtures/function-families/look-flags.rs`; the complete original crate and frozen
sixty retain their identities and denominators. Exact original source/build
identities and six negative controls are frozen in
[`look-flags-selection.json`](../../tests/fixtures/function-families/look-flags-selection.json)
before fresh admission measurement.

Known blocker chain: complete fieldless nominal observation → independent
source/discriminant and builtin Copy proof in the existing mixed unit → original
variant paths → enum-to-integer casts → checked helper transport → grouped
queries and returned value constructors. The existing scalar-payload enum model
cannot stand in for a fieldless enum; no fake payload, integer layout assumption,
new census or purity assertion is justified. The complete selected helper chain
is now exercised; neighboring enum/ownership families retain separate contracts.

Bounded contract: nonempty local nongeneric fieldless enums, builtin Copy proof,
default Rust representation, complete literal/shift discriminants with checked
arithmetic and implicit successors. No custom repr, aliases to unverified types,
external variants, cfg-erased variant subsets, mutable effects, payload binding,
custom derives, destructor behavior or general trait dispatch is added. Existing
scalar-payload enums retain their separate contract.

Independent expectations: all bit positions, all 18-bit sets and grouped masks;
zero/full/high unrelated bits; idempotent insertion/removal and original value
preservation; source/type/tag/discriminant disagreement must reject. Negative
cases include custom repr, generic/mixed/payload declarations, unresolved named
constant discriminants, overflow and incomplete inventories. `Look::reversed`,
`from_repr`, `as_char`, mutable setters and iterator construction remain outside
this declared family and must not gain support from a prefix of their bodies.


## 3.60 validation

| Unchanged complete selection | Admissions before → after | Losses |
| --- | --- | --- |
| regex-automata 0.4.18 | 98 → 114 /2582 | 0 |
| regex-syntax 0.8.11 | 23 → 36 /956 | 0 |
| lz4_flex / time-core | 8/150 and 11/226 unchanged | 0 |
| Frozen sixty | 23 admitted /30 excluded /7 inactive | 0 |

All source/build identities match. Thirteen targets plus `empty` admit; six
negative controls remain excluded. Syntax gains mirror the same thirteen
byte-identical functions; these are observed incidental gains. Three additional
automata gains were source-audited: `StartKind::write_to_len` (constant size,
unused shared receiver) and `Start::{as_u8, as_usize}` (explicit shared load,
cast and existing widening conversion).

The optimized verifier independently strict-verifies **169 original plus 24
development admissions**. All **182 selected original and 276 development
comparisons** complete, including incompatible interfaces. Full-library matching
is explicitly incomplete under max-pairs zero. Independent numeric-mask oracles
pass all **262144 bitsets** against both original libraries. Six target/std/no_std
fixture contexts pass. Historical 3.59 policies reject both fieldless operations
and shared Copy receiver loads. Forged receiver signatures/adjustments and removed
implicit reads reject under strict import.

Warmup plus seven alternating samples (seconds, median [range]):

| Workload | Baseline 3.59 | Current fast path | Admissions |
| --- | --- | --- | --- |
| Copy | 0.62 [0.60–0.64] | 0.62 [0.61–0.64] | 12 unchanged |
| Cyclic | 0.62 [0.60–0.64] | 0.62 [0.60–0.63] | 4 unchanged |
| ASCII | 0.84 [0.82–0.86] | 0.88 [0.84–0.89] | 10 unchanged |
| Flags | 0.72 [0.71–0.74] | 7.99 [7.93–8.01] | 1 → 14 |

Matching is identical for unchanged workloads. The rejection-only signature fast
path reduced an initial ASCII median of 0.92s; a remaining 0.04s difference is
reported, not declared absent. Flags performs more supported work and all 91
selected pairs. Median peak RSS: ASCII 126100→126200 KiB; flags
120232→148140 KiB. One full-original run before the fast path took
1079.51s /7037116 KiB versus baseline 881.67s /6393560 KiB with sixteen additional
admissions; these single samples are not an equal-work performance comparison.

Evidence: `/tmp/rstyle-look-flags-natural-roster.json`,
`/tmp/rstyle-look-flags-full-frozen-delta.json`, strict/matching logs
`/tmp/rstyle-look-flags-fastpath-{strict,frozen-strict,control-strict}.log`, original runtime oracles
`/tmp/rstyle-look-flags-original-oracles-final.log`, alternating controls
`/tmp/rstyle-look-flags-fastpath-performance-results.json`.
The ~9GB original report is retained at
`/tmp/rstyle-look-flags-natural-target/rstyle/clones/run-aOQHu7/report.json`
(stream only). Final gates pass: **936 tests /5 existing skips**, doctests, formatting and
Clippy; **rstyle 0 /3042 functions, patterns 0 unignored**, existing ignores
unchanged. Native Linux x86/ARM and macOS CI passed at `49c7ec7`; the subsequent
receiver optimization awaits its own CI. This bounded 3.60 family is delivered;
char, mixed/payload enums, general ownership and rewrite safety remain separate.
