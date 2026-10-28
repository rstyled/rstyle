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
| C11c | Evidence + local + lead / typed call observations, leaf summaries, CLI/report and acceptance | delivered (bounded scalar leaf effects) | this checkpoint | Caller-visible scalar writes through original same-unit leaves, distinct call events and exact formal mappings | 2,553 nextest tests and doc tests pass; zero style violations and reviewed pattern candidates; exact 15 baseline IDs unchanged; source/tool-free strict replay passes |

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

The next ready C10b frontier is a witnessed local two-method capability set from
parallel wrapper calls, reusing the existing checked source/method index. Exact
existing-trait relations must suppress redundant advice; names/signatures alone
cannot establish a proposal. Broader trait/capability proposals, richer effect
families, caller-aware edit intents and retirement gates remain open. Native macOS
and exact Go comparison remain unverified.

## Hashing exceptions

New consumers use borrowed evidence and report-local typed handles, without new
hashes. Existing `source_identity::Scheme::LegacySha256` and
`concrete_types::digest` support actual historical source claims and frozen
source/oracle pins; immutable owners compute them only for those explicit claims.
The `clones::digest` helper is test-only historical test-contract compatibility,
not a new finding identity. New routine source publication uses existing XXH3.
This bounded inventory does not claim every historical fixture has migrated.
