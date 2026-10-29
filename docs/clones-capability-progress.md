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

## C26: reuse completed core plans across captured dependency units

Owner/write set: lead; compiler advisory CLI/adapter, core group text formatter,
compiler advisory tests, embedded skill/contract and project study/matrix/progress.
C26a (`4c1355a`) supplies the borrowed owner-indexed core seam. The new opt-in
`--compiler-plans` retains the already resolved effect graph and its exact
unit/function owner roster. Core diagnostic indices survive sorting and map
into that roster by scoped compiler ID. The same core grouping/stacking policy
then emits ordinary reports plus exact member, step and bundle-carrier keys.
The core group formatter supplies text output, including bundle/pass-through
suggestions. No second planner, graph/body copies, source/type/body proof,
checksums, artifact scans or dependency resolution pass is introduced.

Default evaluation still finishes and drops/moves one unit at a time; only the
explicit plan request retains selected facts until shared planning is done.
Completed facts then move into optional compiler proposal mining. Raw per-unit
evaluations, summaries, finding counts and strict matching are unchanged. Named
suppressions and core exclusions/caller policy apply; reviews and changed-file
presentation leave raw plans intact. Plans describe captured calls and heuristic
edit suggestions, not external caller completeness, semantic type equivalence,
whole-plan independence or rewrite safety.

Nine real-Cargo tests pass (`/tmp/clones-cross-unit-planning-focused-2`):
cross-package signature order, bundles/pass-through, optional source-free
full/compact replay, generated/missing/excluded targets, original suppressions
and policy replay, ambiguous check/test contexts, known-empty single units,
local/foreign compiler-path collisions, core text/CLI requirements and
composition with patterns/reviews/changed-file selection. The previous seven-case
focused run also passes. The first style run reported three spacing issues and
a 51-line propagation function. Cohesive selected-row extraction and phase
separation restore zero violations over 9,714 functions
(`/tmp/clones-cross-unit-planning-style-2`), without thresholds or suppressions.
Clippy passes (`/tmp/clones-cross-unit-planning-clippy-1`); final gates follow.

Before running the production planner, source inspection labeled three concrete
signature/caller relations in `project-compiler-planning-cases.json`. Explicit
policy sets `fn_params.max_params=2`; no defaults changed. Serial replay of the
same C25 `run-ynuMSl` yields the same 1,568 known owners and 509 diagnostics both
ways. Baseline per-unit groups are 147 CLI + 51 core. Optional shared planning
has 195 groups, three with steps in both units, and all three expected
relations. Both `evaluate_retained` and `evaluate_with_effects` include their
shared exact `diagnostics::evaluate_core` caller; `all_effects` includes
`propagation::evaluate`. The entire advisory report minus optional planning,
strict matching and accepted-pair output are exactly equal. Captured
input completeness stays false; both exit 2, with 1,428/722 unavailable CLI/core
owners and 87 unresolved CLI targets. All 3,718 strict functions remain excluded.
Only the three labeled relations are quality-checked, not all raw plans or
numeric findings. Commands, exact keys, observations and limits are in the
project study and `project-compiler-planning-results.json`.

The serial no-plan/plan samples take 16.89/16.81 seconds and
1,440,112/1,440,148 KiB RSS. Filesystem page-cache states differ. This establishes
no speedup; the capability gain is contextual caller-aware planning with the
existing core algorithm. Source/tool-free replay is independently tested on
fixtures; original production sources remain present. Broader quality and
per-capability retirement remain incomplete. Next ready work includes
independent review of production advisory findings, contextual planning-quality
limits and dependency method resolution where it unlocks named consumer cases.

The first full checkpoint passes all 3,314 tests with five existing skips, plus
doc tests (`/tmp/clones-cross-unit-planning-tests-1`, 217.154 seconds). Its
subsequent pattern gate finds one new cross-domain structural candidate,
`2beffe78d944`, between the local fallback constructor and `CapturedSources::new`.
The fallback needs only the number of units, so it now accepts that count instead
of a collected-input slice and constructs the result directly. No unrelated
validator or ignore entry changes. Reviewed patterns now pass
(`/tmp/clones-cross-unit-planning-patterns-2`); unfiltered discovery is exactly
the original fourteen reviewed IDs, none new or absent
(`/tmp/clones-cross-unit-planning-patterns-unfiltered-2`). Final-snapshot style,
full tests and Clippy follow this small constructor change. The production
measurement paths both resolve cross-unit edges and do not use this local fallback.

Final production snapshot: zero style violations over 9,714 functions
(`/tmp/clones-cross-unit-planning-style-final-1`); all 3,314 nextest cases pass
with five existing skips, plus doc tests, in 214.877 seconds
(`/tmp/clones-cross-unit-planning-tests-final-1`). Both original NonZero cases
and the u32 widening composition pass. Final all-target Clippy then exposed a
61-line new acceptance test. Its review setup is now a named fixture helper;
no production implementation or assertions changed. All nine affected cases
pass after that test-only extraction
(`/tmp/clones-cross-unit-planning-focused-final-1`), and final workspace/all-target
Clippy passes (`/tmp/clones-cross-unit-planning-clippy-final-2`). Formatting and
diff checks pass. The prior Clippy rejection is not passing evidence; the full
suite above preceded only this test-helper extraction. Both pattern gates pass
with exactly fourteen existing reviewed IDs. No thresholds, ignores,
suppressions, budgets or extraction permissions changed. The contextual planning
capability is delivered over its declared captured scope; broader quality and
the full migration remain incomplete.

## C27: independently review completed production compiler findings

Offline `compiler_quality.py` reuses completed core diagnostic assessment and
C26 indexed owner associations; it does not analyze Rust, run Cargo or add any
runtime collector, proof, hashing or caching work. Eleven independently
source-labeled functions cover twenty-two parameter/mutation comparisons,
including three newly recovered CLI mutation positives. Exact policy, context,
original source associations and the full positive rule-owner roster are
required. Clean numeric values are absent rather than synthesized as zero;
missing/ambiguous/unavailable owners retain that distinction.

The expanded sample has twenty agreements and two disagreements at each of
two explicit policies. `dependencies::select` and `dependencies::resolve` each
have two explicitly mutated source containers, but the existing shared rustc
expression-use collector also counts the lowered `for` iterator named `iter`.
At the one-target study limit both remain findings; at the two-target boundary
these are outcome discrepancies. Original labels and both disagreement results
are retained in `project-compiler-quality-{cases,results,boundary-results}.json`.
The first eight-function observation agreed on all sixteen checks; expansion
exposed this gap and the labels were not changed to match the producer.

The existing collector already serves legacy style. This finding does not
establish a migration regression. No Rust code or metric version changes here;
never filter roots by the spelling `iter`. A subsequent fix should inspect
compiler root/source origins in that same collector and preserve metric-version
compatibility. Full scope is still incomplete. Neither source quality sample
claims whole-project precision/recall, density quality or retirement readiness.
Commands, results and limitations are in the project study.

Explicit boundary-policy replay of saved `run-ynuMSl` completes in 16.60 seconds
at 1,439,752 KiB peak RSS (`/tmp/clones-compiler-quality-boundary-1`). Exit 2
retains existing scope incompleteness; reducer disagreements correctly exit 1.
This is not a speed comparison. All 119 migration Python tests pass, including
fourteen new reducer tests (`/tmp/clones-compiler-quality-python-tests-final-2.log`).
Style passes with zero violations over 9,714 functions
(`/tmp/clones-compiler-quality-style-1`). Reviewed patterns pass
(`/tmp/clones-compiler-quality-patterns-1`), and diff checks pass. Rust
implementation is unchanged; the C26 full-suite checkpoint remains
the prior Rust validation, not a new full-suite run for this offline-only change.

## C28: reuse compiler origins to remove lowered iterator mutation targets

The existing `rstyle-driver::mutations` expression-use collector now checks the
root binding's HIR span for `ForLoop` desugaring. This constant-time lookup
tags the compiler's generated iterator. The existing core budget evaluator
filters that typed origin before grouping, preserving source bindings regardless
of their spelling. Every mutation event remains available to escaping/delegated
effect propagation and workspace type hints. It adds no parsing, body traversal, type/source proofs,
checksums, artifact scans or graph/body clones. Both legacy style and compiler
advisory reuse that collector. Statement counts and effect propagation retain
their existing semantics; this does not prove iterator purity.

Fresh advisory capture uses compiler-style metric 2. Metric-1 observations and
diagnostic artifacts remain readable and retain their original measurements on
replay, including historical baseline exports. Reports and snapshots carry the
actual observed version. Mixed versions reject before propagation/planning;
cross-version positive reviews stale, numeric negatives cannot retire old
choices, and empty baselines detect metric drift. Shared baseline construction,
comparison and I/O remain reused. Strict clone decisions are unchanged.

One real-Cargo fixture covers automatic/explicit/borrowed/nested iterators,
mutable loop-pattern bindings named `iter`, macro-local mutation, closure
capture, known-empty loops, custom workspace iterators, their borrowed delegated
events/type hints and source-free exact replay. Core regression checks preserve
escaping effects and delegated call propagation for generated roots, while
same-spelled user roots still count. Version tests cover
historical reads, unit consistency, cross-version negatives and positive/empty
baseline drift. The initial focused fixture incorrectly assumed crate-qualified
names in the core group report and failed its lookup; using the actual existing
name representation fixes the harness. All 87 compiler advisory integration
cases then pass (`/tmp/clones-source-mutation-focused-2`).

The first full checkpoint passes all 3,319 tests and doc tests
(`/tmp/clones-source-mutation-tests-1`, nextest 215.548 seconds). Subsequent
read-only review identified that removing generated events could also remove
workspace type hints and borrowed-iterator delegated effects. The final typed
origin seam retains all events and filters only source budgets. The expanded
fixture initially queried a compact name in a full report; correcting that
harness lookup and rerunning passes (`/tmp/clones-source-mutation-focused-final-3`).
Metric 2 requires origins explicitly; metric 1 defaults missing origins to Source
and rejects claims of desugared origins. Final validation follows these changes.

The first origin-based full run caught an incorrect fixture expectation that
the generated borrow of `for value in counter` was delegated. Rustc's blanket
iterator call borrows the generated binding itself here; its flags are false.
That run was interrupted after the failure
(`/tmp/clones-source-mutation-tests-final-1`), not treated as passing evidence.
The corrected fixture asserts those original flags and adds an explicit source
reborrow whose `counter` event is delegated and remains a source budget target.
This focused final fixture passes
(`/tmp/clones-source-mutation-focused-final-4`). Core tests independently check
that generated events marked escaping/delegated retain their effects.

Historical production replay of `run-ynuMSl` preserves the entire advisory and
strict matching output exactly, retaining metric 1 and its C27 disagreements
(`/tmp/clones-source-mutation-legacy-replay-final-1`, exit 2, 16.83 seconds,
1,439,952 KiB RSS). Fresh `run-WbHJ11` captures 1,582 known CLI/core owners;
1,430/735 remain unavailable, with 87 unresolved CLI callees. The unchanged
eleven-function C27 source labels now agree on all twenty-two comparisons at
both explicit limits. At limit one, `select`/`resolve` report exactly their two
source containers. At limit two, neither is a target-budget finding. Both
result artifacts retain their label derivations; the C27 disagreements stay
untouched. Project study records commands and scope/resource limits.

Fresh capture/replay take 56.70/16.48 seconds and 1,790,244/1,453,264 KiB RSS.
Both retain incomplete exit 2. These are distinct workloads, not a speedup
comparison; the measured quality gain is two removed source-policy false
findings. Other generated roots, shadowed-root identity, broad quality and
retirement remain open. All 119 migration Python tests pass
(`/tmp/clones-source-mutation-python-tests-1.log`).

The reviewed pattern gate initially rejected duplicated compiler baseline
construction. Both consumers now reuse `CompilerInventory` with their existing
policy inputs; no new ignore or suppression was added. The final gate passes
with zero candidates and the same fourteen unchanged reviewed exclusions
(`/tmp/clones-source-mutation-patterns-final-2`). Final workspace validation
passes all 3,321 tests, five existing skips and doc tests (nextest 218.045
seconds, `/tmp/clones-source-mutation-tests-final-2`). Final style reports zero
violations and all-target Clippy passes with warnings denied
(`/tmp/clones-source-mutation-style-final-2`,
`/tmp/clones-source-mutation-clippy-final-2`). These changes introduce no hashes,
source rescans or additional proof passes.

## C29: reuse exact compiler-associated dependency targets

Owner/write set: lead owns `clones/advisory/propagation.rs`, test registration,
project labels/results and docs; acceptance worker owns only the new
`tests/migration_compiler_advisory/propagation/associated.rs`; a second read-only
worker reviewed compiler ID and trait-dispatch boundaries. State: delivered.

The adapter previously rejected every dependency definition with `SigFacts`,
even when compiler definition IDs and the existing direct Cargo route identified
one captured body. Removing that redundant exclusion reuses the existing core
resolved graph, effect propagation and planner. No collector, target resolver,
proof algorithm or hashing path was added. Exact `facts.id == callee`, context,
unique-owner and exclusion checks remain. Trait implementation aliases alone
cannot resolve dependency dispatch; open-world/default-trait behavior is outside
this increment.

Four new real-Cargo tests cover static/receiver/generic inherent methods and a
same-named pure method on another owner, unavailable/excluded targets, receiver
argument positions in shared plans, source-free replay and unresolved generic
trait dispatch. The initial fixture incorrectly expected three identical effect
labels to survive core's existing within-caller deduplication and expected empty
resolution maps to serialize as objects. The corrected independent fixture uses
separate callers to test each method and checks omitted empty maps. All fourteen
propagation cases pass (`/tmp/clones-associated-target-focused-3`), including the
unchanged context, ambiguity and missing-density controls. Receiver plan/replay
already passed in the initial run; failed runs are not passing evidence.

Before the production replay, source labels required `Config::parse` and
`Matcher::compile` to resolve. The explicit C28 `run-WbHJ11` replay resolves both
and six other exact core methods. Resolved targets rise 31→39; unresolved targets
fall 87→79. Known/unavailable rosters, unit rule evaluations, shared plans,
functions, unit contexts and strict matching/accepted pairs remain exactly equal.
The sample remains incomplete, exit 2. Results and commands live in
`eval/clones-capability/project-compiler-associated-target-{cases,results}.json`
and `project-study.md`. New replay resources are 17.20 seconds / 1,452,828 KiB RSS;
no speedup or broad quality claim. All-target locked Clippy passes with warnings
denied (`/tmp/clones-associated-target-clippy-1`). Full workspace validation
passes 3,325 tests in 214.862 seconds, five existing skips and doc tests
(`/tmp/clones-associated-target-tests-final-1`). Final style reports zero
violations over 9,717 functions (`/tmp/clones-associated-target-style-final-1`);
formatting and diff checks pass. The reviewed pattern gate passes with zero
candidates and fourteen unchanged reviewed exclusions
(`/tmp/clones-associated-target-patterns-final-1`). No ignore/suppression,
threshold, budget, admission or metric-version changes were needed.

The next ready frontier is independently labeled compiler effect-density quality,
reusing completed core evaluations and the existing offline quality reducer.
Current C27/C28 source labels cover parameters and mutation roots only. Broader
trait dispatch, generated-owner availability, project quality and capability
retirement remain open.

## C30: independently review compiler density measurements

Owner/write set: lead owns existing offline quality/policy reducers, frozen
source labels, explicit study policies/results and docs; independent acceptance
reviewer supplied source-only labels; reducer reviewer owns only the new Python
test file. State: delivered quality evidence; two production discrepancies open.

The existing `compiler_quality.py` now accepts direct-only density samples in
addition to historical parameter/mutation labels. It reuses `legacy_assessment`,
completed core plan owner associations, and the existing density arithmetic and
policy validators. A small policy-validator seam retains the old depth-two
default while this study explicitly requires depth zero. It checks full policy,
exact independently labeled source bytes, known statement observations and
weighted positive event details; known-clean results remain outcome-only.
Missing inputs, source changes, dropped/duplicate owner rosters and unreviewed
source policy cannot become clean numeric evidence. No Rust runtime algorithm,
hash, proof, collector, graph or artifact scan was added.

Five source-reviewed original owners cover filesystem writes, generic reading,
stderr emission, an unclassified leaf and a direct-only wrapper. Three policies
were frozen before report inspection: all positive densities, the 3000 boundary,
and the existing 3-statement/10-line size exemptions. Exact source labeling of
`preceding_comment_line` initially omitted its captured doc comments, making it
unavailable; including the independently reviewed original comments corrected
that harness precondition without changing any numeric label. Every policy now
records three agreements and two statement-denominator disagreements. The event
counts agree: `save_logs` reports two IO events but four statements instead of
two (density 1500 instead of 3000); `collect` reports one IO event but five
statements instead of three (600 instead of 1000). Do not relabel these numbers
to match the producer. A later explicit 2000 threshold probe measures the
finding impact of the same frozen source discrepancy: `save_logs` incorrectly
has no finding instead of the expected density3000 finding.

Read-only pinned-lowering inspection identifies two synthetic bare match arms
per `?`. Their body spans differ, but both arm spans carry
`DesugaringKind::QuestionMark`. The current collector counts both before body
traversal. The next ready semantic fix is to exclude these synthetic arm counts
while preserving traversal, calls and authored operand statements. It needs a
new compiler metric version with versions 1/2 replay preserved, not a hash or
another source/proof pass.

All 134 Python acceptance tests pass
(`/tmp/clones-compiler-density-python-final-2.log`), including fifteen new density
controls and unchanged historical parameter/mutation contracts. The Rust
snapshot is unchanged from integrated-green `cd330e9`; this evidence increment
does not claim another Rust full-suite run. The saved C28 capture `run-WbHJ11`
retains metric 2 and incomplete scope/exit 2 for every replay. Study commands,
resource values and frozen disagreements are recorded in the project study.


## C31: correct advisory try-arm statement counts in the existing collector

Owner/write set: lead owns extraction options, the shared collector, metric
registration, compatibility tests, source-label derivatives and docs. Read-only
review confirmed the seam changes only statement contributions in the existing
walk. State: delivered; integrated checkpoint passes.

Advisory metric 3 excludes the two pinned `QuestionMark` synthetic bare-arm
contributions per `?`; authored arms, operand blocks and macro IO remain
collected. A small borrowed options seam selects frozen legacy versus corrected
advisory semantics at the two existing extraction entry points. No extra walk,
source/type/body proof, hash, artifact scan or graph/body clone was introduced.
The initial global correction exposed 44 legacy density findings; those remain
quality debt before retirement. Preserving the original legacy metric is not a
claim that these corrected-metric findings were fixed. No thresholds, budgets,
suppressions or ignores changed. The helper extractions only name the existing
trait identity lookup and optional graph publication.

Four new real-Cargo tests cover one/multiple question marks, authored arms,
operand blocks, macros, source-free replay and legacy 4/advisory 2 counts with
identical call/effect/edge/mutation/parameter/type observations. Versions 1/2
remain readable with unchanged statement counts; versions 2/3 still require
mutation origins. Cross-version reviews, numeric negatives and even empty
baselines remain distinct. The focused 96 CLI cases and core 7 cases pass; all 134
Python acceptance cases pass. Locked all-target Clippy and formatting/diff
checks pass. Legacy style is zero over 9,719 functions; reviewed patterns is zero.

Fresh metric 3 `run-bS3zMn` evaluates the same bounded two-package scope while
retaining 1,582 known / 2,165 unavailable owners and incomplete scope / exit 2. All five
frozen C30 source labels agree under all four policies. `save_logs` now has
statements 2 / density 3000; `collect` has statements 3 / density 1000; direct events
are unchanged. Threshold 2000 recovers the expected `save_logs` finding.
C30 artifacts remain frozen; the new `project-compiler-try-density-*` cases
change only run/metric identity plus a derivation note. Historical metric 2 replay
preserves exact advisory (including planning), edit-planning and strict matching
output. Capture/replay resources and explicit commands are in `project-study.md`;
there is no speedup, complete-project quality or strict-admission claim.

Broader transitive density quality, other desugarings, generated-owner coverage,
legacy corrected-metric quality debt and per-capability retirement remain open.


Final frozen runtime snapshot: 3,330 tests pass in 216.417 seconds with five
existing skips; doc tests pass (`/tmp/clones-try-metric-tests-final-1`). The two
previously reported nonzero source/caller tests both pass. Locked all-target
Clippy passes (`/tmp/clones-try-metric-clippy-1`); formatting and diff checks pass.
Style reports zero violations over 9,719 functions
(`/tmp/clones-try-metric-style-3`), reviewed patterns reports zero candidates
(`/tmp/clones-try-metric-patterns-1`), and unfiltered patterns preserves all
fourteen reviewed IDs (`/tmp/clones-try-metric-patterns-unfiltered-1`). Python
acceptance remains 134 passing tests (`/tmp/clones-try-metric-python-1.log`).
These are Linux checkpoint results; no macOS gate is claimed.


## C32: review inherited production density without another evaluator

Owner/write set: lead extends the existing offline compiler quality reducer,
labels/policies/results and docs; independent acceptance worker reviews source
expectations; reducer reviewer owns only the new Python test leaf. State:
delivered bounded quality evidence. Runtime Rust snapshot remains `53e1704`.

The existing compiler collector and core propagation remain the implementation.
The reducer accepts depths 1/2 policies through existing validators, reuses source
owner association for manually reviewed callee inventories, and compares exact
positive weighted detail labels/counts as well as statement/event/numeric labels.
It performs no new propagation, collector, source/type/body proof, hashing,
artifact scan or runtime graph/body clone. Missing or ambiguous sampled callees
stay unavailable; global scope/planning incompleteness is retained. Manual
`propagation_reviewed` is an acceptance assertion, not automatic closure proof.

Four independently labeled production owners (`collect`, `tee`, `emit`, and
`StderrTee<R>::read`) agree at each of depths 0/1/2. `tee` gains one IO effect via
`collect` (density 3000); the read method gains one collapsed IO effect via `emit`
(density 1666 → 2666), retaining two potential mutable-reborrow targets. No runtime
callback edges or actual-write guarantees are inferred. Initial depths 1/2 exact
labels predicted internal stdio paths and the wrong pre-order collapse base;
independent source inspection corrected only rendered label spelling/order.
All numeric/source labels remained frozen. Both initial discrepancies and their
correction are recorded with retained inputs/results in `project-study.md`.

All 152 Python tests pass (`/tmp/clones-propagated-density-python-final-1.log`), including 18 new controls for provenance, collapsed
counts, policy, missing/ambiguous inputs, unknowns and clean outcomes without
invented numbers. The four positive detail rosters are compared exactly; `+N
more` remains descriptive rather than additional weighted events. No Rust code
changed: the preceding checkpoint's 3,330 passing tests, clean Clippy/style and
fourteen unchanged pattern IDs remain evidence for that exact runtime snapshot,
not a newly claimed Rust test run. Formatting/diff checks pass. Replay resources
and exact saved capture/policies are in the project study; no speedup is claimed.

Next ready frontier: broaden compiler proposal quality with independent original
project source labels while reusing the existing proposer and completed reports.
Generated owners, more dispatch/call shapes, complete project quality and
per-capability retirement remain open.


## C33: review production abstraction proposals from completed reports

Owner/write set: lead owns the offline reducer, cases/results and docs;
acceptance worker independently reviewed original source; reducer reviewer owns
the new Python test leaf. State: bounded quality evidence delivered; runtime Rust
snapshot remains `53e1704`.

Reuse-first: `compiler_proposal_quality.py` consumes existing `patterns_report`
output and reuses original-source association from `compiler_quality`. Exact
family and separate role sets match labels; candidate fingerprints are opaque.
All raw candidates, scores and counter-evidence survive independently of visibility.
No collector, proposer, hashing, artifact scan, source/type/body proof or runtime
graph/body clone was added.

The fresh default-library `rstyle-core` capture `run-OFIAA9` has 383 available /
735 unavailable owners and seven proposals. Three independently pre-labeled
negative groups are emitted: already-shared `mirrored` traversal wrappers,
distinct normalization pipelines and different dedup contracts. All three exact
structural associations agree; all three remain independently not useful. Four
other proposals remain unreviewed. Incomplete scope/input and reducer exit 1 are
explicit; no new ignores, suppressions, threshold/budget changes or quality-gain
claim. `project-study.md` records the initial unsupported `--lib` attempt and
corrected command, 15.79s / 585,692KiB capture, sources and reproduction.

All 169 Python tests pass, including 17 new controls for roles, opaque IDs,
visibility, unknown/foreign graph owners, original source, contexts, policies and
incomplete absence. Formatting/diff checks pass. No Rust runtime changes: prior
3,330 passing tests, zero legacy style/pattern findings and unchanged unfiltered
pattern IDs remain evidence for that exact checkpoint, not newly executed gates.
Next ready frontier: improve proposal usefulness through existing core policy,
with independent useful positives and these frozen production negatives. Broad
project quality, generated-owner coverage and per-capability retirement stay open.


## C34: share existing recorded-effect warnings with helper proposals

Owner/write set: lead owns core integration, real-Cargo test and docs/embedded
skill; acceptance worker independently reviewed the existing policy seam and
controls; test worker owns the new core test leaf. State: delivered; integrated checkpoint passes.

`parameterize` and `generic_fn` reuse the existing borrowed `effect_mismatch`
helper; no new collector/evaluator/proof/hash or graph/body clone. Candidate
IDs, scores, thresholds, emission, role membership and strict matching stay
unchanged. Existing trait mismatch damping stays unchanged. Empty classes now
say no recorded intrinsic or escaping-mutation effects, rather than pure.
Calls, configured classifications, delegated mutation and propagation remain
outside this comparison; an absent warning proves no common semantics.

Five new core controls cover both families; all 33 candidate tests pass. A new
real-Cargo test confirms that an intrinsic unsafe-class difference adds warning
text without changing the proposal and retains exact saved replay. The production
C33 replay takes 4.41s / 443,248KiB and preserves the entire advisory
and strict report objects exactly: all seven candidates still have no recorded class
mismatch, so the three independently unsuitable groups remain quality debt.
The project study records exact commands and limitations; no speedup or measured
project precision improvement is claimed.


Final C34 checkpoint: all 3,336 tests pass in 215.521 seconds, with five existing
skips, and doc tests pass (`/tmp/clones-helper-effects-tests-final-1`). Both
previously reported nonzero tests pass. Locked all-target Clippy passes
(`/tmp/clones-helper-effects-clippy-1`); formatting/diff checks pass. Style reports
zero violations over 9,719 functions (`/tmp/clones-helper-effects-style-final-1`),
reviewed patterns reports zero candidates (`/tmp/clones-helper-effects-patterns-final-1`),
and unfiltered patterns preserves exactly the fourteen reviewed IDs
(`/tmp/clones-helper-effects-patterns-unfiltered-final-1`). All 169 Python tests
pass (`/tmp/clones-helper-effects-python-final-1.log`). These are Linux results;
macOS was not run locally. No ignores, suppressions or thresholds changed.

Next ready frontier: use completed alignment coverage to correct proposal text
that currently claims functions differ only at the recorded holes despite
unmatched graph nodes. Template/member alignment is not all-pairs coverage or
semantic equivalence. Reuse existing Cluster/Breakdown fields without another
source/graph walk; retain scores, IDs, memberships, strict matching and these
production negative labels. Broad quality and retirement remain unproven.


## C35: retain unmatched behavior in abstraction advice

Owner/write set: lead owns core presentation integration, live-Cargo test, golden
snapshot and docs/embedded skill; acceptance worker independently specified
existing coverage semantics; test worker owns only the new core test leaf.
State: delivered; integrated checkpoint passes.

The shared candidate completion path uses the already-computed mean node
alignment coverage. Partial coverage adds unmatched-node counter-evidence and
requires preserving work outside the aligned graph regions in the refactor
sketch. Cluster observations describe aligned regions and recorded substitutions;
zero substitutions no longer claims an exact clone. Helper/generic inference is
conditional. There is no new graph/source walk, collector, alignment algorithm,
proof or hashing. Candidate IDs, order, membership, scores, thresholds,
existing recorded-effect notes and strict matching remain unchanged.

Five core controls cover both helper families, independently specified means
900/933/999, full/partial zero substitutions, retained unknown-effect warnings
and the trait completion path. A live-Cargo helper with extra work
outside the aligned part remains emitted with the caveat; saved replay and
strict results are equal. Full integrated tests below cover the final snapshot.

The C33 saved production run replays in 4.44s / 443,212KiB. Expectations frozen
from its completed coverage before implementation predict warnings for five of
seven proposals; all five are observed, including the three independently
not-useful groups. Every non-presentation proposal field, all other advisory
data and strict matching remain exactly equal. The three source usefulness
judgments stay negative; adding honest limits does not make those proposals
useful. Scope/input remain incomplete, replay exit 2 and quality reducer exit 1.
The new compact results preserve all seven proposals and C33 labels unchanged.

Next ready frontier: use the existing core planning/effect outputs for further
quality work and independently useful project cases. Partial-alignment wording
is not general source-fragment discovery, safe extraction or retirement.


Final C35 checkpoint: all 3,342 tests pass in 216.255 seconds with five existing
skips; doc tests pass (`/tmp/clones-alignment-evidence-tests-final-1`). Both
previously reported nonzero tests pass. Locked all-target Clippy passes
(`/tmp/clones-alignment-evidence-clippy-final-1`); formatting/diff checks pass.
Style reports zero violations over 9,720 functions
(`/tmp/clones-alignment-evidence-style-final-1`). Reviewed patterns reports zero
candidates (`/tmp/clones-alignment-evidence-patterns-final-1`); unfiltered patterns
preserves exactly the fourteen reviewed IDs
(`/tmp/clones-alignment-evidence-patterns-unfiltered-final-1`). All 169 Python
tests pass (`/tmp/clones-alignment-evidence-python-final-1.log`). These are Linux
results; no macOS gate was run locally. No ignores, suppressions, thresholds or
budgets changed.

The next bounded effect handoff should retain and move the effects vector already
computed in core evaluation, independently of owner/scope retention, then borrow
same-unit results by exact original owner ID after proposal preparation. Missing
results cannot become empty effects. An explicit compiler-only seam must retain
policy/metric/context provenance: current pattern reviews record clustering
policy, so configured/propagated evidence must not silently inherit policy
invariance. Independent ready controls compare `Path::is_file` versus lexical
`is_absolute`, equal-IO `is_dir`, and helper propagation at depths 0/1. These are
acceptance targets, not delivered scope; the seven C33 production proposals are
not expected to recover useful effect contrast. Broad quality/retirement remain
open.


## C36: reuse completed core effects for compiler helper warnings

Owner/write set: lead owns core opt-in retention, shared effect-kind comparison,
compiler handoff/report integration, registration and documentation; test worker
owns two disjoint core test leaves; acceptance worker owns the live-Cargo leaf.
State: delivered; integrated checkpoint passes.

The existing evaluator moves its completed effect vectors into an opt-in
original-owner map after using them for diagnostics. Default APIs retain none.
Compiler patterns borrow those same-unit results after the existing proposer;
helper/generic `core-policy effect mismatch` notes compare recorded kinds,
including configured and inherited calls. Existing intrinsic warnings reuse the
same comparator, with borrowed static kind names instead of allocated strings;
identical recorded warnings are not duplicated. No additional classification,
propagation, source/graph walk, body proof, hash, or graph/body copy is introduced.
Only small owner-ID keys are copied when retaining the moved vectors.

Effect evidence references the containing advisory report's effective config,
metric and compilation unit; availability is separate from graph completeness.
Missing result entries error. Unavailable owners, unresolved workspace callees,
incomplete propagation or capture keep effect-input completeness false. Empty
recorded kinds do not establish purity, even with complete declared inputs.
Annotations are display-only; proposal IDs, scores, order, membership, structural
reviews/baselines and strict matching remain unchanged. Reviews do not assert
that recomputed effect annotations are invariant under policy changes.

Independent controls target direct `Path::is_file` versus lexical `is_absolute`,
equal-IO `is_dir`, and inherited disk/lexical wrappers at depths0/1; exact IDs,
unknown/missing results, moved allocations and source-free contextual replay.
Production replay will retain the C33 negative usefulness labels; do not infer
precision improvement from additional counter-evidence.


Focused evidence: all 293 core tests pass
(`/tmp/clones-completed-effects-core-2`); the four new live-Cargo controls pass
(`/tmp/clones-completed-effects-cli-3`). The first CLI attempt exposed a test
expectation error: an all-target capture has an unavailable generated test
harness, so its test unit and replay must remain incomplete/exit2. Replay also
retains the original scope and correctly rejects a new `--all-targets` flag;
the test now uses saved selection. Neither correction changed production code.
All 169 Python tests pass (`/tmp/clones-completed-effects-python-1.log`). Style
reports zero over 9,726 functions (`/tmp/clones-completed-effects-style-1`).

C33 production replay completes in 4.48s / 443,604KiB
(`/tmp/clones-completed-effects-project-replay-1`), compared with C35's
4.44s / 443,212KiB; these single runs do not establish a speed change. All existing
report fields are exactly equal after removing the new `effect_evidence` field,
including all seven proposals and strict matching. Its 383 known / 735 unavailable
owners remain incomplete, exit 2; the quality reducer retains all three negative
usefulness labels and expected exit 1. The compact effect results reference the
unchanged quality inventory. This scope has no new policy-effect mismatch; do
not claim recovered useful production proposals or retirement.


Final C36 checkpoint: all 3,362 tests pass in 216.846 seconds with five existing
skips; doc tests pass (`/tmp/clones-completed-effects-tests-final-1`). Both
originally reported nonzero tests pass. Locked all-target Clippy passes
(`/tmp/clones-completed-effects-clippy-final-1`); formatting and diff checks pass.
Style reports zero violations over 9,726 functions
(`/tmp/clones-completed-effects-style-final-1`). Reviewed patterns reports zero
candidates (`/tmp/clones-completed-effects-patterns-final-1`), and unfiltered
patterns preserves exactly the fourteen existing IDs
(`/tmp/clones-completed-effects-patterns-unfiltered-final-1`). All 169 Python
tests pass. Linux only; no local macOS claim. No ignores, suppressions, thresholds,
budgets, hashing or strict semantics changed.

Next ready frontier: improve the independently reviewed production proposal
quality debt through existing core proposer policy and source-labeled cases;
keep configured-effect evidence borrowed from the completed evaluation. General
refactoring safety, broad project coverage and per-capability retirement remain
unproven. This effect handoff does not establish those outcomes.


## C37: reject call-only pipelines through the existing proposer

The independently source-reviewed `normalize::{unify_case,counter_loops}` pair
contains different ordered transformation stages. Proposal policy 2 rejects the
fallback helper when every retained alignment consists entirely of distinct,
single-use callee substitutions and parameter/return glue. The existing WL
call-count pass also collects this small shape flag; the existing cluster consumes
completed alignment holes. No hash, body proof, new traversal, graph copy, threshold
or strict-admission change is introduced. Common calls, repeated substitutions,
unknown callees, additional operations and trait/generic proposals remain covered
by independent controls.

The built-in decision is explicit in `evaluation.suppressed`, with its original
source sites and reason. No user ignore or suppression annotation is added.
Compiler proposal detector version 2 isolates the semantic policy: version-1
reviews remain readable but stale, and old proposal baselines require explicit
regeneration. Graph schemas, compiler metrics and strict matching stay unchanged.

Saved run `run-OFIAA9` still has 383 known and 735 unavailable owners. Replay
(`/tmp/clones-forwarded-pipelines-project-replay-1`) takes 4.45s and 443,360KiB RSS,
exiting 2 for incomplete scope. Six remaining candidates are exactly unchanged;
the seventh becomes the explicit policy decision and the finding count falls
by one. All other report fields, including strict results, are exactly equal.
`project-compiler-proposal-forwarding-{cases,results}.json` preserves the three
independent source judgments: one explicit rejection and two emitted not-useful
proposals. The quality reducer intentionally exits 1. Absence over incomplete
input does not count as successful rejection. No speedup or whole-project quality
claim follows.

Final C37 checkpoint: all 3,377 tests pass in 217.011 seconds with five existing
skips; doc tests, locked all-target Clippy, formatting and diff checks pass.
Style is zero over 9,728 functions (`/tmp/clones-forwarded-pipelines-style-final-2`);
reviewed patterns is zero (`/tmp/clones-forwarded-pipelines-patterns-final-1`).
Unfiltered patterns falls from fourteen to thirteen, removing only the intended
`8758139492e7` pipeline (`/tmp/clones-forwarded-pipelines-patterns-unfiltered-final-1`).
The existing ignore ledger is unchanged. All 182 Python tests pass. Full test
and Clippy logs use `/tmp/clones-forwarded-pipelines-{tests,clippy}-final-1`;
Python log is `/tmp/clones-forwarded-pipelines-python-final-1.log`. Both original
NonZero CI regressions pass. This verifies Linux; macOS is not locally verified.

Remaining source-reviewed proposal debt is the opaque dedup closure contracts
and already-shared mirrored reductions. No defensible new filter follows from
the available completed facts; further policy must preserve useful wrappers
and needs independent positive/negative cases. Broader capability and retirement
gates remain open.


## C38: owner-local direct-density review completion

At effect propagation depth zero, a known original owner's core numeric density
assessment needs no callee propagation. The retained-review adapter now uses that
completed measurement even when unrelated owners or workspace callees remain
unavailable. It reads the existing typed policy once and borrows the completed
owner/snapshot rows. Capture or handoff failures still block the negative, as do
missing queried owners, suppression/exclusion, unselected files, metric/context
or policy drift. Positive findings and depth-greater-than-zero completion gates
are unchanged. No collector, propagation pass, hash, proof, graph copy or metric
version is added.

Five live-Cargo tests cover read-only status, dry-run and explicit source-free
pruning, baseline resolution without incomplete-scope export, unresolved generated
callees, inherited-depth control and the existing review gates. One initial
context expectation was corrected: with incomplete identity scope and no exact
context, the existing workflow reports unavailable rather than context_changed.
No production logic changed to satisfy that assertion. Focused tests pass
(`/tmp/clones-direct-density-cli-3`).

`project-compiler-direct-density-review-cases.json` freezes a real-project
experiment before capture. A temporary tracked copy adds one nested write to
`save_logs`: three IO events / two authored statements = density4500 at limit3000.
The frozen C31 original has two writes / two statements = density3000, a numeric
negative at the same policy. The temporary variant is a declared experiment,
not a claimed historical production revision. Its genuine captured finding is
recorded only in a temporary review store; no repository ignore/review is added.
The current side reuses explicit metric3 saved run `run-bS3zMn`. Global incomplete
scope and exit2 persist.

Production comparison is verified against a binary built with the C37 adapter
(`/tmp/clones-direct-density-project-before-1`) and the current adapter
(`/tmp/clones-direct-density-project-current-1`). Identical saved input, policy
and temporary review store change unavailable/zero eligible to no_finding/one
eligible. The ONLY changed top-level sections are review_status and review_prune;
all raw advisory, planning, strict, coverage and context data are exactly equal.
Current source scope retains 1,582 known / 2,165 unavailable owners. The prior
variant capture is `run-pjNQal`, 57.61s / 1,793,136KiB; the review publication
replay is 16.38s / 1,466,452KiB. Before/after status replays are 16.23s /
1,453,636KiB and 17.00s / 1,453,132KiB. Every operation exits2 for global
incompleteness. These single timings establish no speedup. The measured result
artifact preserves both statuses/prune counts and resource observations.

Final C38 checkpoint: all 3,382 tests pass in 219.106 seconds, five existing
skips, and doc tests pass (`/tmp/clones-direct-density-tests-final-1`). Both
original NonZero regressions pass. Locked all-target Clippy, formatting and diff
checks pass (`/tmp/clones-direct-density-clippy-final-1`). Style remains zero
over 9,728 functions (`/tmp/clones-direct-density-style-final-1`); reviewed
patterns remains zero (`/tmp/clones-direct-density-patterns-final-1`), with all
thirteen unfiltered IDs unchanged (`/tmp/clones-direct-density-patterns-unfiltered-final-1`).
All 182 Python tests pass (`/tmp/clones-direct-density-python-final-1.log`).
Linux is verified; macOS and per-capability retirement remain pending. No new
hashes, source/body/type proofs, graph scans/copies, ignores, thresholds, budgets
or strict semantics are introduced.

Next ready candidate: concrete trait-method targets in the existing compiler
effect collector. The current collector records declaration IDs; the existing
compiler Instance selection query can supply a selected ordinary implementation
for concrete calls without a second traversal. First freeze actual production
positive/generic/dynamic/unselected controls and preserve historical metrics.
A separate original-source coverage gap affects workspace-shared files outside
a member package root; it needs a small location-root seam, not another collector.
These are pending candidates, not delivered capabilities or blanket permission
to broaden strict extraction.


## C39: reuse concrete compiler-selected trait implementation edges

The existing Rust effect collector records trait declaration IDs, losing exact
implementation edges for concrete calls. The compiler extraction path already
uses Instance selection. A small shared `ids::selected_instance` seam reuses
that query in the existing owner/typeck context; strict extraction preserves the
same query and result handling. The existing single collector walk uses it only
for advisory concrete trait calls with no non-region parameters, inference or
escaping bound variables and an ordinary Item result. Method calls use captured
node arguments; UFCS calls use the callee FnDef arguments. Receiver removal uses
the original signature. Existing external path policy and unsafe-call classification
also use the original signature, while the graph edge uses the selected impl.

Collector options now name the cohesive legacy/advisory metric choice instead
of a statement-only flag. Metric4 records the changed call-edge semantics;
metrics1–3 remain readable without recomputation or reinterpretation. Historical
mutation-origin requirements remain intact. Existing exact metric comparisons
make retained review negatives unavailable across versions and baseline scopes
drift even for empty inventories. Legacy collectors and strict matching are unchanged.

Five independent live-Cargo tests cover concrete method/UFCS/default calls,
namesake implementations, effect counts, shared parameter plans and receiver
mapping, source/tool-free replay, generic/dynamic declarations, omitted/generated/
excluded implementation owners and configured external-trait effects. They reuse
the existing propagation fixture and consumers; no new graph algorithm is added.
Default methods whose selected ID remains the trait declaration retain existing
alias routing. This does not claim general exact dispatch or broaden strict admission.

`project-compiler-concrete-trait-cases.json` freezes two authored Default-call
labels plus generic/dynamic controls before fresh capture. On run `run-iI2odI`,
SideEffectsConfig::default now records the original Weights Default implementation
edge, and density Policy::default records a resolved direct-dependency edge to
the original SideEffectsConfig Default implementation. Caller and target authored
sources match saved metric3 run `run-bS3zMn`; that run had neither edge. The
source-reviewed Weights leaf has no calls or escaping effects. Generic Decision
edges and the boxed dyn Fn call events remain equal to baseline. Overall owner
rosters differ because source changed, so aggregate totals are not a controlled
gain comparison. The fresh capture has 1,591 known / 2,166 unavailable owners,
remains incomplete/exit2 and takes 58.93s / 1,787,796KiB. This measures edge/owner
coverage, not project IO/planner improvement or speedup. The compact result
artifact records the actual edges and these limits. No hashes, repeated body/type/
source proofs, artifact scans, body/graph copies or second traversals are added.

Final C39 checkpoint: all 3,387 Rust tests pass in 219.230s, five existing skips,
and doc tests pass (`/tmp/clones-concrete-trait-tests-final-1.log`). Both original
NonZero regressions pass. Locked all-target Clippy, fmt and diff checks pass.
Style is zero over 9,730 functions; reviewed patterns is zero and all thirteen
unfiltered candidate IDs match C38. All 182 Python tests pass. Logs use
`/tmp/clones-concrete-trait-{clippy,style,patterns,patterns-unfiltered,python}-final-1`.

Historical saved metric3 run `run-bS3zMn` replays under the exact prior density
policy and retained review/prune arguments. Every top-level report section equals
C38 output `/tmp/clones-direct-density-project-current-1/stdout.log`, including
raw observations, numerical evaluations, plans, review lifecycle, matching and
accepted pairs. It remains metric3/incomplete/exit2, with no relabeled bundle.
Evidence is `/tmp/clones-concrete-trait-history-1`; the result artifact records
this comparison. Linux is verified; macOS and per-capability retirement remain
pending. No new ignores, suppressions, budgets, thresholds or strict semantics.

Next ready candidate: the original-source location-root seam for workspace-shared
files outside a member package root. It should reuse the same collector and
preserve package/build ownership, with frozen outside-package and foreign-file
controls before implementation. Broader capability and retirement work remains active.


## C40: reuse workspace source locations in compiler advisory collectors

State: delivered. Lead owns the capture/root seam, metric compatibility,
consumer path normalization, shared registration and integration. Independent
acceptance owns the new `propagation/shared_sources.rs` leaf; project review owns
`project-compiler-shared-source-cases.json`. Workers are frozen before gates.

The pre-fix tests exposed two different behaviors: relative shared modules already
produce facts and exact edges, but retain `caller/src/../../shared/leaf.rs`, so a
`shared/**` exclusion misses them. Absolute `#[path]` files outside the package
root are unavailable. The Cargo absolute target route alone is already reduced
by Cargo/rustc to a usable relative source location; it is explicitly a nonregression
control, not a claimed gain. These findings refine the initial root-gap hypothesis.

The capture wrapper now passes its existing declared workspace root to advisory
extraction once per compiler invocation. The root is borrowed through capture
options into the existing collector options; no root copy per function is added.
Package/build/selected-unit identities remain unchanged. A requested dependency
outside the workspace retains its existing package-root scope. The existing
`nominal_binding::absolute_path` lexical helper is exposed and reused for
advisory compiler location membership and presentation. No canonicalization,
source reread, hash, body/type/source proof, artifact scan, second traversal or
body/graph copy is introduced. Location availability is checked before calling
body collectors, avoiding work for owners that cannot supply a location.
Legacy collector membership and advisory approximate graph behavior retain
their existing legacy choice; strict extraction/matching is unchanged.

Metric5 records the changed membership/path semantics. Metrics1–4 remain readable
and preserve captured paths/measurements on replay. Unavailable-only historical
bundles contain no observed metric version; their existing effective header
fallback uses the current metric. Existing origin requirements
and mixed-metric rejection remain intact; metric drift makes retained negatives
unavailable and baseline scopes different, including empty inventories.

Seven live-Cargo tests cover relative and absolute shared modules, normalized
workspace exclusions, existing effect propagation and parameter planning,
source/tool-free replay, generated and outside-workspace owners, check/test
contexts and explicitly requested outside-workspace dependencies. The test
harness remains unavailable rather than being counted as an authored function.
Focused propagation gates pass before full validation. The first full run was
intentionally interrupted to add the external-dependency boundary case; its
SIGTERM log is not a passed checkpoint (`/tmp/clones-shared-source-tests-final-1.log`).

The bounded real-source experiment copies tracked revision `a90e8ac`, adds only a
routing package and a one-line absolute module declaration referencing the
unchanged tracked byte-window oracle and arch-readers. This models the existing
rstyle-pdg test integration without capturing the whole package. Authored source
bytes and contextual owner roster are equal across versions. Metric4 baseline
`run-nNoP0x` has 0 known / 30 unavailable owner-contexts; metric5 `run-mb8gH8` has
29 known / 1 unavailable (11 ordinary library + 18 test authored functions;
the generated harness remains unavailable). Original caller_arch -> batch_arch
and the composition test's six source-declared targets are now available to the
existing graph consumers. Strict matching and accepted pairs are exactly equal.

Relative-route exclusion controls evaluate 29 functions before and zero after
with the same `eval/coverage-development/**` policy; raw owner observations are
retained. The absolute-route exclusion also evaluates zero after. Four observed
test-context diagnostics remain unreviewed quality data, not labeled useful
findings. Both primary captures exit2 for incomplete scope and the explicit zero
pair budget. Baseline is 1.02s / 153,744KiB and current is 1.07s / 154,076KiB;
these single small measurements establish no speedup. Exact routes, labels,
context counts and limitations are recorded in the new cases/results artifacts.

Historical metric4 known-source run `run-qDzS6J` replays with all report
sections exactly equal under the prior and current binaries, retaining its old
paths, exclusions and 29 evaluated functions. Unavailable-only `run-nNoP0x`
has no measured metric provenance: its effective header changes from4 to5 through
the existing fallback. Every other report field is equal. No historical version
is inferred from extra source reads or hashing. The result artifact records both
checks and this limitation.

The full Rust suite passes 3,394 tests in 219.599s with five existing skips,
including both original NonZero regressions; doc tests pass. All 182 Python tests
pass. The initial style gate found a missing phase break and callback effect
density; configuration acquisition now occurs once beside invocation setup,
with the owned root borrowed during analysis.

Final frozen-code checkpoint: 3,394 Rust tests pass in 220.350s, five existing
skips, with doc tests passing (`/tmp/clones-shared-source-tests-final-3.log`).
Both original NonZero regressions pass. Locked all-target Clippy, fmt and diff
checks pass. Style has zero violations over 9,731 functions; reviewed patterns
has zero candidates. The thirteen unfiltered candidate IDs exactly match C39.
All 182 Python tests pass. Logs use `/tmp/clones-shared-source-` with
`clippy-final-2`, `style-final-2`, `patterns-final-1`,
`patterns-unfiltered-final-1` and `python-final-1` suffixes. Linux is verified;
macOS remains pending. No new ignores, suppressions, thresholds, budgets,
hashes, source proofs or strict semantics.


Next ready work: independently label the newly reachable shared-source findings
and compare existing evaluator/planner outputs in a bounded real-package scope.
Reuse completed owner associations and core policies; no new consumer algorithm,
source checksum or second body walk is required for that review. Broader
capability delivery and per-capability retirement remain open.


## C41: review completed shared-source findings by compilation context

State: delivered. Acceptance owns only the existing offline
`compiler_quality.py` reducer and its leaf tests; independent source review owns
`project-shared-source-quality-cases.json`. Lead owns integration, measured result
artifact and documentation. The source reviewer freezes expectations before
inspecting diagnostic output. Heavy captures remain serialized.

C40 made original shared-file observations reachable but did not label the four
observed spacing diagnostics. Reuse the existing compiler-quality reducer and
completed core fix groups to review those diagnostics. The reducer previously
selected only check units and parameter/mutation/density planning rules; a small
input seam now selects explicit check/test contexts and spacing labels. This
adds no evaluator, parser, collector, source hash or live analysis pass. Existing
labels keep their default non-test selection. Missing/ambiguous owners, source
associations and incomplete diagnostic ownership remain unavailable or errors;
clean outcomes never invent numeric measurements.

The study uses the explicit C40 saved metric5 run `run-mb8gH8`, preserving its
routing-only scope and generated-harness incompleteness. Independent labels
review source spacing and parameter counts under the default policy; they make
no density/purity, safe-refactor, full-package quality or retirement claim.
Source labels froze four spacing findings (13/8/10/5 top-level statements,
limit4) plus six clean spacing controls. Ten parameter and ten mutated-target
checks are also negative under production defaults. The first measured reduction
returned 18 agreements/12 unavailable because four label strings omitted their
authored `#[test]` attribute. The independent reviewer corrected the source span
metadata against the original files without changing any expected count/outcome.
Exact byte matching was retained. Final reduction reports 30 agreements across
seven original functions/ten owner-contexts. Clean observations contain outcome
only; no measured zero is synthesized. The four completed spacing groups have
no signature steps or bundles; the forwarding reader receives no parameter plan
under the default limit. Global advisory/planning completeness remains false.

The existing reducer now builds a per-unit index from completed core spacing
fix-group locations, validating one-to-one diagnostic ownership in linear work.
The optional boolean test selector defaults to non-test for older labels and is
part of sample identity, propagation-input identity and result rows. The new
controls cover absent/ambiguous contexts, same-named check/test owners, source
mismatches, missing/duplicated locations and an unmatched diagnostic. All 191
Python tests pass. The completed-report reduction takes 0.02s/13,196KiB in one
small observation; this is not a speedup or live-capture benchmark. Artifacts are
`project-shared-source-quality-{cases,results}.json`; reproduction is in the
acceptance README.

Final checkpoint: all 3,394 Rust tests pass in 219.546s with five existing skips;
doc tests pass. Both original NonZero regressions pass. All 191 Python tests,
locked all-target Clippy, fmt and diff checks pass. Style has zero violations
over 9,731 functions; reviewed patterns has zero candidates. Logs use
`/tmp/clones-shared-source-quality-{tests,python,clippy,style,patterns}-final-1`
(the final Python run is `python-final-2`). All thirteen unfiltered candidate IDs match C40
(`/tmp/clones-shared-source-quality-patterns-unfiltered-final-1.json`). Linux is verified; macOS remains pending. No changes to Rust
collectors, metric versions, strict matching, hashes, ignores, suppressions,
thresholds or budgets.

Next ready work: use the existing source-rule and compiler-quality paths to
measure a declared original-package scope, with independent labels for remaining
unreviewed owners. Keep proposal quality and strict matching separate. Broader
migration and per-capability retirement remain active.

### Main rebase and macOS fixture compatibility (2026-10-29)

Rebased the 84 feature commits onto main `1689392`, including its shared
cyclomatic/cognitive metrics and native PDG transport updates. Kept main's
complexity diagnostics, report ranking and configured limits. The shared HIR
collector now uses one local owner ID for both concrete-call selection and
recursion detection; complexity scoring is a small helper around main's existing
scoring path. Advisory complexity remains unavailable rather than claiming a
measurement from missing input. No new hashes, source proofs, extra collector
passes, strict-admission changes, suppressions or threshold changes.

The latest failed macOS run `40793046330` had two fixture path mismatches:
`default_target_exclusion_can_be_disabled_from_a_saved_generated_source` supplied
an unresolved absolute temporary path while Cargo saved a resolved workspace
root, and `member_proposal_locations_use_workspace_source_root_separately_from_package_root`
expected an unresolved root. Fixtures now resolve their generated source or
expected root once, accounting for macOS `/var` versus `/private/var`. Production
lexical matching and replay remain unchanged, with no additional filesystem work.

All 3,407 Rust tests pass in 217.466s, with five existing skips; doc tests and all
191 Python tests pass. Locked all-target Clippy, formatting and diff checks pass.
The post-rebase style scan has zero violations over 9,763 functions, all with
complexity measurements; reviewed patterns has zero candidates and thirteen
reviewed entries. Twelve unfiltered candidate IDs match the pre-rebase baseline;
main expanded the remaining `shape_ok` group with `CountFact` and already carries
its reviewed ledger entry. No new ignores were added. Logs are
`/tmp/clones-main-rebase-*`.
Both macOS regressions pass on Linux; macOS execution awaits CI after the push.
This checkpoint integrates main and repairs fixtures; it makes no new migration
coverage or performance claim.

### C42 — reuse shared complexity measurements (2026-10-29)

Compiler advisory metric 6 reuses main's expanded-AST complexity collector and
the existing function collector, scorer, evaluator and numeric-review workflow.
The collector runs once for explicitly requested advisory capture. Scoring
borrows construct trees and chains the existing recursion events without copying
the trees. Historical metrics 1–5 remain readable; absent modern scores retain
other facts, report unavailable complexity and cannot establish clean review
negatives. Strict clone matching is unchanged. No new hashes, source proofs,
ignores, suppressions, threshold changes or budgets were introduced.

Independent original-project labels cover four rstyle-core functions, including
the cyclomatic and cognitive boundaries. Capture `run-4zkGtw` measured all 407
original functions and produced eight agreements across eight labeled rule
outcomes. The 806 generated owners remain unavailable and eight callees remain
unresolved; overall scope is honestly incomplete. Capture took 18.21s with
588,176KiB peak RSS; this is an observation, not a speedup claim. The reducer
consumes completed observations using existing compiler-quality helpers without
rescoring. Artifacts and reproduction are in `eval/clones-capability/README.md`.

macOS run `40830574364` exposed two further fixture aliases: an absolute shared
source and an external-dependency source-root assertion. Both failures were
reproduced on Linux with a symlinked TMPDIR before the fix. The common workspace
fixture now resolves its temporary parent once, leaving production lexical
matching unchanged. All nine focused tests pass with that aliased TMPDIR.
Native macOS verification still requires the next CI run.

Final integrated validation: 3,413 Rust tests pass in 215.905s with five existing
skips; doc tests pass. All 198 Python tests pass. Locked all-target Clippy,
formatting and diff checks pass. Style reports zero violations over 9,766
functions; reviewed patterns reports zero candidates. All thirteen candidate IDs
(including the reviewed entries) match the main-rebase unfiltered baseline.
Final logs use `/tmp/clones-advisory-complexity-` with `tests-final-3`,
`clippy-final-2`, `style-final-3`, `patterns-final-2` and `python-final-2`.

Next ready performance frontier: an explicit advisory-only capture/report seam
that avoids unrequested strict certification and matching, while preserving
existing strict admission and honest missing-data reporting. That seam is not
implemented in C42; no speedup is claimed for it.

### C43 — compiler advice without unrequested strict work (2026-10-29)

`--compiler-advisory-only` reuses the existing raw-facts codec, streaming
publication seal and successful Cargo/unit ledgers. The driver returns normally
after publishing raw observations; diagnostic capture-only remains a distinct
stopped, unpublished path. Raw replay checks its exact publication phase,
original mode, artifact bytes and selected compilation contexts. It never enters
strict bundle import. Strict execution is `not_requested`, matching is absent,
pair completeness is false and raw references have no strict namespace.

The existing advisory evaluator, propagation, proposer, compiler plans, reviews
and baselines consume the same borrowed observations. The existing effects/caller session constructors accept borrowed function rosters
through `AsRef<[HirFnFacts]>`, shared by UnitStore and raw observations without
inventing a UnitStore, adding compatibility wrappers or copying bodies.
Build contexts reuse the existing settings interner. Missing/generated owners,
unresolved callees and incomplete planning keep their original outcomes. No
SHA-256, replacement hash, artifact scan, new proof or matching permission was
introduced. Legacy commands and strict matching remain available unchanged.

Ten real-Cargo acceptance tests cover combined/raw parity, full/compact/text
reports, finding exits, source-free replay, reviews, raw strict-import rejection,
resealed wrong phase/context, ledger-mode changes, selected dependency
effects/plans and separate check/test owners. The test build's generated owner
remains unavailable rather than becoming a clean observation. The broader
45-test transport/effect/caller run initially exposed two incorrect acceptance
assumptions; corrected ten-test integration run passes. The missing generated
owner and raw JSON rejection reasons were preserved in product behavior.

Four instrumented debug captures of rstyle-core, ordered combined/raw/raw/combined,
produce exactly equal compiler advisory reports: 407 measured originals, 806
unavailable generated owners, eight unresolved callees and six diagnostics.
All four honestly exit 2. Combined samples take 16.32–16.70s and raw samples
10.58–10.61s, with peak RSS 589,024–589,120KiB versus 555,336–555,468KiB.
This is about 36% less median wall time and 6% less peak RSS for this declared
scope, not a statistical or cross-platform benchmark. Existing stage observations
show certification/import/handoff removal; matching itself was nearly empty.
Raw extraction still captures source/type/correspondence/projection observations.
Commands, contexts, stage counts and limitations are recorded in
`docs/performance/advisory-only-samples.json`.

C42 and the macOS fixture fix are now natively verified: CI run `40834197766`
passes all tests, Clippy, style and pattern gates on Ubuntu x86, Ubuntu ARM and
macOS.

C43 integrated validation on the final source snapshot: 3,427 Rust tests pass in
219.585s, with five existing skips; workspace doc tests pass. All 198 capability
Python tests pass, as do the 50 script tests with Cargo on PATH. Locked all-target
Clippy, formatting and diff checks pass. Style reports zero violations over
9,783 functions, with all complexity observations available. Reviewed patterns
report zero candidates and thirteen existing review entries; the separate
unfiltered run has exactly the same thirteen candidate IDs as the main-rebase
baseline. No thresholds, budgets, suppressions or review entries changed.
Logs use `/tmp/clones-advisory-only-` with suffixes `tests-final-2`,
`clippy-final-3`, `python-final-3`, `python-scripts`, `style-final-4`,
`patterns-final-3` and `patterns-unfiltered-final-1`.

Next ready performance frontier: avoid correspondence and generic/slice instance
projection capture when only compiler advice is requested. These are still
captured in C43. Audit conditional-source diagnostics and compare advice,
patterns, plans, reviews and baselines before removing any extraction work;
strict capture and admission must retain their existing observations.


### C44 — collect only requested advisory extraction inputs (2026-10-29)

The existing crate collector now exposes `advisory_crate_facts`: it retains the
same function/source/type observations, unit source ownership, nominal census,
generated-source capture, library resolution and inherent inventories. Ordinary
raw/strict capture calls that collector and then performs its existing source
correspondence and generic/slice instance projections. Advisory-only capture
returns before those strict inputs are constructed. No second owner traversal,
proof, checksum, body/graph copy or cache was added.

Correspondence also appends deferred-source diagnostics and filters inactive-body
diagnostics, so raw artifacts intentionally differ. The advisory availability
checks read the separate signature-level `macro_generated_function` marker;
effects/callers inspect original syntax/native nodes rather than correspondence.
Those observations remain captured. Original conditional expressions retain the
consumer's existing unknown/unsupported outcomes instead of acquiring permission.

Three real-Cargo acceptance cases compare compiler advice/patterns/plans over
inactive cfg bodies, a disabled function, a workspace-shared I/O leaf/caller,
generic scalar and selected generic-slice calls. They check strict inputs remain
present in combined mode and absent in raw mode, generated owners keep their
missing-data marker, and complete shared-source facts support source-free
baseline/review/plan replay. The focused runs exposed a fixture mismatch and an
existing review limitation: generic-slice projection accepts a specific reader
signature, while parent-relative source paths such as `src/../shared.rs` cannot
currently form review locators. The tests use the selected-slice family and a
canonical-path baseline fixture, retaining parent-path shared-source parity
controls separately. No product outcome was relaxed. Supporting parent-relative
review locators through the existing lexical normalizer is the next capability
fix; it is not implemented in C44.

Four fresh debug rstyle-core captures, two C43 followed by two C44 samples,
produce exactly equal compiler advisory reports; all retain incomplete scope and
exit 2. Summed raw publications shrink from 63,140,408 to 60,508,680 bytes (4.2%).
Peak RSS changes from 555,104–555,112KiB to 519,600–520,080KiB (about 6.4%).
Wall samples are 10.72–10.91s before and 10.44–10.46s after. This small ordered
sample does not establish a reliable runtime speedup or cross-platform result.
Commands, context, stages and limitations are recorded in
`docs/performance/advisory-extraction-samples.json`.

C44 integrated validation: all 3,430 Rust tests pass in 218.218s, with five
existing skips, and workspace doc tests pass. The 149-test focused advice,
NonZero and widening-reader run passed 148 cases and exposed the parent-path
baseline test limitation described above; the final full run includes all three
corrected acceptance cases. Locked all-target Clippy, formatting and diff checks
pass. The sole subsequent source edit separates collector/strict-processing
phases with a blank line; style then reports zero violations over 9,784 functions.
Reviewed patterns report zero candidates with thirteen existing review entries;
the separate unfiltered run retains exactly the thirteen main-rebase IDs. No
thresholds, budgets, suppressions or review entries changed. Logs use
`/tmp/clones-advisory-extraction-` with `tests-final-1`, `clippy-2`,
`style-final-2`, `patterns-final-1` and `patterns-unfiltered-final-1` suffixes.

Next ready capability fix: reuse `nominal_binding::absolute_path` when constructing
review-relative locators, retaining root containment and context/metric/stale
checks. This must unlock parent-relative shared-file reviews and baseline replay
without reopening files, adding hashes or changing strict matching. A separate
later performance cut may omit the per-function call-instance visitor in
advisory-only collection; it remains active in C44.


### C45 — review parent-relative shared source paths (2026-10-29)

Review locator construction now reuses `nominal_binding::absolute_path` for both
captured source and declared root before containment. Authored paths such as
`src/../shared.rs` and `src/.././shared.rs` yield the same `shared.rs` locator.
Previously their compiler findings were available but reviews were unavailable
and complete baseline export failed. No filesystem reads, canonicalization,
hashing, new proof or metric version is needed; existing representable locators,
policy/context staleness and strict matching retain their contracts. This is
lexical path normalization, not a physical symlink identity guarantee.

Focused tests pass all nine cases. Two new real-Cargo tests exercise both
combined and advisory-only capture, unchanged effect propagation and parameter
plans, baseline export, exact dismissal, equivalent-path recapture and source-free
replay. Outside-package sources remain unavailable and cannot export a complete
baseline; relative, empty, escaped and non-UTF8 locators remain rejected.

A bounded routing package uses the unchanged tracked
`crates/rstyle-core/src/paths.rs` through `#[path="../shared.rs"]`, with production
default policy. Its existing cognitive-complexity finding is 16 against limit12.
The measurement replays the same saved raw capture to isolate review behavior;
it is not a full-project quality or performance claim. A separate attempted
full rstyle-pdg capture failed the existing source census file/byte budget before
publishing units. That project remains unmeasured; the budget is unchanged.

The same saved capture moves from unavailable review coverage to complete
coverage with `visible_new`, then to `hidden_exact` after explicit dismissal;
complete baseline export contains one `shared.rs` entry. The raw evaluation is
exactly equal throughout. See `docs/performance/review-source-path-results.json`.

Integrated tests pass all 3,434 Rust cases in 218.583s with five existing skips;
workspace doc tests and locked all-target Clippy pass. Formatting and diff checks
pass. One subsequent whitespace-only edit separates path normalization from
locator construction; style then reports zero violations over 9,784 functions.
Reviewed patterns report zero candidates with thirteen existing entries. The
separate unfiltered comparison preserves the same thirteen main-rebase IDs.
No thresholds, budgets, suppressions or review entries changed. Logs use
`/tmp/clones-review-path-` with `focused-1`, `tests-1`, `clippy-1`, `style-2`,
`patterns-1` and `patterns-unfiltered-1` suffixes.

The earlier macOS fixture fix is committed as `565803b`; C43 native CI passed on
macOS and both Ubuntu targets. C44 native CI is tracked separately from these
local C45 results. Next ready performance frontier: omit unrequested call-instance
observations during advisory-only collection after parity controls; large-source
census coverage remains a distinct unresolved limitation.


### C46 — omit unrequested call-instance collection (2026-10-29)

The existing function collector now takes an explicit call-instance policy.
Advisory-only collection selects `NotRequested`; ordinary raw/strict collection
selects `Capture` before its existing correspondence/projection processing.
This removes the additional instance HIR visitor, generic/trait selection and
callback inventory serialization when their strict consumers are not requested.
The same owner walk still captures original source/signature/body, concrete and
consumer types, compiler advisory observations, inherent/trait observations and
unit ownership. No global environment guard, second traversal, source proof,
cache, body/graph copy or hashing was added. Missing call-instance data is omitted,
not replaced by a fabricated complete empty inventory. Explicit instance
feasibility diagnostics use their existing independent source entry points.

One acceptance test exercises nonempty strict generic and concrete trait call
records plus an immediate Option callback containing a generic call. The raw
advisory mode omits the instance key on every owner while preserving the entire
advisory report, patterns and plans. Original I/O leaf/caller and local dynamic
trait propagation retain five I/O owners; generated callee evidence remains
unavailable/unresolved, leaving overall and planning coverage incomplete.
Source-free replay with no compiler on PATH retains the identical report.
The initial fixture incorrectly expected a local dynamic call to be unresolved:
the existing core collector conservatively propagates local implementation
effects. A generated callee now supplies the independent missing-data control;
no product behavior or existing expectation was weakened. The five-case focused
run, including C44 extraction and strict callable observations, passes.

Two serial before and two after fresh rstyle-core captures preserve exactly equal
compiler advisory reports over 1,213 captured owners, with scope incomplete and
exit 2 throughout. Raw publications shrink from 60,508,680 to 49,108,968 bytes
(18.8%); the call-instance record is present on 1,213 owners before and zero
after. Peak RSS changes from 519,476–519,836KiB to 433,768–433,800KiB (16.6%).
Wall samples are 10.33–10.48s before and 8.97–9.27s after. These ordered samples
show a local reduction, not a statistical or cross-platform speedup guarantee.
Fresh raw bytes also differ in invocation routes, producer stamps and Cargo
dependency identities; byte-identical raw artifacts are not claimed. Commands,
full stage timings and caveats are recorded in
`docs/performance/advisory-instance-samples.json`.

C46 integrated validation passes all 3,435 Rust tests in 218.157s, with five
existing skips; workspace doc tests and locked all-target Clippy pass. Native C44
CI run 40840198496 is now confirmed successful on macOS and both Ubuntu targets.
C45/C46 CI state is separate from these results.

Next audited frontier: advisory-only mode conflicts with local source/guard/
mutation/helper consumers. Its core planner and effect/caller sessions do not
read the strict unit census, namespace, generated-helper, library or aggregated
inherent metadata. Original source inventory and generated-function markers are
captured independently. Omit those strict-only unit metadata passes through the
existing collector seam, preserving all ordinary raw/strict paths, then validate
shared roots, generated/missing owners, reviews/baselines and a bounded large-source
project without raising census budgets. No alternative ownership proof or new
source codec is needed for the requested advisory consumers.

C46 final dog-food gates: zero style violations across 9,785 functions, all
complexity measurements available; zero reviewed pattern candidates with the
thirteen existing entries. The separate unfiltered comparison retains exactly
the thirteen main-rebase IDs. Formatting and diff checks pass. Logs use
`/tmp/clones-advisory-instance-` with `focused-2`, `tests-1`, `clippy-1`, `style-1`,
`patterns-1` and `patterns-unfiltered-1` suffixes. No thresholds, budgets,
suppressions or saved review entries changed.


### C47 — keep strict unit metadata out of advisory-only capture (2026-10-29)

`collect_crate_facts` now returns the existing single function roster directly.
Ordinary `raw_crate_facts` runs the unchanged `capture_unit_evidence` pass before
its unchanged source correspondence and projections. Advisory-only capture skips
that strict census, dependency namespace, generated-helper unit, library and
aggregated inherent metadata pass. Original per-function source snapshots/ranges,
signatures, compiler context, concrete/native observations and compiler collectors
remain captured. Main's separate original-source inventory remains unchanged;
the signature-level generated-function diagnostic does not depend on unit helper
capture. No file/byte budget, source ownership format, proof, checksum, copy or
strict matching permission was introduced or changed.

The consumer audit initially found census reads in local consumer planning, then
traced the requested mode: advisory-only rejects those local consumers, and
`--compiler-plans` uses the existing core planner over captured `FnFacts` instead.
Compiler advice/propagation/patterns/reviews/baselines do not read these strict
unit inventories. Existing raw transport already retains full shared original
source snapshots with exact ranges; no alternative ownership format is needed.

Two acceptance tests add a 257-module/258-function Cargo package and a bounded
original/nominal/generated/library fixture. The large package reports exactly
two original parameter findings, one two-owner parameter plan, complete source
inventory and completed review/baseline replay after deleting source. The same
package still fails strict capture at the unchanged census budget. Bounded
combined/raw reports and source inventories are exactly equal; the strict unit
inventories remain populated in combined mode, absent only in raw advisory mode,
and generated owner unavailability plus source/compiler context remain intact.
The 33-test focused extraction/propagation/shared-review run passes.

Real-project capture now publishes rstyle-pdg: 13,528 functions, 7,034 measured
owners, 6,494 unavailable owners, 142 unresolved callees and 826 existing-core
findings across 809 inventoried source files. The preceding C46 capture published
zero functions because strict census exceeded its 256-file/8MiB budget. Capture
and handoff now have zero failures, but advice correctly remains incomplete.
This is an availability gain, not independently labeled finding quality or a
claim of clean/complete project coverage. One before/after rstyle-core control
preserves exactly equal advice while raw output shrinks from 49,108,968 to
42,080,275 bytes (14.3%). The successful rstyle-pdg capture takes 85.86s, peaks at
3,342,996KiB RSS and publishes 601,866,824 raw bytes; the failed preceding capture
takes 39.27s and does less useful work, so their times are not a speed comparison.
Commands, stages, counts and limitations are recorded in
`docs/performance/advisory-unit-metadata-results.json`. Substantial full-function
extraction/publication resource cost remains; no budget was increased to obtain
these results.

C47 integrated validation passes all 3,437 Rust tests in 219.063s with five
existing skips; doc tests, locked all-target Clippy, formatting and diff checks
pass. Style reports zero violations over 9,785 functions with all complexity
measurements available; reviewed patterns report zero candidates with thirteen
existing entries. The separate unfiltered comparison retains exactly the thirteen
main-rebase IDs. Logs use `/tmp/clones-advisory-unit-` with `focused-1`, `tests-1`,
`clippy-1`, `style-1`, `patterns-1` and `patterns-unfiltered-1` suffixes. No
thresholds, budgets, suppressions or review entries changed.

Next audited resource frontier: compiler review identity snapshots normalized
parameter/result types, so dropping all concrete-type capture would change saved
identity. Extract a small shared signature-only normalization seam from the
existing concrete-type collector, retaining exact interface observations/tokens;
then omit its body enrichment and remaining strict/local-consumer function
metadata only in advisory-only mode. Retain the original native body/source
records; do not replace bodies with fictitious empty facts or invent a new codec.
Verify nested interfaces and review/baseline keys before claiming this cut safe.


### C48 — retain signatures without unrequested body enrichment (2026-10-29)

The existing concrete-type collector now shares its interface observation,
validation and token installation through an explicit signature/body scope.
`capture_signature` retains the same normalized `SignatureFacts` and concrete
interface record; full capture still performs its unchanged body/binding token,
receiver/callback, product-selection and declaration-origin installation. The
function collector selects the signature seam plus the existing advisory collector
only for advisory-only capture. Ordinary raw/strict capture retains full concrete,
inherent/trait, location/formal/control and instance collection.

Compiler review identity reads normalized signatures, so dropping the whole
concrete pass would have changed saved identities. This seam preserves them
without copying bodies, creating alternative type/proof algorithms, adding a cache
or introducing checksums. Original native bodies, bindings, snippets, source
snapshots/ranges, generated markers and selected compiler contexts remain; omitted
strict/local-consumer metadata is absent, not a fabricated complete-empty record.

One real-Cargo acceptance case exercises nested local nominal parameters/results,
shared receiver/slice access, field and mutable scalar writes, loop/break, From
and Default implementations, an immediate generic Option callback and an I/O
leaf/caller plan. Combined/raw full signatures, interface observations, authored
body/binding structure, source inventory and complete compiler advisory reports
are equal. Nonempty strict receiver/field/formal/control inventories remain in
combined mode and are absent raw. Two I/O owners and four authored update mutation
sites are independently expected; generated owners stay unavailable. Source-free
review dismissal retains evaluation and plans. All 38 focused extraction/review
cases pass, including large units, instance controls and saved review semantics.

One serial before/after capture of each unchanged project preserves exactly equal
compiler advisory reports and original source inventories. rstyle-core raw output
changes from 42,080,275 to 37,678,019 bytes; wall samples are 8.69s and 6.91s and
peak RSS 425,248 and 392,856KiB. rstyle-pdg keeps all 13,528 captured functions,
7,034 measured / 6,494 unavailable owners, 142 unresolved callees and 826 findings;
raw output changes from 601,866,824 to 543,482,874 bytes (9.7%), wall samples from
85.63s to 63.60s (25.7%), and peak RSS from 3,356,228 to 2,937,464KiB (12.5%).
Both projects remain incomplete and exit 2. These are single ordered local
samples with cache effects possible, not a statistical/cross-platform guarantee.
Commands, stages and limitations are recorded in
`docs/performance/advisory-function-enrichment-samples.json`. Native source/body
extraction and the 543MB raw publication remain material costs.

Main remains at `1689392` (complexity metrics), already an ancestor of this branch.
C47 native CI (run 40844652318) is confirmed successful on macOS and both
Ubuntu targets, including tests, style and patterns. This includes the macOS
temporary-directory canonicalization fix from `565803b`. C48 native CI remains
separate from this checkpoint's local validation.

The first integrated run passes all 3,438 Rust cases in 218.686s with five
existing skips, plus doc tests, locked all-target Clippy, formatting and diff
checks. Style then found five mutations of one facts target across mutually
exclusive branches. Separate advisory/full builders now take facts by value,
preserving the same operations/order without copying body graphs; style reports
zero violations over 9,791 functions. Measurements precede this ownership-only
helper extraction. The final integrated run passes all 3,438 tests in 218.205s
with five existing skips, plus doc tests. Locked all-target Clippy, formatting
and diff checks pass. Reviewed patterns have zero candidates; all 13 unfiltered
candidate IDs match the main-rebase baseline, with no new dismissals.

Next ready capability work: independently label selected original rstyle-pdg
complexity findings and clean controls using the existing `compiler_complexity.py`
evaluation harness. The newly reachable 7,034-owner scope is availability evidence,
not yet independently labeled quality. Freeze source construct counts before
comparing captured measurements; preserve package incompleteness, metric/context
identity, saved review and baseline semantics. This advances project usefulness
and disagreement validation rather than requiring another representation change.


### C49 — independently label newly reachable project complexity (2026-10-29)

Acceptance/review slice; lead owns the frozen rstyle-pdg labels, existing-reducer
results, temporary-review study and docs. No production Rust changes. Reuse the
existing `compiler_complexity.py`, `compiler_quality.py` and completed shared
core plans/diagnostics; add no scorer, parser, proof, graph copy or checksum.

Four original `data_types` library owners were audited from source before target
score comparison. Their frozen source strings match the explicit C48 saved run
`run-WizKMH` exactly. That capture existed before labels were compared; this is
an independent source-label study, not a prospective capture. Metric 6/policy 1
and default CC/COG limits 12 are explicit. The existing reducer reports eight
agreements: four positive outcomes on `check_contents` (24/17) and
`recursive_element` (19/18), plus four evaluated-no-finding controls on `check`
and `check_nominal_fields`. Clean numeric expectations remain labels, not
fabricated observed scores. Results and exact source/rationale are in
`eval/clones-capability/project-pdg-complexity-{cases,results}.json`.

This validates a bounded sample within the newly reachable project, not all 826
findings or whole-project quality. Scope remains incomplete: 13,528 captured,
7,034 measured / 6,494 unavailable owners and 142 unresolved callees. Go behavior,
broad proposal quality and legacy retirement remain unverified.

Three serialized explicit saved-run replays use a temporary review file outside
the repository. Unit 0 diagnostic 285 (`check_contents` CC24) becomes
`hidden_exact` and stays hidden on unchanged lookup. Raw unit evaluations and
completed plans remain equal (review adds visible diagnostic indices). A policy
replay with CC limit24/COG limit12 correctly removes the CC equality-boundary
finding and retains COG17. All replays exit2 for incomplete scope. Retained status
is **unavailable**, with `Compiler diagnostic identity budget exhausted; uniqueness
is unknown`: the original-owner status roster exceeds its existing 4,096 limit.
No staleness/pruning success is claimed. Study outcomes and caveats are in
`project-pdg-complexity-review-results.json`; no actual repository dismissal,
ignore, suppression or budget change was made.

Validation: all 198 offline Python tests pass; the existing reducer exits0 with
8 agreements/0 unavailable/0 disagreements on the declared samples. Three saved
replays exercise exact review and changed policy. C48's integrated Rust checks
remain the relevant unchanged-production-code evidence (3,438 tests, zero style
and reviewed-pattern violations, locked all-target Clippy and formatting); no
additional full Rust build was needed for this corpus/docs-only increment.
Reproduction and remaining limits are in `eval/clones-capability/README.md`.

Next ready capability work: reuse/select retained owner observations for status
on large saved projects without raising budgets or assuming unique identities
from a truncated prefix. Preserve full target/build-context and duplicate-owner
checks; exact completed diagnostics already support review lookup, while status
currently reconstructs a broader bounded population. This is a demonstrated
headless-workflow limitation. Migration remains active.


### C50 — reuse complete compiler name counts for retained status (2026-10-29)

Lead owns diagnostic completion/retained status, advisory integration, Cargo
acceptance, docs and final gates. The delegated leaf owns only the existing
`compiler_owners` name-index seam and tests; edits were frozen before integration.
A separate read-only audit checked cross-unit/context ambiguity and missing data.

C49 exposed a concrete headless workflow failure: exact diagnostic lookup worked
on a large project but status rejected every owner because of a global 4,096
handoff-size check. The existing name collector already checks the full completed
and excluded declaration roster before resolving diagnostic owners. Its small
count map is now retained only for explicit owner-scope requests, then borrowed
and combined by manifest directory and compilation target. Feature/cfg variants
with the same target share counts; check/test contexts remain distinct. Duplicate
names, including those after the payload prefix, cannot become unique. The
name-only grouping is conservative across source files/declaration kinds.

The retained-status owner payload roster stays capped at 4,096 per unit.
Existing positive diagnostic snapshots retain their separate unchanged path. Actual
per-unit payload omission replaces the inaccurate global handoff-size proxy.
A present row resolves only after full-census uniqueness; omitted rows keep
unavailable scope. Existing numeric policy, context, exclusion, suppression,
complexity-availability and density-completeness gates remain. Baseline retained
comparison shares this seam; whole-scope export limits remain unchanged.
No new body/signature copy, parser, source scan, hash, proof or consumer algorithm
is introduced. Counts move from the existing collector; aggregation borrows name
strings. Strict clone capture/admission/matching stay unchanged.

The source-free 4,098-owner Cargo fixture captures two independent five-parameter
findings, removes source/manifest, and exercises exact dismissal, status and policy
replay. Both exact findings stay hidden without altering raw evaluations/plans;
status reports hidden_exact for the retained prefix and unavailable for the
omitted tail. A max_params5 equality replay reports stale/unavailable and zero
pruning candidates, retaining both entries. Unit cases check duplicate normalized
methods beyond the prefix, unstable names and wrapper compatibility, plus
same-target variants/disjoint names and package/check/test separation.

Two serialized project replays reuse `run-WizKMH` and C49's temporary review file.
The retained CC24 choice now resolves hidden_exact under the original policy;
CC limit24/COG limit12 correctly produces stale because a changed-policy negative
cannot retire that choice. Raw evaluations and plans equal the corresponding C49
outputs. The project stays incomplete (7,034 measured / 6,494 unavailable owners,
142 unresolved callees), exit2. Single replay observations are 9.42s/9.26s and
2,109,336/2,109,560KiB peak RSS; no speedup or cross-platform guarantee is claimed.
The artifact is `eval/clones-capability/project-pdg-complexity-retained-results.json`.

Final validation: 3,443 Rust tests pass in 223.319s, with five existing skips,
plus doc tests. Locked all-target Clippy, formatting and diff checks pass.
Style reports zero violations over 9,798 functions. Reviewed patterns have zero
candidates; all 13 unfiltered candidate IDs match the main-rebase baseline.
All 198 offline Python tests pass. Logs are `/tmp/clones-retained-census-*`.
C48 native CI run40847311578 has passed both Ubuntu jobs plus macOS tests/style;
macOS patterns remain in progress. C50 native CI awaits this commit/push and is
not implied by prior checks. No new ignore, suppression or budget change.

Next ready work: independently evaluate a source-frozen caller-aware parameter
plan from original rstyle-pdg `eligibility::{diagnostic,error,site_of}` using the
existing core planner and an explicit study-only max_params3 policy. Those
four-parameter owners are clean at the unchanged default4; do not call them
default violations. The source-only sample is frozen before target score/plan
comparison, with exact forwarded arguments, a clean control and callee-before-
caller expectations. No new planner or call scan is needed. Migration and
per-capability retirement remain active.

### C51 — validate existing parameter plans against project source (2026-10-29)

The original `rstyle-pdg` eligibility caller/callee/clean-control labels were
frozen before consulting target scores or plans. The study uses explicit
max_params3, leaving repository/default4 unchanged; these four-parameter owners
are not default violations. Reusing C48 saved run `run-WizKMH` and the existing
core evaluator/planner yields eight agreements: three parameter outcomes,
shared edit group, callee-first signatures, exact capability/construct/reason
pass-through, their shared `&str` bundle and no clean-control signature change.
The computed `&site_of(node)` argument stays outside bare forwarding.

The offline comparator reuses compiler-quality source/context associations and
completed diagnostic lookups. It validates step/bundle owner rosters without
rebuilding plans. Mutation tests reject corrupted associations and contracts;
changed/ambiguous/missing source, wrong/ambiguous target context and unavailable
clean controls stay unavailable. Coordinated reordering, forwarding changes,
unrelated bundle owners and clean-control signature changes disagree. Valid
non-parameter groups may omit plans. No new production scan, proof, hash or clone
is introduced; strict matching stays unchanged.

All 207 offline Python tests pass. The checked-in project result records eight
agreements with scope/planning still incomplete: 7,034 measured owners, 6,494
unavailable and 142 unresolved callees. Serialized replay exited2 as expected;
one local sample took13.48s with2,109,120KiB peak RSS, not a speedup claim.
Unchanged Rust retains C50's full test/style/pattern validation. C48 and C49
native CI are now confirmed successful on macOS and both Ubuntu runners.
C50 native CI remains in progress. The macOS temporary-parent canonicalization
fix 565803b is included. Current origin/main1689392 (complexity changes) is already
an ancestor of this branch, so no rebase is necessary.

This sample validates advisory reuse, not full caller closure, safe edits,
package-wide quality or capability retirement. Migration remains active.

### C52 — surface opaque closure contracts in reused helper proposals (2026-10-29)

Lead owns the core summary/cluster/helper seams, live Cargo acceptance and docs;
a read-only worker verified that compiler proposal and concrete-trait propagation
reuse already exist end-to-end. No parallel proposer or new consumer algorithm
was needed. The next bounded deficiency is presentation: ordinary closures are
opaque in the existing graph collector, so even 1000/1000 recorded-node alignment
does not compare their bodies or captures.

The existing WL call-summary pass now retains one boolean. Clusters combine it
only across retained members, and helper/generic proposals borrow that completed
summary for explicit counter-evidence. Iterate nodes remain distinct. No extra
node walk, body/graph copy, source proof, checksum or policy filter is added.
Candidate IDs, membership, scores, structural review/baseline policy and strict
matching stay unchanged. Absence of a warning does not establish equivalence.

Unit controls cover full-alignment warnings with exact equality after removing
only the note, both helper/generic families, no opaque node, and an unrelated
unretained graph that must not leak its flag. A real Cargo fixture uses borrowed
whole-string dedup versus owned edges keyed by four fields. It retains the helper
with the warning, agrees exactly with legacy patterns, and replays after deleting
source/manifest/lockfile. Existing missing/empty graph controls remain unchanged.

The attempted historical run-OFIAA9 replay rejected an unexpected policy field
in its saved pointer-width fact before analysis. Original artifacts were kept;
no importer weakening or successful-replay claim follows. Instead a fresh
advisory-only rstyle-core metric6 capture run-mycjUM has 408 known/806 unavailable
owners and six proposals. All six now warn on opaque closures. Their full
candidate records equal the C37 saved report after removing only the new warning
and source line numbers; source has advanced, so this is not identical-input
performance evidence. The two emitted independently unsuitable groups remain
quality debt, the pipeline is still explicitly rejected, and four other groups
remain unreviewed. Existing quality reduction exits1, capture exits2. One capture
sample is 7.65s/433,784KiB RSS with no speedup claim. Source cases, complete quality
results and a resource/comparison artifact use project-compiler-proposal-closure-*.

Validation: all 3,446 Rust tests pass in 223.866s with five existing skips; doc tests,
locked all-target Clippy, formatting/diff checks and all 207 offline Python tests
pass. Style reports zero violations over 9,799 functions. Reviewed patterns reports zero candidates with the 13 existing ignored entries.
All 13 unfiltered candidate IDs match the main-rebase baseline; no ignore or
suppression changed. Logs use /tmp/clones-c52-*.

Migration stays active. The remaining closure/reduction usefulness negatives need
stronger facts and independent useful controls before changing proposal policy.
A separate ready quality frontier is real-project effect/planner validation of
already selected concrete trait targets; existing C39 evidence deliberately
validates target coverage, not numeric effect/planning improvements. Legacy
retirement and whole-project completeness remain unverified.

### C53 — share exact target-name association across quality tools (2026-10-29)

Lead owns the existing offline association/comparison helpers, independent
multi-target corpus, tests and docs. A read-only source reviewer checked the
proposed production concrete-trait effect study. `StderrTee<R>::read` calls IO
helper `emit`, but its reader and authored callers stay generic; the driver
callback's framework caller is not an authored concrete invocation. Those
sources cannot justify the proposed concrete-dispatch effect/planning claim.
No source labels or complete counts were manufactured to force that study.

A concrete workflow defect was independently identified: shared `sample_owner`
ignored explicit target_name labels, while C51 had a local prefilter. The shared
helper now validates and applies the optional exact name alongside existing
package/kind/check-test selection. Omitted names still need one unambiguous
context; wrong/missing/ambiguous matches remain unavailable. Source inventories
must match that exact captured compilation target. Numeric/spacing/density rows
retain explicit names, and root/propagation label identities distinguish targets. Overlapping named
and unnamed selectors cannot double-count one completed owner.
Complexity, proposal sites/definitions and plans reuse the same association;
C51's redundant prefilter is removed. No Rust runtime, collector, scoring, proof,
copy or hashing change is introduced.

The independently authored two-binary corpus compiles one shared five-parameter
function into first/second plus a clean main per binary. Locked run-2ntpNc has
four measured owners, no unavailable owners and complete small-scope input.
All four outcomes agree with source labels, whereas the previous shared helper
marks all four ambiguous. Existing no-name ambiguity and missing target-specific
source remain unavailable. A single capture took 0.31s with 90,248KiB RSS; no
production-project quality or speedup claim follows.

All 214 offline Python tests pass, including new wrong/invalid target, shared
binary source, propagation input, complexity and proposal role controls.
C49 saved complexity, C51 saved parameter plans and C52 saved proposal results
reproduce exactly (C52 retains its intentional quality exit1). C52 owner labels
now explicitly select rstyle_core. Fixture formatting and diff checks pass.
Rust is unchanged; C52's 3,446 passing tests, zero style/unreviewed patterns and
unchanged 13 unfiltered IDs remain the runtime checkpoint evidence. No new
ignore, suppression, threshold or budget change. Native CI remains separate.

The migration remains active. Real-project concrete trait target coverage is
verified, but the proposed positive effect/planner production study remains
unverified because no suitable authored concrete caller was found. Continue
consumer-led work from demonstrated source/evidence requirements; keep existing
legacy and strict paths until their individual replacement gates pass.

### C54 — select reused compiler proposals by changed files (2026-10-29)

Lead owns CLI, report/baseline integration, docs and the production replay;
a frozen acceptance leaf owns the selection tests. Main 1689392, including the
cyclomatic-complexity changes, is already an ancestor of this branch.

The explicit `--select-compiler-patterns` flag requires compiler patterns and the
existing changed-file manifest. It checks completed candidate usage sites and
definitions once, retaining only a sorted index mask. Text and finding exits
intersect that mask with explicit review visibility. Baseline new/changed counts
borrow the same mask; raw lifecycle classifications, retained choices and header
drift keep their full scope. Default changed-file behavior remains diagnostics
only. Numeric `changed_scope` counts retain their existing meaning.

No collector, evaluator, proposer or source/graph traversal is duplicated. No new
hash, proof, body copy, changed-line inference or selective compilation is added.
Raw evaluations/candidates, observations, plans and strict matching remain global;
missing inputs and unknown selected files still prevent clean results. Optional
masks preserve default report serialization.

Eight acceptance cases cover live opt-in/default behavior, usage versus definition
paths, exact unknown paths and empty selection, source/tool-free text replay,
independent review masks, baseline classifications and policy drift, distinct
check/test occurrences, missing graphs and required CLI flags. The baseline test
compares all raw current fields after accounting for the intentional `selected`
marker. All eight pass.

Saved production rstyle-core run-mycjUM replay retains all six raw proposals.
Selecting align.rs yields candidate index3; selecting clip.rs yields an empty
mask. Raw proposal/numeric evaluations, observations and strict results agree
exactly with unfiltered replay. All three exit2 because the original capture has
408 measured/806 unavailable owners. Single replay observations are about0.94s;
there is no speedup, quality-improvement or complete-project claim. The artifact
project-compiler-pattern-selection-study.json records commands, masks and limits.

Validation on the final counter-sharing implementation: all 3,454 Rust tests
pass in 224.941s, with five existing skips; doc tests and all 214 offline Python
tests pass. Locked all-target Clippy, formatting and diff checks pass. Style
reports zero violations over 9,806 functions. Reviewed patterns reports zero
candidates; all 13 unfiltered IDs match the main-rebase baseline. No suppression,
ignore entry, threshold or budget changed. Logs use /tmp/clones-c54-checked-*.

The last observed macOS failure (8d3f651 CI) compared /var fixture roots with the
compiler's resolved /private/var roots. Existing fix 565803b canonicalizes the
shared temporary parent; eleven subsequent completed CI runs passed all platforms.
For the preceding pushed HEAD 9fe20e0, both Ubuntu jobs are green and macOS remains
in progress at this checkpoint. No new native macOS pass is claimed. Main 1689392
was fetched again and remains an ancestor; no rebase rewrite is necessary.

Migration remains active. This increment improves changed-file review/CI scope;
it establishes neither complete project coverage nor proposal usefulness,
semantic equivalence or any legacy retirement gate.

### C55 — account for selected proposals in offline quality review (2026-10-29)

The previous goal turn made authoritative progress in C54 (7e7872f): code, tests,
measurements and a pushed feature. This increment reuses its completed proposal
reports and the existing offline source-role association. Lead owns reducer,
acceptance, artifacts and docs. Two read-only leaves reviewed production source;
no unfinished worker code is part of the checkpoint.

The new file mask exposed a validation gap: the quality inventory's visible flag
accounted for explicit reviews only. Both masks now share one checked-index
reader, then intersect per unit. Optional review_visible/file_selected fields
retain the two inputs when selection is requested. Historical default fields and
results remain exact. Raw candidates, source-role associations, judgments and
negative-quality exits stay global. Empty selection cannot erase a negative
source review or turn an absent candidate over incomplete input into a negative.
No Rust runtime, collector, scoring/proposer algorithm, proof, source/graph scan,
body copy, hash, admission or matching change is introduced.

Independent source review completes the four previously unreviewed groups from
run-mycjUM. All six emitted groups are not useful at their current scope: call
classes versus directional data edges, distinct recursive tree predicates,
distinct loop rewrites, already shared mirrored traversal, formatting policy
shells, and different deduplication ownership/equality contracts. The seventh
source label preserves the known forwarding rejection. All source bytes match
both the saved inventory and current production files. The review found no
useful emitted positive. Existing closure flags/zero substitutions cannot alone
distinguish useless advice from useful repeated work, so no blanket rejection or
ignore entry was added. The bounded guard source review also found no positive
in rstyle-ccgraph; it does not establish a new guard capability or Go parity.

Measured exact C54 global/align.rs/clip.rs inputs now report 6/1/0 visible
proposals instead of the previous reducer's 6/6/6. All retain six raw emitted
candidates, six structural agreements, one explicit policy rejection and seven
independent not-useful judgments; no emitted candidate remains unreviewed in
this declared sample. All three offline quality commands retain exit1, including
the empty presentation selection. Capture/replays retain exit2 and incomplete
408 measured/806 unavailable-owner scope. Artifacts use
project-compiler-proposal-complete-review-* and
project-compiler-proposal-selection-quality-study.json. No new capture, timing,
speedup, production precision improvement or whole-project claim follows.

All 219 offline Python tests pass, including mask intersection, unit isolation,
malformed indices, known-empty versus missing inputs, historical field parity and
hidden negative-quality command exits. The original C52 default quality artifact
reproduces exactly. Formatting/diff checks pass. Rust source is unchanged from
C54, so its final 3,454 passing tests, doc tests, locked Clippy, zero style violations
and zero unreviewed patterns remain the runtime evidence; those tests were not
repeated for this offline-tool change. Native CI for 9fe20e0 is now confirmed
successful on macOS and both Ubuntu runners; C54 native CI remains in progress.

The migration remains active. Current evidence requires stronger meaningful
visible-computation or closure facts and independently useful production controls
before changing proposal policy. Broader project quality, guard production
positives and all legacy retirement gates remain unverified.

### C56 — rank completed compiler complexity, including clean functions (2026-10-29)

Lead owns CLI, report integration, shared registration, artifacts and docs. The
core ranking leaf owns its borrowed sorting seam/tests; the acceptance leaf owns
CLI fixtures and minimal shared reseal-helper visibility. Both leaves are frozen.
This adds an opt-in consumer of existing measurements, without another collector,
scorer, proof, source/artifact scan or hash. No body/graph copies, metric version,
strict matching, ignore, suppression, threshold or budget changes are introduced.

`--compiler-complexity-top N` requires compiler advisory and a positive N. Core
sorts borrowed facts by its existing COG/CC/location/name ordering, then the
consumer projects only N score/location rows with original report-local owners.
Only a small optional ID-to-owner map crosses core evaluation's merge/exclusion
boundary. Existing findings reuse the same sorter and preserve their output.
Policy exclusions apply; numeric reviews and changed-file presentation do not
filter raw rankings. Default output is unchanged. Ranking does not create
findings or change their exits, but requested missing complexity stays incomplete.

Per-unit `complexity_ranking` records full known/measured/missing-score/missing-owner
denominators separately from the display limit. Unknown scores sort last and
remain null; missing owners retain original reasons. Historical metrics without
scores and modern missing scores outside top1 retain exit2. Advisory-only and
source/tool-free replay reuse the same finished facts, without rescoring.

The exact saved C52 rstyle-core run-mycjUM replay exposes 10 or all408 score rows
versus the previous diagnostics-only function projection. All408 are measured;
806 unavailable owners still prevent a whole-project claim and retain exit2.
Raw diagnostics, observations, plans, proposals, strict results and default
checkpoint output remain exact. Single default/top10/all408 replay observations
are about1.08/0.93/0.92s; concurrent analysis load and single observations preclude a
speedup claim. Artifact: project-complexity-ranking-study.json.

Frozen C42 source labels match three of four full scores and all four threshold
outcomes. Namer::render's old COG4 label omitted three authored direct self.render
calls; documented direct-recursion policy accounts for saved COG7. Its CC12 and
no-default-finding outcome remain unchanged. Original frozen labels are retained
and the discrepancy is recorded, rather than rewriting expectations to output.
The clean lines::has_blank_line score1/0 is now inspectable at rank329; top10
also includes below-threshold functions. This demonstrates useful score visibility,
not broad metric quality, a whole-project maximum or legacy retirement.

The final integrated snapshot passes all3,468 Rust tests in225.411s, with five
existing skips, plus doc tests and all219 offline Python tests. Eight CLI cases
cover clean scores, ordering/ties, positive limits, policy exclusion, context
isolation, raw-result parity, source/tool-free replay, missing owners and actual
resealed historical1–5/modern missing-score replay. Core tests retain finding
filter/order parity and borrowed identity; denominator tests retain unknowns.
Locked all-target Clippy, formatting and diff checks pass. Style reports zero
violations over9,812 functions; reviewed patterns has zero candidates. All13
unfiltered IDs match the main baseline. Logs use /tmp/clones-c56-checked-*;
the final full suite is checked-tests-final.log. The initial full run also
passed, but small integration/metadata changes were followed by this final run.

Main1689392 was fetched again and is already an ancestor, so no new rebase rewrite
is needed. Existing macOS /var versus /private/var fixture fix565803b remains in
place. C54 run40857364923 now confirms success on macOS15 and both Ubuntu runners;
C55 has both Ubuntu jobs green and macOS still in progress at this checkpoint.
No new native pass is claimed for the unpushed C56 snapshot. Migration remains
active; broader project quality and all legacy retirement gates remain open.

### C57 — independently check clean compiler complexity scores (2026-10-29)

The preceding goal turn delivered and pushed C56 (6289496), so this is continued
implementation rather than a no-progress retry. Its newly inspectable clean
scores exposed a validation gap: compiler_complexity.py compared exact numbers
only for diagnostics. A below-threshold score could disagree while the sample
still passed all rule outcomes. Lead owns the existing reducer, measured saved
replays, artifacts and docs; a frozen acceptance leaf owns its existing Python
test file. No Rust runtime changes are part of this increment.

An opt-in --require-scores mode reuses the existing exact source/context/span
association, then reads the completed unit-local complexity ranking. It checks
policy_version/CC/COG without parsing, scoring, proof, source/artifact scans,
body/graph copies or new hashes. Paired row/owner rosters, original names, valid
IDs/navigation metadata, complete measurement counts, available summary counters
and missing-owner denominators are checked. Source-label locations can include
leading docs/attributes while ranked rows locate the function; they cannot replace
exact original byte ownership. Malformed report claims reject; absent rankings,
truncated owners and null measurements stay unavailable rather than becoming0.

The mode adds score_counts/score_comparisons independently of ordinary threshold
counts. Either family can fail the command; a clean threshold result cannot hide
a wrong or unavailable score. Without the flag, both original core and PDG
archived results reproduce exactly, including reports with no ranking. No existing
metric version, strict decision, diagnostic result, policy, suppression, review,
ignore, threshold, limit or evidence budget changes.

Exact saved run-mycjUM core and run-WizKMH PDG replays retain eight agreeing
threshold outcomes per package. All four PDG numeric labels agree. The original
core label set reports three exact agreements and one disagreement: Namer::render
expected COG4 but three authored direct self.render calls add3 under the documented
policy, yielding saved7. Its CC12 and clean default outcome remain unchanged.
Original frozen labels/results remain untouched; the frozen-score result records
the discrepancy, and a separate reviewed core score-label set explicitly documents
the source-label correction. All eight reviewed numeric samples then agree.
Artifacts use project-{core,pdg}-complexity-*-score-* and
project-complexity-score-quality-study.json. These samples do not claim broad
precision/recall or complete quality. Core408/806 and PDG7034/6494 measured/missing
owner scopes stay incomplete and capture/replay exits remain2. Single saved-input
replays took about0.77s/9.59s with all408/all7034 ranking rows; these are observations,
not a speedup comparison or new capture.

All231 offline Python tests pass, including12 new exact-score cases and actual
command negative exits, with historical default-output reproduction. Formatting
and diff checks pass. Rust is unchanged since C56's final3,468 passing tests,
doc tests, locked all-target Clippy, zero style violations and zero unreviewed
patterns; those gates are not redundantly repeated for an offline-tool change.
C56 native CI is still in progress at this checkpoint; C55 is green on macOS15
and both Ubuntu runners. Broader migration and every legacy-retirement gate stay
active; this increment improves numeric-quality validation rather than scoring.

### C58 — retain core policy evaluation membership in quality checks (2026-10-29)

The preceding continuation yielded evidence that changed the next action: native
function spans omit leading docs, so the proposed literal-documentation guard
change would not improve measured coverage. Its implementation and new test leaf
were removed from the checkout; the experiment is retained in /tmp only. A bounded
source review also found no qualifying new selected-concrete-trait IO/planning
positive; generic receivers and external default-method/serializer boundaries
remain explicit rather than being presented as propagation gains.

The next acceptance audit found an actual quality-validation gap. Compiler
observations retain policy-excluded owners, while core evaluation filters them.
The offline numeric reducer inferred evaluated-no-finding from absent diagnostics
without establishing selected membership. This also affected complexity outcomes
and parameter-plan clean controls. Lead owns the shared reducer, planning adapter,
project evidence and docs; a separate acceptance leaf froze complexity exclusion
and suppression tests before implementation.

The shared check now reuses completed core selected-function counts, plan owners
or paired complexity-ranking owners. An exact full selected count proves every
known captured owner was selected; individual completed owners support mixed
scopes. Missing membership stays unavailable, including when some owners were
excluded and the requested ranking omits the sample. No second glob matcher,
collector, evaluator, scorer, parser, proof, source/artifact scan or hash is added.
Runtime analysis, native policies, metric versions, reports and strict matching
are unchanged. Synthetic native fixtures now retain actual selected-population
counters instead of relying on missing denominators.

Complexity and parameter-plan labels additionally require independently reviewed
unsuppressed:true, matching the existing compiler-quality precondition. The source
review checked all eight unique core/PDG complexity owners and the three PDG
parameter owners; their function attributes and preceding lines contain no
applicable named suppression. Only this precondition and its review note were
added; original source and numeric/relationship labels remain unchanged.

A native run-mycjUM replay excludes crates/rstyle-core/src/lines.rs under the same
complexity limits. It retains408 captured known owners but selects404. The old
comparator falsely accepted both clean complexity outcomes for excluded
lines::has_blank_line and exited0. The new comparator reports six agreements and
two unavailable outcomes, exiting2; exact-score mode reports three agreements
and one unavailable. This evidence is recorded in
project-compiler-evaluation-membership-study.json. The original core/PDG score
results and PDG parameter-plan results reproduce exactly, including all eight
threshold/relationship agreements in each corresponding study. Whole-capture
incompleteness remains unchanged. No performance or new runtime coverage claim.

All244 offline Python tests pass, including13 new regressions covering excluded
paths, missing selected counts, mixed ranked membership, actual CLI exit2,
parameter clean-control unavailability and required suppression review. Current
CLI rebuild, formatting and diff checks pass. Rust runtime source is unchanged
since C56's final3,468 passing Rust tests/doc tests, locked all-target Clippy,
zero style violations and zero unreviewed patterns; those expensive gates are
not redundantly repeated for an offline-tool change. Logs use
/tmp/clones-c58-checked-*. C56 CI40861421408 is now green on macOS15 and both Ubuntu
runners; C57's Ubuntu jobs are green while its macOS job remains live at this
checkpoint. The existing macOS fixture fix stays in place.

Migration remains active. Broader source/effect/proposal quality, genuinely useful
production guard positives, full captured-owner coverage and capability-specific
legacy retirement gates remain open.


### C59 — reuse guard advice within synchronous closure return scopes (2026-10-29)

The previous continuation verified the macOS fixture repair on a subsequent green
three-platform CI run; no additional macOS defect was observed. This increment
returns to a measured production advisory capability. A source-only reviewer froze
within/contains/remember cases before detector changes in
project-guard-closure-cases.json. `verification_session::within` contains a useful
scalar fallback inside its session-initialization closure. Returning from the
outer function would break cleanup/operation semantics; the advice must explicitly
keep the return inside that closure.

Lead owns the detector/input seam, analysis summary, rendering, plans, registration,
study and docs. Acceptance workers own the new closure CLI leaf and source labels;
a read-only review checked the integrated return scopes and spans. The existing
Boundaries walk now enters supported synchronous block closures with independent
control-flow state and invokes the existing candidate algorithm. Nested return
flags cannot poison siblings or authorize outer opportunities. No second parser,
consumer algorithm, hash, source scan, proof or cloned fact owner is introduced.
Async, const, static, higher-ranked and expression-bodied closures stay outside
this contract; opaque syntax in a closure keeps that scope unsupported.

Each local opportunity retains its exact closure and guard spans relative to the
original function. The enclosing root stays unsupported/incomplete and has no
root opportunity. Rendering says “within this closure”; planning borrows each
completed opportunity and uses the existing original-region mapper once. Multiple
opportunities can share a function owner and still receive distinct plan IDs.
Analysis finding counts remain per assessment, planning counts opportunities, and
unavailable rule counts now use coverage independently of finding outcome. Thus
a partial witnessed finding cannot accidentally make the whole scope clean.

The production study copies the complete, byte-identical verification_session.rs
and verification_policy.rs modules into a bounded standalone package. This is an
original-module source slice with its own declared compilation context, not a
full original-package capture. Baseline351c3a2 and the changed detector evaluate
the same saved run-cHsQ4u: within gains exactly one independently labeled local
finding and exact terminal-if plan; contains and remember gain none. All three
roots and planning remain incomplete. Matching, accepted pairs and original
function records are identical. project-guard-closure-study.json retains the
comparison, source mapping, exact spans and limitations. Lifetimes, borrow duration,
captures, drops, equivalence, rewrites, Go parity and performance remain unproved.

Final validation logs use /tmp/clones-c59-*. All3,473 Rust tests pass in225.449s,
with5 existing skips; doc tests pass. Focused guard CLI10 and detector11 tests
cover sibling/nested returns, original UTF-8/CRLF offsets, unsupported controls
and source/tool-free replay. All244 offline Python tests pass. Final locked
all-target Clippy, fmt and diff checks pass; workspace style reports0 violations
over9,816 functions. Patterns reports0 unreviewed candidates and all13 unfiltered
IDs equal the main baseline. The first pattern check identified duplicated rule
status iteration; a shared outcome/coverage iterator removed it, without adding
ignores. The entire Rust suite was rerun on this final refactor. Final production
replay is exactly equal as JSON to the checked study. Current origin/main
1689392 is already an ancestor. C57 CI40862639413 is green on all three platforms;
C58 Ubuntu jobs are green while its macOS test job is still live at this checkpoint.

Migration remains active; broader project quality and full captured-owner coverage,
plus capability-specific legacy retirement gates, remain open.

### C60 — reuse local source consumers in advisory-only captures (2026-10-29)

Advisory-only captures can now request the existing source rules and guard advice,
including parameter and guard plans. Removing the CLI conflict alone was
insufficient: raw snapshots lacked original file ownership. The advisory capture
entry point now optionally reuses the existing file census and compiler source
buffers/checksums, retaining the same shared ownership and compilation context.
The full census still uses the same file collector before adding its existing
namespace, documentation and expansion inputs. Compiler-only captures skip this
optional census. No second consumer algorithm, parser, hash, strict proof or live
source fallback is introduced. Strict assessment and matching stay not_requested;
raw snapshots still cannot be imported as certified UnitStore input.

The existing 256-file/8MiB census limits are unchanged. Explicit source requests
that exceed those limits fail capture; a regression covers the file limit while
showing that compiler-only capture still succeeds. Older raw snapshots cannot
acquire missing ownership during replay. The original PDG run-WizKMH therefore
retains zero evaluated source/guard rules and 40,584 unavailable rules over all
13,528 owners, with incomplete coverage and exit 2. No whole-package cleanliness
is inferred from its zero findings. Existing parameter planning also keeps opaque
formal-use evidence incomplete even when direct callers and edit regions exist.

project-core-local-source-cases.json freezes four original rstyle-core library
check signatures before report observation. The explicit study parameter limit
is 3; the repository default remains 4. The source-enabled full-package capture
run-VPEfSy agrees on both four-parameter positives and both clean controls, and
its entire compiler_advisory result equals the matched compiler-only control
run-QH9TTq. Original source slices match the labels byte for byte. The package
still has incomplete source/guard coverage: 1,153 assessed of 1,215 expected owners,
904 evaluated and 2,741 unavailable rules, 806 unavailable suppression/exclusion
observations, 29 finding assessments and zero guard plans. Findings do not confer
complete guard coverage. project-advisory-local-source-study.json records scope,
exact outcomes and limitations.

These are single ordered resource observations, not a benchmark or speedup claim:
source-enabled capture took 7.52s with 403,360 KiB peak RSS and 44,285,215 bundle bytes;
compiler-only took 7.05s with 391,600 KiB and 37,730,290 bytes. Source bytes are shared,
but current transport repeats census metadata per function and reconstructs those
records on decode; no metadata deduplication gain is claimed. Broader PDG source
coverage above the existing census budget remains open.

Acceptance covers combined/raw result parity, clean/finding/incomplete exit
semantics, source/tool-free full and compact replay, strict import rejection,
missing bundles, generated/opaque bodies, absent ownership and explicit budget
failure. The first full run passed all 3,479 Rust tests in 225.758s with 5 existing
skips and passing doc tests. Final Clippy and fmt pass; workspace style has 0
violations over 9,819 functions. The pattern gate has 0 unreviewed candidates and
all 13 unfiltered IDs equal main. All 244 offline Python tests pass. Validation logs
use /tmp/clones-c60-*. The final capture-entry cleanup rerun also passed all 3,479 Rust tests
in 226.225s, plus doc tests. Final production replay exactly preserves analysis,
compiler advice, guard plans and strict execution status.

Origin/main 1689392 remains an ancestor, including the complexity changes. The
previous macOS temporary-parent canonicalization fix remains present; C59 CI
40868094682 is now green on macOS15 and both Ubuntu runners. No new macOS
test defect was observed. Migration and capability-specific retirement gates
remain open.

### C61 — share immutable census metadata through existing transport (2026-10-29)

The previous turn delivered C60 as 83a6a3a. The detailed C55 record already
completes the emitted proposal usefulness review; it is not a new review frontier.
A fresh source-only core review found no justified narrow guard extension, so this
increment addresses the measured C60 source-capture cost instead of adding an
unsupported detector or a parallel consumer algorithm.

Lead owns the option-bundle input seam, schema handling, probe and measurements;
an acceptance worker owns raw codec regressions and a read-only reviewer checks
ownership, compatibility and trust boundaries. The existing inventory owner now
registers census allocations by pointer identity, retaining borrowed owners for
the registration lifetime. The first function carries the existing census record;
subsequent functions carry a dense backward reference. Import retains the decoded
entry and clones its Arc. No full-value comparison, source walk, JSON rendering,
checksum, hash or proof cache is added. Equal independent owners remain distinct;
missing/malformed options consume no IDs. The source dictionary, inherent-unit
inventory behavior, semantic validators and budgets are unchanged.

Shared 7/raw 4 publish this grammar. Shared 4–6/raw 2–3 retain their original census
records and reject new references. Raw 3/shared 6 still accept flat bodies after
the version bump. Wrong-key, forward, dangling and malformed references fail;
source-table usage checks still run. This is ordinary unvalidated evidence
ownership, with no serialized certification or relaxation of strict matching.
Historical snapshots are not merged by content equality on import. Existing
strict-store census value interning is outside the measured decode improvement.

A fresh original rstyle-core library capture run-ChBnep under the unchanged C60
study policy retains exactly equal compiler advice, analysis, guard plans,
accepted pairs and strict execution status. One census definition and 1,214
references replace 1,215 repeated census records. Bundle size falls from 44,285,215
to 37,819,451 bytes. The package remains incomplete with exit 2; no new source or
proposal coverage is claimed. The single fresh capture is 7.19s/401,984 KiB RSS,
not an end-to-end speedup measurement.

The prebuilt census_replay_samples probe measures raw decode separately from
input reads and owner counting. Its two inputs contain the same new captured
records/source table: raw4 shared census and reconstructed raw3 inline census.
Three decodes per process, inline then shared, show median 0.459s vs 0.357s and
peak process RSS 208,448 vs 159,440 KiB. Decoded census owners fall 1,215→1. This is
a single ordered transport comparison, not a statistical benchmark or a claim
about full-analysis runtime. docs/performance/census-transport-samples.json retains
raw samples, method, comparison scope and limitations.

Focused bundle tests cover pointer sharing, exact byte savings, distinct equal
owners/contexts, mixed absent/malformed/valid order, unrelated explicit nulls,
current-only references, historical inline formats, flat bodies and unchanged
raw strict-import rejection. Final Clippy passes; workspace style has 0 violations
over 9,823 functions; patterns has 0 unreviewed candidates and all 13 unfiltered IDs
equal main. All 244 Python tests pass. The first integrated run exposed two fixture assumptions: relabeling a new
bundle as historical without expanding census references, and mutating a census
reference as if it were an inline record. The mutation helper now expands census
references before edits, preventing missing-definition errors from masking the
intended semantic rejection. Normal strict replay still reads the shared format.
Both affected tests pass individually; final integrated validation passes all
3,484 Rust tests in 226.285s with 5 existing skips, plus doc tests. Final locked
all-target Clippy, formatting and diff checks also pass.
Logs use /tmp/clones-c61-*. Migration and capability retirement remain open.

### C62 — select changed-file guard advice with the existing selector (2026-10-29)

The preceding turn verified pushed macOS CI and current main ancestry. A read-only
frontier audit identified guard selection as a missing headless workflow: source
rules already had changed-file selection, while guard findings/text remained
global. Lead owns CLI/report integration and the shared selection adapter; the
acceptance worker owns the new guard_selection CLI leaf. Worker writes are frozen.

Explicit --select-guards requires --guard-clauses and --changed-files. The existing
changed-file manifest, checked borrowed paths, owner aggregation, unavailable
counts and completion contract now serve both source rules and guard advice.
analysis.guard_changed_scope keeps independent selected counts/coverage. Each owner
counts once even with multiple closure opportunities. Source rules need not be
requested. Raw assessments, global counts/coverage, plans, strict outcomes and
ordinary default behavior stay intact. No new detector, source parse/proof,
checksum, hash, matching permission or budget is introduced. Incomplete global
analysis cannot become clean through an empty or clean selection.

Four CLI cases cover ordinary/raw parity, explicit/default exits, guard-only
policy, selected guard/clean/unobserved paths, source/tool-free full/compact/text
replay, multiple closure opportunities and required flags. The C59 unchanged
original session/policy module slice replays with one selected guard in the session
file, zero in the policy file and zero in an empty selection. All variants remain
exit2 for incomplete global coverage. Global assessments/plans and strict fields
are exactly equal. project-guard-selection-study.json retains counts and limits;
this is a workflow gain, not new extraction coverage or a performance claim.

Delivered with final integrated validation: all 3,488 Rust tests pass in 226.269s
with 5 existing skips, plus doc tests. Locked all-target Clippy and formatting pass;
all 244 offline Python tests pass. Initial style checks found expanded orchestration
over the function-length limit; selection setup now lives in the existing
changed-file module and the existing function-assessment loop is a separate helper.
Final style has zero violations over 9,826 functions. Patterns has zero unreviewed
candidates; all 13 unfiltered IDs equal main. The final binary reproduces all four
recorded project selections exactly. Read-only runtime review found no concrete
defects. Logs use /tmp/clones-c62-*. C61 CI is green on both Ubuntu runners; macOS
remains in progress at this checkpoint. Persistent guard reviews/baselines and
capability retirement remain open. Next concrete reuse seam: enable the existing mutation-rule consumer
on advisory-only captures using the gated source census and existing bounded
formal-type collector, with raw/combined parity and unchanged resource limits.


### C63 — reuse direct mutation rules in advisory-only captures (2026-10-29)

The existing `--mutation-rules` consumer now runs with explicit
`--compiler-advisory --compiler-advisory-only`. Fresh requests reuse the original
file census and the existing mutable-u32 formal/builtin control-type collectors.
The saved plan records optional mutation evidence; old captures cannot invent it
on replay. Direct-u32 counts, primitive loop semantics, policy, numeric reviews,
suppression and exclusions use the existing consumer, without strict admission,
body enrichment, helper certification, new hashes or changed resource limits.

The first real-core capture exposed signature/binding type incoherence: rich
signature types were normalized but raw parameter bindings were not. Signature
capture now normalizes only formal bindings through the same Capture and its
completed type observations. It does not construct another type collector or
relax contradiction checks. Ordinary full capture keeps its existing behavior.
Six CLI cases cover raw/combined parity, loops, nonprimitive formal regression,
unknown-call lower bounds, missing optional records and source/tool-free replay.

Fresh rstyle-core library captures retain exactly equal mutation reports and
compiler advice across 1,215 owners (409 known captured owners, 806 unsupported).
All mutation assessments remain incomplete, with zero witnessed write sites:
this is workflow parity, not broader vocabulary coverage or quality evidence.
Both modes exit2. The single raw sample took7.10s/402,396KiB versus
15.55s/585,900KiB combined; no generalized speedup claim follows. Exact commands,
run handles, counts and limits are in project-raw-mutation-study.json.

Main1689392 is already an ancestor, including the complexity additions; fetching
origin/main required no further rebase. macOS fixture-parent canonicalization
565803b is already pushed. C60/C61 CI completed successfully on macOS and both
Ubuntu runners; C62 remained in progress when this checkpoint began.

Final gates: all 3,494 Rust tests pass with five existing skips, plus doc tests.
Locked all-target Clippy, formatting and diff checks pass; all244 offline Python
checks pass. Style has zero violations over9,827 functions; patterns has zero
unreviewed candidates and all13 unfiltered IDs equal main. Initial integration
found the obsolete CLI conflict assertion; the test now reflects supported raw
mutation rules. Style caught an orchestration mutation count and patterns caught
a redundant wrapper; both were refactored without ignores or threshold changes.
Logs use /tmp/clones-c63-*. C62 CI40878398294 is now green on macOS and both
Ubuntu runners. Broader mutation vocabulary and capability retirement remain
open; this checkpoint does not complete the migration.


### C64 — reuse scalar summaries and density in advisory-only capture (2026-10-29)

The next reuse audit traced effects::Session, original statement inventory and
the existing density event cache. Their supported primitive free calls already
retain identity, safety, result tokens and selected context in raw HIR. Local
unit/u32 helpers and external unit/never results need no additional body type
capture, field locations, library proof, strict import or helper certification.
The prerequisite worker audited those paths; acceptance worker owned only the
new only_effects.rs leaf; lead owns CLI/demand integration, docs and validation.
Worker writes are frozen for the checkpoint.

Explicit --mutation-effects, --effect-density or both now work with
--compiler-advisory --compiler-advisory-only. Plan demand reuses the existing
local_source_facts/local_mutation_facts flags and narrow collectors, while the
existing consumers borrow the same published owner roster. No new consumer,
source/proof/artifact scan, hash, metric or resource-budget change is introduced.
Scalar and density completion remain independent requested axes. Unknown calls,
opaque work, missing optional types and deeper helpers remain incomplete; zero
unknown weights never grant ratios or minimum-size exemptions. Existing saved
density baselines, policy reevaluation and full/compact tool-free replay work.

Six independent CLI cases cover scalar-only and density-only demand, two-edge
borrowed chains with repeated call mappings, original loop sites, classified
external unit/never events, callback/macro/depth boundaries, old capture gaps and
source/tool-free numeric baseline/policy replay. Exact supported raw/combined
reports agree. Read-only final runtime review found no concrete defects.

The exact C63 saved core captures were replayed rather than recaptured. Compiler
advice stays equal and 1,214 of1,215 effect rows agree exactly. The unsupported
normalize::flatten_else_if owner differs: combined enrichment attempts a deeper
summary and exhausts work; raw missing target stops earlier. Both remain
incomplete with no ratio; all1,215 package density assessments remain incomplete.
Raw replay observed0.88s/190,520KiB versus4.45s/393,528KiB combined, one sample per
mode, not a fresh-capture benchmark or generalized speedup claim. The recorded
project-raw-effect-study.json preserves commands, handles, counts and differences.
This is a workflow gain without new numeric project coverage or a retirement claim.

Integrated validation: all3,500 Rust tests pass with five existing skips, plus
doc tests. Locked all-target Clippy, formatting/diff checks and all244 offline
Python checks pass. Style has zero violations over9,827 functions; patterns has
zero unreviewed candidates and all13 unfiltered IDs equal main. Fetching
origin/main confirms it remains an ancestor. Logs use /tmp/clones-c64-*.
The full migration remains open; next consumer improvements should prioritize
measured real-source usefulness and existing collectors over adding algorithms
or expanding proofs just to match unsupported diagnostic details.


### C65 — reuse exact review workflow for guard opportunities (2026-10-29)

Completed guard assessments now feed the existing exact/positive review workflow
and original declaration index. No parallel detector, source parser, hash or
proof scan was introduced. Borrowed assessment/source views retain the original
owner population, while snapshots persist only the selected exact source region,
context and existing versioned owner locator. The original function-tail guard
and each closure opportunity are reviewed independently using a saved-run owner
key and zero-based opportunity index. The store envelope advances to schema 10;
schemas 1–9 remain readable without lookup writes.

Exact review masks affect individual guard text and finding exits. Partial sibling
dismissal leaves the owner visible; raw guard assessments, changed-scope totals,
edit plans and strict results remain intact. Changed-file selection limits explicit
dismissal. Completed closure opportunities can be reviewed inside incomplete
owners, but failed/budgeted assessments cannot publish snapshots. Global incomplete
coverage remains exit two even when every known opportunity is hidden. Absent
opportunities remain unavailable during status and never authorize pruning.
Completed-negative lifecycle, guard baselines and legacy retirement remain open.

Seven independent CLI cases cover ordinary guard-only and raw modes, full/compact
source/tool-free replay, line movement and changed source, sibling closure isolation,
old schema read-only lookup, invalid selectors/stores and absent-opportunity pruning.
Four model cases cover exact region equality, independent closure slots, source
changes, contexts and versions. The real-source study reuses the original C59
verification session module capture without recapture. Its within closure moves
from one visible opportunity to zero; raw assessments/plans/functions/strict fields
stay equal and both executions exit two for incomplete scope. Replay samples were
0.064s before review and 0.029s with dismissal; these single samples establish a
workflow result, not a benchmark or speedup. See project-guard-review-study.json.


Integrated validation: all 3,511 Rust tests pass in 229.303s, with five existing
skips, plus doc tests. An older density-envelope test initially rejected newly
supported schema 10; it now checks schemas 7–10 and rejects schema 11. All 153
review model tests also pass. Locked all-target Clippy, formatting/diff checks
and all 244 Python checks pass. Style has zero violations over 9,853 functions;
patterns has zero unreviewed candidates, with the same 13 unfiltered IDs as main.
The new source slicing delegates to the existing byte_range helper through a
small public seam. No added hash implementation was found. origin/main remains
an ancestor, including complexity metrics. C64 CI run 40884157917 passed macOS 15,
Ubuntu 24.04 and Ubuntu 24.04 ARM, confirming the earlier canonical temp-path fix.
Logs use /tmp/clones-c65-*. The capability goal remains active and incomplete;
next work should prioritize useful real-project coverage and consumer-specific
prerequisites over new parallel algorithms or review-only expansion.


### C66 — separate structural proposal input completion (2026-10-29)

The proposal review and baseline adapters previously inherited full advisory
completion, including unrelated complexity/effect gaps. They now consume the
already-completed per-unit pattern graph report. Capture/handoff failures,
unavailable functions, missing graphs and absent pattern requests remain
incomplete. Numeric diagnostic review/baseline and retained-owner accounting
keep their original full-advisory boundary through an explicitly named accessor.
There are no new configuration flags, detectors, proofs, source scans, hashes or
proposal algorithms. The optional proposal-policy patch remains deferred.

An independent Cargo fixture captures a helper proposal and a caller while
omitting its dependency's effect owners. Structural review input is complete;
effect evidence, numeric review and overall advisory remain incomplete, with
exit 2. Raw source/tool-free replay preserves the report. A separate generated
owner fixture proves missing structural input still makes reviews incomplete.
Strict matching remains unrequested in the raw-mode fixture. Proposal baseline
input uses the same structural boundary, while publication still requires full
successful analysis. This does not yet provide independent pattern invocation,
change generated-owner scope, migrate the legacy ignore ledger or retire patterns.

Integrated validation: all 3,513 Rust tests pass in 229.155s with five existing
skips, plus doc tests. Locked all-target Clippy and all 244 offline Python checks
pass. Style has zero violations over 9,854 functions; reviewed patterns has
zero unreviewed candidates and the same 13 unfiltered IDs as main. Formatting
and diff checks pass. Logs use /tmp/clones-c66-*.

A fresh same-worktree rstyle-ccgraph comparison produces exactly equal complete
proposal evaluations: two signature groups, one candidate and no suppressions.
The shared run retains 238 known/176 unavailable owners and 23 unresolved callees;
its structural/effect inputs remain incomplete and exit 2, versus legacy exit 0.
Thus C66 does not itself gain real-project coverage or establish replacement.
The study records the exact run, build context, commands and single timing samples
in project-pattern-completion-study.json. The next priority is an independent
structural consumer scope, then generated-owner scope and legacy ledger migration;
no additional proposer or proof machinery is needed. C65 CI run 40888372155 is
green; native CI for C66 remains pending until pushed.


### C67 — request compiler patterns independently of numeric style (2026-10-29)

`--compiler-patterns` no longer requires `--compiler-advisory`. The existing
`--compiler-advisory-only` capture mode accepts either consumer. No new CLI flag,
proposal policy, proof, hash or source/artifact scan is introduced. A small core
input seam reuses its existing merge, path exclusions and completed effects;
pattern-only evaluation skips numeric rules, density summaries, fix groups,
numeric ownership/scopes and diagnostic review snapshots. Supplied effects avoid
both propagation and local graph rebuilding. Buffer-identity tests verify that
source and completed effect vectors move into the consumer.

The existing containing compiler_advisory report explicitly marks
`diagnostic_execution: not_requested` and omits unit evaluation objects. Combined
requests retain their numeric evaluation and original completion semantics.
Incomplete effect annotations remain visible without invalidating independently
complete structural graph input. An omitted dependency fixture now exits 0/1 in
pattern-only mode versus 2 with numeric advisory, with exactly equal proposals.
Missing/generated owners, uncaptured graphs, failed captures and unobserved
selected files remain incomplete. Numeric baselines and compiler plans still
require numeric advisory. Existing proposal reviews/baselines work independently;
no new review identity/schema or parallel proposer was added.

Changed-file reporting reuses the existing owner aggregation. Combined mode
retains numeric counts. Pattern-only mode counts each visible selected proposal
once at its first matching usage or definition path; raw proposal inventories and
whole-scope completion remain intact. Strict matching runs normally unless the
existing raw capture flag skips it. Six independent CLI tests cover legacy/shared
proposal parity, distinct effect/structural outcomes, source/tool-free replay,
selection, known missing inputs, reviews/baselines and retained strict matching.
Two core tests cover selected effect parity, moved buffers and missing results.
Lead owns shared interfaces/integration; the independent acceptance worker wrote
only its new test leaf, with no concurrent builds or commits.

The exact C66 rstyle-ccgraph capture is replayed in three alternating samples per
mode. Patterns, 238 known owners, 176 unavailable owners and 23 unresolved callees
stay equal; pattern-only execution omits the 238-owner numeric evaluation. Both
modes remain incomplete and exit 2. Median elapsed times are 0.4176s combined and
0.4155s pattern-only, with approximately 104 MiB peak RSS in both. The differences
are negligible: no speedup, useful-proposal quality or increased project coverage
is claimed. See project-independent-pattern-study.json for commands, raw samples
and input context. Generated-owner scope and legacy ledger/routing migration are
the next actual replacement gaps; broader quality/retirement gates remain open.

Integrated validation: all 3,521 Rust tests pass in 230.342s with five existing
skips, plus doc tests; nine focused core effect cases and six new CLI cases pass.
Locked all-target Clippy, formatting/diff checks and all 244 offline Python tests
pass. Style has zero violations over 9,860 functions; patterns has zero unreviewed
candidates and the same 13 unfiltered IDs as main. Logs use /tmp/clones-c67-*.
Native C67 CI remains pending until push; the full migration goal stays active.

### C68 — reuse direct effect collectors for generated owners (2026-10-29)

Expanded function signatures previously returned before either existing event
collector ran. Fresh compiler-input captures now retain their mutation, external
call and intrinsic-effect observations under the separate
rstyle.compiler-generated-effects/1 metric. A borrowed EffectEvents seam extracts
the existing core classification body; the FnFacts wrapper keeps identical
behavior. No duplicate classification, propagation, policy flags, source proof,
hash or source/artifact scan is introduced. Buffers move from the collectors
into the observation and classification borrows their event slices.
Display paths reuse the captured owner source-file helper; the partial record
does not duplicate paths or reread source files.

Per-unit generated_effects rows identify the scoped function and expansion
callsite, retain raw events and display direct classifications. Source/tool-free
replay reuses those events under ordinary core configuration. This is partial
collector coverage: local calls and opaque behavior can have further effects;
empty event lists do not establish purity. Source, complexity, density, pattern
graphs and suppression scopes are not fabricated. Generated owners stay in
unavailable, full coverage stays incomplete and exit 2 remains. Existing authored
metric versions, completed propagation, numeric/planning results and strict
matching permissions are unchanged. Historical captures without this observation
remain unavailable rather than acquiring known-empty event lists.

Independent CLI acceptance covers five generated owners from one invocation,
empty/write/IO/indirect/unsafe outcomes, distinct keys, authored/generated
separation, missing transitive effects, raw source/tool-free replay, changed-call
classification and absent historical records. Core differential tests preserve
mutation deduplication/order, call precedence/dynamic unknown and intrinsic
classification. Codec tests preserve missing rosters and explicit event fields.
The acceptance worker owned only its new leaf; lead owns capture, transport,
report wiring, docs and integration. Worker writes are frozen for integrated gates.

A fresh rstyle-ccgraph library capture gains direct observations for all 176
previously unavailable generated owners: 101 mutation/borrow events, 435 external
calls and 41 classified escaping mutation targets. 135 owners have no classified
direct event, without a purity claim. A derived Debug formatter's escaping mutable
borrow and the Copy-handle Clone control are independently checked against the
source derive. Replaying the exact C66 capture yields no generated rows; authored
numeric evaluation, pattern report, unresolved callees, observations and missing
owners exactly equal the fresh report. Both exit 2. Final capture takes 16.810s; this
is one sample, not a speedup claim. See project-generated-direct-effects-study.json.
Generated propagation and actual pattern/source replacement coverage remain open.

Integrated final-snapshot validation: all 3,528 Rust tests pass with five existing
skips, plus doc tests. Locked all-target Clippy, formatting/diff checks and all
244 offline Python tests pass. Style reports zero violations over 9,867 functions;
reviewed patterns has zero unreviewed candidates and unfiltered discovery retains
the same 13 IDs as main. The style gate's initial accumulator-write and phase
spacing findings were fixed through a small unavailable-owner helper and blank
lines, without suppressions or configuration changes. Logs use /tmp/clones-c68-*.
Native CI for C68 remains pending until push. Generated propagation, structural
coverage, ledger/routing migration and broader replacement gates remain open;
the full goal stays active.


### C69 — reuse propagation through generated callees (2026-10-29)

Generated effects now retain already-collected call edges, function IDs and trait
implementation targets. A borrowed EffectInput/EffectSource seam lets the existing
core graph builder, all_effects setup and bounded propagation consume authored
and generated owners together. No generated FnFacts or source metrics are
manufactured. Completed generated classifications are reused; collector buffers
move and graph inputs borrow. No new policy framework, strict admission gate,
source/artifact scan, authority proof or hash is introduced.

Fresh aggregate metrics use rstyle.compiler-style-metrics/7 and generated records
use rstyle.compiler-generated-effects/2. Historical metric6 and direct-only /1
replay retain their original behavior. Local IDs remain scoped to compilation
units, foreign resolution requires selected direct Cargo contexts, and ambiguous
or missing targets remain unresolved. Generated source, numeric and structural
coverage remains unavailable with exit2. Existing authored suppression boundaries
and local-only unsafe/unknown semantics remain unchanged. Plans project onto
actual authored owners and explicitly retain incomplete coverage.

Independent CLI cases cover generated chains/cycles at depths0–3, delegated
writes, authored suppression boundaries, invocation comments, source/tool-free
configuration replay, exact selected dependency units, dynamic trait target
intersection and generated default methods. Core fixtures compare full facts
against borrowed views. Acceptance worker owns only its trait test leaf; all
worker writes are frozen. Lead owns shared seams, registration and integration.
Fresh rstyle-ccgraph capture completes propagation for all176 generated owners
and resolves two previously missing local generated targets. Their expanded call
edges expose two additional missing dependency targets, so unresolved total stays
23. Authored summaries/findings and structural proposals remain exactly unchanged;
no numeric quality gain is claimed. Replaying the exact C68 metric6 capture yields
an exactly equal compiler report. The fresh sample takes17.088s; replay takes0.440s,
measuring different work with no speedup claim. See
project-generated-propagation-study.json. Broader generated source/structural
coverage and legacy command/ledger migration remain open.

The first integrated run found eight failing cases: outdated generated-unresolved
expectations and an associated check/test coverage isolation issue. Expectations now distinguish
resolved generated observations from unavailable source metrics; a separate omitted
dependency case retains the missing-input review guard. Coverage follows each
unit's resolved dependency scope rather than allowing a missing test harness to
invalidate its separate library unit. All201 focused compiler-advisory tests pass;
final integrated validation follows below.


Final source-snapshot validation: all3,537 Rust tests pass in233.051s with five
existing skips, plus doc tests. All201 focused compiler-advisory cases, locked
all-target Clippy, formatting/diff and244 offline Python tests pass. Style has
zero violations over9,891 functions. Patterns has zero unreviewed candidates and
the same13 unfiltered IDs as main. Logs use
/tmp/clones-c69-*. Prior C68 native CI is green on Ubuntu ARM, Ubuntu x64 and
macOS15 (run40901258489); C69 native CI remains pending until push. The next
reuse-first frontier is legacy patterns option/routing compatibility, followed
by explicit ledger migration; generated structural boilerplate is not a
prerequisite for the legacy collector's authored scope. Full replacement and
broader useful-project quality/retirement gates remain open.

### C70 — share legacy mining controls with compiler proposals (2026-10-29)

Compiler patterns now accept the existing MineArgs controls: threshold,
min-coverage, pair-coverage, normalize, single-call-guard, max-holes and repeatable
hide-implements. Both CLI paths reuse the same argument type, conversion and core
Options/proposer. These flags require compiler-patterns; defaults and strict
matching remain unchanged. This delivers configurable proposal reevaluation over
saved compiler graphs rather than another mining algorithm. Score/top display
filters, command routing and deliberate legacy ledger migration remain open.

Reports retain actual Params and nonempty hide strings. Existing exact review
snapshots and baseline headers now consume those same concrete values rather
than hardcoding defaults, including empty inventories. Changed inputs stale
reviews and drift baseline scope; missing positives remain unavailable. The
schema-1 hide field is additive, absent historical lists mean the previously
fixed empty oracle, and empty lists are omitted to preserve default publication.
No proof, hash, source scan, trust policy or new configuration framework is added.

Independent acceptance covers getter work versus forwarding controls, literal
helper hole limits, all input values, source/tool-free reevaluation, exact/stale
reviews, empty baseline drift and CLI prerequisites. A trait-hiding oracle fixture
exactly agrees with legacy evaluation. The acceptance worker owns only the new
leaf; lead owns shared CLI, report/review/baseline seams, registration and docs.
All worker writes are frozen for integrated validation.

The retained C69 rstyle-ccgraph run replays with an exactly unchanged default
compiler report. Explicit single-call-guard/max-holes4 produces three candidates
instead of one; its full evaluation exactly equals a same-scope legacy run with
those options. Independent source inspection rates both extra candidates low
value: trivial count getters and collection pipelines with different output and
ordering. No quality gain or extraction recommendation is claimed, defaults and
ledger remain intact. Replay samples take0.432/0.415s versus23.822s for legacy
source extraction; these measure different work and do not establish a speedup.
See project-compiler-mining-options-study.json. Generated coverage remains
incomplete and replay exits2. Multi-unit grouping and retirement remain open.

All62 focused pattern/review tests, locked all-target Clippy, formatting/diff
checks and244 Python tests pass. Style reports zero violations over9,894 functions.
The initially oversized snapshot helper was split at its retained-endpoint lookup,
without changing semantics or adding suppressions. All3,543 Rust tests pass in
233.545s with five existing skips, plus doc tests. Patterns has zero unreviewed
candidates and the same13 unfiltered IDs as main; logs use /tmp/clones-c70-*. C69 native CI remains in progress
(run40906051494); no C70 native platform result is claimed before push.

### C71 — share score/top presentation for compiler proposals (2026-10-29)

Shared PatternDisplayArgs and the existing core inclusive score/top selection now
serve legacy and compiler patterns. Per-unit display masks refer to raw candidate
and signature-group indices; they borrow the retained evaluation and do not mine,
prove, hash or copy graphs again. Reviews apply before top, changed files intersect
afterward, and raw snapshots/baseline inventories remain independent of display.
Finding exits and text consume the same masks; missing input remains exit2 even
with top0. Defaults omit presentation and preserve prior reports.

Independent acceptance covers inclusive score boundaries, independent group masks,
review refill, changed-file intersection, top0 raw baselines/text, per-unit limits,
source/tool-free replay and missing-input/CLI prerequisites. The first focused run
exposed an existing changed-scope accounting omission: when numeric advisory and
patterns were both enabled, the path distribution counted only numeric findings.
It now includes displayed patterns alongside numeric findings.

The retained C69 rstyle-ccgraph capture preserves the default report exactly. With
single-call-guard/max-holes4 the raw three-candidate evaluation is identical under
top1 and top0, showing one/zero indices respectively. Both preserve incomplete
exit2. This improves presentation, without claiming more useful proposals, better
coverage or a speedup. See project-compiler-presentation-study.json. Command routing,
explicit legacy ledger migration and broader retirement gates remain open.
All3,550 Rust tests pass in233.002s with five existing skips, plus doc tests.
All244 Python checks, Clippy and formatting pass. The pattern gate then identified
duplicate total/selected counting logic; a shared shown-candidate iterator removes
it, with all68 focused pattern/review tests and Clippy passing again. Final style
has zero violations over9,902 functions. Patterns has zero unreviewed candidates
and the same13 unfiltered IDs as main; logs use /tmp/clones-c71-*. No native C71 platform result is claimed
before push. C69 native CI passed; C70 Ubuntu x64 passed while ARM/macOS were
still running at the latest check.

### C72 — opt-in patterns command routing (2026-10-29)

Patterns now accepts engine=compiler while retaining legacy by default. The route
moves shared mining/display options into the existing compiler-patterns,
advisory-only compact workflow, initialized from the same clap defaults. Shared
capture verification and run_captured seams keep extraction, handoff, evaluation,
reviews, reporting and exits single-sourced. No new collector/proposer, body proof,
hash or policy framework is added. Strict matching and numeric rules are not
requested. Scoped unit reports and missing-input exits remain intact.

Global package/manifest/all-target selection remains available. The route consumes
explicit/default legacy rstyle.toml for core exclusions/effects, supports locked
capture and exact source/tool-free replay, and accepts existing scoped review
files/status. Nonempty legacy ledgers require an explicit no-ignore bypass until
deliberate migration is delivered; originals remain unchanged. Compiler-only flags
reject legacy routing and show-ignored rejects compiler routing with the scoped
review alternative. No existing engine is retired.

Independent acceptance targets shared option/report parity, per-unit scope,
source-free replay, exact scoped reviews, generated missing-input exits, unchanged
legacy dismissal/default behavior, malformed/nonempty ledger handling and config
selection. The retained C69 rstyle-ccgraph replay produces an exactly equal compiler
report through routed and direct entries with the same repository config; strict
and numeric execution are not requested, legacy ledger bytes remain unchanged,
and both runs exit2. No coverage expansion, speedup or retirement claim is made.
See project-patterns-routing-study.json. All75 focused pattern/review tests pass.
Clippy initially flagged one long acceptance test; its direct-entry comparison is
now a reusable helper. Legacy style identified orchestration/spacing/assignment
issues; argument construction now uses an immutable Args update, capture/config
preparation is a thin boundary, and accepted-pair counting is shared separately.
No suppressions, threshold changes or ignores were added. Independent production
review found no correctness regression. All3,557 Rust tests pass in234.053s with
five existing skips, plus doc tests,244 Python checks, Clippy and formatting.
Final style reports zero violations over9,912 functions; patterns has zero
unreviewed candidates. The same13 unfiltered candidate IDs as main remain. Logs use
/tmp/clones-c72-*. Native C72 platform checks remain pending until push; C70 native
CI passed while C71 remained in progress at the latest check.

### C73 — explicit legacy dismissal migration (2026-10-29)

Compiler patterns now previews a requested legacy ledger and imports one explicitly
selected legacy ID/unit/candidate into the existing scoped review store. Routed
patterns exposes preview/import alongside the direct clones workflow. Preview
borrows raw candidates and completed snapshot outcomes, reuses ledger::matches,
and keeps top/score presentation independent. Import uses the existing exact
review recorder and atomic save; the original ledger is never written. Growth
entries require explicit current-members-only conversion; later members are not
implicitly dismissed. No new hash, source scan, clone admission or proof gate is
introduced. A missing explicitly requested ledger errors rather than becoming an
empty inventory.

Seven independent acceptance cases cover raw top0 preview, direct entry parity,
exact import, source/tool-free replay, reason override, membership/body/settings
staleness, multi-context selection, growth conversion, incomplete scope and atomic
failure. A literal edit retains the old shape-ID match while invalidating the
imported full snapshot. This verifies that legacy matches suggest migration,
while current scoped observations define review identity.

Read-only replay of the retained C69 rstyle-ccgraph capture finds one matching
legacy dismissal among26 entries;25 unmatched entries remain unavailable over the
incomplete scope. Top0 displays no candidates while preserving the migration
match. Raw proposal evaluation and display masks equal the direct configured
entry, both preserve exit2, and the repository ledger remains byte-identical.
No actual repository dismissals were imported. See
project-patterns-legacy-migration-study.json. This establishes compatibility
workflow, not expanded coverage, quality, speedup or retirement readiness.

Initial focused checks pass all82 pattern/review tests and244 Python checks.
Clippy's oversized acceptance function was split at its direct-entry comparison.
Style's oversized orchestration function now delegates row-to-decision construction
to Decision::new. An intermediate collection helper triggered a new legacy
abstraction candidate and was replaced by that constructor, without suppressions
or ignores. All3,564 Rust tests pass in235.099s with five existing skips, plus
doc tests;244 Python checks pass. Final style has zero violations over9,922
functions. Patterns has zero unreviewed candidates and the same13 unfiltered
IDs as main. Locked all-target Clippy, formatting and diff checks pass; logs use
/tmp/clones-c73-*. Next discovery frontier:
cross-package compiler proposal grouping through the existing core miner,
preserving compilation contexts and shared-source identity rather than flattening
raw function IDs. A useful independently reviewed cross-package positive is still
needed; existing closure caveats do not justify blanket suppression or a new
proof gate. C71 native CI passed; C72 Ubuntu x64/ARM passed while macOS remained
in progress at the latest check (run40914176818). C73 native CI remains pending
until push.


### C74 — explicit authored compiler pattern population (2026-10-29)

Lead owns CLI, consumer completion/selection, baseline header and documentation;
acceptance worker owns the authored_scope leaf. The existing legacy collector
already excludes expansion signatures. Compiler patterns now offers explicit
compiler-pattern-scope=authored, using the captured macro_generated_function
marker to report those unavailable owners outside the requested population.
Captured remains the default. No new collector, proposer, hash, source scan or
proof gate is introduced; IDs move into the scope report. Raw unavailable and
generated effect inventories remain intact. Numeric/complexity/effect completion
and strict decisions retain their prerequisites. Pattern-only completion and
changed-file reports use the requested population, counting outside-scope owners
separately. Missing observations/graphs and unknown origins stay incomplete.

Proposal baseline headers retain population scope (historical absence means
captured), so even empty inventories report drift across populations. Unchanged
positive review identity remains tied to the same existing body/context. Legacy
migration preview also names its requested population. Text names authored scope;
default JSON/text omits new scope fields. A read-only review found changed-file
coverage had treated known source metrics as enough when a requested graph was
missing. Selection now removes those typed owner keys from its completed set,
without another collection/evaluation pass.

Nine independent real-Cargo acceptance cases cover derives, useful helper
findings, source/tool-free replay, check/test populations, numeric incompleteness,
strict decision equality, selected missing graphs, baseline export/header drift,
review identity, missing observations and historical marker-only replay. One
initial test incorrectly expected generated exclusions to make strict matching
incomplete; comparison with standalone strict matching confirms its completed
result is unchanged. Another fixture omitted the existing required review reason;
both acceptance setup issues were corrected without adding runtime gates.

The exact retained C69 rstyle-ccgraph capture contains414 owners:238 known authored
graphs and176 unavailable generated signatures. Captured replay still exits2;
explicit authored replay exits0 with the same raw proposals, measurements,
unavailable roster and effect limitations. Authored baseline export and comparison
complete, retaining one existing/zero new proposals. Captured comparison reports
scope_changed and exits2. The one proposal remains independently unsuitable;
this is a scope-aware workflow gain, not better proposal quality, more captured
semantics, speedup, purity or retirement. See
project-patterns-authored-scope-study.json. All106 focused checks and244 Python
tests pass. Style initially found one long function and two per-target mutation
counts; scope selection, generated-effect retention and coverage counting now
have cohesive helpers without suppressions or threshold changes. Patterns has
zero unreviewed candidates. All3,574 Rust tests pass in236.431s with five
existing skips, plus doc tests. Final style has zero violations over9,930
functions. Unfiltered patterns retains the same13 candidate IDs as main. Locked
all-target Clippy,244 Python tests, formatting and diff checks pass. Logs use
/tmp/clones-c74-*. C72 native CI passed on Ubuntu x64/ARM and macOS; C73 Ubuntu
x64/ARM passed while macOS remained in progress at the latest check
(run40916805535). C74 native CI remains pending until push. Cross-package
proposal discovery and broader independently reviewed quality remain open; no
legacy backend is retired.


### C75 — independent compiler complexity ranking (2026-10-29)

Lead owns compiler request routing, shared core selection seam, execution/report
status and documentation; acceptance worker owns the independent_ranking leaf.
Compiler complexity ranking now runs without full numeric advice or pattern
proposals. It directly reuses captured measurements, the evaluator's existing
merge/exclusion step and core complexity ordering. Ranking-only requests skip
numeric rules, generated effect classification, propagation and fix planning.
Diagnostic/effect execution is explicitly not_requested; empty unrequested effect
outputs do not mean purity or measured-empty. Direct unresolved calls remain
visible, without generated-body resolution or inherited unresolved references.
They do not invalidate measured complexity. Missing owners and historical/null
scores still make the ranking incomplete. Combined numeric/pattern behavior,
compilation identities and strict matching remain unchanged. No new collector,
score algorithm, scope flag, proof, hash, source scan or dependency permission.

Five independent acceptance cases cover ranking-only unresolved dependency calls
versus numeric incompleteness, identical ranked scores and combined numeric
findings, exclusions, source/tool-free replay, missing generated owners and
historical/modern scores, and separate check/test denominators. One initial test
incorrectly required the generated test harness population to be complete; it
now independently checks completeness against missing-owner/measurement counts.
All 13 focused ranking checks pass. Independent read-only implementation review
found no correctness issues. Clippy's orchestration length limit was addressed
with a shared execution-state mapping; style's length finding was addressed by
moving existing completed-result construction into a constructor, preserving
ownership and avoiding copies or repeated evaluation.

The retained C69 rstyle-ccgraph replay preserves identical rankings and measured
owner observations for 238 functions, while skipping 176 generated effect-owner
classifications. Both modes still exit 2 for 176 unavailable complexity owners;
no authored ranking scope or coverage improvement is claimed. Six serialized
alternating warm replays have median wall times 0.305s for numeric advice plus
ranking and 0.285s for ranking alone. These small samples include replay/JSON costs
and do not establish a general speedup. See
project-independent-ranking-study.json. Strict execution remains unrequested in
this study; the existing strict additive-ranking controls remain in the suite.

The first full run passed all 3,579 Rust tests before the style-only constructor
refactor. The final frozen code was checked again with the full Rust suite;
final results and native status follow below. Both legacy backends remain active.
Next ready semantic investigation: explicitly generated owners are outside the
original-source rules' applicability, yet currently block source baselines.
Assess that contract through the existing source assessment/review adapters,
retaining keys/reasons and keeping missing authored data incomplete. Do not
substitute expanded HIR statement counts for original spacing evidence or add
another policy/proof layer. Broader proposal quality and cross-package discovery
remain open; the migration is not complete.

Final frozen validation passes all 3,579 Rust tests in 235.969s with five existing
skips, plus doc tests, 244 Python checks, locked all-target Clippy, formatting and
diff checks. Style has zero violations over 9,934 functions; reviewed patterns has
zero unreviewed candidates and the same 13 unfiltered IDs as main. Logs use
/tmp/clones-c75-*. C73 and C74 native CI pass on Ubuntu x64/ARM and macOS
(runs 40916805535 and 40920137235). C75 native CI remains pending until push.
Current remote main is 1689392 and is already an ancestor of this branch.


### C76 — independent local advice and original-source applicability (2026-10-29)

Lead owns request routing, source assessment/report adapters, docs and project
study; acceptance workers own local_only and generated_applicability test leaves.
Local source, guard, mutation and effect consumers now run with the existing raw
advisory mode without requiring compiler numeric advice, ranking or patterns.
Only requested metadata is captured. Local-only runs omit compiler observations
and the compiler report, reuse the same local collectors/evaluators, and skip
strict certification and matching. No new hash, source scan or authority proof.

Explicit generated signatures are outside original-source rule applicability,
matching the legacy authored-source scope. Classification reuses the existing
source inspection result plus captured expansion marker, after original source
and context checks. It retains each generated owner, reason and not-applicable
assessment with no measurement, plus an outside_source_functions count. Unknown
origins, missing authored ownership and invalid bytes stay incomplete. Generated
owners do not supply original source review observations, resolve/prune previously
reviewed authored findings, or complete a parameter planner's edit population.
Guards, effects, compiler numeric advice and strict matching are unchanged.

Nine new acceptance cases plus existing baseline/raw-advice controls pass (53
focused checks). A fixture initially conflated direct mutation completeness with
helper-effect summary completeness; independent expectations now preserve the
direct caller's unknown boundaries while asserting completed effect summaries.
One overlong test was split into a cohesive review setup helper for Clippy.

Fresh rstyle-ccgraph source-only capture run-VfkAd5 evaluates 476 length/parameter
rules over 238 authored functions and retains 176 generated signatures outside
applicability, with zero unavailable source rules and zero findings. Complete
empty baseline export and comparison both exit zero. Raw artifacts contain no
compiler numeric/pattern observations; strict execution is not requested. See
project-original-source-applicability-study.json. Spacing was not requested; no
whole-workspace, speedup, proposal-quality or retirement claim is made. Missing
and source/tool-free controls are fixture evidence; real-project replay retained
live sources. Full integrated gates are pending below. C75 native CI was still
running on Ubuntu x64/ARM and macOS at the latest poll (40923413434).

Next bounded investigation: source review location lookup currently reuses a
dispatch declaration locator that rejects type/const generic impl owners. Reuse
its declaration-naming seam (or the existing core locator) without extending
dispatch eligibility. Three source-inspected study candidates in rstyle-core are
Ctx::step and Ctx::plan in stacking.rs, and GroupReports::group in groups.rs;
all use impl<F: Borrow<FnFacts>>. A study-only length10/params2 policy would expose
nonempty source findings. First measure their current review/baseline failures,
then test generic owner/context disambiguation and unrelated-impl stability. No
implementation or measured gain for this next candidate is claimed yet.

The initial full suite passed 3,584 cases and exposed four stale expectations
that generated test-harness owners make source review/changed-scope coverage
incomplete. Updated those cases to require complete authored-source coverage and
an explicit positive outside-owner count, preserving check/test review isolation
and exact changed-path occurrence assertions. The changed-file findings gate
now correctly exits1 instead of incomplete2. Legacy style found assess at51
lines; extracted its unchanged guard execution mapping into a borrowed helper.
Final frozen validation follows after these corrections.

While the final suite ran, lightweight source-only replay of retained core run-ChBnep
confirmed the next gap: 409 authored owners /806 outside signatures give complete
source assessment under the study policy, including exact counts17/3,37/2,34/3
for Ctx::step, Ctx::plan and GroupReports::group. Baseline export still exits2
("Incomplete analysis cannot publish a source baseline"); direct length dismissal
of Ctx::step also exits2. Evidence is retained in /tmp/clones-c77-core-before.json,
/tmp/clones-c77-core-baseline-before.log and /tmp/clones-c77-dismiss-before.log.
This is confirmed remaining review-locator work, not a C76 baseline-completeness
claim for rstyle-core or a C77 implementation.

Final frozen validation passes all3,588 Rust tests in237.130s with five existing
skips, plus doc tests,244 Python checks, locked all-target Clippy, formatting and
diff checks. Legacy style reports zero violations over9,936 functions; reviewed
patterns has zero unreviewed candidates and the same13 unfiltered IDs as main.
Two independent read-only reviews found no correctness blockers. Logs use
/tmp/clones-c76-*. C75 Ubuntu x64/ARM passed; macOS remained in progress at the
latest poll. C76 native CI is pending push. No legacy backend is retired; the
full capability migration remains active.


### C77 — generic original declaration review names (2026-10-29)

Lead owns the existing source location/index and naming seam, registration,
docs/study and integration. Acceptance worker owns only generic_methods.rs;
read-only review owns no writes. The previous source-only core replay established
complete source assessment but failed generic method dismissal and baseline
identity. This increment extends the existing declaration locator rather than
adding another consumer or requiring compiler numeric advice. Generic owner and
local-trait arguments use the shared canonical original-token helper; type/const
specializations stay distinct. Bounds and where clauses on these names require
no dispatch proofs. Existing same-module naming, exact original context/header
checks and bounded immutable source ownership remain. No hash, identity digest,
source fingerprint scan, new collector or dispatch/strict matching permission.

The shared cached declaration index can now name root methods whose original
parser could not obtain a complete owner/trait name, as well as external methods.
Read-only review caught two issues before integration: nongeneric owners in
generic local-trait impls initially rejected impl type binders, and routing all
root lookups through Locations added unrelated-file resource limits. Both are
fixed. Already complete root names keep the original SourceIndex path; its
existing owner parser now records whether the name is complete. Missing generic,
lifetime or trait-argument names use cached Locations. Original root names such
as crate::::many or names missing trait arguments were incomplete; they are not
transferred to the new identity. External lifetime-only nominal names stay stable.

Eight independent real-Cargo acceptance cases cover root/external generic
methods, nonempty source baselines and saved reviews after unrelated impl
insertion, exact type/const/local-trait distinctions, generic traits on nongeneric
owners, source/tool-free replay, existing root review beside an unrelated300KiB
file, malformed retained root names that cannot hide/prune, and missing/duplicate
evidence. Existing tests that explicitly rejected generic owner names now require
successful review for those two forms while keeping aliases, qualified/imported
owners/traits, default trait methods and lifetime-generic-only traits unavailable.
Focused50 PDG cases and37 source CLI cases pass; all eight final new cases pass.
Clippy identified an overlong test; its unrelated-impl setup is now a coherent
helper. Legacy style spacing findings were corrected without changing limits.

Retained rstyle-core run-ChBnep verifies original captured function bytes still
match current source for Ctx::step (17 lines/3params), Ctx::plan (37/2) and
GroupReports::group (34/3). Under study-only length10/params2, all five positive
findings now save and replay hidden_exact. The retained before-command shows
Ctx::step length dismissal exiting2 for missing indexed declaration. Full source
assessment remains409 authored/806 outside functions,263 raw findings. All77
positive parameter findings now have complete review coverage. Length review
still has two unavailable plain-prelude-trait impl identities (reported by the
compiler as std::default::Default); the
package baseline still refuses export and the overall reviewed run exits2.
These limitations are explicit in project-generic-source-review-study.json.
No full-core baseline, speedup, broader proposal quality or retirement claim.

The next confirmed source workflow blocker is naming the original config::Weights
and config::EffectPaths implementations, whose source spells `impl Default`.
Existing captured compiler names already contain the resolved trait path,
independently of numeric capture. Assess reusing those names as opaque review
locators after existing declaration/context checks, without resolving imports
again or treating the names as dispatch authority. This gain is not yet measured.
Final integrated snapshot: 3,597 Rust tests pass in238.869s with five existing
skips, plus doctests; Clippy, formatting and244 Python checks pass. Legacy style
reports0 violations over9,939 functions. Reviewed patterns reports0 unreviewed
candidates; unfiltered discovery retains exactly the same13 IDs as main.
Native CI for the preceding C76 commit (run40926858455) has both Ubuntu jobs
passing; macOS remains pending. C77 native validation follows the pushed commit.

### C78 — reuse captured compiler names for source reviews (2026-10-29)

Lead owns declaration indexing, registration, docs/study, integration and Git;
acceptance worker owns only the new compiler_named_methods CLI leaf; reviewer
checks fallback boundaries and next consumer blockers without edits.

Existing HirFnFacts.name is always captured from rustc independently of numeric
advisory requests. Original methods on supported same-module nominal owners now
retain their checked declarations when only a nonlocal/prelude/imported trait
origin lacks a local source name. After existing source/context/file/header
checks, review uses the entire compiler name as an opaque locator. Existing
owner_observations duplicate detection still rejects ambiguous final locators.
The existing binding map distinguishes nominal, imported and other bindings;
there is no separate import-name set. Duplicate entries explicitly become
ambiguous through one map-entry lookup. There is no additional name census,
import resolver, collector, hash, fingerprint
scan, policy version or dispatch proof. Strict matching is unchanged.

Focused acceptance caught a root shortcut that accepted plain trait syntax as a
local source trait even when no such local declaration existed. Root lookup now
consults the existing local trait map before using its source-name route. Actual
local source locators remain unchanged; old incorrectly attributed root names
remain unobserved, cannot hide current findings, and cannot be pruned as known
negatives. Alias owners, unsafe/default impls, default trait bodies and other
previously unavailable owner forms retain their bounded scope. Empty and
impl-index compiler names remain unavailable.

All six new real-Cargo cases pass: root/external prelude Default, explicit versus
import-aliased trait spelling, unrelated impl movement, changed measurement
staleness, source/tool-free raw replay without numeric observations, colliding
compiler names, missing/unstable names/context, and old root review handling.
Two PDG cases exercise unnamed declaration retention and original header/context
checks. Existing external parameter controls now require successful review for
qualified and imported trait origins while retaining unsupported-owner cases.
Clippy passes; full integrated validation is in progress.

Measured project gain uses the same retained core run-ChBnep and study-only
length10/params2 policy as C77. Both config.rs Default method bodies still match
the current original source bytes. Weights::default measures13 lines and
EffectPaths::default16. Both save/replay hidden_exact with complete length review
over186 positives. Full source baseline export now exits0 with263 entries, and
comparison exits0 with263 existing/0 new and unchanged scope. Source assessment
remains complete over1215 owners:409 authored,806 generated outside,818 evaluated
rules,0 unavailable. Numeric compiler reports, strict assessment and matching
remain unrequested. Evidence is project-compiler-named-source-review-study.json;
no fresh capture, speedup, default-policy cleanliness or retirement claim.

C76 native run40926858455 has now completed successfully on both Ubuntu runners
and macOS. C77 run40930466411 is pending. The next concrete reuse-first consumer
blocker is source analysis rejecting the repository's full rstyle.toml sections;
reuse the existing core config parser while retaining explicit spacing opt-in
and unknown-key errors, then measure under the real policy.

Final integrated snapshot: all3,605 Rust tests pass in240.251s with five existing
skips, plus doctests. Clippy, formatting and244 Python checks pass. Legacy style
reports0 violations over9,941 functions; reviewed patterns has0 unreviewed
candidates and unfiltered discovery retains the same13 IDs as main. Initial
dog-food feedback removed redundant import-set state and made declaration
collision handling explicit; no ignores, thresholds or limits changed. Final
binary replay repeats263 baseline entries,263 existing/0 new and two exact
Default length reviews. C77 native CI has both Ubuntu jobs passing; macOS remains
pending. C78 native CI follows the pushed commit. The overall migration remains
active; the full-config parser reuse task is ready with retained rejection
evidence in /tmp/clones-c79-config-before.stderr and independent fixture targets.
