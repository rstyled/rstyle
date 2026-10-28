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
| C04 | Local + lead / source rules and opt-in CLI | delivered (checkpoint) | source-rule commit | Length/parameters with separate availability, execution, scope and outcomes | Five leaf tests, six source-rule CLI cases; live labeled corpus agrees; both legacy gates and full tests pass |
| C05 | Acceptance + lead / comparison | validating | pending | Compare same-scope legacy/new results against independent labels | Fifteen Python tests and actual serial captures: ten evaluated agreements; explicit unavailable/context/regression classifications |
| C06 | Local / `analysis/guards` | awaiting integration | pending | Bounded advisory guard opportunities | Unregistered leaf and seven authored tests; not compiled or delivered |
| C07 | Pair / `analysis/pair_differences` | awaiting integration | pending | Ordered native correspondence and differences | Unregistered leaf and authored fixtures; real native capture validation pending |
| C08 | Pair / helper proposals | ready | pending | Parameterized helper proposals with counter-evidence | Depends on C07; identity forwarder negative remains required |

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
pattern discovery is retained as
`/tmp/clones-migration-checkpoint-patterns-unfiltered.json`.

Live fixed-corpus comparison artifacts are in
`/tmp/clones-capability-checkpoint-c04`: exact commands, separate fresh target
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
dependency. Unregistered guard/difference leaves and pre-existing untracked work
provide no compiled dependencies. No SHA was introduced; no legacy capability is
retired. Next ready integration is C06, then real native C07 acceptance and C08.

## Hashing exceptions

New consumers use borrowed evidence and report-local typed handles, without new
hashes. Existing `source_identity::Scheme::LegacySha256` and
`concrete_types::digest` support actual historical source claims and frozen
source/oracle pins; immutable owners compute them only for those explicit claims.
The `clones::digest` helper is test-only historical test-contract compatibility,
not a new finding identity. New routine source publication uses existing XXH3.
This bounded inventory does not claim every historical fixture has migrated.
