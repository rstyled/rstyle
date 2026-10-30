# Caller-visible scalar mutation effects

Request the optional consumer separately from direct mutation budgets:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --locked --mutation-effects --format json --report-detail compact
```

Add `--compiler-advisory --compiler-advisory-only` to run this same summary
consumer, static `--effect-density`, or both without strict clone assessment.
Fresh requests retain the existing file census and bounded formal/control types;
the existing summary session and density cache borrow raw facts directly. No body
type enrichment or helper certification is needed for this bounded vocabulary.
Saved replay, density reviews/baselines and changed-file policy remain available
without source files or compiler tools. Old raw captures missing optional types
remain incomplete. Unsupported body diagnostics/work-budget outcomes may differ
from combined mode, without becoming complete measurements.

`mutation_effects` (`rstyle.mutation-effects/4`) describes original direct writes
and calls through at most two original same-selected-unit helper edges:
`caller → wrapper → leaf`. It does not require strict clone admission. The
vocabulary covers ordinary nongeneric free functions with owned `u32` or direct
`&mut u32` inputs, `u32` or unit results, and primitive scalar bodies. A leaf is
call-free; a wrapper requires every original call to resolve to a complete leaf.
Third-edge chains, external functions, function pointers, recursion, opaque
arguments, aliases, reference redirection, closures, unsafe operations and
nontrivial owned values remain incomplete. Checked direct witnesses in an
assessed function survive an unknown remainder; an incomplete wrapper cannot
supply a complete summary.

Checked ordinary `if`/`else`, unlabelled `loop`, primitive boolean `while` and
half-open `u32` `for` bodies retain original call and assignment sites once.
Both arms contribute distinct source occurrences; false conditions and empty
ranges still retain authored sites. Original condition/head/binder/body and
transfer correspondence use the direct inventory's existing checks. Immutable
scalar and wildcard range binders retain lexical scopes. A shadowed local
assignment does not invalidate an unrelated mutable formal with the same name.
Initializer expressions are visited before introducing local shadowing, and
range endpoints before introducing loop binders. Reference redirection, macros,
raw references and other opaque operations retain conservative boundaries.

This explicit effect observation does not change ordinary call-roster or formal-use
certification. Conditional operand calls, pattern conditions, labels, value breaks,
inclusive ranges, arbitrary iterators and mutable range binders remain incomplete.
Neither branch selection nor iteration multiplies static sites or establishes
execution frequency. A complete summary still requires all observed original
calls to resolve within the existing two-edge bound and work budgets.

The separate compiler `consumer_formal_types` observation records direct mutable
builtin `u32` formal types. Source spelling and rendered compiler type names do
not establish primitive referents. This observation supplies no graph admission,
alias, access or rewrite permission. Original source ownership, native binding
identity, signatures, call arguments and selected-unit context still have to agree.

The report also exposes the independent `statement_metric`
`rstyle.original-source-statements/1`. Each function's `statement_inventory`
contains availability and execution for a complete source-syntax count. Every
statement in every lexical block counts once, including block tails and control
statements. A bare match-arm body contributes one additional statement; block
arms contribute their contained statements. Both branches, empty loop bodies'
authored statements and source after transfers count once. Nested expressions
and calls do not count separately unless they contain blocks. This is an owned
original-source metric, not a native HIR or PDG node count.

Closures, nested items, macros, attributes, async/await, const and unsafe blocks
make the entire count unavailable. No supported-remainder denominator is emitted.
Function budgets prevent the inspection altogether. Its availability is separate
from scalar-effect coverage: a match can have a known source statement count
while its scalar effects remain incomplete. Full and compact saved replay use
the same owned source bytes without reopening project files.

Request `--effect-density` for the separate `rstyle.static-effect-events/1`
policy metric, optionally with `--effect-config effects.toml`. It can run alone or
alongside `--mutation-effects`; completeness requires every requested axis.
Density-only may complete for a classified safe external free call even though
its raw scalar mutation assessment remains incomplete.

Each direct escaped mutable binding contributes one mutation event regardless
of assignment count. Each original completed helper-call occurrence contributes
one mutation event if it maps any caller-visible write. Callee-local rebinding
does not escape. Configured non-mutation kinds use the legacy longest native-path
prefix classifier. A local original call contributes its own configured kind
and the distinct kinds of its complete summary's original inner calls, once per
kind per outer call. Repeated outer calls remain separate events. Summary kind
sets are computed once per owning analysis/unit and borrowed by occurrence;
there are no graph clones or hash cache keys.

Safe external free calls require independent native identity, safety and result
type, checked owned-u32 arguments, and a configured path classification. Unit,
u32 and never results qualify; methods, generic syntax, opaque/nested arguments,
mutable external arguments and unclassified calls remain incomplete. Missing
safety metadata in older captures does not become safe. This does not extend
same-unit scalar helper summaries or strict graph admission.

```toml
[side_effects]
max_density_milli = 500
min_statements = 3
min_lines = 10

[side_effects.weights]
mutation = 1
io = 3

[side_effects.paths]
time = ['std::thread::yield_now']
```

Weights/default path lists and minimum-size policy reuse legacy configuration.
The config also accepts workspace-relative `exclude` globs; unknown keys and
`transitive_depth` are rejected (the two-edge bound is fixed). Paths are native
compiler paths, including unqualified names for root local functions. With complete
numerator, statements and code lines, either minimum can exempt a small function.
Otherwise checked integer arithmetic computes
`1000 * weighted_events / max(statements, 1)` and reports a finding only when
strictly above the limit. Unknown work prevents both ratios and minimum-size
exemptions, even with zero unknown weight. Implicit arithmetic panics lie outside
this explicit static-event metric; unsafe/global/Drop-bearing syntax is unsupported.
It makes no runtime frequency, purity, termination or rewrite-safety claim.

Schema 4 retains raw scalar evidence and statement inventory, plus per-function
`density.event_inventory`, `measurement`, source policy and completeness. Named
`rstyle-allow(side_effect_density): reason` hides only visible findings; it does
not repair unavailable evidence. Exclusions are explicit policy skips retaining
raw evidence. `--fail-on-findings` uses visible findings; incomplete requested
scope still exits two. Full/compact replay recomputes policy from saved original
facts and current effect config without compiler tools or live source.

Each function's evidence reports `caller_visible_mutation`:

| Value | Meaning |
| --- | --- |
| `none_proven` | Complete bounded analysis found no caller-visible scalar writes. |
| `writes_formal` | Complete bounded analysis found writes through mutable formals. |
| `witnessed_writes_formal_with_unknown_remainder` | Checked write witnesses survive incomplete evidence elsewhere. |
| `unknown` | Incomplete evidence with no checked caller-visible write witness. |

These values make no claim about purity, panic freedom, termination, general
effects, dynamic write frequency, density or safe extraction. `none_proven` is
limited to this mutation vocabulary. An unused unsupported formal still prevents
a complete result: dropping a value may execute behavior absent from explicit
assignments.

Direct assignment-site counts stay with their original owner. Callee-owned local
rebinding does not become a caller write. Every summarized original call has its
own event, while repeated calls reuse completed leaf and wrapper proofs in the
owning analysis. Repeated inner calls retain separate paths even when they reach
the same original assignment. Argument mapping projects the original leaf formal
into the wrapper first, then the wrapper formal into the caller. Root sets
deduplicate only identical owner-scoped bindings; witness occurrences stay distinct.

Function `unit` and call `summary_index` select the report's summary row with the
same `unit` and `summary_index`. Summary `function_index` is the original owning
unit's function ordinal. `summary_site_index` indexes that summary's `sites`
witness roster. Each witness
retains `function_index` and `site_index` of its original direct assignment,
`call_path` of original helper call occurrences, and `mapped_root` projected into
the summary owner. The flattened `span`, `root`, `operation` and `caller_visible`
fields belong to the original write. Write spans are relative to that original
function's source. A direct witness has an empty path; a nested witness has one
wrapper call reference with its owner, occurrence index, original span and native
identity. The outer call event supplies the remaining edge. No nested leaf site
is presented as a wrapper direct assignment. These are report-local handles,
never stable review identities or executable serialized proof authority.

`--max-analysis-functions` bounds inspected owners including required callees.
Additional bounded call, summary and witness work can also exhaust the analysis.
Retained summary sites, path entries and expanded call mappings consume the same
existing witness allowance; shared proof does not make repeated expansion free.
Summary queries distinguish remaining depth, so a depth-limited result cannot
supply a deeper proof. Complete call-free leaf proofs are shared across depths.
Omitted prerequisites remain incomplete. Requested incomplete effect scope exits
two; the consumer adds no mutation-budget findings. Direct `--mutation-rules`
configuration and metrics remain separate. Default runs do not request effects.

An explicit `--replay-run PATH --mutation-effects` reevaluates retained original
facts without source files or compiler tools. It recomputes summaries inside each
immutable owning unit; it never imports a serialized clean-effect verdict or
shares summaries across analyses or selected units.


## Saved numeric density reviews

With `--effect-density`, apply `--review-file reviews.json`. To record a choice,
use `--replay-run RUN --dismiss-density FUNCTION_KEY --review-reason TEXT`.
Only a completed, selected, visible numeric finding with one checked original
owner qualifies. Review-store schema 7 adds `density_entries`; older stores stay
readable. JSON `density_review` uses `rstyle.density-review/1`.

A review retains the exact detector/metric versions, original locator/build
context, full density weights/path lists/minima/threshold and two-edge bound,
event-kind counts, statement count, code-line count and resulting density.
Changes to any of these become stale. This is a numeric-policy choice: body
edits preserving the complete measurement and identity can retain a choice;
it does not certify semantic equivalence. No source/graph clones or hashes form
review identity. Lookup borrows existing exact-review infrastructure.

Hidden reviews affect text and finding exits via `review_hidden_findings` and
`visible_findings`; raw scalar evidence, events, measurement and completeness
remain intact. Source/tool-free full and compact replay recompute the current
measurement before review matching. Unknown work, budgets, exclusions,
suppressions and minimum-size exemptions cannot supply a completed numeric
negative for pruning. `--review-status` accounts for all retained families;
`--review-prune --review-prune-dry-run` previews only independently completed
selected negatives under the retained policy. Changed policy, omitted families,
unselected owners, unavailable evidence and missing owners remain retained.

Density also propagates nonmutation event kinds through retained same-unit helpers
with original owned-u32 parameters and arguments. The helper body must have a
known scalar signature, complete original syntax, no caller-visible direct
mutation, and no unresolved noncall boundary. Every called external function
still needs independent native identity, safety, a supported result and an
explicit configured classification. Two total call edges include the external
edge: a caller wrapping a helper's external call can complete; an additional
wrapper remains unavailable. This path reads previously observed, context-checked
owners and completed nonbudget summary attempts. It does not grant a scalar
mutation summary or perform more extraction. Per-owner/depth cached kind sets
are borrowed; source/graph values are not cloned.


## Changed-file density findings

Add `--changed-files changed.json` to `--effect-density` to select file-level
presentation and finding exits. The shared manifest is
`{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}`; paths must be
normalized workspace-relative Rust files. No Git query or live source lookup
is needed, including during source/tool-free replay.

`mutation_effects.changed_scope` reports selected-file coverage, matched and
unavailable owners, and visible findings after saved reviews. Per-owner
`changed_file` records membership. Whole-scope `findings`, `visible_findings`, raw
events, measurements, exclusions, suppressions and execution remain available.
Unknown files and unclassified owners prevent a clean selection. An explicit
empty selection has complete empty presentation, but cannot hide incomplete
whole-scope analysis or exhausted budgets. Unselected reviews cannot be dismissed
or pruned as completed selected negatives.

When source rules and density are requested together, both use the same manifest
but retain independent coverage and findings. Other consumers' finding exits and
planning populations retain their existing behavior. Changed-line analysis and
saved CI finding baselines remain open.

## Saved density CI baselines

Export an explicit inventory after a complete analysis:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --effect-density \
  --effect-config effects.toml --write-density-baseline density-baseline.json
```

The destination must be new. Export validates the whole requested analysis and
writes a same-directory temporary file before atomic publication. Incomplete
capture, unknown density evidence, identity exhaustion or incomplete requested
consumers cannot publish an inventory. Export does not accept `--changed-files`;
a selected subset cannot replace a whole-scope baseline. No review file is needed.

Compare a current analysis or an explicit saved run with
`--density-baseline density-baseline.json --fail-on-new-density`. This separate
gate exits one for selected current findings without an exact baseline snapshot,
or a change in effective density policy, exclusions or build-context roster.
It exits two for incomplete requested analysis. Ordinary `--fail-on-findings`
continues to include existing findings. Baseline comparison never hides raw
measurements or changes saved reviews. Saved numeric reviews apply before
selected existing/new counts, and `--changed-files` may select comparison output.

`density_review.baseline` reports current coverage, selected visible existing/new
counts, header drift, current owner states and retained-entry status. An existing
finding requires the same original locator, build context, detector/metric
versions, complete numeric event distribution and source denominator. Moving
lines or changing a body without changing that completed measurement can retain
an existing numeric finding. This is not semantic or refactoring approval.

Retained status is separate from current coverage. Only a completed selected
numeric negative under the retained policy is `resolved`. Suppression, exclusion,
minimum exemptions, missing owners, unselected owners and unavailable identities
do not prove resolution. They stay explicit and do not themselves count as new
current findings. Missing capture scope and deletion are not distinguished.
Policy/context/exclusion drift fails the new gate even for empty inventories or
empty selected presentation. Deliberate source suppressions remain source policy.

Use separate baseline artifacts for different compiler/target/build contexts.

The closed `rstyle.density-baseline/1` JSON model stores policy/version/context
headers and at most 4,096 exact finding snapshots; input and output are bounded to
4 MiB. Duplicate original identities, malformed snapshots, unknown fields and
invalid headers reject. Publication never overwrites an old baseline; create a
new artifact to review an intentional update. Source/tool-free full and compact
replay support export and comparison. Durable identities cover original root
files, inline modules and free functions in an independently checked ordinary
external-module closure (`name.rs` or `name/mod.rs`). The external locator index
retains the exact captured cfg context and original file/range/header joins;
ambiguous, missing, generated or remapped module ownership stays unavailable.
External methods are outside this locator extension. Closure files/bytes,
declaration population and parser depth are bounded; exhaustion publishes no
partial locator population. This bounded density workflow does not establish general project
coverage or retire legacy style/patterns.
