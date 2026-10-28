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

A closure or unsupported graph operation can exclude a strict pair input while
its original source and signature still support these rules. The default `r0`
profile lacks the required ownership observations and reports unknown evidence.
The scope denominator covers captured function owners; inactive definitions stay
in `owned_source_inventory`. Failed capture leaves the expected denominator unknown.

Without `--source-rules`, analysis is `not_requested`. With it, complete analysis
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
