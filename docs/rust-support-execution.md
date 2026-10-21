# Rust support execution frontier

Authority: [delivery plan](rust-support-parallel-delivery.md) and the repository-root
[roadmap](../rust-feature-coverage.md). Updated 2026-10-21. The complete P01–P22
routing remains in the plan; this record tracks the active dependency frontier.

## Reconciled baseline

- HEAD/remote: `f71eff3`, branch `feature/rust-feature-coverage`, Data-v3 3.59.
  Native macOS, Linux x86 and Linux ARM checks independently observed successful.
- Existing task work: Data-v3 3.60 fieldless flag enums and shared Copy
  value receivers, including complete implicit receiver reads (now committed `4fcc007`). Finish
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
  checks. Exact initial wrapper then exposed one remaining stale count (17→18 functions,
  136→153 pairs). Corrected and focused wrapper/doctests pass. Final complete
  wrapper passes: **936/936**, five existing skips, doctests pass; log
  `/tmp/rstyle-wave-baseline-tests-final.log` (328.392s).
- Formatting and Clippy pass; required style wrapper reports 0/3041, patterns
  wrapper passes with 0 candidates and unchanged existing ignores. Logs:
  `/tmp/rstyle-wave-baseline-{clippy.log,style.json,patterns.log}`.
  Full-original/strict/matching/delta evidence is still pending; **not delivered**.

## Ownership and readiness

Lead exclusively owns shared type/transport/eligibility/policy/producer and test
registration files listed in plan section 5, all staging/commits, global formatting
at quiescent checkpoints, large captures, strict unit replay and performance runs.
Workers may propose shared patches; they must not edit those files independently.
No new branches/worktrees. Existing clean baseline checkout is retained.

| ID / owner | State | Target / hard prerequisite | Exclusive initial write set |
| --- | --- | --- | --- |
| Existing 3.60 / lead | validating | 13 original Look/LookSet flag functions plus empty; complete source/Copy → variant/cast → receiver load → helper chain; full original measurement pending | Existing dirty flag modules, their tests/oracles/contract; shared seams remain lead-owned |
| P01.A / A | awaiting integration (unbuilt leaves) | regex-syntax 0.8.11 `is_meta_character(char)`; freeze source/build and truth table, then propose vocabulary | Audit only initially; dedicated char leaf modules/oracle/tests after approval |
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

### Refined next frontier

The bounded B replacement audit is also
[blocked](extraction-profiles/scalar-result-frontier-audit-v1.md): the smallest
located scalar Result original needs character/table origins, closure destructuring
and binary-search protocol proof. Do not activate isolated Result vocabulary.
A's shared-interface review found that globally widening `scalar()` would also
activate unsupported aggregate/Option cases and invalidate an enum-cast integer
guard. Use a dedicated capability-gated char primitive path; retain existing
integer gates. Original char spelling already survives producer serialization.

- Connected production increment committed as `4fcc007`; exact original 14-function
  selection plus six negative controls frozen in `0406406` before fresh original
  measurement. Neither commit claims complete natural acceptance. All shared
  production files remain unchanged during the final suite.


## Green baseline and current execution

Integrated production `4fcc007` plus source selection `0406406`: exact section-6
baseline gates all pass. Existing ignores remain unchanged. This is a green
implementation checkpoint, not yet completed natural-family delivery.

- Stable CLI/driver snapshot: `/tmp/rstyle-look-flags-final-bin/`, built from
  `0406406`. Complete unchanged regex-automata capture now runs as session 62733,
  timeout 1800s, isolated output `/tmp/rstyle-look-flags-natural-target`.
  Logs `/tmp/rstyle-look-flags-natural-capture.{log,stderr}`. No broad matching:
  max-pairs zero remains explicitly incomplete; strict selected replay follows.
- A approved for dedicated `character_scalars.rs` and `variant_capture/character_leaf.rs`
  leaves, plus its existing contract/oracle only. No builds/captures/registration
  or shared source edits. Shared integration follows the current family's evidence.
- Lead retains shared seams, binaries, captures, strict replay, staging and commits.
  B/C/D audits establish named prerequisites; no unsupported activation granted.


### Resume commands and integration boundary

A returned its unregistered leaves: `crates/rstyle-pdg/src/character_scalars.rs`
and `crates/cargo-rstyle/tests/variant_capture/character_leaf.rs`. No shared files
changed and no builds were run. Review original-literal/source coverage and
historical document gating before registration; no char support is claimed yet.
D is preparing the plan-authorized libm float acceptance contract/oracle design,
without activation. B's replacement Result audit remains blocked as recorded.

Next: poll session 62733 (confirmed live after 3m); obtain the `Report:` path from
`/tmp/rstyle-look-flags-natural-capture.log`, then run
`/tmp/rstyle-coverage-python/bin/python /tmp/rstyle-look-flags-natural-audit.py REPORT`.
That script streams the large report, checks all 2582 source/build identities,
all declared positives/negatives and every gain/loss. Do not load the whole report
with json.load. Follow with one owning-unit strict replay, all selected matching,
frozen sixty/other package deltas and bounded performance controls, serially.
The current complete-original capture uses stable 3.60 binaries; do not rebuild
or replace them. Character shared integration waits for this coherent family
boundary. Root roadmap remains at delivered 3.59 until evidence warrants updating.
