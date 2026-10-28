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

JSON `mutation_analysis` includes effective policy, scope/execution, findings,
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
exits 0, or 1 with `--fail-on-findings` and an over-budget rule. Full and compact
reports retain the same consumer evidence. Legacy suppressions, ignores and
review entries do not apply to this opt-in consumer.

Capture once, then reevaluate thresholds with `--replay-run RUN_DIRECTORY`, the
same extraction profile, `--mutation-rules` and a new `--mutation-config`.
Replay uses checked retained original source and compiler observations; it needs
neither the original source tree nor Cargo/compiler execution. Wrong source,
context or native correspondence is an error rather than an unavailable fact.
There is no new hash or global proof cache.

The independent corpus is `eval/clones-capability/effects-cases.json`. Its staged
next/later cases remain requirements, not delivered support. General transitive
effects, alias/field provenance, helper summaries and density remain migration
work. Exact Go comparison and macOS validation are unverified.
