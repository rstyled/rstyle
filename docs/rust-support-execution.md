# Rust support execution frontier

Authority: [delivery plan](rust-support-parallel-delivery.md) and repository-root
[roadmap](../rust-feature-coverage.md). Updated 2026-10-22. All P01–P22 routes
remain active in the plan; this record contains current decisions and evidence.

## Current boundary

Branch `feature/rust-feature-coverage`; current implementation is Data3.74:
the original base64ct length/getter family now completes default and std contexts,
following the Transition predicate, checked-length and stored-slice chains.
The root roadmap records validated checkpoints;
broader coverage remains active and historical evidence stays unchanged.

Compiler nightly-2026-09-30, commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`; native aarch64 Linux,
installed x86 Linux and ARM macOS targets. Original captures use debug/default
features, lib/non-test, edition 2021 unless their family contract states otherwise.
GitHub check-runs for flags delivery `98e084c` pass Linux x86, Linux ARM and macOS;
character leaf `f61065d` and escape implementation `9967f3d` also pass all three
native CI jobs.

Preserve unrelated `crates/rstyle-pdg/src/slice_access/`, `docs/pdg-and-clones.md`
and `ferrante_toplas_87.pdf`. Lead owns shared seams, registration, staging and
commits. No new branches/worktrees. Serialize large captures, owning-unit strict
replays, performance and integrated gates; never overwrite active binaries.

## Flags: connected support and measured evidence

[Contract](extraction-profiles/look-flags-v1.md): complete checked-Copy fieldless
enums, source/discriminant proof, variants/casts, shared Copy receiver loads and
actual helper composition. Commits `4fcc007`, `0406406`, `49c7ec7`.

| Unchanged selection | Baseline → current admissions | Losses |
| --- | --- | --- |
| regex-automata 0.4.18 | 98 → 114 /2582 | 0 |
| regex-syntax 0.8.11 | 23 → 36 /956 | 0 |
| lz4_flex 0.14.0 | 8 → 8 /150 | 0 |
| time-core | 11 → 11 /226 | 0 |
| Frozen sixty | 23 admitted /30 excluded /7 inactive | 0 |

All source/build identities and denominators match. Primary 13 targets plus empty
admit; six declared negative controls remain excluded. Thirteen syntax gains are
byte-identical mirrored functions, observed incidental gains, not held-out targets.
Three extra automata gains were source-audited: StartKind's size query and Start's
explicit-load/cast/widening helpers. The optimized verifier strictly checks **169 original plus 24 development
admissions**, with **182 original and 276 development comparisons** complete. Full-library matching
remains explicitly incomplete under max-pairs zero.

Retained evidence:

- Original report `/tmp/rstyle-look-flags-natural-target/rstyle/clones/run-aOQHu7/report.json`
  (stream only); compact audit `/tmp/rstyle-look-flags-natural-roster.json`.
- Frozen run `target/hir-coverage/run-bo_05z1o`; delta
  `/tmp/rstyle-look-flags-full-frozen-delta.json`.
- Strict/matching `/tmp/rstyle-look-flags-fastpath-{strict,frozen-strict,control-strict}.log` and
  corresponding matching JSON. Original replay 121.49s /5916484 KiB; other
  complete packages 53.72s /756960 KiB.
- Independent original-library oracles pass all 262144 flag sets in both libraries:
  `/tmp/rstyle-look-flags-original-oracles-final.log`.
- Final integrated gates: 936 tests pass /5 existing skips (326.693s); doctests,
  fmt, Clippy pass; rstyle 0 /3042 functions, patterns 0 unignored. Includes
  historical-policy and receiver-signature tampering assertions. Logs
  `/tmp/rstyle-look-flags-final-{tests.log,style.json,patterns.log,clippy.log}`.

Warmup + seven alternating samples: copy median 0.62→0.62s, cyclic 0.62→0.61s,
ASCII 0.83→0.92s (unchanged admission/matching). Expanded flags 0.72→7.95s,
1→14 admissions; this is different supported work. Results retained in
`/tmp/rstyle-look-flags-performance-results.json`. A rejection-only receiver
signature fast path passes focused capture and forged-signature strict-import
tests. Review confirms unconditional independent receiver/callee verification.
Follow-up controls finished: copy/cyclic 0.62→0.62s; ASCII 0.84→0.88s; flags
0.72→7.99s. Residual ASCII slowdown remains reported. Results:
`/tmp/rstyle-look-flags-fastpath-performance-results.json`.

## Dependency frontier

| Package / owner | State and next concrete step |
| --- | --- |
| P01 character / A + lead integration | Shared integration validated: six development functions admit, seven neighboring cases exclude; literal/OR/dispatch tampering and unused-char 3.60 policy downgrade reject. 939 tests pass (five existing skips); fmt/Clippy/doctests pass; rstyle 0/3051, patterns 0. Original syntax36→38/956, no losses;57 original admissions strict-verify, three comparisons complete. Frozen60 unchanged. Original `regex-syntax::is_meta_character`; dedicated char domain and historical gating, no global widening of `scalar()`. Original numeric-set oracle passes all 1112064 valid Unicode scalars. |
| P01 escape caller / A+C+lead | Range and exact ASCII receiver leaves integrated. Complete original-copy fixture admits five/rejects five across three targets × std/no_std, strict tampering included; independent original oracle1112064 scalars/64 positives. Final focused tests pass; rstyle0/3070. Full gates pass:945 tests/5 skips, fmt/Clippy/doctests, rstyle0/3070, patterns0. Original syntax38→39/956, no losses;58 admissions strict-verify, six comparisons complete. Frozen60 unchanged. Native CI passes all three platforms. Equal-work3.60→3.62 medians: Copy0.65→0.64s, cyclic0.63→0.64s, ASCII0.87→0.87s, flags7.96→8.01s; admissions/matching unchanged. |
| P08 checked values / B + lead integration | Data3.70 connects original base64 encoded_len through checked usize add/mul, immutable Some if-let, early None return and nested result flow. Default7→8/167, no losses; six target/feature contexts strict-verify. The separate zerovec chain still needs generics/let-else/Result/?/closures/borrowed aggregates. |
| P06 borrowed result / C | Data3.69 completes original haystack → is_char_boundary → utf8::is_boundary with distinct slot reads/referent origins, saved receiver substitution and conditional checked reads. Two original gains, zero losses; all134 strict. Broader aggregate/nested-reference results remain closed; next family needs its own original source audit. |
| P05 ownership / D | Data3.68 proves selected passive Copy dependencies beside original PatternID/SmallIndex index_type_impls! invocations. All fields/source/Copy and owning-scope checks remain; direct dependency operations gain no permission. Complete automata120→132/2582, zero losses; all132 strict-verify. No census redesign justified. |
| P02 floats / lead | Data3.66 completes original k_sin with exact immutable i32 equality and complete float alternatives; libm6→7/1131, zero losses, all7 strict-verify/six selected comparisons complete. Independent original oracle5384 inputs; three-target × std/no_std composition and resealed guard/merge/source attacks pass. |

Contracts: [character](extraction-profiles/character-leaf-v1.md),
[checked consumption](extraction-profiles/saved-checked-consumption-v1.md),
[Result audit](extraction-profiles/scalar-result-frontier-audit-v1.md),
[borrowed results](extraction-profiles/borrowed-result-chain-v1.md),
[source ownership](extraction-profiles/source-ownership-frontier-v1.md),
[floats](extraction-profiles/float-kernels-v1.md).

## Resume order

Data3.74 completes the same original base64ct family under std by proving the
exact pinned alloc macro import namespace. Three targets: std **3→6/150**,
default **6/141 unchanged**, zero losses. All36 base64ct and12 contextual peer
admissions strictly replay;30 predeclared comparisons complete. Native original
length oracles pass131074 cases per feature context; private-source witnesses
are labeled separately. Frozen sixty and complete package rosters are unchanged;
all58 admissions strictly replay. Gates:1067 tests/5 skips, doctests/fmt/Clippy,
rstyle0/3398, patterns0 unignored; existing15 ignores unchanged.
Evidence: `/tmp/rstyle-alloc-*`; frozen `run-pk54tszf` versus `run-gn15_b0l`.
See the [source selection](../eval/coverage-development/base64ct-std-import-selection.json).
Full-library matching remains incomplete; broader P01–P22 work remains active.

Next bounded assignments (original first-wave workers are already integrated):

| Frontier | Readiness gate and exclusive leaf work |
| --- | --- |
| P05 external macro imports | Delivered3.74: pinned complete alloc import inventory unlocks the three original std length/getter functions. Macro bodies, allocation/drop and other import grammars remain independently closed. Lead owns source-boundary/shared policy edits. |
| P07 range/view composition | [Frozen selection](../eval/coverage-development/range-getters-selection.json): `Input::get_range` and `Match::range → Span::range → local From<Span>`. Original-library endpoint oracle passes4704 cases in each native default/no-default context. Observation now binds the selected local From path/hash to its captured definition; unchanged native no-default roster85/590, zero gains/losses. Required next: independent trait-owner proof, exact pinned non-Copy Range fields/drop-free concrete instance, fresh result forwarding and helper composition. No trait or Range admission added. |
| P08 concrete Result frontier | [Frozen next selection](../eval/coverage-development/scalar-result-next-selection.json): UnicodeWordBoundaryError::check and cfg-selected helper/new; default/no-default/word-boundary contexts have independent Ok/Err expectations. Needs exact non-Copy nominal-error owner/drop proof plus Result construction/tag/forwarding. Cargo evidence remains pending; zerovec's broader blockers remain separate. |
| P09 loop/callee frontier | [Frozen next selection](../eval/coverage-development/scalar-loop-callee-selection.json): ryu::pow5factor_32 → multiple_of_power_of_5_32. Positive values permit at most13 backedges; selected debug assertions make zero panic before division. Needs complete loop/value exits, checked increment, helper/source proof and separate divergence evidence; no acyclic-gate relaxation. Cargo evidence remains pending. |

PatternSet separately requires complete opaque Box-field/drop proof. Fold brief
evidence into semantic commits; the full P01–P22 routing remains active.
