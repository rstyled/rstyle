# Opt-in source and signature rules

`cargo rstyle clones --source-rules` evaluates function length and parameter count
from captured source and signatures. Select `--extraction-profile hir-data-v3`
to capture the original source ownership these consumers require:

```sh
cargo rstyle clones --manifest-path path/to/Cargo.toml \
  --extraction-profile hir-data-v3 --locked --source-rules \
  --analysis-config analysis.toml --format json --report-detail compact
```

`analysis.toml` accepts only these two sections; omitted settings use the legacy
rule defaults. It is separate from the JSON `--matching-config` policy.

```toml
[fn_length]
max_lines = 50
[fn_params]
max_params = 4
count_self = false
```

Length uses the existing code-line policy over the entire captured function,
including the signature and closing brace. Blank and comment-only lines do not
count; the legacy string/comment parsing limitations still apply. Destructuring
counts as one parameter. `count_self` controls whether a receiver counts.
Findings require a value strictly greater than the configured limit.

The JSON `analysis` object separates execution, scope coverage, fact availability
and each rule's outcome. Every assessed function has its report-local key, name,
source path, byte span and source checksum. `actual: null` with `not_evaluated`
means unavailable evidence; known zero parameters is an evaluated result.
Full and compact reports retain the same source-rule assessments. Text output
lists findings at their source byte spans.

An exact saved capture can be [reevaluated](clones-saved-runs.md) with changed
source-rule policy or newly requested consumers without running the compiler.

A closure or unsupported graph operation can exclude a strict pair input while
its original source and signature still support these rules. The default `r0`
profile lacks the required ownership observations and reports unknown evidence.
The scope denominator covers captured function owners; inactive definitions stay
in `owned_source_inventory`. Failed capture leaves the expected denominator unknown.

Without a requested local consumer, analysis is `not_requested`. With one, complete analysis
returns zero even when it finds violations; add `--fail-on-findings` for exit one.
Failed, unknown or budget-exhausted scope returns two. `--max-analysis-functions N`
limits local work and retains explicit unevaluated rows beyond the limit.
Strict matching completion and `--fail-on-accepted-pairs` retain their own meaning;
neither source-rule success nor accepted similarity establishes refactoring safety.

Legacy exclusions, suppressions and review entries do not apply to this opt-in
consumer. Legacy style and patterns remain supported and continue to gate this
repository. This bounded consumer does not yet replace them. The independent
[acceptance corpus](../eval/clones-capability/README.md) records comparison scope
and remaining migration requirements.

## Bounded guard-clause advice

`--guard-clauses` independently requests a syntactic advisory consumer. It can
run alone or alongside `--source-rules`, and uses the same original source
ownership query. Each function's optional `guard_clause` assessment records its
execution, outcome, availability, coverage and reasons. It contributes to local
analysis coverage and findings; the source length/parameter `assessments` array
stays empty when those rules were not requested. Numeric policy is then null.

The bounded opportunity is a final top-level `if/else` with one scalar fallback
(boolean, integer, character or unit, optionally directly returned) and a
continuing branch with at least two statements ending in a value. The report
identifies which branch is the fallback and advises reviewing condition inversion,
lifetimes and drops before lifting the continuing branch. It performs no rewrite
and establishes no effect, equivalence or refactoring-safety proof.

Already flat code, two working branches and single-expression work branches
produce no opportunity under this contract. Macros, closures, unsafe/async
syntax, attributes, loops, labels, `match`, `?`, let conditions and else-if chains
remain explicitly unsupported. The detector bounds source to 256 KiB before its
parser; exceeding that bound reports budget exhaustion without truncation.
Missing ownership stays unknown. A finding can coexist with unsupported
functions, so the overall requested scope can remain incomplete and exit two.
`--fail-on-findings` returns one only for complete analysis; failures and incomplete
coverage take precedence. `--max-analysis-functions` applies to either local
consumer. Similarity decisions remain independent.
