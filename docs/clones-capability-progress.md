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

## C11e: direct scalar assignment sites in ordinary branches

Delivered the staged `branch-source-site-counts` requirement. Evidence owns
direct-only branch/source/native joins and unit fixtures; acceptance owns
`tests/migration_mutation_branches`; the lead owns corpus promotion, registration
and docs. The contract inventories both original arms once with exact lexical
root identities, preserving unknown/unsupported boundaries and existing work
limits. It does not broaden call/formal-use/effect-summary certification. The
preexisting standalone lexical-shadowing requirement is checked explicitly rather
than presented as newly implemented behavior.

Fourteen source/native unit cases and twelve independent live CLI cases cover
original arm sites, no-else/nested/else-if scope, distinct roots, lexical shadowing,
transparent parentheses, missing/contradictory evidence, aliases, reference
redirection, unknown calls and RHS work, conditional assignment boundaries,
consumer isolation, owner budgets and source/tool-free strict full/compact replay.
The audit found compound assignments inside conditions needed an explicit
unsupported boundary, now checked by both native and live regressions. Original
condition subtrees are borrowed and checked once; no extra index, source/graph
cloning, hashing, admission permission or work-budget increase was added.

The patterns gate prompted one shared typed native-effect lookup and an explicit
argument-projection interface for the existing product/reference diagnostic
lenses (`82f20f8`). Their distinct normalization, owner checks, errors and schemas remain.
The focused 97-test run, formatting, workspace/all-target Clippy with warnings
denied, all 56 Python comparison tests and diff checks pass. Final legacy style
reports zero violations across 9,160 functions. Reviewed patterns is clean, and
unfiltered discovery retains exactly the same 15 baseline IDs.
The frozen integrated checkpoint passes all 2,924 nextest tests (five skipped)
in 187.291 seconds, plus doc tests. Both originally reported NonZero regressions
and original product/reference diagnostic cases pass. Logs and JSON reports are
retained under `/tmp/clones-direct-branches-*`; runtime is checkpoint evidence,
not a measured speedup claim. No ignores, suppressions or thresholds changed.

Next is mutation-rule policy compatibility: captured named suppressions and
workspace path exclusions using the existing source-policy helpers and original
immutable observations. Trusted exclusions must be explicit policy skips while
raw evidence remains inspectable; suppression alone cannot conceal incomplete
analysis. Invalid comments and unavailable paths remain visible. Independent
acceptance targets cover rule-specific hiding, malformed annotations, excluded
opaque owners, exact paths and source/tool-free policy replay. Broader effects,
loops, saved mutation reviews, quality evaluation and legacy retirement remain
open; native macOS and exact Go comparison are unverified.

## C11f: captured mutation visibility policy

Delivered named mutation-rule suppressions and original workspace path
exclusions. Evidence owns the combined mutation/source/comment observation and
unit tests; policy owns shared adapters, compiled matching and decisions;
acceptance owns `tests/migration_mutation_policy`; the lead owns report schema 2,
scope counters, exits, registration and documentation. Raw evidence and rule
results remain independent of visibility. Only trusted exclusions permit a
complete policy skip; suppression alone never repairs incomplete evidence.

The combined inspection borrows original source and shares the existing bounded
parse/native pass. Checked comments reuse physical-line association and the core
suppression parser. Only explicitly named mutation rules hide findings; invalid
annotations remain visible, including on excluded owners. Workspace globs compile
once and apply to original checked paths. Missing path/source authority and
unassessed owners cannot acquire exclusions. Generated test-harness owners remain
unavailable independently of authored source-file exclusions. Schema 2 separates
raw evidence completeness from policy scope, preserves measured raw rules and
reports skipped decisions without fabricated zero measurements. Finding exits
and text consume visible findings; other consumers remain independent.

Nine engine tests, fourteen policy/adapter tests and eleven live CLI acceptance
cases cover raw-evidence equality, borrowed source, comment/budget boundaries,
rule-specific and malformed annotations, opaque owners, exact paths, lib/test
population, independent consumers and source/tool-free strict full/compact replay.
The final snapshot passes all 2,958 nextest tests (five skipped) in 188.950 seconds,
plus doc tests. Both originally reported NonZero regressions pass. Formatting,
workspace/all-target Clippy with warnings denied, all 56 Python comparison tests
and diff checks pass. Legacy style reports zero violations across 9,179 functions;
reviewed patterns reports zero candidates. Unfiltered discovery retains exactly
the same 15 baseline IDs as C11e. The pattern gate prompted a single-pass mutation
finding tally. No ignores, thresholds, admission permissions, hashes or work
budgets changed. Logs and JSON reports are under `/tmp/clones-mutation-policy-*`;
runtime is checkpoint evidence, not a measured speedup claim.

Next is direct loop assignment-site inventory, including the staged primitive
`for _ in 0..count` case. Original while/loop children exist, but for-loop capture
needs checked original head/binder/body correspondence before the consumer can
count those sites. Keep static-site counting independent of dynamic frequency,
termination, call summaries and formal-use certification. Saved mutation reviews,
broader effects/density, project quality and legacy retirement remain open.
Native macOS and exact Go comparison remain unverified.

## C11g: direct loop assignment-site inventory

Delivered the staged primitive `for _ in 0..count` static-site requirement,
ordinary boolean while conditions and unlabelled loop bodies. Capture owns
original for-loop head/binder/body and exact compiler core Range evidence;
the mutation consumer joins source/native nodes, loop-local roots and original
transfer destinations. Each authored assignment counts once, including after a
valueless unlabelled break/continue and inside a zero-range or false-while body.
Nested branches/loops, immutable scalar binders, shadowing and conditional
transfers retain lexical roots. Labels, value breaks, pattern conditions,
inclusive ranges, arbitrary iterators and mutable binders remain incomplete.
No iteration, termination, call-summary, formal-use or density conclusion follows.

Compiler builtin never-type observations use a separate consumer record keyed by
original HIR identity, kind and span, checked against the existing bounded native
inventory and owning function. They do not change graph type normalization or
admission permissions. Duplicate, wrong-owner, mismatched and malformed records
reject; missing evidence stays unknown without consulting display text. The full
suite caught regressions in the initial shared never-type token approach; that
approach was removed before delivery. Original graph behavior is preserved.
There are no new hashes, source/graph copies, ignores, thresholds or budgets.

Fourteen focused engine cases and fifteen live/replay CLI cases cover exact
static counts, correspondence changes, typed binders, transfers, macro origins,
summary isolation and existing budget boundaries. Full/compact strict replay
works without source files or compiler tools, including conditional continue
bodies, and reconfigures mutation limits while retaining original evidence.
The frozen `loop-source-site-not-runtime-frequency` corpus case is promoted to
`bounded-direct-loops`; its runtime frequency remains explicitly count-dependent.

Integrated validation: 2,987 workspace tests pass (five skipped), plus doc tests,
with nextest runtime 190.817 seconds. Both originally reported NonZero regressions,
generated conversion admission and the u32 widening reader pass. The focused run
passed 148 tests; final full validation also covers the strengthened coordinated
foreign-owner negative case. Clippy with warnings denied, formatting and diff
checks pass. Legacy style reports zero violations across 9,218 functions;
reviewed patterns pass with zero candidates, and unfiltered discovery retains
exactly the same fifteen baseline IDs. All 56 migration comparison Python tests
and 50 auxiliary script tests pass. Native macOS and external Go comparison remain
unverified.

Next ready work is saved mutation reviews over completed visible findings, then
broader effect/density and project-quality contracts. The migration remains active;
legacy retirement gates are not established by this slice.

## C11h: saved numeric mutation reviews

Added independent `--dismiss-mutation-per-target` and `--dismiss-mutated-targets`
choices over explicit saved runs, review files and reasons. Schema 6 adds numeric
mutation entries while preserving schemas 1 through 5 and populated existing
families. A current dismissal requires a completed evaluated visible finding,
selected scope and a unique original declaration. Identity retains the rule and
threshold, full sorted per-root count distribution, original locator, build
context and detector versions. Binder/comment/RHS changes preserving that numeric
measurement retain the choice without approving semantics. Changed distributions
or thresholds are stale even when the maximum and root count are unchanged.

Exact choices filter only the selected rule's presentation and finding exit;
raw evidence, raw rules, policy decisions, coverage and execution stay intact.
Lower-bound findings, suppressed/excluded owners, unknown evidence and exhausted
budgets cannot authorize dismissal or negative pruning. Status distinguishes
omitted analysis, missing declarations, changed context, stale policy/measurement
and unavailable observations. Prune preview/apply requires a completed selected
negative with the same rule, threshold, versions and context. Changed-file
selection reuses source-rule scope rather than adding another selector.

Legacy pattern discovery identified duplicated source/mutation owner handling.
Both now share bounded collection, original identity resolution, ambiguity checks
and a borrowed retained-owner index; measurement and positive/negative rule
decisions remain consumer-specific. Numeric snapshots contain no source/graph
copies or hashes. No ignores, thresholds, admission permissions or work budgets
changed. Source/tool-free full/compact strict replay and text exits are covered.

Integrated validation: all 3,013 workspace nextest tests pass (five skipped) in
192.885 seconds, plus doc tests. Both originally reported NonZero regressions and
the u32 widening reader pass. The full suite covers source and mutation review
paths together after shared-owner extraction; the earlier focused run passed 171
tests. Clippy with warnings denied, formatting and diff checks pass. Legacy style
reports zero violations across 9,267 functions; reviewed patterns reports zero
candidates, and unfiltered discovery retains exactly the fifteen C11g baseline
IDs. All 56 migration comparison Python tests and 50 auxiliary script tests pass.
Logs and JSON reports are under `/tmp/clones-mutation-review-*`; runtime is
checkpoint evidence, not a measured speedup claim.

A separate locked compact `hir-data-v3 --mutation-rules` capture over the declared
23-function acceptance corpus completed with 20 evaluated and three unavailable
owners, honestly returning exit 2 for incomplete mutation coverage. It is smoke
evidence for explicit boundaries, not whole-project clean or retirement evidence.

Next ready work is broader effect/density and project-quality contracts. The full
migration remains active; legacy retirement, native macOS and external Go
comparison remain unverified. Main/PR19 has not been merged into this branch.

## C11i: checked control sites in scalar effect summaries

Extended the explicit scalar mutation-effect observation with the direct
inventory's checked ordinary branches and primitive loops. Original call and
write sites appear once, including mutually exclusive arms, false while/empty
range bodies and source after plain transfers. Leaf/wrapper summaries preserve
original write ownership, distinct call paths and projected lexical roots under
the existing two-edge/work bounds. Unsupported controls, aliases, reference
redirection, opaque operations and missing native correspondence remain explicit
boundaries. This gives no runtime frequency, purity, termination or density claim.

A separate effect-control request leaves ordinary call-roster and formal-use
certification conservative. A live acceptance case found that name-only reference
poisoning confused a shadowed scalar local with an unrelated mutable formal.
The scan now tracks original lexical block/for-binder shadow scopes, visiting
initializers and range endpoints before installing new bindings. Reference
redirection after a shadow scope and in an initializer still poisons the formal;
macros, raw references and opaque work retain the existing conservative behavior.
No hashes, graph permissions, ignores, threshold changes or larger budgets added.

Four low-level and six live/replay cases cover original control joins and resealed
contradictions, site counts, summary reuse, shadowed roots, call paths, unsupported
remainders, omitted owners, depth limits and source/tool-free strict full/compact
replay. Independent control requirements extend the frozen helper-effect corpus.
Focused validation passes all 149 mutation/effect and branch/loop boundary tests.
Older direct-inventory tests now explicitly require complete scalar effects while
retaining incomplete formal-use coverage and exit 2 for the combined request.
A separate locked compact `hir-data-v3 --mutation-effects` capture over the frozen
three-function control corpus completes all three owners: five leaf-write sites,
two conditional wrapper calls and one caller call.

Integrated validation: all 3,023 nextest tests pass (five skipped) in 192.622
seconds, plus doc tests. Both originally reported NonZero regressions and the u32
widening reader pass. Clippy with warnings denied, formatting and diff checks
pass. Legacy style reports zero violations across 9,274 functions; reviewed
patterns passes with zero candidates, and unfiltered discovery retains exactly
the fifteen C11h baseline IDs. All 56 migration comparison and 50 auxiliary script
tests pass. Logs and reports are under `/tmp/clones-effect-controls-*`. Runtime is
checkpoint evidence, not a measured speedup claim.

Next ready work remains a complete bounded event/statement contract for density
and broader project-quality reporting. This extension does not retire legacy
effects or establish full C11 completion; native macOS and external Go comparison
remain unverified.


## C11j: independent original-source statement denominator

Added a bounded statement inventory prerequisite for general effect density,
exposed by `--mutation-effects` schema 3 as the independent versioned metric
`rstyle.original-source-statements/1`. Every lexical block statement/tail counts
once, plus bare match-arm bodies; nested blocks, both branches, loop bodies and
source after transfers preserve authored sites without runtime multiplication.
Expressions/calls do not contribute extra units unless they contain blocks.
The input reuses original whole-file ownership/signature checks and source syntax
budgets, without new hashes, graph permissions or source/graph copies. This is
an explicitly defined source metric, not a native HIR/PDG count or an assertion
of identical legacy counting.

Opaque closures/items/macros/attributes, async/await, unsafe/const syntax and
compile-time array lengths/repeats/const generic arguments make the entire
inventory unavailable. No remainder count is published. An exhausted function
budget skips this inspection. Source statement availability and scalar-effect
coverage remain independent: supported source match syntax can supply a complete
count while scalar-effect analysis stays incomplete. Neither an available count
nor a completed scalar-write proof authorizes a density ratio. Event vocabulary,
configured call classification/weights and complete numeric policy evaluation
remain the next required work; general density and full C11 are incomplete.

Five low-level cases cover independently expected counts, opaque/compile-time
boundaries, original ownership/signature contradictions and preparse budgets.
Live control-chain counts are 10/3/1 for leaf/wrapper/caller. Additional live cases
cover empty and match bodies, opaque syntax and explicit function budgets;
existing full/compact strict replay verifies equal inventories without project
source or tools. All 3,029 workspace nextest tests pass (five skipped) in 192.978
seconds, plus doc tests. Both original NonZero regressions and the u32 widening
reader pass. Workspace/all-target Clippy with warnings denied passes; all 56
migration comparison and 50 auxiliary script tests pass. Runtime is checkpoint
evidence, not a measured speedup claim.

Legacy style reports zero violations across 9,286 functions after a spacing-only
phase-break correction; reviewed patterns passes with zero visible candidates,
and unfiltered discovery retains exactly the fifteen C11i baseline IDs. Formatting
and diff checks pass. A separate locked compact `hir-data-v3 --mutation-effects`
capture over the independent three-function control corpus completes with the
same 10/3/1 source counts. Logs/reports are under `/tmp/clones-statements-*`.

Next ready work is the complete event numerator: original resolved-call
classification, direct versus delegated mutation event units, non-mutation and
unknown effects, checked weighted arithmetic and policy/threshold decisions.
Do not infer it from the deduplicated scalar root union. Full migration remains
active; legacy retirement, native macOS and external Go comparison remain
unverified. Main/PR19 has not been merged into this feature branch.

## C11k: bounded static effect density

Added independent opt-in `--effect-density` with `--effect-config`, sharing one
completed scalar-summary session with optional `--mutation-effects`. Schema
`rstyle.mutation-effects/4` records requested axes, effective policy, raw scalar
and statement evidence, static event inventories, numeric outcomes and visible
findings separately. Density-only can complete for classified safe external free
calls while its raw scalar summary remains incomplete; combined flags require
both axes. Strict matching/admission and direct mutation-budget metrics are unchanged.

Direct caller-visible mutable roots contribute once per checked native binding;
each original completed helper-call occurrence with projected writes contributes
one mutation event. Configured non-mutation native path labels propagate through
at most two original local edges, with one event per kind per outer call. Repeated
calls retain separate source events. Per-summary kind sets are retained once in
the owning unit/analysis and borrowed, with indexed checked call occurrences;
there is no JSON hashing, source/graph clone or global cache.

External direct free calls require native identity and safety, checked owned-u32
arguments, unit/u32/never results and configured classification. Explicit generics,
associated/method calls, nested arguments, mutable external arguments and opaque
work remain incomplete. Missing older safety metadata remains unknown. Native
never-type evidence is consumed locally; the extractor's shared type normalization
is unchanged. No graph permission or extraction policy was extended.

The independent source denominator and complete event inventory gate checked
weighted milli arithmetic. Existing legacy weights, path-prefix classifier,
minimum statements/lines and strict comparison are reused. Unknown work cannot
acquire a ratio or minimum exemption even with zero unknown weight. Captured
named density suppressions preserve raw measurements; explicit path exclusions
are policy skips with raw evidence retained. Full/compact saved replay recomputes
config policy without live source/compiler tools. Implicit arithmetic panics are
outside this static event metric; unsafe/global/Drop-bearing forms are unsupported.

Independent acceptance labels live in `eval/clones-capability/density-cases.json`.
CLI cases cover repeated roots/calls, local kind propagation, external unit/never
calls, unknown-zero-weight controls, source policy, budget/config failures,
combined requested axes, text/finding exits and source/tool-free replay. Native
metadata tests separately check missing identity/safety, path contradictions,
never result evidence and unsupported selections. Numeric tests cover strict
thresholds, unavailable denominators, minimum exemptions and all overflow stages.

The final frozen snapshot passes all 3,048 nextest tests (five skipped) in
194.833 seconds, plus doc tests. Both original NonZero regression tests and the
u32 widening-reader composition pass. Workspace/all-target Clippy with warnings
denied and formatting/diff checks pass. The 56 migration comparison and 50 script
tests pass. Legacy style reports zero violations across 9,311 functions; reviewed
patterns has zero visible candidates, and unfiltered patterns preserves exactly
the prior fifteen reviewed IDs. No ignores, thresholds or budgets changed.

A separate locked compact `hir-data-v3 --effect-density --mutation-effects`
capture over three independent scalar functions completes both requested axes:
leaf has one mutation event/two statements (500 milli), wrapper has two delegated
events/two statements (1,000), caller has one delegated event/one statement
(1,000). The latter two are visible findings at limit500. The initial smoke
attempt correctly failed without its fixture lockfile; after generating that
lockfile offline, the locked run passes. Logs/reports are under
`/tmp/clones-density-*`. Runtime is checkpoint evidence, not a speedup claim.

Full migration remains active. Next ready quality work includes
saved density review semantics and independent legacy/new policy comparison over
the declared scope; broader effects, project coverage and legacy retirement remain
open. Native macOS and external Go comparison remain unverified. Main/PR19 has not
been incorporated into this feature branch.

## C11l: saved numeric density reviews

Committed as `c7d9e11` and pushed to `feature/rust-feature-coverage`.

Lead owns CLI/report integration, review publication, schema and index seams;
contract, workflow and acceptance workers supplied frozen leaf changes. Review
store schema 7 adds `density_entries`, while schemas 1–6 remain readable without
lookup/status rewriting. `--dismiss-density` requires an explicit saved run,
review file, nonempty reason and requested density analysis. Full policy, metric
versions, event-kind distribution, source statement/code-line counts and original
locator/build context define exact numeric equality. No body approval or semantic
equivalence is claimed; measurement-preserving body edits can preserve a review.
No new hashes, graph/source clones, admission permissions or budget changes occur.

Reviewed findings filter text and visible finding exits; raw density/effect
measurements remain intact. Retained status and pruning require completed numeric
negatives under the same policy/current versions. Unknown, exempt, suppressed,
excluded, unselected and budget-limited evidence cannot authorize numeric negative
pruning. Shared numeric lifecycle and owner-selection/retained-record helpers
reuse completed assessments while preserving independent mutation/density checks.

Independent CLI acceptance covers full/compact/text source/tool-free replay,
raw measurements, exits, invalid selectors, unknown/suppressed/excluded cases,
changed thresholds/weights/path policy/event distribution/denominators, omitted
families, scoped selection and preview/apply/idempotent pruning. Pure contract
cases additionally cover checked arithmetic, schema closure and legacy envelopes.

The frozen implementation passes all 3,065 nextest tests (five skipped) in
195.491 seconds plus doc tests. Both original NonZero regressions and the u32
widening-reader composition pass. Final workspace/all-target Clippy with warnings
denied, formatting and diff checks pass. Python migration/script suites pass
56/50 tests. Legacy style reports zero violations across 9,385 functions; reviewed
patterns reports zero visible candidates without new ignores. Unfiltered patterns preserves exactly the prior fifteen reviewed IDs. Comparison
evidence is retained under `/tmp/clones-density-review-*`.
Next ready work remains independent legacy/new density policy comparison over
declared scopes and broader project coverage. No legacy capability is retired;
native macOS and external Go comparison remain unverified. Main/PR19 is merged
upstream but has not been incorporated into this feature branch.

## C11m: density comparison and classified external helper propagation (`e012dcd`)

A ten-function independently authored density corpus now compares separate
legacy/new captures under identical effective policy. The shared serial harness
retains compiler/build context, explicit raw reports, captured legacy owner
locations, commands/exits/timings, snapshots of source/config/labels/comparison
code, and unchanged offline fixture lockfiles. Two configuration files express
one policy: legacy explicitly accepts depth 2, while the new frontend fixes its
two-edge bound and rejects a depth input field. Fully expanded weights, path
lists and exclusions must match before capture and comparison.

The initial comparison exposed an unimplemented accepted case: a known safe
classified external call completed directly but could not propagate through one
local helper. The case's expected result remained unchanged. Density now borrows
already observed original local owners via a read-only Session index query,
requiring exact definition/path/selected-context/owned-argument association and a
completed nonbudget summary attempt. The density consumer separately requires a
known original signature and complete body/call-boundary roster with no escaped
direct write. Classification follows at most two total call edges, including the
external edge. Checked mutation summaries remain independent; this extension
cannot certify their unknown external bodies. Per-owner/depth kinds are borrowed
from explicitly separated summary/retained cache key domains. No hashes,
source/graph clones, new extraction permissions or budget changes are introduced.

The comparator checks event distributions, source denominators, arithmetic,
threshold/minimum policy, source suppression/exclusion and visible outcomes.
Independent review confirms both intentional metric changes: repeated delegated
mutation call occurrences and inherited classified-call occurrences remain
separate in the new metric, while legacy deduplicates roots or inherited
kind/label pairs. Legacy clean reports establish outcome agreement rather than
an unpublished numeric value. The known unknown-call case is explicitly
`expected-uncovered`, outside the measured denominator and never evaluated-clean.
It forces `fully_complete: false` even when the acceptance command succeeds.
Failed capture/handoff, missing/ambiguous owners, policy/context drift and
unexpected incomplete completed cases still fail the comparison.

Contract and acceptance workers supplied isolated Python adapter/tests and
Rust density-helper implementation/tests; lead owns harness/fixture/config,
documentation, integration and commits. Real Cargo tests cover direct and
wrapped classification, exact two-edge/third-edge boundary, unknown/mutable/
unsafe/macro/cycle controls, source-alias/native-display disagreement,
source/tool-free policy replay, and observation exhaustion after a retained
helper. The frozen implementation passes all 3,070 nextest tests (five skipped)
in 196.009 seconds plus doc tests. Both original NonZero regressions and the u32
widening-reader composition pass. Final workspace/all-target Clippy with warnings
denied, formatting and diff checks pass. Python comparison/harness tests pass
76/76, and script tests pass 50/50. Legacy style reports zero violations across
9,391 functions; reviewed patterns reports zero visible candidates, and separate
unfiltered comparison preserves exactly the prior fifteen reviewed IDs.

A rebuilt-binary live density comparison passes ten cases: six evaluated
agreements, two independently reviewed intentional differences, one excluded
policy agreement and one expected-uncovered unknown. Eight numeric cases are
measured; full coverage remains false. The unchanged source-rule control passes
28 evaluated and three excluded-policy rows after harness reuse. Explicit reports,
commands and context are under `/tmp/clones-density-comparison-checkpoint` and
`/tmp/clones-density-source-comparison-checkpoint`; test/style logs are under
`/tmp/clones-density-helpers-*`. These bounded execution results are not a speedup
or broad project-coverage claim.

Next ready work includes project-scoped density coverage/quality and changed-scope
headless baselines. Broader non-scalar effects and advanced anomaly consumers
remain open; native macOS and external Go comparison remain unverified. No
legacy capability is retired and upstream PR19 remains outside this branch.


## C13b: changed-file density presentation and CI findings (`f5ac003`)

Density now accepts the shared bounded `rstyle.changed-files/1` manifest without
requiring source rules. Lead owns CLI/configuration, shared path membership,
report/review integration and docs; an acceptance worker owns the isolated density
changed-scope tests, and a read-only worker audits selection/completeness boundaries.
Source rules and density retain independent selected coverage under one manifest.
Density presentation applies saved numeric reviews before publishing selected
visible counts. Raw measurements, events, owner rosters, whole-scope totals and
execution remain intact. Empty selections cannot hide incomplete analysis;
unknown selected files remain unobserved. Review selection borrows original owner
paths with a bounded identity roster, so unselected choices cannot be dismissed
or pruned as completed negatives. No new hash, source read/copy, parsing,
extraction permission, budget or threshold is introduced.

Eight real-Cargo cases cover selected clean and violating files, raw measurements,
full/compact source/tool-free replay and changed selection, empty/unknown paths,
whole-scope budgets and unselected unknown calls, suppression/exclusion, saved
numeric reviews and unselected pruning protection, combined source/density gates,
source-only compatibility, and invalid suppression visibility. The review case
uses the existing root-file/inline-module identity contract; external module
review identity remains unavailable rather than being relaxed by selection.

The frozen code passes all 3,078 nextest tests (five skipped), including the
original NonZero regressions and u32 widening composition, plus doc tests. Final
workspace/all-target Clippy with warnings denied, formatting and diff checks pass.
Legacy style reports zero violations across 9,399 functions; reviewed patterns
reports zero visible candidates, with exactly the prior fifteen unfiltered IDs.
Python comparison/harness tests pass 76/76 and script tests pass 50/50. Logs are
under `/tmp/clones-density-selection-*`. Initial style findings were resolved by
extracting bounded owner selection and a shared consumer/selection completion
check; neither ledger entries nor thresholds changed. This is coverage/workflow
progress, not a speedup claim.

Next ready work is project-scoped density coverage/quality and saved CI finding
baselines. Native macOS, external Go comparison, changed-line analysis and legacy
retirement remain unverified or open. Upstream PR19 is still outside this branch.


## C13c: explicit saved density CI baselines (`5b42926`)

The lead owns CLI/report integration and shared observation reuse; model and IO
workers own the isolated `review/density_baseline/` leaves; independent acceptance
owns `tests/migration_effects/density/baseline.rs`. A read-only audit checked empty
inventories, source-policy changes, unavailable retained observations, identity
bounds and publication ordering.

`--write-density-baseline NEW_PATH` exports complete whole-scope numeric findings;
`--density-baseline PATH --fail-on-new-density` compares selected current findings
and policy/exclusion/build-context headers. The new exit gate is separate from
existing `--fail-on-findings`; raw findings, measurements and saved reviews remain
intact. Density reviews and baselines share one completed observation roster,
including checked original locators and exact numeric snapshots. Current numeric
identity and uniqueness still require independent retained source inspection;
loaded baseline values supply comparison only. Whole-analysis failure or
incompleteness prevents publication and clean comparison.

The bounded closed baseline stores policy/version/context/exclusion headers even
when there are no findings. Contexts use canonical full-value ordering, not hashes.
Changed headers fail the new gate even with empty selected presentation or no
current positive findings. Only a completed selected numeric negative under the
retained policy is resolved. Missing, unselected, suppressed, excluded, exempt and
unavailable retained owners remain explicit rather than resolved; these statuses
are separate from current comparison coverage and do not by themselves count as
new current findings. Export is whole-scope only and atomically refuses overwrite.
Input/output is bounded to 4 MiB and 4,096 positive snapshots, context entries and
exclusion patterns. No source/graph clone, new extraction permission, hash,
threshold, suppression or budget increase is introduced.

Thirteen pure-model/IO tests and thirteen real-Cargo acceptance tests pass. They
cover empty headers, invalid/duplicate identities and numeric values, size/entry
bounds, unknown fields, exact/moved/changed/new findings, changed policy/context/
exclusions, numeric resolution and unavailable/unobserved retained entries,
reviews and changed selection, incomplete export, create-new refusal and
source/tool-free full/compact replay. The frozen code passes all 3,104 nextest
tests (five skipped) in 197.777 seconds and workspace doc tests, including the
original NonZero regressions and u32 widening composition. Workspace/all-target
Clippy with warnings denied and formatting pass. Legacy style reports zero
violations across 9,428 functions; reviewed patterns reports zero visible
candidates, with exactly the prior fifteen unfiltered IDs. Python
comparison/harness tests pass 76/76 and script tests pass 50/50. Formatting and
diff checks pass. Evidence is under `/tmp/clones-density-baseline-*`.

Shared numeric review presentation and atomic artifact publication replace
parallel family implementations. The integrated run caught a missing nested
serde flatten attribute in that extraction; restoring it preserves the existing
top-level density/mutation review JSON contract. All 55 focused density/review
tests and the full rerun pass without changing review decisions or measurements.

Original root-file and inline-module locators remain the current positive identity
scope. Broader external-module identities, source-rule CI baselines, project
coverage/quality, native macOS, external Go comparisons and capability-specific
retirement remain open. PR19 is still outside this branch.

## C13d: original external free-function review identities (`f344f37`)

The lead owns declaration-index routing, existing analysis boundaries and
parameter-review acceptance; the locator worker owns `dispatch/locations.rs` and
its unit tests; acceptance owns density external-module cases and length/spacing
live review cases. This slice extends the
shared durable location query used by source and numeric reviews, making density
baselines usable for ordinary external free functions. It does not add external
enum/capability analysis or external method review permission.

A lazily retained locator index checks the complete ordinary selected-module
closure from immutable captured source buffers. It keeps the exact compiler cfg
context, unique full qualified declaration names and exact file/range/header
correspondence. Source buffers share existing immutable snapshot handles. Query
lookups borrow the completed index, with no new hash or serialized authority.
Root/inline queries keep their original declaration index and method behavior.
External paths use `name.rs` and `name/mod.rs`, including nested inline modules;
remapped/generated ownership, duplicate/missing/orphan files, unknown selection,
external methods and exhausted limits remain unavailable. Whole-closure limits
include existing 256-file/8-MiB census bounds, a 256-KiB per-file and 256-delimiter
parser preflight, 32 module levels and 4,096 visited source items. Failed or
exhausted indexing publishes no partial locator population.

Independent unit cases cover ordinary/nested layouts, sibling names and equal
byte offsets, cfg isolation, original BOM/CRLF offsets, ambiguous closure/name
ownership, changed bytes/header/path/identifier, generated declarations and
budgets, including impl/trait members in the whole declaration population.
Routing cases verify external identities cannot authorize dispatch or
capability findings, and retained lookup checks current context and source.
Five real-Cargo cases cover numeric baseline export, recapture/movement, changed
measurements, multiple external paths, saved density reviews and source/tool-free
full/compact comparison and export. Source-rule cases now prove ordinary external
length/parameter/spacing dismissals preserve raw findings, while remapped-module
identity remains unavailable and budget refusal remains intact. All 48 focused
source-review/external-module cases pass. The final frozen code passes all 3,126
nextest tests (five skipped) in 198.256 seconds and workspace doc tests, including
the original NonZero regressions and u32 widening composition. Workspace/all-target
Clippy with warnings denied, formatting and diff checks pass. Legacy style reports
zero violations across 9,441 functions; reviewed patterns reports zero visible
candidates, with exactly the previous fifteen unfiltered IDs. The extra query
wrapper was removed to share owner/file selection without a new abstraction or
ledger entry. Evidence is under `/tmp/clones-external-locations-*`.

Broader external methods, source-rule CI baselines, project quality/coverage,
native macOS, external Go comparison and capability-specific legacy retirement
remain open. PR19 remains outside this branch.
Next ready work is explicit source-rule CI baselines over shared completed
length/parameter/configured-spacing observations, retaining empty policy/context
headers and keeping numeric resolution distinct from suppression visibility.

## C13e: source-rule CI baseline inventories

The model worker owns the closed source baseline format and independent unit
cases; the I/O worker extracts bounded create-new baseline publication; acceptance
owns real-Cargo cases. The lead integrates source observations, policy headers,
CLI/configuration, reports, shared baseline comparisons and documentation. All
worker writes are finished before integrated validation.

`--source-rules --write-source-baseline NEW_PATH` exports complete whole-scope
parameter, length and configured-spacing inventories. Input comparison uses
`--source-baseline PATH`, with a separate `--fail-on-new-source` gate for new or
changed selected visible findings and policy/exclusion/build-context drift.
Existing findings and ordinary finding exits remain intact. Empty inventories
retain full headers. Disabled spacing is explicit; source/tool-free full/compact
replay shares the same current assessments. Export refuses changed-file selection,
incomplete analysis, invalid suppression, overwrite, and exhausted budgets.

All three families reuse one completed owner/measurement roster with saved
reviews applied before baseline counts. Exact comparisons borrow compact
snapshots; only requested export copies measurement metadata. Source and density
baselines share bounded I/O, checked construction/publication, current comparison,
retained-state mapping and report/gate behavior. No source/graph clone, new hash,
extraction permission, threshold, budget or review ledger change is introduced.
Source baseline negatives additionally require a completed numeric observation
under the retained policy without suppression. Missing/excluded/unavailable
owners remain distinct from resolved; unselected owners and disabled spacing
remain unselected. Existing saved-review pruning keeps its visibility semantics.

The closed source model uses a 4-MiB byte limit and a combined 4,096-entry bound,
with independently checked version/policy/context/exclusion headers and exact
per-family/locator/compilation-target uniqueness. Original root/inline and checked
ordinary external free-function owner limits still apply. Broader external
methods, project-scoped quality/coverage, native macOS, external Go comparison
and capability-specific legacy retirement remain open. PR19 is still outside
this feature branch. The next ready frontier is project-scoped coverage/quality
measurement using the independently specified source and density consumers.

Integrated validation on the frozen C13e snapshot passes all 3,151 nextest tests
(five skipped) in 199.869 seconds and workspace doc tests, including both original
NonZero regressions, u32 widening composition and 14 real-Cargo source-baseline
cases. Eleven pure source model cases cover closed schema, exact family policy,
headers, bounds and duplicate identity. The existing source/density review
regressions pass. Formatting, workspace/all-target Clippy with warnings denied
and diff checks pass. Legacy style reports zero violations across 9,478 functions;
reviewed patterns reports zero visible candidates. Unfiltered patterns retains
exactly the previous fifteen IDs, with no ledger edits. Evidence is retained in
`/tmp/clones-source-baseline-*`, with final gates named `*-final.*`. This is a
delivered source-baseline increment, not full migration or legacy retirement.

## C13f: project-capture type consistency

The lead owns receiver capture, regression registration and integration; the
capture worker owns independent dispatch-token refresh and its leaf regression.
The project-quality worker independently labels four production functions in
`rstyle-ccgraph::frac` before reading reports. The study remains unfinished.

A bounded real `rstyle-ccgraph` library capture exposed two inconsistencies that
small acceptance fixtures missed. Method signatures were normalized in a
post-analysis environment that reveals hidden opaque implementations, although
the operands retain body type-checking types. Receiver observations now use the
compiler's body-aware post-typecheck environment. Declared signature results
still require exact independent agreement with HIR; iterator semantic admission
is not extended. A real-Cargo regression covers foreign and locally defined
opaque iterator returns, `map/max/unwrap_or`, complete source-rule coverage and
continued strict clone exclusion. Modified result claims fail direct verification
and strict bundle import in both verification policies.

Call dispatch refreshed its type tokens only when every operand and result had
a normalized type. One unsupported sibling left otherwise available composite
operand tokens stale. Ordinary original direct free calls now refresh each
available operand/result independently, retaining the original roster length and
all independent contradiction checks. Specialized method and generated
propagation routes preserve their observed dispatch vocabulary until the full
structural roster is known. The first integrated test run caught that distinction;
the wider partial-refresh experiment was narrowed before delivery.
The second regression combines borrowed-byte operands with unsupported vector
results/arguments: source rules remain complete while density remains explicitly
incomplete with exit 2.

The unchanged bounded project command completed in 18.649 seconds before the
receiver fix and 18.770 seconds after it, both with exit 2; these are failed-run
observations, not speed or coverage comparisons. Its third capture passes both
type boundaries but stops on `Mutation native inventory repeats a HIR identity`.
The next ready repair is the established synthetic match-guard wrapper sharing
its guard expression's HIR identity, preserving rejection of actual duplicate
nodes. Retained evidence is `/tmp/clones-project-ccgraph-capture-{1,2,3}`.
No project-wide clean, quality score, native macOS, Go parity or retirement claim
follows from these failed captures. PR19 remains outside this feature branch.

The frozen final implementation passes all 3,153 nextest tests (five skipped) in
200.488 seconds and workspace doc tests. Formatting, workspace/all-target Clippy
with warnings denied and diff checks pass. Legacy style reports zero violations
across 9,480 functions; reviewed patterns has zero visible candidates, and
unfiltered patterns retains exactly the previous fifteen IDs. No thresholds,
suppression/review ledgers, work budgets or extraction permissions changed.
Final checkpoint evidence is `/tmp/clones-project-capture-*-final.*`.
The independent project reducer remains a draft pending a usable real project
report; its 28 synthetic tests do not establish production coverage or accuracy.

## C13g: real-project coverage and unsupported-call isolation

Capture repair commit: `9b2347b`; the following study increment records the
repeatable measurement assets and integrated checkpoint.

The lead owns native indexing, call routing, registration, docs and integration;
the call worker owns leaf unit/real-Cargo regressions, the study worker owns the
reducer, and the independent source labels precede report inspection. Worker
writes are finished before integrated validation.

A checked match-arm guard wrapper may share exactly its expression child's HIR
identity. Native indexing now admits that established role only in the middle
child of a three-child match arm, with a single expression child, plain wrapper
metadata and matching source span/snippet when identities alias. Distinct peeled
wrapper identities are independently checked. Both roles count toward the
unchanged 16,384-node work bound; real-node duplication still fails. Six focused
unit cases include exact-limit/one-more boundaries, malformed wrappers and repeated
real guards. Real Cargo covers ordinary and macro guards without effect authority.

Associated constructors can carry `resolved_direct_callee` while retaining a
specialized observed dispatch vocabulary. Both scalar call routes now classify
resolution kind/associated metadata before checking the scalar free-call type
contract. Native/source argument-roster correspondence remains mandatory. Local
and external associated calls stay unavailable, not pure; malformed genuine free
records and inconsistent free-call dispatch still fail. Unit regressions cover
inherent/trait metadata and both ownership flags; real Cargo covers byte-slice
arguments to `Cursor::new` and a local inherent constructor with vector result.

The bounded `rstyle-ccgraph` library capture now emits a complete captured roster
and no capture/handoff failures in 20.416 seconds. Its 414 owners retain incomplete,
budget-exhausted source/density consumers and exit 2. Length and parameters each
measure 238 owners; spacing measures 208. The source rules report 240 raw findings.
Density measures none of 414 owners. Eleven independent source sample checks
agree; one spacing check remains unknown because statement inspection exhausts
its budget. No threshold, review/suppression, work limit, extraction permission or
hashing rule changed. Failed earlier runs are not speed baselines.

The checked-in study policies, preauthored labels and explicit bounded reducer
make this measurement reproducible. The reducer validates full unit/owner roster,
policy/context, aggregate consistency and per-owner outcomes, retaining incomplete
reasons and refusing duplicate object keys or output overwrite. Twenty-nine pure
and CLI-load tests pass. See `eval/clones-capability/project-study.md` for commands,
counts and limitations. Production coverage is now measured, not complete. The small statement-budget case and durable external-method review locators are
the next ready frontier; all 238 original owners already have length and parameter
measurements, while the other 176 owners are generated by expansion;
density quality, native macOS, Go comparison and legacy retirement remain open.
PR19 is merged in main but remains outside this feature branch.

The frozen implementation passes all 3,164 nextest tests (five skipped) in
200.988 seconds and workspace doc tests, including the original NonZero failures,
u32 widening, both new real-Cargo regressions and the native/call unit controls.
Formatting, workspace/all-target Clippy with warnings denied and diff checks pass.
Legacy style reports zero violations across 9,484 functions; reviewed patterns
reports zero visible candidates and unfiltered patterns retains exactly the
previous fifteen IDs. No ledgers changed. The 29 reducer tests also pass.
Final checkpoint evidence is `/tmp/clones-project-study-*-final.*` and
`/tmp/clones-project-study-python.log`. This delivers capture isolation and a
repeatable coverage study, not full migration or capability retirement.

## C13h: original external method review identities

The locator worker owns the external declaration leaf and unit controls; parameter
acceptance owns the dedicated parameter-review leaf; numeric acceptance owns
length/spacing leaves. The lead owns shared naming visibility, source/density
baseline acceptance, registration, docs/embedded skill and integrated validation.
Worker writes finish before the frozen checkpoint.

Ordinary external module locations now index inherent and same-module local-trait
impl methods on uniquely named nongeneric structs/enums. Durable names reuse the
existing root method convention (`Owner::method` or `<Owner as Trait>::method`),
with declaration kind `method`; existing locators and store schemas are unchanged.
Associated, shared and mutable receiver methods retain their exact original
signature/source/header checks. Member cfg selection precedes collision checks.
Ambiguous qualified identities refuse the whole locator index. Imports, aliases,
generic owner/impl forms, qualified/nonlocal owners or traits, default trait body
methods and generated/remapped ownership remain unavailable. A wildcard import
conservatively prevents method owner resolution. Unrelated explicit imports do
not. The existing immutable census, parser, module/item population and retained
entry limits are unchanged. Source snapshots are shared, queries borrow the
completed index, and no new hash or serialized authority is introduced.

The locator extension does not extend dispatch, capability, strict pair, caller,
helper or effect permissions. A route regression explicitly checks that an
externally located receiver method still lacks dispatch/capability authority.
Unit controls distinguish inherent and multiple trait origins, unsupported and
shadowed/imported/generic owners, cfg selection, duplicate identities, original
BOM/CRLF/Unicode bytes and unchanged whole-declaration budget refusal.

Real-Cargo acceptance covers durable parameter/length/spacing reviews, retained
raw findings, full/compact source/tool-free strict replay, namespace/owner namesake
isolation, movement/body retention and numeric staleness. Source baselines preserve
all three numeric families on an external method across movement and replay. A
separate associated-method density baseline exercises already complete scalar
body evidence; the locator does not manufacture that evidence. Unsupported owner
forms retain raw measurements but refuse dismissal without publishing a store.
The shared owner scope is documented in source rules, finding reviews, the matrix
and the embedded skill. Broader generic/imported owners, default trait methods,
project spacing/density quality, native macOS, Go comparison and legacy retirement
remain open. PR19 remains outside this feature branch.

Strict replay of the earlier 414-owner project capture completes in 4.390 seconds
with no capture/handoff failures, resolving original identities for 121 of 123
parameter findings and 116 of 117 length findings. The remaining parameter owners
are a cross-module qualified trait impl and a lifetime-generic owner; the remaining
length owner is lifetime-generic. All 240 raw findings remain visible and no review
store is created. Existing source budget/unsupported coverage still requires exit
2. This is a separate replay observation, not a fresh capture or performance
comparison. Evidence is `/tmp/clones-project-method-review`; its command and saved
run path are recorded in `run.json`.

The frozen checkpoint passes all 3,182 nextest tests (five skipped) in 202.011
seconds and workspace doc tests, including both original NonZero failures and
u32 widening composition. All ten new real-Cargo cases pass, and focused dispatch
unit tests pass. Formatting, workspace/all-target Clippy with warnings denied and
diff checks pass. Legacy style reports zero violations across 9,493 functions.
Reviewed patterns reports zero visible candidates, and unfiltered patterns retains
exactly the previous fifteen IDs. The initial exact plain-path duplicate was
removed by reusing the existing source helper; no review/ignore ledger changed.
No threshold, work budget, extraction permission, semantic admission or hash
policy changed. Final integrated evidence is `/tmp/clones-external-methods-*-final.*`.
This is a delivered original-method review increment; the full migration remains
active, with the project statement-budget case and broader capability quality
and retirement gates still open.

## C13i: lifetime-only external method review identities

The locator worker owns the lifetime syntax/declaration leaves and index tests;
independent acceptance owns the parameter-review leaf. The lead owns source
baseline acceptance, semantic-boundary controls, docs/embedded skill and integrated
validation. All implementation writes freeze before checkpoint gates.

The measured project spacing gap (`frac::split_decimal`) remains unavailable:
146 raw punctuation bytes exceed the existing whole-function parser limit of 128.
Resetting that limit per statement would increase aggregate parser work. No
parser budget or unavailable outcome changed to admit this case. The next ready
source identity gap instead concerns two project functions on lifetime-only
owners, `LabelNames<'_>::partition` and `AdmittedPair<'_>::check_correspondence`.

External locators now accept unique same-module structs/enums with plain lifetime
parameters and matching impl lifetime arguments. Explicit arguments must have the
owner's exact arity and use an impl binder, `'_` or `'static`. Names omit lifetime
spelling, retaining `Owner::method` or `<Owner as LocalTrait>::method`; canonical
collisions refuse the complete index. Nongeneric local trait origins remain
supported. Type/const parameters, bounds, where clauses, lifetime-generic traits,
qualified/imported/aliased owners, ambiguous/generated ownership and exhausted
bounds remain unavailable. Exact immutable census, original function bytes,
header/receiver/cfg checks and existing population limits still apply. No schema,
hash, source clone, work budget, dispatch/effect permission or strict matching
policy changed.

Unit acceptance covers struct/enum, named/placeholder/static lifetimes, arity and
binding failures, mixed generics, cfg/namespace ambiguity, original source/header
contradictions and lifetime-erased method collisions. Real-Cargo acceptance checks
numeric review identity through movement and lifetime/binder renaming, stale
arity, unsupported const/generic-trait owners, tool/source-free parameter replay
and all three numeric source-baseline families. A semantic-boundary control keeps
located lifetime-owner methods unsupported by dispatch/capability consumers.

Real-Cargo acceptance exposed a pre-existing capture/replay mismatch for small
scalar methods under `impl Owner<'_>`: rustc rewrites the placeholder as a
synthetic impl lifetime, which capture accepted as inherent ownership while the
strict source verifier requires explicit in-order binders. Lifetime-owner capture
now uses a HIR source gate to refuse synthetic, specialized, reordered/repeated or
bounded lifetime impls before publishing that ownership. It uses borrowed native
parameter/path rosters; no full impl reparse or verifier relaxation is added.
Concrete type/const specializations retain their independently supported prior
routes. The initial full checkpoint caught an overbroad gate blocking digit
helpers; narrowing it to lifetime handling restores all eighteen focused
regressions, including both original NonZero tests. Original function/source
records remain available. Named exact lifetime helper ownership remains supported. The acceptance explicitly checks ownership presence/absence
and strict numeric review replay on named, placeholder and static forms. HIR does
not retain the optional `::` spelling before type arguments, so that existing
source syntax rejection remains separately enforced by strict verification.

Strict replay of the same 414-owner saved project capture now resolves identities
for 122 of 123 parameter findings and all 117 length findings, recovering both
measured lifetime-owner gaps. The remaining unavailable parameter owner uses a
cross-module qualified trait. All 240 raw findings remain visible; no review
store is created. Capture/handoff failures remain empty, and existing spacing
incompleteness still requires exit 2. The 4.407-second replay is not a fresh
capture or speed comparison. Exact command and results are retained under
`/tmp/clones-project-lifetime-review-final`.

The frozen source passes 49 focused dispatch/locator unit tests and all eleven
focused external-method Cargo cases. Formatting, all-target workspace Clippy with
warnings denied and diff checks pass. Legacy style reports zero violations across
9,501 functions. Reviewed patterns reports zero visible candidates; the unfiltered
scan retains exactly the previous fifteen IDs. No ignore/review ledger changed.
The final frozen checkpoint passes all 3,193 nextest tests (five skipped) in
203.340 seconds and workspace doc tests, including both original NonZero
regressions, the digit-helper regressions and u32 widening composition. Full gate
logs are `/tmp/clones-lifetime-{style,patterns,patterns-unfiltered,clippy,tests}-final-2`,
with focused regression evidence in `/tmp/clones-lifetime-regressions`. No threshold,
work budget, review suppression, semantic permission or hash policy changed.
Full migration, broader project density/spacing quality, native macOS, Go
comparison, qualified-owner identity and capability retirement remain open. Next
ready consumer work is the project density evidence gap; the statement-budget
case requires an independently justified algorithm within the existing limits.

## C14: reuse-first compiler advisory observations

The reuse-first steering supersedes the pending wider-integer/tuple advisory
scan. That uncommitted prototype was preserved outside the worktree at
`/tmp/rstyle-deferred-integer-tuples-9vpi3gb5` rather than committed as another
collector. The existing driver already supplies the ability through
`mutations::collect` (`ExprUseVisitor`), `effects::collect` and
`extract::function_facts`. The new opt-in `--compiler-advisory` captures those
observations during the selected DataV3 compilation and feeds them directly into
`rstyle_core::evaluate`. Existing core code supplies mutation/effect policy,
bounded call-graph propagation, fix groups and parameter stacking. No parallel
implementations of those algorithms were added.

Observations decode once into owned facts and move into the existing evaluator;
there is no body re-proof, SHA-256, new hashing, graph clone, cache fingerprint or
work-budget increase. The small versioned adapter refuses missing fields that
legacy deserialization would otherwise default to zero/empty facts. The enclosing
selected-unit association retains compilation context. Check/test units evaluate
separately; unknown generated owners, absent captures and unresolved workspace
callees stay explicit. Core metric versions remain separate from source/native
mutation-effect metrics. Configuration is explicit and reported in full; capture
without the flag does no advisory collection. Strict admission and matching do
not consume this observation namespace.

Six real-Cargo acceptance tests cover u128 tuple writes, borrowed Vec mutation
without strict admission, IO propagation and parameter fix plans, source/tool-free
saved-run policy reevaluation and finding exits, missing prior captures, generated
owners and check/test separation. Two core tests enforce metric-version and
missing-field distinctions. This is not a purity or refactoring-safety proof.
Saved numeric reviews, changed-file filtering and baselines do not yet apply to
the compiler advisory field.

A fresh explicit rstyle-ccgraph library capture completes in 20.685 seconds with
empty capture/handoff failures: all 238 original-source owners now have compiler
observations, with 176 generated owners unavailable out of 414. Core evaluation
assesses all 238 under its defaults. Twenty unresolved workspace callees retain
incomplete coverage and exit 2. Its four accepted strict pairs exactly match the
previous saved project report. Gcd independently retains two owned mutations and
zero escaping events; its nine lowered HIR statements are not relabeled as the
source contract's five lexical statements. This is a coverage gain, not a timing
speedup or strict density comparison. Reproduction and limitations are recorded
in [the project study](../eval/clones-capability/project-study.md) and
[compiler advisory contract](clones-compiler-advisory.md).

The frozen source passes all 3,201 nextest tests (five existing skips) in 204.329
seconds plus workspace doc tests, including both original NonZero failures and
u32 widening composition. Formatting, diff checks and all-target workspace Clippy
with warnings denied pass. Legacy style is zero across 9,516 functions. Reviewed
patterns is zero; unfiltered discovery retains exactly the prior fifteen IDs.
No ignore or review ledger changed. Gate logs are
`/tmp/clones-reuse-{clippy-1,tests-final-1,style-final-1,patterns-final-1,patterns-unfiltered-final-1}`;
project capture evidence is `/tmp/clones-project-reuse-capture-1`.

Full migration and capability retirement remain incomplete. Next reuse seam:
existing `sigmine::mine`, `prep::prepare` and `candidates::mine` proposal policy,
with actual compiler graph observations, explicit unavailable inputs and an
ownership/borrowing seam where needed. Do not invent graphs or extend another
proposal algorithm before examining those inputs. Cross-package propagation,
broader project quality and native macOS validation also remain open.

## C15: move existing proposal inputs through preparation

The next reusable consumer is the existing core abstraction proposer, not another
proposal algorithm. Its preparation previously cloned the input facts, cloned
active facts again, rebuilt every graph and cloned facts again while replacing
those graphs. The legacy patterns command now moves its owned facts into
`patterns_report::run_owned` and `prep::prepare_owned`. Preparation retains the
source, node and edge buffers and applies only recorded callee/signature changes.
All rewrites are selected against the original active inputs before any graph
changes, preserving trait resolution and suppression order. Borrowed callers keep
their existing APIs. No hash, checksum, source re-proof, new cache, work-budget
increase or proposal-policy change was added. An unused evaluator-retention API
was deferred until a consumer actually needs it.

Regression tests check buffer retention, unchanged unrelated metadata,
unresolved calls, absent versus known-empty graphs, candidate identity and review
policy. This is a necessary ownership seam, not new clone capability or a measured
end-to-end speedup. Compiler graph capture and connection of the compiler advisory
adapter to this proposer remain the next separate vertical slice. Existing strict
admission and matching remain unchanged.

The frozen source passes all 3,204 nextest tests (five existing skips), workspace
doc tests, all-target workspace Clippy with warnings denied, formatting and diff
checks. Both original NonZero regressions and u32 widening composition pass.
Legacy style is zero across 9,519 functions; reviewed patterns is zero, and
unfiltered discovery retains exactly the prior fifteen candidate IDs. No ignore
or review ledger changed. Final gate logs are
`/tmp/clones-owned-{tests-2,clippy-2,style-2,patterns-1,patterns-unfiltered-1}`.

## C16: reuse compiler graphs and existing abstraction policy

The opt-in `--compiler-patterns` extends compiler advisory capture by directly
calling the existing `pdg::pdg_of` collector. A separate versioned graph record
preserves the original style metric contract. The adapter decodes the graphs
once, joins them to the same owned function observations, calls
`evaluate_and_retain`, then moves policy-selected inputs into the existing
`patterns_report::run_owned`. Signature mining, preparation, clustering,
alignment, proposal families, ranking, counter-evidence and existing-trait/source
suppression all remain in core. No second proposer, new graph permission,
threshold change, cache, hash or source re-proof was added.

Each compilation unit reports graph input availability and actual core default
parameters alongside the core proposal report. Missing capture, generated owners
and known-empty graphs remain distinct. Input completeness is not exhaustive
discovery or complete Rust semantics: graph approximation and existing bounded
blocking remain explicit limitations. Core path exclusions apply before mining;
raw observation rosters remain visible. Proposal findings affect the explicit
finding exit, with incomplete coverage still taking exit 2. Legacy ignores and
newer context review/baseline workflows are not silently applied to this field.
The workspace-member acceptance case also fixes the previous adapter's ambiguous
location base: source paths normalize against captured workspace `source_root`,
separately from `package_root`, using retained locations without filesystem work.

Nine new real-Cargo tests cover all five proposal families, existing-trait,
getter and forwarding negatives, legacy-report agreement, source-free replay and
text output, missing/empty/generated inputs, check/test context isolation, core
exclusions, workspace-member paths and unchanged nonempty strict decisions.
Four core tests enforce missing signature/node/edge fields, version and graph
bounds, known-empty graphs and retained body buffers through policy selection.

A final fresh rstyle-ccgraph library capture takes 19.228 seconds and supplies
graphs for all 238 original owners. The other 176 generated owners and twenty
unresolved workspace callees keep coverage incomplete. Its two signature groups
and one candidate exactly equal the separate legacy library discovery report;
the four accepted strict pairs remain unchanged. The sole candidate was already
reviewed unsuitable in the unchanged legacy ledger. This is measured capability
access, not a new useful-refactoring or speedup claim. See the project study for
source review, reproduction, exact run and comparison evidence.

The frozen checkpoint passes all 3,217 nextest tests in 204.524 seconds (five
existing skips), workspace doc tests, all-target workspace Clippy with warnings
denied, formatting and diff checks. Both original NonZero regressions and u32
widening composition pass. Legacy style is zero across 9,528 functions;
reviewed patterns is zero and unfiltered discovery retains exactly the prior
fifteen candidate IDs. No ignore or review ledger changed. Full gate logs are
`/tmp/clones-pattern-adapter-{tests,clippy,style,patterns,patterns-unfiltered}-final-1`.

Review/baseline integration for compiler advisory findings, cross-package
propagation, broader project quality, native macOS verification and all unmet
capability retirement gates remain open. The goal remains incomplete.

## C17: borrow default graphs through reused core clustering

The lead owns `rstyle-core` normalization/clustering, the focused ownership tests,
performance evidence and these docs. Default clustering previously cloned every
captured graph even though no normalization rules ran. It now borrows the
original immutable graphs; enabled normalization keeps the existing owned
rewrites. Both legacy patterns and compiler-backed proposals use this path.
There is no new cache, hash, body proof or strict matching change. The owned
normalization API remains compatible.

On the same retained rstyle-ccgraph library input (238 functions/graphs, 3,847
nodes and 4,858 edges), the complete core proposer produces an exactly equal
report while allocation/reallocation calls fall from 183,312 to 166,555 and
cumulative requested bytes from 12,333,551 to 10,469,752. Seven instrumented
samples have effectively unchanged median time (108.667 versus 108.838 ms), so
this is an allocation reduction, not a measured wall-time speedup or capability
quality improvement. See `docs/performance/borrowed-proposal-graphs.md` and its
sample artifact for scope, dependency and instrumentation limitations.

The frozen checkpoint passes all 3,219 nextest tests in 204.461 seconds (five
existing skips), workspace doc tests, all-target workspace Clippy with warnings
denied, formatting and diff checks. The 255 core tests include original-buffer
identity without normalization and owned rewrites that preserve their original
input. Both original NonZero regressions and u32 widening composition pass.
Legacy style is zero across 9,529 functions; reviewed patterns is zero, and
unfiltered discovery retains exactly the prior fifteen candidate IDs. No ignore
or review ledger changed. Full gate logs are
`/tmp/clones-borrowed-graphs-{tests,clippy,style,patterns,patterns-unfiltered}-1`.
Review/baseline integration, broader quality and the other recorded retirement
gaps remain the next ready work; the full goal remains incomplete.

## C18: reuse saved reviews for compiler proposals

The lead owns the compiler proposal/review adapter, its core ownership seam,
shared artifact codec and dismissal helper, acceptance tests and documentation.
An explicit `--review-file` now looks up compiler proposal choices through the
existing exact-review rules, outcome/summary model and single atomic workflow.
Schema 8 preserves the earlier families. Recording a choice requires an explicit
saved-run unit/candidate selector and reason; ambiguous selectors reject. IDs
select findings but never authorize persisted equality. No project review or
ignore ledger was changed.

Core returns the exact prepared facts used by its proposer. Referenced site
graphs move into shared immutable owners, and saved entries share those buffers.
Without a review file, review conversion does no work; empty proposal sets also
return immediately. Retained status reuses completed lookups rather than
comparing graphs again. Full graph, signature, membership, kind, policy and build
context equality controls visibility, without a new hash, source parse or body
proof. Stable local callee locators reuse existing owner names; ambiguous names
remain unavailable. Definition signatures are retained independently of their
bodies. Opaque macro/closure sites also retain their captured text, so hidden
edits cannot be lost behind the approximate graph; harmless text edits can
conservatively stale such reviews.

Raw core reports and strict matching remain unchanged. JSON carries separate
visible candidate indices and review decisions; text and finding exits apply
those decisions. Missing input or unavailable positive review identity retains
incompleteness. Bounded proposal discovery cannot prove absence, so missing
proposals remain unavailable and compiler proposal entries are never
pruned automatically. Existing independently completed negative rules for other
families remain intact. Review stores reuse the existing bounded artifact codec
and 4 MiB convention, with failed selectors and oversized writes preserving the
existing bytes. Mutation and density dismissals now share the same reason and
recording helper instead of repeating that policy.

Acceptance cases cover all five core proposal families, layout/binder stability,
impl renumbering and definition-body edits, source-free replay, changed graphs,
check/test isolation, failed writes, omitted families and missing-proposal prune
protection. Model cases cover shared buffer ownership, complete graph/policy
fields, relevant snapshot changes, ambiguous locators and artifact limits.
Compiler-style review/baseline migration, project proposal quality, cross-package
propagation and the other capability retirement gates remain open.

The full suite passes all 3,232 nextest tests in 203.905 seconds (five existing
skips), followed by workspace doc tests. Both original NonZero regressions and
u32 widening composition pass. All-target workspace Clippy with warnings denied
also passes. The initial full run caught an older density schema test that still
rejected schema 8; it now verifies schema-8 compatibility and rejects future
schema 9, and the complete corrected suite passes. Evidence is
`/tmp/clones-compiler-reviews-tests-final-2` and
`/tmp/clones-compiler-reviews-clippy-final-3`.
Legacy style remains zero across 9,574 functions; reviewed patterns remains zero.
No thresholds, budgets or ignore entries were changed. Final evidence is
`/tmp/clones-compiler-reviews-style-final-2` and
`/tmp/clones-compiler-reviews-patterns-final-4`.
Unfiltered discovery retains exactly the prior fifteen candidate IDs, with
results in `/tmp/clones-compiler-reviews-patterns-unfiltered-final-1`.

Read-only replay of the retained rstyle-ccgraph capture takes 2.12 seconds and
201,248 KiB maximum RSS. Its sole proposal is visible-new, with no file created
or project dismissal made. Raw proposal/style reports and all four accepted
strict pairs are exactly unchanged. Existing generated/unresolved inputs retain
exit 2 and incomplete coverage. This is a replay observation, not a speedup or
proposal-quality claim; reproduction and evidence are in the project study and
`/tmp/clones-compiler-reviews-project-1`. Formatting and diff checks pass. Full
capability migration remains incomplete.

## C19: reuse exact reviews for completed compiler diagnostics

The lead owns the core diagnostic-ownership seam, compiler advisory review and
presentation adapters, shared captured-owner/positive-review helpers, acceptance
cases and documentation. Compiler style diagnostics now accept read-only exact
lookup through `--review-file`. Explicit saved-run unit/diagnostic indices select
a current finding for dismissal; they are never persisted identity. Schema 9
preserves earlier review families and uses the existing single atomic bounded
publication. No repository review or ignore choice was changed.

The existing core evaluator retains owner indices alongside sorting when a
review is requested. This avoids ambiguous location-based joins and does not
rerun collectors, propagation, rule policy or fix planning. Default evaluation
keeps its original diagnostic collection and no owner roster allocation. The
adapter uses shared compiler owner/trait locators, captured signatures and build
contexts. Multiple diagnostics share their immutable owner/signature and policy;
saved entries share those owners. There are no graph/body copies, new hashes,
source parses or body proofs. Proposal and diagnostic status now use one shared
positive-lookup workflow and reuse completed equality outcomes. Their visibility
is applied in one pass over compilation units.

All six existing style-rule families use full metric/version, policy, signature,
actual/limit and ordered label/count equality. Lines and diagnostic messages do
not control identity. Relevant count/distribution, signature, policy and context
changes make choices visible again. Different rules on one owner have distinct
review scopes; unrelated rule findings cannot hide or stale each other. Movement
and parameter renaming can preserve observations; mutation-root label renaming
may conservatively stale details. Invalid source suppressions cannot be reviewed
away. Numeric equality is a review contract, not body equivalence or purity.

Raw diagnostics, summaries and caller/fix plans remain intact. Separate visible
indices and decisions drive text and finding exits. Missing/generated owners and
unresolved calls keep coverage incomplete even if every known diagnostic is
hidden. Retained missing diagnostics remain unavailable; this increment does not
establish the completed negative assessments required for automatic pruning.
Omitted compiler advisory is not-requested. Compiler baselines, changed-file
presentation, cross-package propagation, broader project quality and retirement
remain open.

Retained status additionally indexes completed outcomes once and groups current
positives by owner locator. It no longer scans the entire lookup roster for every
stored entry; empty stores skip status-index construction. These indices are
analysis-local and never serialized. Exact outcomes take precedence across
multiple contexts independently of row order. This is an algorithmic reduction,
not a measured wall-time speedup.

Focused acceptance before final checkpoint validation passes 41 cases, including
all compiler proposal regressions, the six compiler style families, exact owner
association through sorting, default evaluation without an owner roster, shared
owner/policy buffers, distinct rule domains, required snapshot fields, version
and threshold checks, source-free replay, movement, changed counts/distributions,
check/test separation, atomic failed selectors and missing/generated input.
Evidence is `/tmp/clones-compiler-diagnostics-focused-3`; the subsequent
order-independent retained-status test is included in the full checkpoint suite.

The frozen source passes all 3,245 nextest tests in 206.299 seconds (five existing
skips), workspace doc tests and all-target workspace Clippy with warnings denied.
Both original NonZero regressions and u32 widening composition pass. Legacy style
is zero across 9,613 functions; reviewed patterns is zero. Final logs are
`/tmp/clones-compiler-diagnostics-tests-final-1`,
`/tmp/clones-compiler-diagnostics-clippy-final-2`,
`/tmp/clones-compiler-diagnostics-style-final-2` and
`/tmp/clones-compiler-diagnostics-patterns-final-1`.
Unfiltered discovery retains exactly the prior fifteen candidate IDs, with
results in `/tmp/clones-compiler-diagnostics-patterns-unfiltered-final-1`.

The real-project read-only study makes all 39 diagnostics under an explicit
one-target budget available to exact review lookup. The pre-recorded gcd case
matches actual two targets, limit one and visible-new status. Raw style/proposal
reports and all four accepted strict pairs are unchanged; no review file or
choice is written. Existing 176 generated owners and twenty unresolved callees
retain incomplete coverage and exit 2. The reference/lookup replays take
2.12/2.11 seconds and 202,208/202,168 KiB peak RSS respectively: single-run
observations, not a speedup or an overhead bound. Policy, independent case,
commands and limitations are in the project study. Evidence is
`/tmp/clones-compiler-diagnostics-project-{raw,review}-1`.
Formatting and diff checks pass. Full capability migration remains incomplete.

## C20: reuse compiler diagnostic snapshots for CI baselines

The lead owns the compiler baseline adapter, the small shared baseline handle
seam, CLI/report integration, acceptance cases and documentation. The opt-in
`--write-compiler-baseline` and `--compiler-baseline --fail-on-new-compiler`
workflow reuses completed compiler diagnostic snapshots and the existing
baseline comparison, counts, coverage, gate and bounded atomic create-new I/O.
The shared baseline report accepts a consumer-local selection handle, retaining
existing source/density output shapes while compiler reports use unit/index.
Those handles are presentation only and never enter persisted identity.

All six numeric rule families retain full effective policy, metric/detector
versions, signatures, actual/limit values, ordered details and build contexts.
Comparison borrows snapshot values and indexes positive observations; only an
explicit export copies compact snapshots. One immutable effective-policy value
is now shared across current compilation units, diagnostic owners and baseline
publication instead of serializing a fresh policy for every unit. No new hash,
source parse, graph/body copy, body proof or second evaluator is introduced.
Absent review/baseline requests still avoid diagnostic owner-roster retention.

Baseline evaluation finishes before saved-review publication. Export and new
finding exits reuse existing completion/gate machinery. Empty inventories check
policy and build scope; raw findings and caller/fix plans remain unchanged.
Explicit reviews affect visible new-finding counts but never erase raw export
entries or hide header drift. Missing prior positives remain unavailable: the
adapter does not synthesize completed negatives or claim resolution/pruning.
Invalid suppressions cannot be exported as clean numeric baseline entries.
Incomplete scopes keep exit 2 and cannot publish, including generated test-harness
owners. Whole-scope bounds are 4 MiB and 4,096 owners/entries/contexts. Compiler
changed-file presentation, proposal baselines, cross-package propagation,
broader project completeness and per-capability retirement remain open.

Ten independent real-Cargo cases exercise the six numeric families, movement,
changed/new measurements, full/compact source-free replay without tools, ordinary
versus baseline finding exits, empty-policy/release-context drift, generated
check/test incompleteness, positive-only retained status, explicit review
visibility/raw export, unavailable input, invalid suppressions, closed artifacts
and create-new publication. Shared source/density baseline regressions remain in
the full checkpoint suite. Final gate and project observations follow below.

The pattern gate identified three same-domain overlaps during development.
Snapshot inventory validation now shares one generic pass between density and
compiler baselines; retained-row enumeration also shares one implementation.
Compiler observations feed the existing comparison directly. Density validation
keeps the same snapshot, policy/version, build-context and unique-owner checks.
Empty retained compiler inventories skip positive-index construction entirely.
No pattern ignore, suppression, limit or budget was changed.
The existing inventory interface now enforces header-before-findings validation
for source, density and compiler baselines. Their rule-specific checks remain
separate; no source/density completeness or policy semantics were relaxed.

Final context review found that units with no captured functions have no
function-backed build observation. Shared baseline context collection now rejects
that missing context explicitly rather than inventing an empty context list.
A real-Cargo empty-crate regression covers it. Source/density baselines inherit
the same missing-data distinction; ordinary advisory evaluation is unchanged.

The final integrated snapshot passes all 3,255 nextest tests in 208.373 seconds
with five existing skips, plus workspace doc tests. Both original NonZero CI
regressions and the u32 widening composition pass. Workspace all-target Clippy
with warnings denied passes; reviewed patterns reports zero. Evidence is
`/tmp/clones-compiler-baseline-tests-final-3`,
`/tmp/clones-compiler-baseline-clippy-final-5` and
`/tmp/clones-compiler-baseline-patterns-final-4`.
Style is zero across 9,638 functions; evidence is
`/tmp/clones-compiler-baseline-style-final-3`. Final unfiltered discovery contains
fourteen previously reviewed IDs and no new IDs. Prior `76534998402b` is not
returned by bounded discovery in this snapshot; its existing ledger entry and
all other review entries remain untouched. Evidence is
`/tmp/clones-compiler-baseline-patterns-unfiltered-final-2`.

The retained project replay preserves all 39 raw diagnostics, the core proposal
report and all four strict pairs. Its 176 unavailable generated owners and twenty
unresolved callees correctly prevent baseline export: exit 2, explicit incomplete
reason and no file created. The independently pre-recorded case, commands and
single-run resource observations are in the project study. Reference/refused
export take 2.07/2.02 seconds and 201,568/201,684 KiB peak RSS respectively;
these are not a speedup or overhead bound. Evidence is
`/tmp/clones-compiler-baseline-project-{raw,export}-1`.
Formatting and diff checks pass. Full migration remains incomplete; compiler
changed-file presentation is the next ready workflow slice. No legacy command
or strict contract is retired by this checkpoint.

## C21: reuse completed compiler diagnostics for changed-file presentation

The lead owns the selection adapter, shared coverage seam, CLI/report integration,
acceptance cases and docs. `--compiler-advisory --changed-files PATH` now uses the
existing bounded manifest and exact completed core diagnostic paths. Selection
runs after explicit reviews and retains raw evaluations, totals and caller/fix
plans. Separate raw diagnostic indices control text and finding exits. Numeric
baseline comparison uses the same selection while header drift stays global;
whole-scope export still conflicts with selection. Invalid suppressions and core
proposals remain global.

The shared source/density coverage aggregator now accepts finding paths separately
from owner availability; findings cannot promote an unobserved path to complete.
Compiler selection indexes scoped observation IDs and borrows captured paths.
Only selected report path strings are owned for publication, with no graph/body
copy, source parsing, hash, replacement checksum or repeated proof. Absent explicit
review/baseline requests still avoid diagnostic owner rosters and snapshots.
Unknown selected paths remain incomplete, empty selection selects no numeric
findings, and global generated/missing/unresolved inputs still keep exit 2.
Check/test occurrences remain separate. No source-rule analysis is implicitly
requested by a compiler-only selection.

Ten real-Cargo integration cases cover exact nested namesake paths, raw reports
and strict results, text output, empty/unknown files, source/tool-free replay,
missing/generated inputs, check/test occurrences, reviews, baseline selected
counts/global drift, invalid suppressions and global proposals. Two shared unit
cases cover independent coverage/count aggregation and owned path publication.
No limits, budgets, suppressions or ignore entries are changed. Full capability
migration and legacy retirement remain incomplete.

The pattern gate identified two counting-helper overlaps during development.
Presentation and exit counts now share the existing unit finding method, using
selected indices before reviewed indices while preserving global raw totals.
No new generic counting framework or ledger exception was introduced.

The frozen integrated snapshot passes all 3,267 nextest cases in 208.866 seconds
with five existing skips, plus workspace doc tests. Both original NonZero CI
regressions and the u32 widening composition pass. Workspace all-target Clippy
with warnings denied, formatting and diff checks pass. Style reports zero across
9,646 functions; reviewed patterns reports zero. Unfiltered discovery contains
fifteen previously reviewed IDs: `76534998402b` returns in this bounded scan;
all other fourteen IDs match C20. No review ledger changes were made.
Evidence is `/tmp/clones-compiler-selection-tests-final-1`,
`/tmp/clones-compiler-selection-clippy-final-2`,
`/tmp/clones-compiler-selection-style-final-3`,
`/tmp/clones-compiler-selection-patterns-final-2` and
`/tmp/clones-compiler-selection-patterns-unfiltered-final-1`.

The retained project replay selects the two numeric findings in `frac.rs` while
preserving all 39 raw diagnostics/fix plans, the global proposal, missing-input
lists and four accepted strict pairs. The selected path itself has eight
unavailable owners, and both runs correctly exit 2. Reference/selected resource
observations are 2.13/2.04 seconds and 201,632/201,568 KiB peak RSS, without a
speedup or overhead-bound claim. Cases, commands and limitations are recorded in
the project study; logs are `/tmp/clones-compiler-selection-project-{raw,selected}-1`.
Compiler proposal baselines, completed-negative/prune prerequisites, cross-package
propagation, broader real-project completeness/quality and capability retirement
remain open. No new hashes or strict-contract changes are introduced.

## C22: reuse completed compiler proposal snapshots for CI baselines

The lead owns the compiler proposal baseline adapter, shared inventory-key seam,
CLI/report integration, acceptance cases and docs. The explicit
`--write-compiler-pattern-baseline` / `--compiler-pattern-baseline
--fail-on-new-compiler-patterns` workflow reuses prepared shared review snapshots
and the existing baseline comparison, counts, retained-positive accounting and
bounded atomic create-new I/O. No second miner, evaluator, source parser, body
proof, hash or graph copy is introduced. Snapshot preparation remains opt-in to
reviews/baselines; export clones only snapshot metadata and shared owner handles
before serialization.

Membership keys borrow all ordered site/definition locators, retaining the role
boundary, proposal kind and compilation target. Exact equality retains graph,
signature, opaque-source, version and context distinctions. Full compiler policy,
default clustering policy and canonical build scope participate in headers even
when no proposals exist. Explicit reviews affect new visible counts but not raw
exports or header drift. Numeric changed-file presentation never filters this
global proposal gate; export conflicts with selection. Missing prior proposals
stay unavailable, never resolved or pruned, because bounded discovery cannot
establish a completed negative. Raw core diagnostics, fix plans, proposals and
strict matching remain intact.

The shared inventory validator now accepts a consumer-specific borrowed identity
key; existing numeric baselines retain their rule/owner/target keys. No policy,
limit, budget, suppression or review ledger changes are made. Nine real-Cargo
cases cover distinct site/definition roles on one owner, all five current proposal families, movement and graph changes,
full/compact source/tool-free replay, empty-policy/context drift, exact reviews,
global selection behavior, absent positives, generated/missing graphs, distinct
check/test units, invalid artifacts and create-new publication. Final integrated
gates and project observations follow below. Broader project quality/coverage,
cross-package propagation, completed-negative prerequisites and retirement remain
open; the full migration is not complete.


The pattern gate exposed duplicated compiler-header construction and baseline
request selection. Both compiler families now share header construction through
a small inventory interface and one typed consumer selector. Snapshot/header
validation remains family-specific; numeric artifact shapes and behavior stay
unchanged. No ignores or discovery settings were changed.

The frozen integrated snapshot passes all 3,276 nextest cases in 210.316 seconds
with five existing skips, plus workspace doc tests. Both original NonZero CI
regressions and the u32 widening composition pass. Workspace all-target Clippy
with warnings denied, formatting and diff checks pass. Style reports zero across
9,663 functions; reviewed patterns reports zero. Unfiltered discovery contains
the same fourteen reviewed IDs as C20; `76534998402b` is not returned in this
bounded scan, and its ledger entry remains untouched. Evidence is
`/tmp/clones-compiler-pattern-baseline-tests-final-1`,
`/tmp/clones-compiler-pattern-baseline-clippy-final-2`,
`/tmp/clones-compiler-pattern-baseline-style-final-2`,
`/tmp/clones-compiler-pattern-baseline-patterns-final-2` and
`/tmp/clones-compiler-pattern-baseline-patterns-unfiltered-final-1`.

The retained project reference exactly preserves the previous advisory and strict
reports: 39 numeric diagnostics, one proposal and four accepted pairs. The 176
unavailable generated owners and twenty unresolved callees correctly prevent
proposal baseline export, with exit 2, an explicit reason and no file created.
Reference/refused export resource observations are 2.24/1.99 seconds and
201,024/202,008 KiB peak RSS; these different-output single runs are not a speedup
or overhead-bound claim. Cases and commands are recorded in the project study;
logs are `/tmp/clones-compiler-pattern-baseline-project-{raw,export}-1`.
This delivers a shared CI workflow over completed proposals, not broader project
completeness, better proposal quality or legacy retirement. The next frontier is
compiler negative-status prerequisites and cross-package observations, followed
by measured project-quality/coverage work rather than additional parallel miners.

## C23: reuse completed compiler numeric scopes for retained choices

The lead owns the core completion seam, compiler diagnostic review/baseline
adapter, acceptance cases and documentation. Explicit status, pruning and numeric
baselines now reuse the existing core evaluator's suppression outcomes and
selected/excluded owner census. Excluded observations move into the temporary
inventory without copying source buffers; included positive snapshots and retained
rows share one resolved owner. Default analysis and ordinary positive lookups
retain no additional scope inventory. The shared positive workflow skips its
positive-only retained pass when completed numeric scopes supply the records.
No new source parsing, repeated rule evaluation, body/graph copying, hash,
checksum, strict admission change or real-project dismissal is introduced.

All six numeric families can establish a negative for a selected known owner
under the same full policy, detector/metric versions and compilation context.
Suppressed, invalid-suppression and excluded inputs remain unavailable. Unselected
owners remain not-requested; missing owners are not-observed only in a complete
identity scope. Five direct-rule negatives survive unrelated generated inputs;
effect density additionally requires complete propagation. Policy drift stays
stale and build contexts remain distinct. Explicit saved-run pruning and shared
baseline resolution consume the same completed outcomes. Status alone never
writes, and compiler proposals retain their positive-only contract.

Seven new real-Cargo cases cover all six negatives, source-free replay/pruning,
read-only status/dry-run, suppression/exclusion, policy/context/selection drift,
missing/generated owners, direct-versus-density completeness and strict matching
budget independence. Effect-density scope depends on compiler capture and
propagation completion, never strict pair completion. The existing
baseline expectation now requires completed same-policy resolution. A core test
checks unchanged report output, suppression alignment and original excluded-source
buffer identity; default and positive-only paths retain no scope roster.
Focused checks pass: 259 core tests and all 24 compiler diagnostic review cases,
including the strict-budget independence regression.
Final integrated checks and read-only retained-project observations follow below.
Cross-package propagation, broader project quality/coverage and legacy retirement
remain open; the full migration remains incomplete.

The final integrated snapshot passes all 3,284 nextest cases in 211.582 seconds
with five existing skips, plus workspace doc tests. Both original NonZero CI
regressions and the u32 widening composition pass. Final all-target Clippy with
warnings denied, formatting and diff checks pass; style remains zero across
9,676 functions. Test evidence is `/tmp/clones-compiler-negative-tests-final-2`;
Clippy/style evidence is `/tmp/clones-compiler-negative-{clippy,style}-final-1`.
The earlier full run was intentionally interrupted to correct advisory scope's
unnecessary dependency on strict matching completion; it is not passing evidence.

The read-only retained-project study preserves all raw evaluations, the 39 numeric
findings, one proposal and four accepted strict pairs; no review file or choice
is created. Its 176 generated unavailable owners and twenty unresolved callees
still require exit 2. Reference/status each take 2.09 seconds with
201,792/201,984 KiB peak RSS. These single observations establish neither speedup
nor an overhead bound. Independently recorded cases and commands are in the
project study; evidence is `/tmp/clones-compiler-negative-project-{raw,status}-1`.
This delivers completed numeric review/baseline lifecycle over known owners,
not broader project completeness or new project-quality findings. Cross-package
observations and measured project coverage/quality remain the next frontier.

Final reviewed patterns reports zero. Unfiltered discovery contains exactly the
same fourteen reviewed IDs as C22, with no new or absent IDs; the ignore ledger
is unchanged. Evidence is `/tmp/clones-compiler-negative-patterns-final-1` and
`/tmp/clones-compiler-negative-patterns-unfiltered-final-1`.

## C24: reuse core propagation across scoped Cargo dependency targets

The lead owns the borrowed core graph/evaluation seam, compiler advisory adapter,
acceptance cases and documentation. Core graph construction now accepts scoped
resolved target iterators; its existing propagation reads borrowed function facts.
The ordinary graph builder borrows its existing target lists without an extra
per-call vector. The compiler adapter resolves each edge once while building the
shared graph and passes completed effect vectors into ordinary core rule
assessment, avoiding a second propagation pass. Source buffers, bodies and graphs
are not copied. No new collector, propagation algorithm, hash, checksum, artifact
scan or source/body proof is introduced.

The connection uses unique captured non-test Cargo units, their exact planned
records, package/target/features/platform and direct dependency edges. Owners
remain keyed by unit and compiler function ID. Renamed dependencies work; only
known free-function definitions are connected. Associated methods, uncaptured,
generated and excluded targets, duplicate Cargo descriptors, multiple same-path
dependencies and local/dependency path collisions remain unresolved. The report's
`resolved_dependency_callees` exposes callee-to-unit mappings. Check/test inputs
never merge. Existing depth, outward kinds, boundaries, policy, versions and
unit-local fix groups/plans/proposals remain intact. Strict pair budgets and
admission remain independent. Missing dependency propagation prevents completed
density negatives; the five direct-rule scopes stay independent.

Ten real-Cargo cases cover renamed dependencies, local/cross-package hop limits,
suppression boundaries, omitted/generated/excluded inputs, source-free replay,
check/test ambiguity, duplicate crate paths, local/foreign collisions and retained
density unavailability. A core test verifies unchanged reports, original source
buffer identity and rejection of missing completed effects. Focused cases pass;
final integrated gates and the independently pre-labeled two-package project
study follow below. The full migration is not complete: cross-package associated
methods/planning, broader measured quality/coverage and retirement remain open.

The integrated snapshot passes all 3,295 nextest cases in 212.979 seconds, with
five existing skips, plus workspace doc tests. Both original NonZero regressions
and the u32 widening composition pass. Evidence is
`/tmp/clones-cross-unit-propagation-tests-final-1`. Final all-target Clippy with
warnings denied passes after extracting the density review fixture setup into
a helper; the earlier Clippy failure is not passing evidence. Formatting and
diff checks pass. Style reports zero violations over 9,694 functions
(`/tmp/clones-cross-unit-propagation-style-final-3`). Reviewed patterns reports
zero candidates with the existing fourteen reviewed entries
(`/tmp/clones-cross-unit-propagation-patterns-final-1`); no ledger entries,
thresholds, suppressions or budgets were changed.

The expanded two-library project capture stopped before advisory evaluation:
the driver could not publish `rstyle-pdg` evidence because its existing source
census exceeded the file/byte budget. The terminal exit is 2 after 44.16 seconds,
with 4,833,952 KiB peak RSS reported by `/usr/bin/time`. Its saved run is
`run-S1tXzo`, and its advisory unit roster is empty. Evidence and the exact
command are recorded in `eval/clones-capability/project-study.md` and
`/tmp/clones-cross-unit-propagation-project-capture-1`. This establishes no
project coverage gain or speedup; the sixteen independently labeled free-target
expectations remain unverified for that larger project. The existing census
budget was not raised. Ten smaller real-Cargo acceptance cases pass, including
two-hop depth, replay, missing inputs and conservative saved-density status.
The next ready work is to address project capture availability through existing
owner/source collection seams, then rerun these pre-labeled targets before
claiming broader project gains. No new hashing or redundant body proof is needed.

Final unfiltered pattern discovery contains exactly the same fourteen reviewed
IDs as C23, with no new or absent IDs. Evidence is
`/tmp/clones-cross-unit-propagation-patterns-unfiltered-final-1`; the ignore ledger
is unchanged.

## C25: preserve compiler advisory units across deep HIR transport

The lead owns the existing shared function-body codec, its bundle/diagnostic
version handoffs, real compiler regression, Python navigation compatibility and
production measurements. The failed C24 PDG census is a genuine scope limit;
its 256-file/8-MiB budget is unchanged. A smaller production selection of
`cargo-rstyle` plus `rstyle-core` instead exposed a separate handoff defect:
the version-5 writer could emit generated HIR bodies that its ordinary JSON
reader rejected for nesting. The CLI unit then contributed no advisory facts.

Version 6 keeps readable shallow bodies and streams deep bodies as borrowed
preorder records with original child counts. Import moves the nodes into the
exact ordered tree; there is no body/graph copy, new collector, evaluation
algorithm, source/type/body proof, hash, checksum or artifact scan. The JSON
nesting limit and all semantic/evidence budgets are unchanged. Versions 4/5
remain readable in their original tree grammar. Shared diagnostic snapshots
use version 3 with the same codec and retain version-2 tree reads. Source/graph
validation and strict clone matching remain unchanged. The Python diagnostic
reducers only count/locate raw functions and accept the new snapshot version;
the narrow navigation reader accepts bundle version 6 without becoming another
HIR or evidence verifier.

Five core codec cases cover exact field/order preservation beyond the default
JSON limit, shallow wire stability/borrowing, old formats, malformed/orphan/hidden
child rosters and strict excluded-body plus diagnostic replay. The real-Cargo
96-field derive case preserves its original function's advisory observation and
source-free replay while generated owners remain unavailable. Its final focused
run passes (`/tmp/clones-flat-hir-cli-tests-3`); earlier failures were test setup
compile/argument mistakes, not passing evidence. All 24 Python navigation tests
pass with the documented pinned `uv` dependencies. Plain system Python lacked
`jsonschema`; that unsuccessful invocation is not passing evidence. Final
all-target Clippy passes (`/tmp/clones-flat-hir-clippy-final-1`). Style returns to
zero over 9,703 functions after adding the phase separation it requested
(`/tmp/clones-flat-hir-style-2`). Final full-suite/pattern gates follow below.

Independent production labels precede capture in
`eval/clones-capability/project-compiler-core-cases.json`. The new run retains
1,190 CLI observations out of 2,618 functions and resolves 28 core targets,
including all seven pre-labeled callees. The baseline discarded this entire
unit on JSON nesting failure. The core unit's entire evaluation and its 1,100
source location/outcome/reason rows are unchanged. Both runs exit 2: generated
owners and 87 CLI workspace targets remain unavailable/unresolved. There are
no strict admitted functions in this scope; the complete empty pair roster
does not establish broad matching coverage. The raw CLI diagnostic quality has
not been independently reviewed. Resource observations are 48.07/58.03 seconds
and 1,785,696/1,788,984 KiB RSS before/after; the latter does additional useful
work and establishes no speedup. Commands, limitations and machine observations
are in the project study and `project-compiler-core-results.json`.

The full migration remains incomplete. Next ready work is to reuse the existing
compiler/core planning seams for broader advisory planning and independently
review the newly available production findings. Associated-method dependency
resolution, large-unit source availability and per-capability retirement remain
open; do not raise the census budget or add redundant proofs to address them.

Final reviewed patterns reports zero; unfiltered discovery contains exactly the
previous fourteen reviewed IDs, with no new or absent IDs. Evidence is
`/tmp/clones-flat-hir-patterns-{final-1,unfiltered-final-1}`. The ignore ledger is
unchanged. Final Clippy is `/tmp/clones-flat-hir-clippy-final-2`.

The first full suite passed 3,300 cases and exposed one malformed new historical
fixture: it relabeled a version-5 inventory table as version 4. The reader
correctly rejected it. The fixture now removes the absent-in-version-4 empty
table; no implementation check was relaxed. All five codec cases pass directly
(`/tmp/clones-flat-hir-body-tests-final-1`); the integrated rerun follows below.

The larger production capture also replays through strict import with no
capture/handoff failures and exact equality of the full advisory, matching and
accepted-pair reports. Exit remains 2. Evidence is
`/tmp/clones-flat-hir-project-replay-2`, with 16.67 seconds and 1,439,088 KiB RSS.
The earlier replay request repeated `--locked` and was correctly rejected for
attempting to respecify original build options; it is not passing evidence.

The final integrated rerun passes all 3,301 nextest cases with five existing
skips, plus workspace doc tests (`/tmp/clones-flat-hir-tests-final-2`). Both
original NonZero CI regressions and the u32 widening composition pass. The
earlier 3,300-pass/one-fixture-failure run remains recorded as unsuccessful.
Final Clippy, formatting, diff checks, zero-violation style and both reviewed/
unfiltered pattern gates pass. No uncommitted unrelated source or pre-existing
untracked work is included in this increment.

## C26a: reuse core planning with borrowed, indexed owners

Owner/write set: lead; `rstyle-core/src/{groups,stacking}.rs`, capability
matrix/progress. This is the core input seam for C26, not a delivered cross-unit
advisory CLI or a production coverage/speedup claim.

The existing fix-group and parameter-stacking algorithms now accept borrowed
`FnFacts` through `build_indexed` and `plans_indexed`. Callers supply completed
rule assessments by exact fact index in the resolved graph's roster. Results
retain ordered group members, step owners and exact bundle carriers alongside
the ordinary core reports. Equal display names or source locations do not merge
these identities. The legacy entry points delegate to the same implementation;
source-location diagnostic association remains their existing compatibility
behavior. Grouping, caller hops, callee ordering, bundle support, pass-through
policy and report serialization are unchanged.

No new planner, hash, artifact scan, source/body/type proof, graph copy or fact
copy is introduced. Numeric owner vectors are retained with completed plans;
report fields are constructed once and moved. Existing graph-index preconditions
apply; the seam does not add another graph validation pass or infer missing calls.

Four independent regression cases cover same-location contexts, scoped call
footprints, borrowed/legacy report equality, and duplicate display names in
bundle carriers and pass-through steps. Before the fourth case was added, all
263 core tests passed. Final integrated gates are recorded below.

Next: opt-in compiler advisory planning must reuse the already resolved C24
graph, map completed diagnostic owners into that exact roster, and expose
unit-qualified group/step/carrier keys. Keep default capture/report behavior,
missing-data outcomes and strict matching unchanged. Independently label the
production planning targets before measuring saved-run capability gains. C26
and the overall migration remain incomplete.

C26a checkpoint: all 3,305 nextest cases pass with five existing skips, plus
workspace doc tests, in 213.554 seconds (`/tmp/clones-indexed-planning-tests-1`).
This includes all four new owner-index cases, both original NonZero regressions
and the u32 widening composition. Final workspace/all-target Clippy passes
(`/tmp/clones-indexed-planning-clippy-final-1`), as do formatting and diff checks.
Style is zero over 9,706 functions (`/tmp/clones-indexed-planning-style-1`).
Reviewed patterns passes; unfiltered discovery retains exactly the same fourteen
reviewed IDs with none added or absent (`/tmp/clones-indexed-planning-patterns-1`
and `/tmp/clones-indexed-planning-patterns-unfiltered-1`). No review entries,
thresholds, budgets or extraction permissions changed. This checkpoint validates
the reusable core seam only; contextual CLI planning remains the next ready task.
