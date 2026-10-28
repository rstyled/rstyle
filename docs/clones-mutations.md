# Direct scalar mutation budgets

`cargo rstyle clones --extraction-profile hir-data-v3 --mutation-rules`
checks original assignment sites independently of strict clone admission. It
reports writes to owned `u32` local/formal bindings and direct `&mut u32` formal
pointees, including builtin compound assignments. Declaration initialization is
excluded. One compound assignment counts once. Counts describe syntactic sites,
not execution frequency, panic freedom, purity or rewrite safety.

Ordinary unexpanded `if`/`else` branches contribute each original assignment site
once, including mutually exclusive arms and nested branches. An `if` without
`else` still has the sites authored in its arm; the report does not claim that
they execute. Original native branch/condition/arm correspondence is required.
Branch-local declarations retain distinct lexical binding roots, and leaving an
arm restores the enclosing scope. Loops, pattern conditions and assignments
inside conditions remain outside this bounded branch vocabulary.

This extension applies to the direct mutation-rule inventory. Conditional call
observations, helper-effect certification and parameter-use proofs retain their
separate incomplete boundaries. An unsupported or unknown branch operation
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
reports retain the same consumer evidence. Saved ignore and review entries do
not apply to this opt-in consumer.

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

The independent corpus is `eval/clones-capability/effects-cases.json`. Its staged
next/later cases remain requirements, not delivered support. General transitive
effects, alias/field provenance, helper summaries and density remain migration
work. Exact Go comparison and macOS validation are unverified.
