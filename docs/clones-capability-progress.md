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
| C07 | Pair + lead / native differences, opt-in CLI/report | delivered (checkpoint) | `63ef4c5` | Ordered native correspondence and differences in accepted strict pairs | Thirteen native unit and three real-Cargo CLI cases; literal/formal roles, context/width, whole-tree rejection and budgets; matching unchanged |
| C08 | Pair + acceptance + lead / native helper consumer, CLI/report | delivered (checkpoint) | `26b7b9c` | Advisory u32 expression helper pairs with literal arguments, ordered evidence and explicit prerequisites | All eight frozen cases pass real-Cargo tests and retained live captures; nineteen native/consumer unit cases plus ten CLI cases; no inferred groups or safe-rewrite claim |
| C11a | Local + acceptance + lead / saved capture reconciliation and CLI/report | delivered (bounded replay) | `428416b` | Exact standalone saved-facts reevaluation without compilation or original files | Eight unit/five CLI cases, retained C06 capture, full tests and legacy gates; mutation/effects remain open |
| C09a | Pair + local + acceptance + lead / review lifecycle, original snapshots and CLI/report | delivered (bounded helper review) | `1269c62` | Explicit saved-run dismissal and read-only versioned exact review, stale/unavailable coverage | Twenty-seven unit/adapter and seven CLI cases; retained live replay, full tests and both legacy gates; whole-store status/legacy migration remain open |
| C10a | Local + acceptance + lead / dispatch source index, CLI/report | delivered (bounded local dispatch) | `fabd24d` | Source-owned two-variant dispatch with exact local method/trait signatures | Twelve evidence/nine CLI tests; full tests and both legacy gates pass; broader families remain open |
| C11b | Local + acceptance + lead / mutation evidence, policy, CLI/report | delivered (bounded direct mutation) | `3329163` | Direct scalar assignment-site budgets with root provenance and unknown-aware lower bounds; config-only strict replay | Eighteen evidence/three policy/seven CLI tests; full tests and both legacy gates pass; general effects/density/helper summaries remain open |
| C12a | Evidence + pair + lead / original body views, overlap model, CLI/report and acceptance | delivered (bounded helper edit coordination) | `21ba934` | Explicit helper-wrapper intents, exact original overlap witnesses and review groups; unknown insertion/name/scope and independence | Seven evidence/ten pure/six CLI tests; retained compact-to-full strict replay; full tests and both legacy gates pass; local guard/caller intents remain pending |
| C11c | Evidence + local + lead / typed call observations, leaf summaries, CLI/report and acceptance | delivered (bounded scalar leaf effects) | `c68bcc5` | Caller-visible scalar writes through original same-unit leaves, distinct call events and exact formal mappings | 2,553 nextest tests and doc tests pass; zero style violations and reviewed pattern candidates; exact 15 baseline IDs unchanged; source/tool-free strict replay passes |
| C10b | Evidence + pair + acceptance + lead / shared declaration index, wrapper witnesses, pairing and CLI/report | delivered (bounded witnessed capabilities) | `0547b2e` | Two-member local capabilities backed by original ordered calls, independent of strict matching; exact local trait suppression | Nineteen independent source cases, six wrapper and eight pairing unit cases, eleven CLI tests; 2,578 full tests, both legacy gates and strict source/tool-free replay pass; broader families and retirement remain open |

| C09b | Evidence + review + acceptance + lead / shared source identity index, typed local review and CLI/report | delivered (source-local dismissal) | `78cec55` | Explicit saved-run dismissal for dispatch and ordered capability findings, independent of graph matching; helper v1 preserved | 34 review unit tests, three source identity tests and 17 review CLI tests pass; 2,598 workspace tests and doc tests pass; both legacy gates clean |
| C09c | Lead + acceptance + contract / retained-entry status, original identities and CLI/report | delivered (bounded whole-store status) | `d15bdc4` | Explicit whole-store read-only assessment with completed negatives, omitted scopes/families and unavailable evidence distinguished | 12 status/context unit tests and 11 real-Cargo CLI tests; 2,621 workspace tests plus doc tests, both legacy gates and retained live replay pass |
| C09d | Lead + contract + acceptance / selective review pruning | delivered (completed negatives only) | `644aef8` | Explicit saved-run preview/apply, original inventory validation and atomic schema-preserving removal | Eight contract and eight CLI cases; validation below |
| C04b | Evidence + policy + acceptance + lead / captured comments, source reports and comparison | delivered (length/parameter suppression compatibility) | `7436414` | Legacy named source suppressions from immutable snapshots, invalid diagnostics and raw/visible outcomes | Five evidence, six policy and six CLI cases; comparison and checkpoint validation below |

| C04c | Contract + policy + acceptance + lead / shared glob matcher, captured path decisions and source reports | delivered (workspace path exclusions) | `b65f15c` | Default/configured exclusions with explicit complete skipped rows, independent guard coverage and source-free policy reevaluation | Four matcher, six decision and seven CLI cases; checkpoint evidence below |
| C05b | Acceptance + runner + lead / independent exclusion labels and comparison | delivered (bounded exclusion comparison) | `6958b92` | Separate policy-excluded agreement from measured source-rule agreement; retain excluded legacy roster and context | 38 independent Python tests and live frozen-corpus comparison; checkpoint evidence below |

| C04d | Evidence + policy + acceptance + lead / direct statement view, shared spacing predicate and opt-in source policy | delivered (bounded spacing) | `0e7234e` | Original source spacing independent of strict matching, explicit unsupported transformations and source-free policy reevaluation | Focused tests and integrated checkpoint evidence below |
| C05c | Acceptance + runner + lead / spacing corpus, comparator and independent labels | delivered (bounded spacing comparison) | `bf11857` | Optional independently labeled spacing measurements, explicit disabled-policy agreement and three-rule exclusion checks | 56 Python tests, live serial comparison and integrated checkpoint evidence below |
| C12b | Evidence + acceptance + lead / caller census, signature regions, overlap and CLI/report | delivered (bounded caller-aware signature intents) | `30f6a06` | Advisory parameter signature intents with exact same-unit witnessed callers and separate census coverage | Eight real-Cargo acceptance cases, 2,709 workspace tests, both legacy gates and source-free strict replay; checkpoint evidence below |

| C12c | Evidence + acceptance + lead / immutable formal evidence, bounded bundle search and report | delivered (bounded witnessed forwarding) | `b03b349` | Repeated per-target unchanged-scalar forwarding with exact positional support witnesses | 2,724 workspace tests, both legacy gates and source-free live replay; checkpoint evidence below |
| C12d | Evidence + acceptance + lead / original use scan, global retention, caller report | delivered (bounded scalar all-use evidence) | this checkpoint | Complete owned-u32 use inventories and advisory selected-intent pass-through outcomes | 2,743 workspace tests; style zero; exact 15 baseline pattern IDs; fresh source/tool-free strict replay |

| C09e | Model + workflow + acceptance + lead / completed parameter facts, review storage/status/prune and CLI/report | delivered (exact parameter reviews) | this checkpoint | Saved numeric parameter-count choices with context/policy staleness and selected hiding | 2,835 workspace tests; style zero; same 15 unfiltered pattern IDs; source/tool-free strict replay; bounded free-function identity |

| C09f | Model + workflow + acceptance + lead / shared numeric review, completed length measurements and CLI/report | delivered (numeric length reviews) | this checkpoint | Saved numeric function-length choices with independent family hiding and shared original owner resolution | 2,857 workspace tests and doc tests; zero style violations; same 15 unfiltered pattern IDs; strict source/tool-free replay; conservative same-policy pruning |

| C09g | Model + workflow + acceptance + lead / completed spacing measurements, shared review presentation and CLI/report | delivered (configured spacing reviews) | this checkpoint | Saved direct-statement spacing choices with explicit policy and independent three-family hiding | 2,880 workspace tests and doc tests; zero style violations; same 15 unfiltered pattern IDs; source/tool-free strict replay; empty-policy and conservative pruning acceptance |

| C11d | Evidence + acceptance + lead / bounded helper composition, original witnesses and report | delivered (two-edge scalar summaries) | this checkpoint | Caller-visible writes through complete original wrappers with composed formal roots and distinct paths | 2,898 workspace tests and doc tests; zero style violations; same 15 unfiltered pattern IDs; exact-fit/one-short retention, depth/cycle and strict source/tool-free replay cases |

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

## Helper-proposal checkpoint (2026-10-28)

`--helper-proposals` includes native explanations and adds a separate versioned
helper report. The fixed initial policy requires exclusively u32 signatures and
expression nodes, two builtin operations per member and at most one literal hole.
Proposals retain ordered formal positions, member roots, operation counts and
source-backed literal arguments; common literals are not promoted to parameters.
Forwarding, small bodies and changed formal roles/operators/shapes remain
explicit negatives. Unknown calls and writes retain their original unavailable
owner rows; findings can coexist with incomplete coverage. Two connected pair
proposals never imply a three-member proposal.

Each immutable endpoint retains its completed/negative native proof and bounded
width/u32/count summary. A privately constructed comparison binds those results
to exact borrowed members before helper evaluation; no graph copies, repeat
admission or hashes were added. Default helper analysis remains not requested;
complete findings use opt-in exit one and incomplete scope takes exit two.
Legacy configuration, suppressions, review entries and strict decisions stay
independent, and no legacy capability is retired.

All nineteen native/consumer unit cases and ten real-Cargo CLI cases pass in the
full checkpoint: 2,398 nextest tests, five skipped, doc tests pass; nextest runtime
158.751 seconds. Formatting, workspace/all-target Clippy, zero legacy style
violations across 8,294 functions and the reviewed pattern gate pass. The new
empty/single literal argument representation follows the fixed one-hole policy;
no ignores or thresholds changed. Logs are retained at
`/tmp/clones-migration-c08-{tests,clippy,style,patterns}.*`, with separate
`patterns-unfiltered.{json,log}` comparison evidence.

All eight frozen proposal cases also pass fresh offline/locked captures retained
in `/tmp/clones-migration-c08-live`: original sources/manifests/lockfiles, exact
commands, reports, stderr, exit status, elapsed time and owning evidence bundles.
`results.json` identifies the precommit C08 checkpoint rather than claiming that
the older base revision contained the new consumer. Positive cases each produce
one proposal; negative cases produce zero with complete scope; call/write cases
remain incomplete and exit two. The nontransitive case produces exactly two pair
findings over three independently evaluated accepted pairs. Linux aarch64 only;
macOS and exact Go comparisons remain unverified.

Next is versioned review with full exact locator/context/expression membership,
separate from navigation and report handles. Original free-function ownership and
captured effective build settings provide the first ready identity route;
ordinal/ambiguous/generated locators must remain explicitly unavailable. The
standalone saved-run replay gap can also advance independently by reusing saved
Cargo/plan/unit reconciliation and existing strict import, without new trust
flags. Neither later capability is delivered by this checkpoint.

## C11a: standalone saved-run reevaluation

State: delivered bounded workflow slice; commit: this checkpoint. Lead owns
capture/replay routing, selection provenance, CLI registration and docs; local
consumer and acceptance workers supplied frozen loader/test leaves. This delivers
the saved-facts configuration workflow within C11, not mutation/effect evidence.

Explicit `--replay-run` now reconciles standalone plans, unique Cargo root indices,
successful final Cargo completion, exact selected artifacts and compiler ledgers
before the existing strict bundle import. Check/test roots may contain identical
Cargo metadata; their compiler-unit modes and ledgers remain separate. Dependency
captures still require their closure inventory and artifact associations. Replay
rejects selection/build overrides and reports saved selection provenance rather
than implying the current request flags describe the original capture.

Eight loader cases and five real-Cargo CLI tests cover source policy changes,
newly requested guards/helpers, deleted source/manifests, unavailable compiler
tools, original all-target populations, invalid overrides and missing/changed
completion/ledgers/bundles. The existing dependency replay test additionally
rejects a missing closure inventory. The retained original C06 standalone capture
now passes strict replay: 16 owners, 32 complete source-rule assessments and two
findings; `/tmp/clones-migration-c11-replay-retained.{json,log}`.

Integrated checkpoint: 2,411 nextest tests passed, five skipped, 159.090 seconds;
doc tests passed. Workspace/all-target Clippy and formatting passed. Legacy style has zero
violations across 8,302 functions; the reviewed pattern gate passes with zero
new candidates. The separate unfiltered comparison is retained. Logs use
`/tmp/clones-migration-c11-replay-*`. Linux aarch64 only; macOS and exact Go
comparison remain unverified. No legacy capability is retired and no new hashes,
trust flags, suppressions or ignores were added.

Next ready work is C09: the pure review lifecycle leaf is frozen and unregistered
in `clones/review/`, explicitly excluded from this checkpoint. It needs validated
source/context/expression snapshots, a real helper-proposal adapter and CLI
persistence before it becomes a delivered capability. C10 trait/dispatch, C11
mutation/effects, C12 caller/edit plans and selective later consumers remain open.

## C09a: versioned helper review and explicit dismissal

State: delivered bounded helper workflow; commit: this checkpoint. Lead owns
CLI/report routing, review workflow IO, snapshot caching, registration and docs;
pair/local/acceptance workers supplied disjoint frozen lifecycle, adapter and CLI
leaves. Those leaves are now registered and included in checkpoint validation.

`--helper-proposals --review-file PATH` applies exact versioned reviews without
writing. Explicit `--dismiss-helper-pair FIRST SECOND --review-reason TEXT`
requires a saved run and writes only the independently checked helper pair.
Atomic replacement follows complete store/selector/reason validation. Missing
files are empty read-only stores. Reports retain unfiltered findings, visible and
hidden counts, exact/stale/unavailable pair status, reasons and retained entries;
incomplete helper/review scope still exits two. Strict matching is unchanged.

Persistent snapshots use original unique free-function package/source/qualified
locators, typed target and effective build/matching/consumer settings, signatures
and ordered u32 expressions. Exact equality replaces hashes. Line movement and
formal renaming preserve reviews; relevant literals/operations/context/versions
stale them. Full all-pair evidence is mandatory for growth in the pure lifecycle;
current live proposals still contain exactly two members. Member snapshots and
negative conversions are cached per report-local function; one immutable checked
store view serves all pair lookups. No graph cloning/re-admission is introduced.

Pure lifecycle and adapter fixtures cover the nine frozen labels, malformed wire
claims, missing/duplicate/foreign pair rosters, exact expression/item/depth bounds,
UTF-8/CRLF offsets and borrowed store reuse. Seven real-Cargo CLI cases cover
explicit dismissal, deleted original files/unavailable tools, read-only lookup,
line/binder stability, stale literals, feature/module separation, incomplete
owners, invalid input without writes, and helper findings that lack reviewable
method/conditional locators. All 2,445 nextest tests passed, five skipped,
160.251 seconds; doc tests, formatting and workspace/all-target Clippy passed.
Legacy style has zero violations across 8,360 functions and the reviewed pattern
gate passes without new ignores. Separate unfiltered comparison is retained.
The retained C08 literal-hole capture also passes explicit dismissal followed by
read-only replay with tools disabled and unchanged review bytes, at
`/tmp/clones-migration-c09-review-live/{results,dismiss,lookup,reviews}.json`.
Logs use `/tmp/clones-migration-c09-review-*`.

Scope is bounded original free functions within the captured workspace. Methods,
nested/generated/ambiguous owners and outside-workspace locators are unavailable;
legacy review migration, source-rule reviews, live multi-member groups and a
whole-store status/changed-scope command remain open. Unmatched stored entries
are retained rather than inferred deleted or pruned. The pure member-loss fixture
does not establish live whole-store deletion detection. No legacy retirement or
macOS/Go parity is claimed.

Next C10 frontier: original exhaustive two-variant local enum dispatch, each arm
calling a unique local shared-receiver zero-argument/u32 inherent method on that
arm's payload. Signature/source evidence must be usable independently of body
admission. Positive acceptance includes one opaque method body. Negative controls
include incompatible arity, same free callee, an outer receiver, unrelated trait
names and no dispatch witness; exact existing-trait relations suppress proposals.
Guards/wildcards/opaque arms and unresolved trait origins stay explicitly
unsupported. General owner/trait relations need a checked local evidence route;
raw display strings and impl ordinals are not authority. C11 mutation/effects,
C12 edit plans and later selective consumers remain independent open work.

## C10a: local enum-dispatch advice

State: delivered bounded dispatch family; commit: this checkpoint. Lead owns CLI/report routing,
registration and docs; pair worker owns the source-evidence leaf, acceptance
worker supplied frozen source labels and CLI cases, and local worker reviews
namespace/coverage behavior independently.

`--enum-dispatch` uses original source/signature evidence independently of strict
pair matching. The bounded witness is an exhaustive two-variant local enum match
whose arms call corresponding payload methods with shared-self/zero-argument/u32
signatures. Opaque method bodies do not block available declaration evidence.
Exact existing local trait-member relations suppress advice; unrelated traits,
wrong receivers, incompatible arity and same free callees are distinct negatives.
Unsupported arms, ownership ambiguity and internal work limits remain explicit.
The report retains original locations, per-owner reasons and complete/incomplete
scope; a known finding can coexist with an incomplete result. Source-only replay,
full/compact agreement and explicit analysis budgets have independent CLI checks.

No legacy retirement, general trait/capability/generic family or refactoring
safety claim is implied. The frozen snapshot passes all 2,466 nextest tests (five skipped), 162.915
seconds, and doc tests. Workspace/all-target Clippy, formatting and diff checks
pass. Legacy style has zero violations across 8,439 functions; reviewed patterns
has zero new candidates, and unfiltered comparison retains the exact previous
15 candidate IDs. No ignores, suppressions, thresholds or strict admission were
changed. Logs use `/tmp/clones-migration-c10-*`.

Nine real-Cargo CLI tests and twelve evidence tests cover the independent labels,
original header/arm spans, exact source-owner isolation, namespace closure,
primitive shadowing, competing mutable/shared method origins, opaque same-callee
arguments, explicit source/declaration/module limits, budgets and mixed coverage.
The retained fresh compact example also finds one opportunity across three
complete owners, then strictly replays with original source/manifest deleted and
all external tools disabled. Its consumer report is unchanged and explicit
finding failure exits one; `/tmp/clones-migration-c10-live-8fomlkdg/results.json`.
Linux aarch64 only; macOS and exact Go comparison remain unverified.

The next independently prepared frontier is direct scalar-write provenance with
unknown-aware mutation/effect boundaries;
C11 evidence, C12 edit plans and selective later consumers remain open.

## C11b: direct scalar mutation budgets

This checkpoint adds opt-in `--mutation-rules` and explicit TOML
`--mutation-config` independently of strict clone admission. It reuses legacy
mutation limits while defining a deliberately distinct original assignment-site
vocabulary. Direct owned `u32` bindings and `&mut u32` formal pointees retain
compiler identity, declaration/lexical-scope spans, formal ordinal, operation and
caller-visible provenance. Builtin compound assignments count once; declaration
initialization is excluded. Counts describe static sites, not runtime executions.

Complete bounded inventories report exact counts; unknown calls and unsupported
syntax retain typed boundaries and witnessed lower bounds. A witnessed threshold
breach survives incomplete scope; no clean result or density ratio is inferred
from an unknown remainder. Raw addresses, reference reassignment, mutable aliases,
macros, attributes and unsupported shadow patterns cannot mint stale roots.
Missing evidence stays unavailable; contradictory consumed source/native/context
joins fail. Source ownership is checked before work budgets, original syntax is
parsed once per query, and native/binding indices borrow immutable inputs.
No new hashes, graph copies, extraction permissions or proof caches were added.

The frozen snapshot passes all 2,494 nextest tests (five skipped), 164.135 seconds,
and doc tests. Eighteen evidence tests, three policy tests and seven real-Cargo
CLI tests cover initial positive/negative cases, independent spans/provenance,
replay/configuration, partial findings, opaque-root controls and resource bounds.
Workspace/all-target Clippy, formatting and diff checks pass. Legacy style has
zero violations across 8,531 functions; reviewed patterns has zero new
candidates. Unfiltered patterns retains exactly the same 15 candidate IDs as C10.
No ignores, suppressions, thresholds or strict admission were changed.
Logs: `/tmp/clones-migration-c11-*` (final full tests in `tests-final.log`).

The retained bounded compact live capture uses `--locked`, then deletes original
source/manifest and disables external tools. Strict replay changes the per-root
limit from two to three, retaining identical assignment evidence, original
function rows and matching while the finding disappears (exit one to zero):
`/tmp/clones-migration-c11-live-8ig3hqmz/results.json`.
[Mutation scope and usage](clones-mutations.md) document limitations. General
effects, aliases/fields, helper summaries, density, review integration and legacy
retirement remain open. Linux aarch64 only; macOS and exact Go comparison remain
unverified.

C12a below advances required-edit overlap planning from explicit finding intents
and checked original source regions. Caller-aware parameter plans still need an
explicit parameter edit and complete call/argument provenance; similarity
components do not establish edit independence.

## C12a: required helper body edit coordination

This checkpoint introduces explicit `--helper-proposals --edit-plans` for the
advisory intent of a shared helper with original-signature wrapper bodies. Only
actual completed helper findings supply plans. Checked original half-open file
body spans, workspace-relative source resources and package/context witnesses
supply existing required edits. Overlap witnesses retain their exact
intersections; connected review groups neither merge helper proposals nor prove
stack order. Disjoint known bodies leave declaration insertion/name/scope and
whole-plan independence unknown.

Source views retain immutable file owners by `Arc`. Endpoint resolution is reused
across pairs; exact snapshot equality uses a pointer fast path and bounded byte
work without hashes. The same source region across lib/test contexts overlaps;
conflicting snapshots and missing ownership remain incomplete. Source ownership
precedes a shared conservative mutation/planning syntax budget. Plan/region/byte
budgets preserve checked witnesses while refusing a complete truncated result.
Typed unavailable endpoint observations retain unknown/unsupported distinctions.
Full/compact reports use the same consumer, and strict source-free/tool-free
replay retains identical footprints and matching.

Seven evidence tests, ten pure overlap tests and six real-Cargo CLI tests cover
the initial contract. All 2,517 nextest tests pass (five skipped), 163.816 seconds,
plus doc tests. Workspace/all-target Clippy, formatting and diff checks pass.
Legacy style has zero violations across 8,569 functions; reviewed patterns has
zero candidates, and unfiltered patterns retains exactly the same 15 IDs as C11.
Logs: `/tmp/clones-migration-c12-*` (final full suite in `tests-final.log`).
The retained live capture uses `--locked` and compact output, then strictly replays
full output with original source/manifest deleted and external tools disabled.
Three plans and one overlap group, their witnesses and matching are identical;
explicit helper-finding failure exits one in both cases:
`/tmp/clones-migration-c12-live-7oughwkv/results.json`.
[Planning scope and usage](clones-edit-planning.md) document the output indices,
intent and limitations. This is a bounded initial C12 slice; local guard intents,
caller-aware parameter plans, full replay handle-reordering/tamper cases and
legacy group retirement remain pending. Linux aarch64 only; macOS and exact Go
comparison remain unverified. No suppressions, ignores, thresholds, strict
admission or extraction permissions were changed.

C11c below advances caller-visible scalar mutation through checked original local
leaves. General effects/density and the broader migration remain incomplete.

## C11c: caller-visible scalar leaf mutation effects

The opt-in `--mutation-effects` consumer preserves original direct assignment
metrics while adding distinct call events and exact caller-formal mappings to
original callee assignment witnesses. Owned scalar rebinding stays local. Complete
bounded signatures and bodies, exact selected-unit definition identity, original
source ownership, native argument rosters and binding provenance are prerequisites.
A small separate compiler observation supplies direct mutable builtin-u32 formal
types; rendered names and source spelling supply no referent authority. This is
observational evidence, with strict graph admission kept independent.

One analysis-local session borrows each immutable owning unit, checks each original
owner once and reuses completed leaf outcomes. Assessment and leaf queries share
one typed cache lifecycle; errors and unobserved budget outcomes never publish a
completed result. Each original call occurrence remains separate. Visible-site
budgets are checked before mapping allocation; no graph or source snapshots are
cloned and no new hashes are introduced. Unknown callbacks, external/indirect or
nonleaf calls, Drop-bearing signatures, aliases, opaque argument evaluation and
reference coercions prevent whole-function clean conclusions. Independent known
write witnesses survive unknown remainder. This is a depth-one scalar mutation
axis, without purity, general effects, density, rewrite safety or legacy retirement.

The [consumer contract](clones-mutation-effects.md) describes report indices,
coverage, limits and replay. All 2,553 nextest tests pass (five skipped), 169.358
seconds, plus doc tests, workspace/all-target Clippy, formatting and diff checks.
Both original NonZero regressions pass. Legacy style reports zero violations
across 8,644 functions; reviewed patterns has zero candidates, and unfiltered
patterns retains exactly the same 15 IDs as C12a. Logs are retained under
`/tmp/clones-migration-c11c-*`, with final gates named `*-final.*`.
The final locked compact live capture and full strict replay retain identical
effects and matching after removing original source/manifest files and disabling
external compiler tools: `/tmp/clones-migration-c11c-live-3c4ccjnm/results.json`.
Two call occurrences reuse one leaf summary, with two owner observations and two
mapped write witnesses. No suppressions, ignores, thresholds, strict admission
or extraction permissions were changed.

The C10b frontier below is a witnessed local two-method capability set from
parallel wrapper calls, reusing the existing checked source/method index. Exact
existing-trait relations must suppress redundant advice; names/signatures alone
cannot establish a proposal. Broader trait/capability proposals, richer effect
families, caller-aware edit intents and retirement gates remain open. Native macOS
and exact Go comparison remain unverified.

## C10b: witnessed local capability sets

The opt-in `--capability-proposals` consumer joins two original free wrappers
that call two distinct zero-argument shared-self/u32 methods on their sole shared
nominal formal. Original tuple order supplies operation roles; no names-only
signature mining or behavioral equivalence is inferred. Method bodies can remain
opaque because the proposal concerns interface usage. Exact original trait/member
relations suppress redundant advice, while mixed/partial/different trait coverage
stays explicitly unavailable.

Both abstraction consumers share one original declaration index per selected unit,
including ownership, context, function/header checks and existing closed local
method resolution. Borrowed source/module handles distinguish namesakes without
new hashes, source clones or compiler observations. Owner and distinct-owner pair
budgets are separate from strict matching, with population preflight before pair
allocation. Findings can coexist with incomplete scope. Default, full/compact,
failure behavior and source/tool-free replay have independent acceptance cases.
The [consumer contract](clones-capability-proposals.md) records exact scope and
report joins. Shared function-scope accounting also replaces duplicated mutation
effect accounting, with one borrowed status fold and unchanged completion rules.

All 2,578 nextest tests pass (five skipped), 174.521 seconds, plus doc tests,
workspace/all-target Clippy, formatting and diff checks. Both original NonZero
regressions pass. Legacy style has zero violations across 8,696 functions;
reviewed patterns has zero candidates and unfiltered patterns retains exactly
the same 15 IDs as C11c. No suppressions, ignores, thresholds or strict admission
permissions were changed. Final logs: `/tmp/clones-migration-c10b-*-final.*`, with
Clippy at `/tmp/clones-migration-c10b-clippy.log`.

A locked compact live capture requests both capability proposals and enum
dispatch using their shared index. Full strict replay retains identical capability,
dispatch and matching reports after removing original source/manifest files and
disabling external tools. The single finding exits one under explicit
`--fail-on-findings` in both runs:
`/tmp/clones-migration-c10b-live-zma97vjk/results.json`.
Linux aarch64 only; native macOS and exact Go comparison remain unverified.
Broader signatures, generic/helper rewrites and retirement remain open. The next
ready review frontier is versioned dismissal for dispatch/capability findings,
using original declaration/context/ordered-role identity rather than run-local
owner or function handles.


## C09b: source-local abstraction reviews

The shared `--review-file` workflow now covers completed enum-dispatch and
ordered capability proposals. Explicit saved-run selectors retain a reason;
ordinary lookup never writes or prunes. Helper-only schema-1 files retain their
wire shape, and adding a source-local entry upgrades the envelope to schema 2
without changing helper entries. Local family payloads retain exact original
qualified declarations and consumer-relevant build context, with ordered
variant/member or wrapper-method relations. Matching policy is excluded because
these source consumers are independent of strict graph admission.

Source identity reuses the existing immutable checked declaration index. Module
qualification and declaration ranges are indexed once; canonical original tokens
normalize caller formal/match bindings and preserve selected method headers and
full nominal declarations. Source/graph snapshots are not cloned and no new
hashes are introduced. Opaque callee-body changes remain outside this advisory
identity; changing role order, selected members, declarations or effective context
requires fresh review. Endpoint conversion is cached per report-local wrapper,
then minimal persisted values are materialized for each exact pair. Both review
families share borrowed ordered locator/context staleness comparison and generic
review accounting; exact comparisons allocate no temporary member-key collections.

The original consumer counts remain unfiltered. A separate `abstraction_review`
report records per-family visible counts and per-finding status. Finding exits use
visible counts, while incomplete source or review scope remains exit two even
when every known finding is hidden. All fallible consumer/location construction
precedes review-file publication, and a failed review selector or snapshot cannot
write a partial multi-family decision. Existing-trait suppression remains a
separate detector outcome, not a selectable user dismissal.

Independent labels in `eval/clones-capability/abstraction-review-cases.json` and
ten real-Cargo tests cover both families, source/tool-free full/compact replay,
read-only lookup, relevant staleness, matching-policy independence, inline-module
isolation, incomplete hidden scope, invalid writes and helper v1 preservation.
Integrated validation passes: 2,598 nextest tests, five skipped, in 171.199
seconds, plus doc tests. Workspace/all-target Clippy with warnings denied,
formatting and diff checks pass. Legacy style reports zero violations across
8,765 functions; reviewed patterns reports zero candidates, with the same 15
unfiltered candidate IDs as the previous checkpoint. No ignores or thresholds
changed. Logs are `/tmp/clones-migration-source-review-tests-final.log` and
`/tmp/clones-migration-source-review-clippy.log`, with final style/pattern
artifacts under the same prefix.

Retained live evidence at
`/tmp/clones-migration-source-review-live-jy5bpv8m/results.json` confirms both
findings can be dismissed and replayed in full/compact form without original
source, manifest or tools. Matching and consumer reports agree, and lookup
leaves the review file unchanged.

The next review frontier is explicit whole-store status for retained entries,
with an honest distinction between absent findings, omitted scope and changed
source/context. Broader signatures, caller-aware plans, richer effects, legacy
compatibility and retirement remain open. Native macOS and exact Go comparison
remain unverified.

## C09c: retained review status

State: delivered bounded whole-store status; commit: this checkpoint. Lead owns
read-only whole-store status,
locator/context adapters, CLI/report integration and gates. Acceptance owns
source-authored CLI cases; contract review owns status accounting and independent
context/ambiguity tests. The C09b dismissal checkpoint is `78cec55`.

Explicit `--review-status --review-file` reports one row for every retained helper,
dispatch and capability review without writing, pruning or migrating stores.
Current findings retain existing exact/growth/stale behavior. Unmatched reviews
require uniquely observed endpoints, exact retained contexts and a completed
negative function/pair assessment before `no_finding`. Omitted families, missing
endpoints, unsupported evidence, version changes and exhausted budgets cannot
become false clean negatives. A missing endpoint does not prove deletion.

Source status reuses the checked declaration index; helper status separates
original locator/context observation from positive expression conversion.
Unresolved entries share lazy bounded indexes; already exact/stale entries avoid
that extra work. Reports retain exact values and borrowed proposal comparisons,
without new hashes or cloned source/graphs. Status is opt-in; its report field is absent when unrequested, and existing
review reports and exits stay unchanged. Explicit incomplete status exits two. Original
consumer reports and visible-finding accounting remain separate.

Integrated validation passes: 2,621 nextest tests, five skipped, in 172.900
seconds, plus doc tests; workspace/all-target Clippy with warnings denied,
formatting and diff checks pass. Style is clean across 8,791 functions. Reviewed
patterns has zero candidates; the unfiltered 15 IDs exactly match C09b, without
new ignores or thresholds. Logs/artifacts use
`/tmp/clones-review-status-{tests,clippy,style,patterns}-final.*`; independent
unit/CLI development logs use the same prefix.

Retained live evidence at `/tmp/clones-review-status-live-vf_u0k83/results.json`
confirms exact status for both source families in full/compact strict replay
without source, manifest or tools; matching/consumer reports and review-file
bytes stay unchanged. Eleven CLI cases additionally cover known negatives,
existing-trait suppression, omitted families/endpoints, old identity versions,
unsupported helpers, context changes, budgets and schema-1 preservation. Twelve
unit tests cover status accounting, context selection, ambiguity, member order
and truncated-index uncertainty. No extraction permissions changed.

Next review frontier is explicit pruning with scope-aware retention and legacy
ignore/suppression compatibility; broader
signature consumers, caller-aware plans, effects and selective graph analyses
remain open. No legacy capability is retired. Native macOS and exact Go comparison
remain unverified.

## Hashing exceptions

New consumers use borrowed evidence and report-local typed handles, without new
hashes. Existing `source_identity::Scheme::LegacySha256` and
`concrete_types::digest` support actual historical source claims and frozen
source/oracle pins; immutable owners compute them only for those explicit claims.
The `clones::digest` helper is test-only historical test-contract compatibility,
not a new finding identity. New routine source publication uses existing XXH3.
This bounded inventory does not claim every historical fixture has migrated.


## C09d: selective review pruning

Explicit `--review-prune` requires a saved run and review file, implies retained
entry status, and removes only completed `no_finding` assessments. Preview with
`--review-prune-dry-run`. Stale, changed-context, omitted and unavailable entries
remain; incomplete coverage retains exit two. Capture and handoff failures block
publication. Status inventories are checked against the complete original store
before mutation. Dry runs and no-op applies preserve bytes and never create files;
nonempty applies use the existing atomic save and retain schema and entry order.

Eight pure contract tests and eight CLI cases cover mixed coverage, original
identity, preview/apply/idempotence, source-free replay and publication safety.
The full suite passed 2,637 tests (five skipped) in 174.231 seconds, plus doc
tests. After the final validation/mutation separation and test-helper cleanup,
all 16 affected tests and doc tests passed again. Workspace Clippy with warnings
denied, formatting and diff checks pass. Legacy style reports zero violations
across 8,801 functions; reviewed patterns reports no candidates, and the same 15
unfiltered candidate IDs remain. Logs are retained at
`/tmp/clones-review-prune-{tests,focused,clippy,style,patterns}-final.*`.
Native macOS remains for CI. Legacy ignore/suppression compatibility and the
broader capability migration remain open.


## C04b: captured source-rule suppression compatibility

The opt-in length/parameter consumer now honors named legacy `rstyle-allow`
comments from the captured original source. The existing parser supplies valid
rules and invalid diagnostics; physical association retains the legacy 40-line,
single-line attribute/doc and blank-line semantics. Original measurements remain
in raw findings. Visible/suppressed counts control text and finding exits without
hiding guard or other consumers. Missing suppression observations keep coverage
incomplete. The evidence view retains shared source ranges without new hashes,
file reads, graph copies or a second parse. Saved-run replay follows the original
annotation even after the live source is changed or removed.

Evidence owns the `consumer_facts` view and line-selection tests; policy owns the
narrow parser adapter; acceptance owns real-Cargo/source-free replay cases; the
lead owns report/exit integration, docs and commits. Five evidence tests, six
policy tests and six real-Cargo acceptance cases cover original ownership,
association boundaries, valid/invalid rules, methods, same-named owners, guard
isolation and source/tool-free replay. The comparator retains raw measurements,
checks independently labeled suppression permissions and compares visible
outcomes; its 17 tests plus three runner tests pass.

The full workspace suite passed 2,654 tests (five skipped) in 174.315 seconds,
plus doc tests. Final source-rule scope-count helper extraction and spacing
cleanup are followed by focused evidence/consumer/CLI and doc-test revalidation.
Clippy, formatting and diff checks pass. Legacy style reports zero violations
across 8,811 functions; reviewed patterns has no candidates and unfiltered IDs
remain exactly the same 15. Logs use
`/tmp/clones-source-suppressions-{tests,focused,clippy,style,patterns}-final.*`.

Retained live evidence at
`/tmp/clones-source-suppressions-live-n4k6yn__/results.json` records equivalent
legacy visible diagnostics and new raw/visible counts, plus identical full/compact
strict replay without source, manifest or tools. Native macOS and exact Go
comparison remain unverified. Exclusions, legacy ignore/review migration and
broader replacement gates remain open. No legacy backend is retired.


## C04c / C05b: workspace exclusions and independent policy comparison

`b65f15c` applies configured workspace-relative path globs to the opt-in source
rules through the exact existing legacy matcher. Patterns compile once before
capture; default `target/**` and explicit empty lists preserve existing policy.
Only checked original source ownership can grant an exclusion. Original paths
outside the workspace remain evaluated; missing ownership or invalid/dot paths
remain unavailable. Matching is lexical and uses the original captured root,
without new file reads, hashes, graph copies or source parses.

Excluded owners stay in the roster with complete, not-applicable numeric skips
and explicit exclusion counters, separate from unavailable rules. Suppression
parsing and diagnostics are skipped for excluded source rules. Other consumers
keep their own findings/coverage, and exclusions cannot bypass the function budget.
Four shared matcher tests, six source decision cases and seven real-Cargo CLI
cases cover glob boundaries, namesakes, external sources, invalid configuration,
independent guards, budgets, full/compact reports and source/tool-free replay.
The initial 2,671-test run had one new fixture failure: its `../target` path was
intentionally unavailable under the path contract. Using an absolute original
path corrected the fixture; all 17 exclusion tests and doc tests then passed.
Clippy, formatting and diff checks passed, style reported zero violations across
8,821 functions, and reviewed patterns passed with the same 15 unfiltered IDs.
Logs are `/tmp/clones-source-exclusions-*-final.*`.

C05b adds an independent boolean exclusion label and an explicit legacy per-owner
operator declaration. A valid labeled skip is `policy-excluded-agreement`, never
measured clean. Missing/unavailable/malformed evidence remains uncovered; known
policy mismatches are regressions. Complete unique rosters, exact skip fields,
configured limits and absent legacy numeric diagnostics are required. Historical
absent exclusion fields mean included. Aggregate counters grant no permission.
The frozen corpus adds `excluded::over_both_limits`, independently specified as
six code lines/five parameters before exclusion. The runner validates its fixed
policy and retains all 17 legacy owners while reconciling the included summary.
It interprets legacy relative paths against the declared original root, preserving
raw locations and avoiding a second glob implementation.

Thirty-eight Python tests pass. Live serial captures at
`/tmp/clones-exclusion-comparison-live-final` yield ten measured agreements and
two policy-excluded agreements; new scope is complete with 17 assessed owners,
one excluded owner, two excluded rules and no unavailable source evidence.
The first comparison checkpoint run found two guard-fixture failures because
its temporary workspace copied the corpus root but omitted the new module.
The copy helper now includes both source files; all five guard tests pass.
The final integrated snapshot passed all 2,671 workspace tests (five skipped) in
174.606 seconds plus doc tests, 38 Python tests, Clippy, formatting and diff checks.
Style reports zero violations across 8,821 functions; reviewed patterns passed.
Logs use `/tmp/clones-exclusion-comparison-{tests,clippy,style,patterns}-final.*`.
Unfiltered patterns retain exactly the same 15 candidate IDs as `b65f15c`. Native
macOS and exact Go comparison remain unverified for these changes. Legacy review
migration, broader source/effects/planning scope and capability retirement remain
open; neither legacy backend is retired.


Next ready source-rule work is spacing evidence: derive an independently checked
scalar direct-statement count during the existing owned source parse, cross-check
original body shape, and retain unsupported macro/cfg/async transformations. Legacy
spacing counts compiler HIR statements, so a raw source AST count alone does not
establish parity. Reuse the existing blank-line policy; excluded/suppressed,
disabled, threshold and incomplete cases need separate acceptance before delivery.
Broader review migration and planning/effect capabilities remain parallel work.


## C04d: checked direct statements and opt-in spacing

An explicit `[fn_spacing]` source-policy section requests spacing; its default
minimum is five statements. Omitted spacing remains unrequested. Zero disables
the rule with an explicit complete, not-applicable clean assessment and no
measured statement count. The shared `rstyle_core::spacing::is_finding` predicate
also serves legacy style: it counts direct phases rather than lines and accepts
any original blank line, including whitespace-only and signature blank lines.
The assessment retains the count and minimum-minus-one limit even when a blank
line makes it clean. This is advisory formatting policy, not a body-effect proof.

Evidence derives the scalar count during the existing verified source parse,
checks the original block and ordered direct statement kinds/ranges/snippets,
and counts the tail expression. Missing/tampered rosters remain unknown. Nested
expressions/closures/initializers do not add direct statements and need not pass
strict graph admission. Async, direct macros, attributed or empty/verbatim direct
statements remain unsupported. No new extraction permission, hash, file read,
source/graph copy or second parse is introduced. One existing indexed position
table gives direct spans without repeated source-prefix scans. Ordinary
`inspect` avoids this work; `inspect_with_statements` is selected only for enabled
spacing. Existing length/arity and guard queries keep their previous prerequisites.

Acceptance covers threshold four/five, tails, blank lines, nested opaque work,
multiline initializers, unsupported macros/cfg/async, disabled policy, suppression
isolation, exclusion of all requested rows, budget incompleteness, full/compact
reports and source/tool-free strict replay with a changed threshold. Pure policy
cases cover the inclusive boundary and legacy blank-line semantics. Evidence
cases include missing/changed/duplicated children, ownership, UTF-8/CRLF and a
1,001-statement roster. The focused run passed 62 tests plus doc tests; 38 existing
Python comparison tests and Clippy pass. The final full workspace run passed
2,689 tests (five skipped) in 181.849 seconds plus doc tests. Formatting and diff
checks pass. Style reports zero violations across 8,830 functions; reviewed
patterns passed with exactly the same 15 unfiltered candidate IDs as `6958b92`.
Logs use `/tmp/clones-spacing-{focused,tests,style,patterns}-final.*`
and `/tmp/clones-spacing-clippy.log`. Retained live evidence at
`/tmp/clones-spacing-live-h4wt51ce/results.json` verifies finding/clean/incomplete
outcomes, identical full/compact source-free strict replay and config-only
disabling with complete clean policy results.
The comparison adapter continues to score length/parameters only; its denominator
has not silently expanded to include spacing.

Remaining work includes macro/cfg/async statement transformations, independently
reviewed project-scale spacing quality and replacement gates, legacy review
migration, broader abstraction signatures, effects and caller-aware planning.
Native macOS and exact Go behavior remain unverified for this slice. No legacy
backend is retired.


Next ready work is independent spacing comparison labels/live legacy comparison
for the supported statement scope, followed by the next caller-aware planning
intent. Macro/cfg/async expansion handling requires its own evidence work rather
than treating source AST length as a compiler statement count.


## C05c: independent spacing comparison

The comparator scores spacing only for explicit independent labels, retaining
historical length/arity-only denominators. Six frozen examples separate direct
counts from physical lines, nested work and blank-line policy. Suppression checks
raw measurements before comparing visible diagnostics. Exclusion skips all three
requested rules. Zero minimum requires complete null-count/not-applicable policy
results and complete clean legacy scope; `policy-disabled-agreement` is separate
from measured agreement. Malformed numeric values (including booleans), missing
policy/counts, duplicate owners/rules, unsupported evidence and exhausted work
remain uncovered. The serial runner freezes spacing policy and snapshots the
additional source module; guard acceptance copies the same module.

All 56 independent Python tests pass. A fresh serial legacy/new capture at
`/tmp/clones-spacing-comparison-live-20261028` reports 28 measured agreements and
three policy-excluded agreements. Six spacing measurements agree, including a
raw suppressed finding and a high-count blank-line clean case. Artifacts retain
the dirty fixture snapshot and exact prebuilt executable metadata. This provides
bounded native Linux comparison, not project-scale quality or exact Go parity.

The integrated checkpoint passes all 2,689 workspace tests (five skipped) in
175.707 seconds plus doc tests; workspace/all-target Clippy, formatting and diff
checks pass. Legacy style reports zero violations across 8,830 functions and
reviewed patterns passes. All 15 unfiltered candidate IDs are unchanged from
`0e7234e`. Logs use `/tmp/clones-spacing-comparison-{tests,clippy,style,patterns}.*`
and `/tmp/clones-spacing-comparison-patterns-unfiltered.*`.

Caller-aware parameter intents are the next ready planning work: retain witnessed exact call
regions while reporting caller census completeness separately; current direct
call observations cannot establish absence of callers hidden by opaque work.


## C12b: caller-aware parameter signature intents

An explicit `--source-rules --parameter-plans` request derives advisory signature
intents from the existing visible `fn_params` assessments. Source rules are
evaluated once; their checked exclusion/suppression decisions and unavailable
population are reused. No strict pair or effect proof is required. Each intent
retains its exact original signature region plus witnessed same-unit call
regions and original argument/formal-root mappings. Formal provenance does not
prove unchanged forwarding, pass-through, bundling or safe argument removal.
The report separately declares captured selected-unit scope, same-unit target
resolution and per-unit caller census coverage. Unknown external/function-value
uses, replacement design and stack order remain explicit.

The caller session scans each bounded original owner once, indexes exact selected
definition targets, validates native call/argument order and path/context joins,
and borrows retained observations for target queries. Unsupported/nested argument
evaluation and unexamined macro/control/closure/indirect/associated work preserve
known witnesses while making the census incomplete. Excluded source owners still
contribute callers. No source/graph byte copies, routine hashes or repeated target
query parsers are introduced. Region resolution caches one signature/body owner
per function; both share the immutable source census. The overlap consumer now
accepts an explicit required-region count, validates every region within an
existing comparison allowance and retains the original two-region helper
contract. Disjoint expressions in the same caller do not imply overlapping edits.

Independent acceptance covers exact two-callee/multiple-call regions, reordered
formal roots, namesakes, no finding/opt-in/CLI requirements, suppression/exclusion,
excluded callers, useful partial witnesses, unavailable source, budget and
source/tool-free full/compact strict replay. Caller tests cover cached outcomes,
source/signature gaps, boundary ownership, exact context/path contradictions and
budget exhaustion; signature tests cover UTF-8/CRLF, visibility, where clauses,
methods and opaque bodies. Broader all-use/pass-through inventories, forwarding
bundles, external closure, generic/nominal call vocabularies, local guard intents
and project-scale quality/retirement remain open. Native macOS and exact Go
comparison are unverified for this increment.

The final frozen implementation passes all 2,709 workspace tests (five skipped)
in 176.940 seconds plus doc tests. Workspace/all-target Clippy, formatting and
diff checks pass. Legacy style reports zero violations across 8,877 functions;
reviewed patterns passes and all 15 unfiltered baseline candidate IDs are
unchanged. All 56 independent Python comparison tests pass. Logs and reports use
`/tmp/clones-parameter-{tests,clippy,style,patterns,python}-final.*` and
`/tmp/clones-parameter-patterns-unfiltered-final.*`.

A fresh locked compact capture at `/tmp/clones-parameter-live-6yp1jifu` retains
one exact signature and two witnessed callers. After removing original source
and manifest and disabling compiler/tool lookup, full and compact strict replay
preserve parameter planning and matching exactly. A zero function budget stays
explicitly incomplete with a null expected-plan population; config-only threshold
reevaluation produces a complete empty intent set. Commands, exit codes, timings
and reports are retained in that directory. No extraction permission, new hash,
ignore entry or policy threshold was added to make the checkpoint pass.

The next bounded planning slice is per-target witnessed forwarding bundles.
Formal-root provenance alone cannot establish unchanged values: a mutable formal
may have been assigned before the call. Initial bundle witnesses therefore need
checked immutable owned scalar reads, distinct formal roots and target positions,
with repeated exact call occurrences retained. Incomplete census may preserve
witnesses but cannot establish exhaustive support. Pass-through candidates need
a separate complete formal-use inventory, including other reads, writes, borrows
and opaque boundaries; neither a formal ordinal nor a bundle establishes safe
argument removal.


## C12c: witnessed unchanged-scalar forwarding bundles

Parameter signature plans now retain repeated forwarding candidates for the same
exact target. Checked `unchanged_formal` evidence is separate from source-root
`formal` provenance. Only immutable owned-u32 direct reads qualify; literals,
local aliases, mutable and borrowed formals do not. Missing/unsupported evaluation
retains its availability. Original mutability, binding identity, native argument
order and scalar type must agree. The existing source scan supplies this evidence
without another parse, graph/effect proof, hash or source-byte copy.

The candidate search groups unique eligible target-position sets, considers their
pairwise intersections, and checks support across every retained call. Each
published candidate has at least two exact source call occurrences, ordered
caller-to-target formal mappings and distinct caller roots. Repeated-root
positions are excluded. Candidate vocabulary and unproved maximality are explicit;
this does not enumerate all subsets or promise a parameter-struct rewrite.
One 65,536-operation search allowance is shared across all signature plans.
Exhaustion preserves only candidates with completed support scans and remains
incomplete; partial support counts never become completed findings.

Seven real-Cargo cases independently cover triples, permutations, shared subsets,
exact namesakes, repeated roots, literals, aliases, transformed arguments,
mutable/borrowed formals, useful incomplete-census witnesses and source-free
full/compact replay. Four evidence tests distinguish immutable root evidence,
resealed mutability/native contradictions, absent/unsupported data and known-empty
literal/local evidence. Four pure search tests cover positions, support, excluded
repeated roots and support-scan exhaustion. Broader all-use/pass-through evidence,
external closure, broader signatures, project-scale quality and legacy retirement
remain open. Native macOS and exact Go comparison remain unverified.


The frozen checkpoint passes all 2,724 workspace tests (five skipped) in 177.007
seconds plus doc tests. Workspace/all-target Clippy with warnings denied,
formatting and diff checks pass. Legacy style reports zero violations across
8,892 functions; reviewed patterns passes and the same 15 unfiltered baseline
candidate IDs remain. All 56 Python comparator tests pass. Retained logs use
`/tmp/clones-bundles-{tests,clippy,style,patterns,python}.*`; unfiltered pattern
reports use `/tmp/clones-bundles-patterns-unfiltered.*`.

Fresh locked live evidence is retained at `/tmp/clones-bundles-live-nvtm8gyk`.
Two permuted original calls produce one bundle with independently checked target
positions and caller mappings. After removing source/manifest and disabling
Cargo/compiler/driver lookup, strict full/compact replay preserves planning and
matching exactly. Zero function budget remains incomplete; config-only threshold
reevaluation produces a complete empty plan set. Exact commands, timings, exit
codes and reports are retained. No extraction permission, hash, ignore,
threshold relaxation or legacy retirement was introduced.

The next ready slice is a separate, bounded formal-use inventory in the existing
source scan. Native path resolution must be checked for ordinary reads as well
as call arguments. Initializers, shadowing, return/tail expressions, writes,
borrows and opaque syntax need explicit occurrence or incompleteness evidence.
Only complete inventories with at least one use, all directly forwarded unchanged
into selected signature intents, can justify an advisory pass-through candidate;
unused parameters and omitted syntax cannot supply that conclusion.


## C12d: original formal-use inventories

The original call/mutation source scan optionally records owned-u32 formal
bindings and exact native occurrences. Reads, assignments, compound assignments,
shared/mutable borrows and call arguments remain distinct. Initializers precede
shadow declarations, local aliases remain separate roots, and wildcard discard
does not hide initializer evaluation. Existing mutation/effect assessments keep
their independent contracts. Unsupported syntax, missing binding/child evidence,
opaque work and budgets cannot become complete zero-use inventories.

Parameter planning retains each owner inventory once per defining unit. Complete
immutable formals with nonzero uses, all forwarded unchanged into selected visible
signature intents, receive advisory pass-through candidates. Complete empty
inventories are unused. Other-use witnesses and unavailable target resolution
remain distinct. One global 65,536-entry retention allowance covers declarations,
occurrences and boundaries across relevant units. A separate shared 65,536-row
allowance bounds report placeholders as well as observed formal rows; truncation
preserves expected counts and reports budget exhaustion. Scoped native IDs and ordered
indexes avoid repeated source parsing, source/graph copying and new hashing.

Six original-evidence tests cover declaration/use roles, unchanged mutation
assessments, repeated and zero uses, call positions, aliases, return/borrow native
children, absent evidence, duplicate native nodes and syntax budgets. A caller
session regression bounds retention across owners. Six classifier/quota tests
separate unknown targets from counterexamples and bound report rows. Six real-Cargo
cases cover forwarding, other reads/writes, borrowed/opaque boundaries, exact
UTF-8/CRLF spans, owner isolation and source-free full/compact replay.

The frozen snapshot passes all 2,743 workspace tests (five skipped) in 177.206
seconds plus doc tests. Workspace/all-target Clippy with warnings denied,
formatting and diff checks pass. Legacy style reports zero violations across
8,953 functions; reviewed patterns passes and the same 15 unfiltered baseline
candidate IDs remain. All 56 independent Python comparison tests pass. Logs and
reports use `/tmp/clones-uses-{tests,clippy,style,patterns,python}.*` and
`/tmp/clones-uses-patterns-unfiltered.*`.

Fresh locked live evidence is retained at `/tmp/clones-uses-live-8cgsp2u8`.
Two permuted calls retain complete formal-use inventories and advisory forwarding
outcomes. After source/manifest removal and disabling build-tool lookup, full and
compact strict replay preserve planning and matching exactly. Zero function
budget remains incomplete; config-only threshold reevaluation returns a complete
empty plan set. Commands, timings, stderr, reports and exit statuses are retained.
No extraction permission, hash, ignore entry, threshold relaxation or legacy
retirement was introduced. Broader formal families, safe argument removal,
external caller closure, local guard intents and cross-family edit coordination
remain open. Native macOS and exact Go comparison remain unverified.


## C04e: bounded header and optional body parsing

The shared source-rule entrance now avoids parsing unrequested bodies. After
immutable ownership checks, ordinary inspection borrows the original header
through the captured body opener, parses attributes/visibility/signature once,
and retains the existing exact signature roster/span/receiver checks. Native
range, brace, UTF-8 and complete function extent contradictions fail before
usable header facts. Unsupported body kinds/children do not erase independent
signature evidence, including native async captures.

Requested direct statements use a single full-function parse only inside the
existing conservative lexical budget (256 KiB, depth 128, punctuation 128, word
starts 128). Otherwise the bounded header supplies available facts, statements
are unknown and typed inspection execution is BudgetExhausted. Overbudget
headers retain owned source but leave signature/comment syntax unknown.
Unrequested statement facts remain NotApplicable. Guards use the same lexical
bound before parsing, covering delimiter-free unary/return chains as well as deep
and long flat bodies. No synthetic parser suffix, body copy, new hash, graph
proof, permission or larger limit was introduced.

Independent library cases cover recursive bodies constructed without parsing,
large bodies/small headers, unsupported roots, header exhaustion, ownership
precedence, forged return/where boundaries, visibility, receiver, UTF-8 and CRLF.
The 1,001-statement roster regression still exercises the lower-level native
roster validator directly; entrance budget behavior has separate acceptance.
Real locked Cargo captures preserve a parameter finding while spacing/guards
exhaust, then source/tool-free full and compact strict replay preserve analysis
and matching. A native async/header-only case checks source-rule independence.

The frozen integrated snapshot passes all 2,752 nextest tests (five skipped) in
178.929 seconds, doc tests, workspace/all-targets locked Clippy with warnings
denied, and all 56 Python comparison tests. Both originally reported NonZero
regressions pass. Logs are retained under `/tmp/clones-header-{tests,clippy,python}`.
Legacy style passes with zero violations across 8,963 functions after one
formatting-only phase separator; reviewed patterns passes and unfiltered patterns
retain the same 15 exact IDs as C12d. Format/diff checks pass; retained logs and
JSON use `/tmp/clones-header-{style,patterns}`. Guard edit intents and shared
cross-family physical edit coordination remain the next ready planning slice;
whole-plan independence, arbitrary propagation/removal, broader signatures,
macOS, exact Go comparison and legacy retirement remain unverified/open.


## C12e: guard regions and cross-family edit coordination

The lead owns CLI/report wiring, shared family adapters and documentation;
leaf workers own guard/source-region evidence, union coordination and independent
acceptance. The frozen slice adds opt-in `--guard-clauses --guard-plans` with one
required whole terminal-if region per completed advisory finding. The span comes
from the existing bounded parse; the library maps the body-contained original
subrange to its checked immutable census snapshot without another parse, source
copy, graph proof or hash. Contradictory native body/snippet/ranges and corrupt
ownership fail before a usable region is returned. Guard finding populations
remain independent of source-rule suppressions and path exclusions.

`edit_coordination` automatically joins requested helper/parameter/guard intents.
It borrows their existing footprints, uses deterministic family-plus-local
references and dense union IDs, then reuses the existing bounded overlap engine.
A single union retention cap and relation/region/snapshot budgets apply across
families. Missing regions, conflicting snapshots, incomplete populations, failed
execution and exhausted limits remain visible. Exact known-region witnesses can
coexist with incomplete caller scope. Expected counts remain unknown for
incomplete families. Whole-plan independence stays unknown and stack order is
not proved; no rewrite or argument-removal permission is introduced.

The necessary caller prerequisite retains exact original native call witnesses
inside bounded plain conditionals. Condition/branch scans explicitly preserve an
opaque control/effect boundary and incomplete formal-use coverage; lexical roots
are invalidated before/after each piece and between branches. Binding/opaque or
attributed conditions remain skipped. Mutation-only inspection is unchanged.
This enables a real call-inside-tail-if overlap witness while retaining unknown
caller coverage, rather than omitting the call or certifying conditional effects.

Independent tests cover original UTF-8/CRLF terminal-if offsets and shared owners,
corrupt/missing/expanded evidence, nonempty containment, conditional source/native
joins and root invalidation, equal family-local IDs, union truncation, snapshot
conflicts and shared comparison budgets. Real locked Cargo cases distinguish
calls before/inside guards, own-signature disjointness, complete-empty versus
opaque/budget-incomplete planning and source-policy independence. Combined
full/compact strict replay removes source files and tools and preserves all four
planning reports. Helper/guard overlap is tested at the pure union boundary:
the current bounded expression-helper vocabulary cannot itself contain a guard.

Focused verification passes 377 tests plus doc tests. The frozen integrated
snapshot passes all 2,779 nextest tests (five skipped) in 179.514 seconds, doc
tests, workspace/all-targets locked Clippy with warnings denied, zero style
violations across 9,003 functions, and all 56 Python comparison tests. Reviewed patterns passes after sharing the consumer population-completeness
helper; unfiltered patterns retains exactly the same 15 baseline IDs as C04e.
Format/diff checks pass. Retained logs use `/tmp/clones-guard-plans-*`.
External caller closure, broader formal/signature families, replacement design,
lifetimes/drops, safe removal/propagation, exact Go/macOS comparison and legacy
retirement remain open. No new extraction permission, ignore, threshold change,
cryptographic hash or budget increase was introduced.


## C12f: reuse completed snapshot equality

Production owns `planning/overlap.rs`, independent acceptance owns
`planning/snapshot_reuse_tests.rs`, and the lead owns registration and docs.
The overlap ledger now retains successful exact equality for borrowed immutable
slices, identified by pointer and length, separately for each physical resource
and analysis. The first snapshot remains the canonical borrowed bytes. Repeated
regions reuse completed equality only while the resource is known; later conflict
or exhaustion remains absorbing and invalidates all affected plans. Region visits
bound retained identities. No source copies, hashes, serialized identities,
schema changes or increased budgets are introduced.

Eight independent model-level tests cover repeated equal owners, third owners,
resource and session isolation, late conflicts, exhaustion after successful
equality, and offset/length distinctions within one allocation. Twenty uses of a
second 1 MiB owner now charge exactly 1 MiB of byte comparison, with complete
planning; distinct owners retain the existing 16 MiB exhaustion test. This is
measured comparison accounting, not a wall-time speedup claim.

Focused planning/caller/guard verification passes 75 tests plus doc tests,
including locked live capture and full/compact strict replay after removing
source files and compiler tools. The frozen workspace passes all 2,787 nextest
tests (five skipped) in 179.248 seconds, doc tests, locked workspace/all-targets
Clippy with warnings denied, zero style violations across 9,006 functions and
all 56 Python comparison tests. Both original NonZero regressions pass.
Reviewed patterns passes and unfiltered patterns retain exactly the same 15 IDs
as C12e. Formatting and diff checks pass. Retained logs use
`/tmp/clones-snapshot-*`. Native macOS, exact Go comparison, broader planning
coverage and legacy retirement remain open.

## C13: changed-file source-rule presentation and CI findings

Scope owns `analysis/changed_scope.rs` and bounded manifest tests; the source
adapter owns `analysis/changed_file_reports.rs`; independent acceptance owns
`tests/migration_rules/changed_scope.rs`; the lead owns CLI, shared registration,
report integration, docs and embedded skill. `--source-rules --changed-files PATH`
loads a closed `rstyle.changed-files/1` JSON manifest before capture. Input limits
are 1 MiB, 4,096 raw paths and 4,096 UTF-8 bytes per normalized workspace-relative
Rust path. Selection is sorted/deduplicated and uses checked captured paths.
It introduces no Git queries, live source reads, hashes, copied source evidence,
repeated parsing or graph proof.

The report retains all function assessments, suppression/exclusion outcomes,
raw/visible totals and original scope, and adds selected per-path observations,
coverage and visible findings. Text diagnostics and the source-rule finding gate
use the selection. Guards and other consumers retain their own finding gates;
parameter/guard planning populations remain unfiltered. Unknown selected paths
and unclassified owners cannot become clean. Empty or inactive-only files without
captured functions remain explicitly unobserved in this initial contract.
An explicit empty selection is complete-empty, while whole-analysis incompleteness
still exits two. In particular, a generated all-targets test harness leaves the
whole source analysis incomplete even when selected original file owners are all
complete.

Independent cases cover exact same-name files, repeated lib/test owners,
unchanged violations versus selected clean files, selected violations, source
suppression/exclusion, invalid/oversized manifests, missing paths and budgets,
unfiltered guard findings and parameter plans, and full/compact strict replay
after deleting source files and disabling compiler tools. Changing the manifest
reuses one saved capture without altering its assessments. Changed-line inference,
saved finding baselines, broader consumer selections and legacy retirement remain
open.

The frozen checkpoint passes 77 focused tests, including locked live captures
and source/tool-free full/compact strict replay, plus all 2,809 workspace nextest
tests (five skipped) in 180.151 seconds and doc tests. Locked workspace/all-targets
Clippy with warnings denied, zero style violations across 9,024 functions, both
original NonZero regressions, all 56 Python comparison tests, format and diff
checks pass. Reviewed patterns passes; unfiltered patterns retains exactly the
same 15 IDs as C12f. Retained evidence uses `/tmp/clones-changed-*`.
Native macOS, exact Go comparison and per-capability legacy retirement remain
unverified/open.

The next ready review slice is exact saved `fn_params` findings. Existing review
locators and build-context components can be reused, while source-rule evidence
and effective parameter policy need their own versioned contract. Report-local
keys, source checksums and finding counts alone cannot serve as stable review
identity. This remains readiness work, not a delivered baseline capability.


## C09e: exact saved parameter-count reviews

Model owns `review/source_rules.rs`; workflow owns bounded original observations
and retained status in `review/source_workflow/`; acceptance owns
`tests/migration_parameter_review/`; the lead owns completed-signature retention,
shared CLI/report/storage/prune integration, docs and embedded skill. Delivered in this checkpoint after frozen-snapshot validation.

`--source-rules --review-file` now looks up exact parameter-count choices.
Explicit saved-run `--dismiss-parameter` records only detector versions, original
qualified locator, build context, numeric policy, parameter count and receiver
presence, plus a reason. Report handles select the finding but are not persisted.
No hashes, body tokens, source copies, reparsing or graph proofs are added.
Body/binder/layout edits preserve review; parameter count, receiver, numeric
policy and detector changes require review. Lib/test owners remain distinct.
Schema 3 adds optional typed parameter entries and preserves v1/v2 reading without
lookup rewrites or later local-family schema downgrades.

The adapter borrows completed measurements and shared original declaration
indices. Initial durable identities cover uniquely indexed root/inline-module
original free functions; external module files and ambiguous ownership remain
unavailable. Observations and retained parameter entries are bounded to 4,096;
a truncated roster cannot hide or dismiss findings because uniqueness is unknown.
A borrowed locator/target index avoids scanning unrelated stored entries, and
hidden report-local keys use a set rather than repeated per-function decision scans.

Text and finding exits hide exact selected parameter findings only. Raw source
assessments/counts and advisory parameter planning remain unfiltered. Changed-file
selection controls dismissal eligibility and hidden finding subtraction without
removing unselected status observations. Other source rules and guards keep their
gates; incomplete scope still exits two. All fallible consumers finish before
atomic review publication. Suppressed, clean, unavailable, unselected or invalid
selectors leave retained bytes unchanged.

Whole-store status includes parameter entries. Completed negatives can be pruned
only under the retained numeric policy and current detector versions. A raised
threshold remains stale, future versions remain unavailable, and missing owners
remain unobserved rather than deleted. Independent acceptance covers read-only
lookup, minimal storage, strict full/compact source/tool-free replay, raw planning,
recapture stability/staleness, lib/test isolation, changed scope, status and
explicit preview/apply/idempotent pruning. The frozen final snapshot passes all
2,835 workspace nextest tests (five skipped) in 181.624 seconds and doc tests,
including both original NonZero regressions. Locked workspace/all-targets Clippy
with warnings denied, zero style violations across 9,071 functions, all 56 Python
comparison tests, formatting and diff checks pass. Reviewed patterns reports zero
candidates; unfiltered patterns retains exactly the previous 15 IDs. Shared review
outcome presentation and source policy completion avoid duplicating the new
family's formatting and exclusion/suppression logic. Logs use
`/tmp/clones-parameter-review-*`. Broader source-rule review, exact Go
comparison, native macOS and per-capability legacy retirement remain open.


The next ready slice is numeric `fn_length` review: reuse completed code-line
measurements and `max_lines`, share original owner resolution with parameter
reviews, and keep per-family decisions and hidden counts distinct. Proposed
identity retains the measured count, so comment/blank insertions and same-count
body edits preserve review; count changes and changed policy require review.
This is readiness, not delivered behavior. Acceptance must cover same-policy
completed negatives versus changed-policy stale negatives, mixed rule stores,
source/tool-free replay, selected hiding and unfiltered raw planning.


## C09f: numeric function-length review and shared source-owner resolution

Model owns shared typed exact-choice mechanics and the length snapshot; workflow
owns source-rule observations and per-family status/dismissal; acceptance owns
`tests/migration_length_review/`; the lead owns source adapters, CLI/storage/report
seams, pruning, docs and embedded skill. This slice is delivered over its bounded
original free-function scope.

`--source-rules --review-file` assesses numeric function-length choices alongside
parameter choices. An explicit saved-run `--dismiss-length` records detector
versions, checked original locator/build context, `max_lines` and completed
`code_lines`. No body bytes, checksums, report handles or source positions are
stored as identity. Code-line counting is not repeated. Same-count edits and
blank/comment insertions preserve review; changed count or policy requires review.
This deliberately reviews the measured numeric boundary rather than every body
edit. Schema 4 adds optional typed length entries; prior schema shapes remain
readable and read-only lookup/status never upgrade retained bytes.

One bounded original-owner resolution feeds both source-rule families. Each has
separate decisions/hidden key sets/counts and detector policy. Generic exact-choice
storage shares bounded validation and borrowed locator/target lookup without
merging distinct typed snapshots. The existing independent local-abstraction
model remains separate. Both numeric models retain explicit semantic validation.
Root/inline-module original free-function coverage, the shared 4,096-owner bound
and the 4,096-entry bound per numeric family remain unchanged; ambiguous, missing and truncated identities grant no dismissal.

Length review affects selected length diagnostics/finding exits only. Other source
rules and guards, raw counts/assessments and parameter plans retain their meaning.
Same-policy/current-version completed negatives permit explicit pruning; raising
`max_lines` yields stale rather than a prunable negative. Source suppression,
changed-file intersections, lib/test context separation, schema coexistence and
source/tool-free strict full/compact replay have independent acceptance targets.
The frozen final snapshot passes all 2,857 workspace nextest tests (five skipped)
in 183.049 seconds and doc tests, including both original NonZero regressions.
Workspace/all-targets locked Clippy with warnings denied, zero style violations
across 9,110 functions, all 56 Python comparison tests, formatting and diff checks
pass. Reviewed patterns reports zero candidates; unfiltered discovery retains
exactly the previous 15 IDs. The shared completeness gate uses typed family
selection, preserving short-circuit proof checks without indirect callback cost.
Logs use `/tmp/clones-length-review-*`. Native macOS, exact Go comparison and
per-capability legacy retirement remain unverified or open.

The next ready source-review slice is `fn_spacing`, using completed direct-statement
measurements and explicit `min_statements` policy. Omitted spacing must remain
not-requested; disabled or changed policy must not prune retained positives. A
new blank line can establish a completed same-policy negative only through the
existing completed assessment. No additional source scan/hash is needed. Mixed
three-family stores, independent hiding, unsupported transformed source and
source/tool-free replay need independent acceptance before delivery.


## C09g: saved spacing choices from completed statement evidence

Model owns `review/spacing_rules`; workflow owns shared source observations and
per-family status; acceptance owns `tests/migration_spacing_review`; lead owns
configuration/analysis adapters, CLI/storage/report/prune seams and documentation.
This slice is delivered over the existing bounded original free-function scope.

The intended choice retains explicit `min_statements` and completed direct-statement
count, alongside existing versioned locator/context identity. No new source scan,
parser, hash or graph prerequisite is introduced. Omitted policy must stay
not-requested even with no captured functions. A same-policy blank-line negative
can permit explicit pruning; changed or disabled policy must leave retained
positives stale. Three independent source families share original owner resolution.
Mixed stores, selected hiding, source-free strict replay and schemas 1–4 read-only
compatibility pass independent acceptance. Eight model tests and 14 real-Cargo
spacing-review cases cover boundary counts, same-count edits, blank-line negatives,
changed/disabled/omitted policy, empty captures, source/tool-free full/compact/text,
unsupported statements, per-target isolation and three-family/raw-plan independence.
All 195 focused review/source-rule tests passed before final integration.

The frozen final snapshot passes all 2,880 workspace nextest tests (five skipped)
in 185.254 seconds and doc tests, including both original NonZero regressions.
Locked workspace/all-targets Clippy with warnings denied, zero style violations
across 9,136 functions, all 56 Python comparison tests, formatting and diff checks
pass. Reviewed patterns reports zero candidates; unfiltered discovery retains
exactly the previous 15 IDs. Source-review presentation groups only the optional
reports; serialized field names, per-family completeness and hidden counts remain
independent. No new ignores, suppressions, threshold changes or SHA paths.
Logs use `/tmp/clones-spacing-review-*`. Native macOS, exact Go comparison and
per-capability legacy retirement remain unverified or open.

The next dependency-ready quality slice is bounded depth-two scalar mutation
summaries: caller → wrapper → leaf, using existing original calls and scalar roots.
The current nested-helper oracle remains explicitly unknown until implementation.
Nested witnesses must retain original summary/site ownership and exact call paths;
callee sites must never become caller direct assignments. Repeated paths, reordered
formals, direct-plus-nested writes, owned rebinding, recursive/deeper/opaque calls,
context joins, budgets and source/tool-free strict replay need independent cases.
Depth-specific completed query results must not poison other summary queries.
This is readiness, not delivered behavior; no transitive fixpoint, alias analysis,
purity, density or safe-rewrite claim is implied.


## C11d: bounded two-edge scalar mutation composition

Evidence/engine owns typed summary witnesses, depth-specific completed queries,
call composition and unit tests; acceptance owns `tests/migration_effect_composition`;
the lead owns report schema/export, original oracle scope transition, registration
and documentation. An independent read-only audit covers ownership, argument
projection, exact contexts, cycles, cache separation and resource preflight.
This bounded slice is delivered. The independent audit confirmed depth separation,
original ownership and budget preflight, and prompted an additional
exact-fit/one-short wrapper publication test included in the final suite.

The target is `caller → wrapper → leaf` over the existing scalar signature and
call vocabulary. Every nested witness retains original assignment owner/site and
ordered original call occurrences; summary-level projected roots remain distinct
from the original write root. Repeated paths are not collapsed. Only complete
bounded wrappers supply summaries, and checked direct witnesses survive incomplete
assessment. Third-edge chains, recursion, unknown arguments and opaque boundaries
remain incomplete. Existing work limits must charge summary sites and path entries
before publication; no new hashes, source/graph clones, admission permissions or
budgets are introduced. Schema 2 explicitly names `summary_site_index` and exports
original ownership/path metadata. The prior owned-scalar nested-helper negative
is deliberately promoted to a two-edge positive with a new third-edge negative.

Seven new engine tests and eleven independent live CLI cases cover depth-query
ordering, canonical leaf and repeated wrapper reuse, reordered mutable formals,
distinct original owners/paths, third edges, cycles, unknown remainder, aliases,
reference redirection, coercions, observation/call/summary/site budgets and strict
full/compact replay after deleting sources and removing compiler tools. Direct
mutation-rule evidence remains identical when effects are also requested.

The frozen checkpoint passes 2,898 nextest tests (five skipped) in 186.383 seconds,
plus doc tests. Both originally reported NonZero regressions pass. Formatting,
workspace/all-target Clippy with warnings denied, all 56 Python comparison tests
and diff checks pass. Legacy style reports zero violations across 9,148 functions;
reviewed patterns reports zero candidates. The unfiltered result retains exactly
the same 15 baseline IDs as C09g. The pattern gate identified duplicated call
assessment, now shared with explicit remaining depth. No ignores, suppressions,
thresholds, admission permissions or work budgets changed. Evidence logs are
`/tmp/clones-two-edge-effects-{focused,full,clippy,python,patterns}.log`, style JSON
and the separate `patterns-unfiltered.json` under the same prefix. Runtime is
checkpoint evidence, not a measured speedup claim.

The next ready mutation slice is ordinary `if/else` direct assignment-site
inventory, advancing the staged `branch-source-site-counts` requirement. Its
contract must count both original arms once, preserve native lexical roots and
unknown boundaries, and keep conditional call/formal-use and effect-summary
certification conservative. General effects/density, broader signatures,
legacy retirement, native macOS validation and exact Go comparison remain open.
