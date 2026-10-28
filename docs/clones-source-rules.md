# Opt-in source and signature rules

`cargo rstyle clones --source-rules` evaluates function length and parameter count
from captured source and signatures. Select `--extraction-profile hir-data-v3`
to capture the original source ownership these consumers require:

```sh
cargo rstyle clones --manifest-path path/to/Cargo.toml \
  --extraction-profile hir-data-v3 --locked --source-rules \
  --analysis-config analysis.toml --format json --report-detail compact
```

`analysis.toml` accepts path exclusions and these sections; omitted settings use the legacy
rule defaults. It is separate from the JSON `--matching-config` policy.

```toml
exclude = ["target/**"]

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

Spacing is a separate opt-in within the source policy. Add this section to request it:

```toml
[fn_spacing]
min_statements = 5
```

Spacing counts direct original statements, including a tail expression, and uses
the legacy blank-line policy over the entire function source. Nested operations
and multiline initializers do not add direct statements. At least five statements
with no blank line produce a finding under the default threshold; the reported
limit is four. A blank line in the signature also satisfies the existing policy.
Omitting this section requests only length and parameters. `min_statements = 0`
disables spacing: its explicit clean assessment is not applicable and carries no
measured count. `effective_config.min_statements` is null when unrequested.

The requested count uses one bounded full-function parse and checks the original block and each
direct statement against captured kinds, byte ranges and snippets. Missing or
changed children remain unknown. Direct macros, attributed statements/items,
empty/verbatim statements and async lowering are initially unsupported. Length
and parameter checks can still evaluate those functions; spacing incompleteness
keeps the requested overall scope incomplete. Statement work runs only when
spacing is enabled and uses one indexed byte-position table, with no new hashes,
file reads, graph copies or second parse. Saved-run replay can enable or change
spacing policy without original source files or tools. The independent comparison
adapter scores spacing only for independently labeled cases, using the direct
count and authored blank-line outcome. Disabled and excluded policy agreement
remain separate from measured agreement; historical length/arity cases retain
their original denominator.

The JSON `analysis` object separates execution, scope coverage, fact availability
and each rule's outcome. Every assessed function has its report-local key, name,
source path, byte span and source checksum. `actual: null` with `not_evaluated`
means unavailable evidence or an explicit complete exclusion; known zero parameters is an evaluated result.
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

Configured path exclusions and existing source-rule suppression comments apply as
described below. Legacy review entries do not apply. Legacy style and patterns remain supported and continue to gate this
repository. This bounded consumer does not yet replace them. The independent
[acceptance corpus](../eval/clones-capability/README.md) records comparison scope
and remaining migration requirements.

## Workspace path exclusions

The top-level `exclude` list uses the same glob matcher as legacy style and
defaults to `["target/**"]`. Set `exclude = []` to evaluate every captured source
owner. Invalid patterns fail before capture. Paths are relative to the original
captured workspace root; files outside that root remain evaluated. Matching uses
captured paths and verified original source ownership, with no filesystem reads.

Excluded functions remain in the report with `source_exclusion.status` set to
`excluded`. Their requested source rules are `not_evaluated`, with complete coverage and
`not_applicable` availability. They are counted in `scope.excluded_source_functions`
and `scope.excluded_rules`, separately from unavailable evidence. Missing source
ownership or invalid original paths yield `unavailable` exclusion decisions and
incomplete scope, counted in `scope.unavailable_source_exclusions`.

Exclusion precedes numeric evaluation and suppression diagnostics. Other consumers,
including guard advice and strict matching, retain their own results and coverage.
Exclusions do not bypass the analysis function budget. Saved captures can be
reevaluated with different exclusion lists after original files have been removed.
The independent comparator checks labeled exclusions separately as
`policy-excluded-agreement`; an exclusion does not count as a measured clean result.

## Existing source-rule suppression comments

Source rules honor the existing `// rstyle-allow(fn_length, fn_params, fn_spacing): reason`
syntax from the exact captured file. A nonempty reason is required. Only named
requested length, parameter and spacing findings are hidden; other known legacy rule names grant
no permission to hide guard, mutation, pair or abstraction findings. Unknown rule
names or malformed annotations invalidate the whole comment, leave measurements
visible and add an `invalid_suppression` diagnostic.

Association follows the legacy physical-line policy: inspect at most 40 lines
above the original signature, skipping individual lines beginning `#[` or `///`.
The nearest remaining line is the candidate; a blank line stops association.
Multiline attributes and block documentation retain those legacy limitations.
Free functions, methods and same-named declarations use their own captured byte
ranges. Replay never reads an edited live comment or requires the original file.

Numeric assessments retain their original outcomes and values. JSON
`analysis.findings` counts all measured findings, including invalid annotations;
`visible_findings` and `suppressed_findings` distinguish presentation policy.
Every source-rule function has `source_suppression` with status `none`, `allowed`,
`invalid` or `unavailable`, retaining the captured candidate and declared rules
or reason. Text and `--fail-on-findings` use visible findings. Missing suppression
observations do not hide findings or permit a clean result; their count is
`scope.unavailable_source_suppressions`. Unknown, failed or budget-exhausted
coverage still exits two, even with no visible findings.

The evidence view shares the immutable source snapshot, scans backward over the
bounded preceding lines and reuses existing ownership/signature checks. It adds
no hashes, file reads or independent function parse. Legacy style remains active;
this compatibility slice does not retire a backend or migrate ignore ledgers.

## Changed-file presentation and CI findings

Add `--changed-files changed.json` alongside `--source-rules` to select source-rule
diagnostics for text output and `--fail-on-findings`. The manifest is explicit
consumer policy; it does not limit capture, matching, assessment or edit planning.

```json
{"schema":"rstyle.changed-files/1","paths":["src/lib.rs","crates/parser/src/read.rs"]}
```

Paths are relative to the captured workspace root, use `/`, and must name `.rs`
files. Absolute paths, empty/dot/parent components, backslashes, colons and control
characters reject before capture. The input is bounded to 1 MiB, 4,096 path entries
and 4,096 UTF-8 bytes per path. Entries are sorted and deduplicated after checking
the raw limits. Selection uses captured paths without live file reads, Git queries,
canonicalization, hashes, another parse or copied source evidence.

JSON retains every function assessment and the original `analysis.findings`,
`visible_findings`, suppression counts and whole scope. The optional
`analysis.changed_scope` adds selected visible findings and per-path matched and
unavailable function counts, coverage, and `observed`/`unobserved` status. Distinct
lib/test compilation owners remain distinct. Per-function `changed_file` is true
for selected paths, false for other checked paths, and omitted when path membership
is unknown. Without a selection, that field and `changed_scope` are omitted.

A requested path with no captured function owner is unobserved and incomplete,
including empty or inactive-only files in this initial contract. No matching
function is insufficient evidence of a clean file. Unclassified function paths
also leave a nonempty selection incomplete. An explicit empty list is a complete
empty selection. Selected owners with unavailable rule or policy evidence retain
incomplete coverage and any known findings.

For a complete analysis and selection, `--fail-on-findings` uses selected visible
source-rule findings. Whole-analysis failures, unknowns and exhausted budgets still
exit two, even with an empty selection or a clean selected file. Guard, mutation,
proposal and accepted-pair gates retain their own populations and policies.
Parameter and guard edit plans continue to include all applicable findings.

The same manifest option works with `--replay-run`: change the selection against
one saved capture after removing original source files and compiler tools.
Full and compact reports retain identical selected results. This provides an
explicit file-level CI scope; it does not infer changed lines or maintain a saved
finding baseline.

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
remain explicitly unsupported. Statement and guard parsers share a conservative
lexical bound: 256 KiB, delimiter depth 128, 128 punctuation bytes and 128 word
starts. This byte scan counts syntax inside comments and literals too. Exceeding
any limit reports budget exhaustion without truncation or parsing that body.
Ordinary source/signature inspection instead parses only a bounded borrowed
header through the captured original body opener. Large or unsupported bodies
retain independently checked length, parameter and suppression facts. An
oversized header preserves owned source but leaves syntax facts unknown. Native
range contradictions and source ownership errors still fail the query.
Missing ownership stays unknown. A finding can coexist with unsupported
functions, so the overall requested scope can remain incomplete and exit two.
`--fail-on-findings` returns one only for complete analysis; failures and incomplete
coverage take precedence. `--max-analysis-functions` applies to either local
consumer. Similarity decisions remain independent.
