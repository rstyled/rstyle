# Rust support execution frontier

Authority: [delivery plan](rust-support-parallel-delivery.md) and the repository-root
[roadmap](../rust-feature-coverage.md). Updated 2026-10-21. The complete P01–P22
routing remains in the plan; this record tracks the active dependency frontier.

## Reconciled baseline

- HEAD/remote: `f71eff3`, branch `feature/rust-feature-coverage`, Data-v3 3.59.
  Native macOS, Linux x86 and Linux ARM checks independently observed successful.
- Existing task work: uncommitted Data-v3 3.60 fieldless flag enums and shared Copy
  value receivers, including complete implicit receiver reads. Preserve and finish
  this coherent increment before integrating first-wave production changes.
- Unrelated untracked work is preserved: `crates/rstyle-pdg/src/slice_access/`,
  `docs/pdg-and-clones.md`, `ferrante_toplas_87.pdf`.
- Compiler: nightly-2026-09-30, commit
  `5c543b0b8c73c7b72bc8284ced4fb22ead15734d`, host aarch64-unknown-linux-gnu;
  installed targets also include x86_64-unknown-linux-gnu and aarch64-apple-darwin.
  Cargo debug/default workspace features; family captures lib/non-test, edition
  2021, default features unless their contract explicitly adds std/no_std contexts.
- CLI capture uses trusted-generated verification; independent retained-bundle
  replay must use strict policy. The supported historical inventory runner is
  available; its sixty identities/denominators remain fixed.
- Previous delivered evidence: 929 tests/5 skips, 0 rstyle violations/3014 functions,
  0 pattern candidates, frozen 23 admitted/30 excluded/7 inactive.
- Initial modified-tree run: 933/936 passed; two legacy expectations rejected
  newly supported shared Copy receivers and one expected the prior capability
  count. Expectations updated; existing semantic tampering controls retained.
- Clippy with warnings denied now passes after extracting focused receiver test
  checks. Exact test/doctest wrapper is running (session 15168), log
  `/tmp/rstyle-wave-baseline-tests.log`. Formatting/style/pattern gate reruns
  remain pending; this checkpoint is **not green**. No new ignores/suppressions.

## Ownership and readiness

Lead exclusively owns shared type/transport/eligibility/policy/producer and test
registration files listed in plan section 5, all staging/commits, global formatting
at quiescent checkpoints, large captures, strict unit replay and performance runs.
Workers may propose shared patches; they must not edit those files independently.
No new branches/worktrees. Existing clean baseline checkout is retained.

| ID / owner | State | Target / hard prerequisite | Exclusive initial write set |
| --- | --- | --- | --- |
| Existing 3.60 / lead | validating | 13 original Look/LookSet flag functions plus empty; complete source/Copy → variant/cast → receiver load → helper chain; full original measurement pending | Existing dirty flag modules, their tests/oracles/contract; shared seams remain lead-owned |
| P01.A / A | ready (activation pending baseline) | regex-syntax 0.8.11 `is_meta_character(char)`; freeze source/build and truth table, then propose vocabulary | Audit only initially; dedicated char leaf modules/oracle/tests after approval |
| P08.B / B | blocked (source audit complete) | Natural saved checked-sub Option caller; complete blockers before activation | Audit only; proposed bitset_cardinality subtree and dedicated saved tests |
| P06.C / C | blocked (source audit complete) | Single borrowed-result getter → wrapper → consumer; reject unsuitable Input chain with named dependencies | Audit only; proposed helper_references/helper_views and dedicated tests |
| P05.D / D | blocked (source audit complete) | Natural residual in-package module ownership family; existing path tests are controls | Audit only; proposed census/selected-module proof and dedicated source tests |

Readiness requires original identities/build contexts, complete known blocker DAG,
independent positive/negative expectations, bounded contract, uncertainty and a
conflict-free write assignment. Source audits do not claim delivered support.

## Validation scheduling and evidence

Finish current process handles rather than restarting on observation timeout.
Then run the exact section-6 formatting, Clippy, test/doctest and style/pattern
wrapper gates from the root with stable binaries and `CARGO_TARGET_DIR=target`.
Retain logs under `/tmp/rstyle-look-flags-*`; full captures and frozen replays live
in ignored target directories. One large capture/owning-unit replay/performance
run at a time. Existing 3.59 reference rosters are retained under
`/tmp/rstyle-helper-index-*` for unchanged-identity delta checks.

After the baseline gates, approve ready first-wave leaves serially at shared
interfaces. Integrate focused contracts/oracles and connected activation commits;
only a fully gated natural/strict/matching checkpoint is delivered. Reassess the
frontier after every family; broader P01–P22 coverage remains active.


## First-wave source findings

- A: original regex-syntax `is_meta_character` is bounded and ready; exact
  identities and full Unicode numeric-set oracle are frozen in
  [character contract](extraction-profiles/character-leaf-v1.md). Dedicated
  production interfaces need lead review after the current baseline.
- B: [saved checked audit](extraction-profiles/saved-checked-consumption-v1.md)
  found a natural zerovec caller requiring generics, let-else, Result/?, closures
  and borrowed aggregate validation. Bitset-only activation is not justified;
  reassignment should select a ready concrete Option/Result family.
- C: [borrowed chain audit](extraction-profiles/borrowed-result-chain-v1.md)
  freezes Input::haystack → Input::is_char_boundary → utf8::is_boundary. Hard
  prerequisites: lifetime-only complete owner proof, stored-reference fields,
  mixed Anchored child proof and separate slot-read/result-referent summaries.
  Ordinary PatternID declarations do not establish a need for general macros.
- D: [source ownership audit](extraction-profiles/source-ownership-frontier-v1.md)
  found no tractable residual path/cfg positive. Reprioritize to C's demonstrated
  lifetime-only callable/complete owner prerequisite; nominal lifetime capture
  already exists. Shared-reference and mixed-enum children still need complete
  proof. No census change is justified. No production authority granted.

- Contract/frontier commit `cde2959` pushed to the feature branch. B is auditing
  a concrete scalar/unit Result or scalar Option if-let replacement; no production
  work is authorized while the current integrated baseline runs.
