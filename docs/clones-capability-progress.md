# Clones capability migration progress

Execution starts at `dd7f4fb` on 2026-10-27. The supplied execution prompt and
migration plan define the scope; this ledger records delivered slices rather
than declaring the wider migration complete. The [matrix](clones-capability-matrix.md)
and [acceptance corpus](../eval/clones-capability/README.md) freeze the first scope.

## Baseline and ownership

The starting revision passed 2,332 workspace tests (five skipped), formatting,
Clippy, and both legacy dog-food gates: zero style violations across 8,200
functions and zero reviewed pattern candidates. Logs retained for that exact
revision: `/tmp/rstyle-clean-{tests,style,patterns,clippy}.log` and corresponding
style/pattern JSON. These results precede migration implementation; checkpoint
gates will be repeated. Native Linux aarch64 only; other platform gates and exact
Go behavior comparisons remain unverified.

Pre-existing untracked work, including the supplied migration documents,
`slice_access/`, `docs/pdg-and-clones.md`, scalar-loop coverage fixtures and the
PDF, is excluded from task commits. The lead owns shared registration, report/CLI,
this ledger, and Git staging. Evidence owns `consumer_facts`; local consumers own
`analysis/source_rules`; acceptance owns `eval/clones-capability` and the matrix.
Heavy captures and integrated gates run serially.

## Delivery ledger

| ID | Owner / write set | State | Commit | Capability / scope | Evidence and remaining work |
| --- | --- | --- | --- | --- | --- |
| C01 | Acceptance / corpus and matrix | delivered (acceptance specification) | `112c085` | Independently labeled source/signature boundaries, excluded-body case, strict pair and negative abstraction; prepared guard/difference cases | JSON/count checks, fixture formatting and offline Cargo check; producer acceptance not yet claimed |
| C02 | Evidence / `consumer_facts` and lead registration | delivered (checkpoint) | `0d97fd8`, `3080a7d` | Source/interface availability independent of strict graph admission | Nine evidence tests plus live public/receiver capture tests; visibility span correction; no new extraction permission |
| C03 | Lead / clones report and CLI acceptance | delivered (checkpoint) | `ffce699`, `3080a7d` | Source-located accepted strict pairs | Full/compact/text navigation agrees; pair budget remains incomplete; test module discovery fixed |
| C04 | Local + lead / source rules and opt-in CLI | delivered (checkpoint) | `644151d` | Length/parameters with separate availability, execution, scope and outcomes | Five leaf tests, six source-rule CLI cases; live labeled corpus agrees; both legacy gates and full tests pass |
| C05 | Acceptance + lead / comparison | delivered (bounded comparison) | `3e1f4c9` | Compare same-scope legacy/new results against independent labels | Fifteen Python tests and actual serial captures: ten evaluated agreements; explicit unavailable/context/regression classifications |
| C06 | Local + lead / guards and independent opt-in | delivered (checkpoint) | `115cd07` | Bounded advisory guard opportunities | Seven unit and five real-Cargo CLI cases; graph-excluded callee does not gate advice; full checkpoint passes |
| C07 | Pair + lead / native differences, opt-in CLI/report | delivered (checkpoint) | this C07 implementation commit | Ordered native correspondence and differences in accepted strict pairs | Thirteen native unit and three real-Cargo CLI cases; literal/formal roles, context/width, whole-tree rejection and budgets; matching unchanged |
| C08 | Pair / helper proposals | ready (independent labels frozen) | acceptance-preparation commit | Parameterized helper proposals with counter-evidence | Eight source-authored cases in `proposal-review-cases.json`; depends on C07; labels are not implementation evidence |
| C09 | Acceptance + lead / versioned review | ready (independent labels frozen) | acceptance-preparation commit | Persistent review separate from report handles | Nine lifecycle/collision/context cases prepared; no review implementation claimed |

No capability is retired. Stable review identity, broader proposals, conservative
effects/mutation and work planning remain later required migration work.

## First consumer checkpoint (2026-10-28)

The frozen source-rule implementation passes formatting, workspace/all-target
Clippy with warnings denied, 2,354 nextest cases (five skipped) and doc tests.
Nextest runtime was 157.777 seconds. Both originally reported NonZero failures
pass. Legacy style reports zero violations across 8,229 functions; reviewed
patterns passes without new dismissals or policy changes. Retained logs:
`/tmp/clones-migration-checkpoint-{clippy,tests,style,patterns}.log`, with style
JSON at `/tmp/clones-migration-checkpoint-style.json`. The separate unfiltered
pattern discovery found 15 candidates and is retained as
`/tmp/clones-migration-checkpoint-patterns-unfiltered.json`.

Live fixed-corpus comparison artifacts are in
`/tmp/clones-capability-checkpoint-c04` and repeated against committed Rust
revision `644151d` at `/tmp/clones-capability-644151d`: exact commands, separate fresh target
directories, captured legacy roster, input/context snapshots and reports. All ten
independent length/parameter labels agree; the new path evaluates 32 rules over
16 captured owners with complete scope, including the closure-body example. The
labeled strict pair is accepted. Capture elapsed times were 0.114s legacy and
0.565s new; these different workflows do not establish a speedup. Prebuilt binary
provenance comes from the preceding workspace build, with paths/stat recorded.
Legacy lacks `--locked`; the runner uses offline mode and checks unchanged lock
bytes. The new capture uses `--locked` explicitly. These are native Linux
aarch64 results; macOS and exact Go comparisons remain unverified.

Checkpoint Rust validation includes every task-owned registered source/test
dependency. Unregistered guard/difference leaves at that checkpoint and
pre-existing untracked work provide no compiled dependencies. No SHA was
introduced; no legacy capability is retired.

## Guard checkpoint (2026-10-28)

`--guard-clauses` is independent of `--source-rules`. The aggregate retains
per-consumer outcomes and separate numeric/guard evidence, with shared scope and
execution accounting. Unsupported syntax, absent ownership and work exhaustion
remain incomplete. Guard advice establishes no purity, equivalence or safe rewrite.

Formatting, workspace/all-target Clippy, both legacy gates and all 2,366 nextest
cases (five skipped) plus doc tests pass. Nextest runtime is 157.335 seconds.
Style reports zero violations across 8,246 functions; unfiltered patterns retains
the same 15 reviewed candidates. Logs and JSON are under
`/tmp/clones-migration-c06-checkpoint-{clippy,tests,style,patterns}.*`, with separate
`patterns-unfiltered.json`. The next pair-difference leaf remains unregistered and
does not supply compiled dependencies to this commit.

The live guard corpus report `/tmp/clones-migration-c06-guards.json` records one
finding, 15 evaluated functions and one explicitly unsupported closure example:
completed execution, incomplete scope, exit two as required. The complete
single-positive fixture returns zero by default and one with `--fail-on-findings`.
Full/compact/text and the graph-excluded-call case pass real-Cargo tests. Repeat
legacy/new source-rule comparison at `/tmp/clones-capability-c06-checkpoint`
still yields ten agreements.

A manual saved standalone-capture source-rule reevaluation exits two because
`--replay-run` currently requires `dependency-closure.json`; see
`/tmp/clones-migration-c06-source-replay.log`. Standalone saved-facts reevaluation
remains a required later workflow gap. This is not a passed replay gate or a
reason to weaken import validation. Next is real native C07 integration, C08
helper proposals and versioned C09 review; quality/replay can advance independently.

## Native pair-difference checkpoint (2026-10-28)

`--explain-pairs` adds a separate versioned report over accepted strict pairs.
It retains ordered native correspondence and compact literal/formal/operator/shape
holes without changing matching decisions. Whole-tree validation is bounded and
borrowed, performed once per requested endpoint and reused across pairs, including
negative outcomes. No new extraction permission, hash, suppression or threshold
change was introduced. Unknown matching denominators, unsupported/ambiguous trees
and exhausted native budgets cannot become a complete empty explanation.

Thirteen native unit cases and three real-Cargo CLI cases cover binder changes,
ordered subtraction, repeated reads, unsupported whole rosters, exact identity,
context/native-width isolation, depth/parameter budgets, full/compact/text and
opt-in/default behavior. The integrated checkpoint passes formatting, all-target
workspace Clippy with warnings denied, all 2,382 nextest cases (five skipped),
doc tests and both legacy gates. Nextest runtime is 157.981 seconds. Style reports
zero violations across 8,271 functions. Unfiltered patterns retains the same 15
candidates without new dismissals. Logs are retained at
`/tmp/clones-migration-c07-{tests,clippy,style,patterns}.*` and the separate
`patterns-unfiltered.{json,log}`.

The fresh fixed-corpus report `/tmp/clones-migration-c07-live.json` evaluates all
nine accepted pairs: four complete native comparisons and five explicitly
unsupported comparisons, completed execution with incomplete coverage and exit
two. Both independently labeled difference cases pass: add-one/add-two has four
correspondences and one right-literal hole; reordered/original subtraction has
four correspondences and both ordered formal-role holes. Owning evidence is kept
under `/tmp/clones-migration-c07-live-target`. Linux aarch64 only; macOS and exact
Go comparisons remain unverified.

C08 helper proposals are next: consume completed ordered evidence, retain useful
computation and literal arguments, reject changed formal roles/operators and
never infer an all-member group from connected pair edges. The eight frozen
proposal labels remain requirements, not implemented behavior. Versioned review,
quality/effects, saved standalone reevaluation and planning remain required;
legacy consumers are not retired.

## Hashing exceptions

New consumers use borrowed evidence and report-local typed handles, without new
hashes. Existing `source_identity::Scheme::LegacySha256` and
`concrete_types::digest` support actual historical source claims and frozen
source/oracle pins; immutable owners compute them only for those explicit claims.
The `clones::digest` helper is test-only historical test-contract compatibility,
not a new finding identity. New routine source publication uses existing XXH3.
This bounded inventory does not claim every historical fixture has migrated.
