# Scalar literal OR patterns

Data-v3 extraction 3.1.0 adds `rust.scalar-or-patterns`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Earlier selections do not admit this slice.
This extends F06 only to flat alternatives of integer/bool literals. Named
constants, ranges, wildcard alternatives, payload bindings and nested patterns
remain excluded. The existing 1–16 arm/final unguarded wildcard boundary remains.

Compiler capture retains a `pat_or` node with ordered literal children and each
child's resolved type, source span and literal. Original-source scanning permits
this grammar only under the new capability. Independent source inventory visits
the OR node and its children; eligibility requires at least two literal children
with the scrutinee's scalar type. Existing source and graph-size limits bound
actual work; no additional arbitrary alternative-count cutoff is needed.

Lowering evaluates the scrutinee once and emits a `match.or` control selector
with that saved operand and an ordered JSON list of literal spellings. Its true
frontier enters the arm/guard; its false frontier proceeds to the next arm. A
failed guard also proceeds to the next arm. Literal tests have no effects, so
one selector represents the disjunction without duplicating the arm or guard.
Repeated alternatives remain represented. Branch joins and early exits use the
existing match lowering. This does not change exceptional-flow limitations.

Serialized CFG/PDG verification, strict import and projection retain the selector
construct, literal list, operand and control edges. Graph changes cannot silently
reuse a literal/wildcard selector. The real-Cargo fixture exercises negative and
boolean alternatives, ordered guards, final wildcard, exhaustive pair decisions,
legacy exclusion, unsupported wildcard/range alternatives, and rejection of a
changed serialized selector. The frozen natural inventory measures admission
gains separately; fixture admission alone is not a natural coverage claim.

Measured inventory: `time-core::weeks_in_year` becomes admitted; total admissions
rise from 8 to 9 out of the unchanged 60 selected contexts. See the
[refresh report](../../eval/hir-coverage/REPORT-data-v3.md). Unsupported-type OR
patterns do not gain child evidence merely because they share the syntax.

Extraction 3.2.0 extends this slice with integer ranges and mixed literal/range
alternatives; see the [range contract](scalar-range-patterns-v1.md). Literal-only
OR selectors keep their existing representation.
