# Rust support execution frontier

Authority: [delivery plan](rust-support-parallel-delivery.md) and repository-root
[roadmap](../rust-feature-coverage.md). Updated 2026-10-21. All P01–P22 routes
remain active in the plan; this record contains current decisions and evidence.

## Current boundary

Branch `feature/rust-feature-coverage`; baseline `f71eff3` (Data-v3 3.59), implementation checkpoint
`e12652d` follows `49c7ec7`. Data-v3 3.60 flags are delivered. Character implementation `f61065d` passes full gates and original strict replay;
performance controls pass; Data-v3 3.61 delivered. No broad coverage completion claim.

Compiler nightly-2026-09-30, commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`; native aarch64 Linux,
installed x86 Linux and ARM macOS targets. Original captures use debug/default
features, lib/non-test, edition 2021 unless their family contract states otherwise.
GitHub check-runs for flags delivery `98e084c` pass Linux x86, Linux ARM and macOS;
character implementation CI is tracked separately.

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
| P08 saved checked / B | Source audit blocked: smallest located natural chain requires generics, let-else, Result/?, closures and borrowed aggregates. Scalar Result alternative additionally requires char/tables and binary-search protocol. No isolated vocabulary activation. |
| P06 borrowed result / C | Input::haystack → is_char_boundary → utf8::is_boundary needs lifetime-only callable ownership, stored-reference fields, complete mixed Anchored child and distinct slot reads/result origins. |
| P05 ownership / D | No tractable residual path/cfg positive found; no census redesign justified. Callable lifetime proof needs driver implementation/direct-callee and independent selected-impl/member gates together. Nominal lifetime capture exists; stored-reference/mixed-child/result-origin proof remains separate. |
| P02 floats / D design | Source contract ready for libm k_sinf/k_cosf; explicit float bits/NaN/target semantics, constants and f64→f32 conversions required. Production vocabulary follows char integration. |

Contracts: [character](extraction-profiles/character-leaf-v1.md),
[checked consumption](extraction-profiles/saved-checked-consumption-v1.md),
[Result audit](extraction-profiles/scalar-result-frontier-audit-v1.md),
[borrowed results](extraction-profiles/borrowed-result-chain-v1.md),
[source ownership](extraction-profiles/source-ownership-frontier-v1.md),
[floats](extraction-profiles/float-kernels-v1.md).

## Resume order

Flags gates, strict replay and selected matching are complete. Verify delivery
Character delivery CI. Exact char builtin path proof and structural
nested-char rejection are integrated; a shared normalized-type reader removes
the only new pattern candidate without ignores. Original capture/strict matching/performance pass; follow with its
escapable-character caller using source-required ASCII/range proof. Retain the
full P01–P22 frontier. Source audits are prerequisites, not support.
