# Direct scalar mutation budgets

`cargo rstyle clones --extraction-profile hir-data-v3 --mutation-rules`
checks original assignment sites independently of strict clone admission. It
reports writes to owned `u32` local/formal bindings and direct `&mut u32` formal
pointees, including builtin compound assignments. Declaration initialization is
excluded. One compound assignment counts once. Counts describe syntactic sites,
not execution frequency, panic freedom, purity or rewrite safety.

Add `--compiler-advisory --compiler-advisory-only` to run the same consumer without
strict clone assessment. Fresh capture reuses the original file census and the
existing mutable-u32 formal and builtin control-type collectors; it performs no
body type enrichment or helper certification. Saved facts support the same policy,
review, suppression and exclusion reevaluation without source files or compiler
tools. Older raw captures cannot supply missing type observations on replay and
remain incomplete. Compiler advisory mutation events are a separate metric.

Ordinary unexpanded `if`/`else` branches contribute each original assignment site
once, including mutually exclusive arms and nested branches. An `if` without
`else` still has the sites authored in its arm; the report does not claim that
they execute. Original native branch/condition/arm correspondence is required.
Branch-local declarations retain distinct lexical binding roots, and leaving an
arm restores the enclosing scope. Pattern conditions and assignments inside
conditions remain outside this bounded branch vocabulary.

Unlabelled `loop`, primitive boolean `while`, and primitive half-open `u32`
`for` ranges contribute each original body site once. Both range endpoints must
be present; wildcard and immutable scalar binders are supported. Captured compiler
range identity, original endpoint/binder/body correspondence and lexical binding
scopes are checked. A zero-length range or false while condition still has its
authored sites; no dynamic execution count or termination claim follows.
Valueless unlabelled `break` and `continue` require the captured innermost loop
destination. Compiler builtin never-type observations are retained separately
for this consumer and joined to original HIR identities and spans; display text
does not supply type authority or graph admission. Inventory continues through source statements after a transfer,
including unreachable statements. Labels, value breaks, while-let, inclusive
ranges, arbitrary iterators and mutable range binders remain outside this scope.

This extension applies to the direct mutation-rule inventory. Ordinary call-roster
and parameter-use proofs retain separate incomplete control-flow boundaries.
The explicit `--mutation-effects` consumer also observes checked static control
sites under its own summary contract (see `clones-mutation-effects.md`).
An unsupported or unknown branch/loop operation
cannot turn into a complete clean inventory, even if other sites are witnessed.

Use an explicit TOML policy with `--mutation-config FILE`:

```toml
exclude = ["target/**", "generated/**"]

[mutation_budget]
max_mutations_per_target = 2
max_mutated_targets = 1
granularity = "root"
```

Limits are strict: a count equal to the limit passes. Defaults reuse the legacy
mutation configuration (three sites per root and three mutated roots). Field
granularity is rejected until field provenance has its own evidence contract.
This consumer has a distinct event vocabulary from legacy mutation analysis,
which also counts mutable borrows. It does not replace legacy behavior.

JSON `mutation_analysis` schema `rstyle.mutations/2` includes effective policy, scope/execution, findings,
expected/assessed/complete/unavailable function counts, and per-function rules.
Each retained site includes its original function-relative byte span, compiler
binding identity, declaration and lexical-scope spans, formal ordinal when
applicable, root provenance and caller-visible pointee flag. The surrounding
function carries its original file span and compilation-unit-qualified key.
Owned formal rebinding is local state; a direct mutable formal pointee write is
caller-visible. These observations are not serialized claims of purity.

Calls, closures, macros, attributes, aliases, reference reassignment and unsupported
operations retain explicit boundaries. Each boundary distinguishes unknown
observations or call behavior from unsupported syntax; both can coexist. A complete inventory gives exact counts;
an incomplete inventory reports only witnessed lower bounds. A witnessed count
above the limit can produce a finding even when the remainder is unknown. A
lower bound below the limit cannot establish a clean rule result. Density stays
not evaluated: this slice has no complete effect vocabulary or statement
denominator. Opaque closure bodies are not flattened into immediate outer writes.

`--max-analysis-functions` leaves unassessed owners in the report. Internal
limits include 256 KiB original source, 128 delimiter depth and conservative
128 punctuation/word counts before parsing (including comments/strings), 16,384
native nodes/depth 128, and 4,096 roots/sites. Exceeding a bound remains explicit
budget exhaustion; it never establishes an empty inventory. Incomplete
requested analysis exits 2, including when it has findings. Complete analysis
exits 0, or 1 with `--fail-on-findings` and a visible finding. Full and compact
reports retain the same consumer evidence. Legacy ignore entries do not apply
to this opt-in consumer. Explicit saved mutation reviews filter presentation as
described below.

Captured comments use the existing physical-line association and named-rule
syntax, for example `// rstyle-allow(mutation_per_target): deliberate repeated writes`.
Only the explicitly named mutation rules are hidden. A malformed annotation
produces an `invalid_suppression` finding and hides nothing. Original comments,
including empty comments, come from the checked captured file; live file edits
cannot change them during replay. A suppression never repairs incomplete raw
analysis: an included opaque owner still exits 2 even when its witnessed findings
are suppressed.

The top-level `exclude` list in the mutation configuration uses the shared
workspace-relative glob policy. Its default is `["target/**"]`; `exclude = []`
opts out. Matching uses the original captured path and workspace root, without
reading the current filesystem. A trusted excluded owner has an explicit complete
policy skip; its raw evidence, boundaries and incomplete raw status remain
inspectable. Skipped rule decisions have no measured `actual` value. Missing
source/path authority cannot authorize a skip. Known invalid comments remain
visible even on excluded owners. This policy does not alter effects, source
rules, guards or planning consumers.

Per-function `policy` separates suppression/exclusion decisions from raw `rules`,
`evidence`, `coverage` and `execution`. Report `findings` counts observed mutation
breaches and known invalid annotations; `visible_findings` controls finding exits
and text. `suppressed_findings` and `excluded_findings` explain hidden breaches.
Top-level `coverage`/`execution` describe the requested policy scope, while
`evidence_coverage`/`evidence_execution` describe the unchanged raw scope.
`evaluated_functions`/`unavailable_functions` retain raw-evidence meanings;
`policy_evaluated_functions`, `excluded_functions` and
`policy_unavailable_functions` partition requested policy owners. Capture errors
and unassessed owners remain incomplete regardless of policy.
With all-target capture, generated test-harness owners can remain unavailable;
an authored source-file exclusion does not authorize skipping those owners.

Capture once, then reevaluate thresholds and exclusions with `--replay-run RUN_DIRECTORY`, the
same extraction profile, `--mutation-rules` and a new `--mutation-config`.
Replay uses checked retained original source and compiler observations; it needs
neither the original source tree nor Cargo/compiler execution. Wrong source,
context or native correspondence is an error rather than an unavailable fact.
There is no new hash or global proof cache.

## Saved numeric reviews

Dismiss each numeric rule separately using an explicit saved run, review file and
reason:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --mutation-rules \
  --replay-run RUN_DIRECTORY --review-file reviews.json \
  --dismiss-mutation-per-target FUNCTION_KEY --review-reason 'Reviewed repeated writes'
```

Use `--dismiss-mutated-targets FUNCTION_KEY` for the other budget. The key is the
saved report's function handle. Only selected, uniquely identified original
owners with completed, evaluated, visible findings can be dismissed. Suppressed,
excluded, unknown, unsupported and budget-limited observations cannot authorize
a dismissal, including witnessed lower-bound breaches.

Review-store schema 6 adds `mutation_entries` while reading schemas 1 through 5
without upgrading or rewriting them during lookup/status. Numeric identity keeps
the original declaration locator, build context, detector versions, selected rule
and threshold, and the sorted complete distribution of sites per mutated root.
Changing that distribution or threshold makes a choice stale. Binder renames,
comments and right-hand-side edits that preserve these counts retain a numeric
choice; they acquire no semantic or effect approval. No source or graph is copied
into the review identity, and no hash is needed.

JSON `mutation_review` uses schema `rstyle.mutation-rule-review/1`. Exact reviews
hide only their selected numeric findings from text and finding exits;
`mutation_analysis.review_hidden_findings` counts them. Raw evidence, rules,
policy decisions, coverage and execution remain unchanged. Unrelated numeric
rules and invalid annotations remain visible; a review cannot repair incomplete
analysis.

`--review-status` distinguishes omitted analysis (`not_requested`), missing
declarations (`not_observed`), stale measurements/policies and unavailable
evidence. `--review-prune --review-prune-dry-run` previews eligible removal;
`--review-prune` applies it. Only a completed selected negative result under the
same rule, threshold, versions and build context authorizes removal. Unknown,
suppressed, excluded and unselected owners do not. Changed-file review selection
uses the existing source-rule scope rather than a separate mutation selector.

The independent corpus is `tests/fixtures/advisory/effects-cases.json`. Its staged
next/later cases remain requirements, not delivered support. General transitive
effects, alias/field provenance, helper summaries and density remain migration
work. Exact Go comparison and macOS validation are unverified.
