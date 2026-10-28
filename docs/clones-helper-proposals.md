# Bounded helper proposals

Request advisory helper candidates on an explicitly selected project with:

```sh
cargo rstyle --manifest-path path/to/Cargo.toml --format json clones --extraction-profile hir-data-v3 --locked --helper-proposals --report-detail compact
```

`--helper-proposals` includes native pair explanations as prerequisites. It adds
`helper_proposals`, separate from local source rules and strict matching decisions.
The detector is `scalar-expression-helper` version 1. Its reported policy requires
at least two builtin operations in each member, exclusively `u32` signature and
expression types, and at most one literal hole. The policy is deliberately bounded
and fixed in this version; legacy pattern configuration does not change it.

For example, `(value + 1) * 2` and `(input + 3) * 2` suggest preserving the ordered
formal parameter and common multiplication while supplying `1` or `3` as a new
literal argument. The report retains each literal's native identity, span, type
and value, both member roots, ordered formal positions, operation counts and the
correspondence count. The full ordered correspondence remains in
`pair_explanations`; member keys join source locations in `accepted_pairs`.
Alpha-only two-operation bodies can also produce a proposal, with no invented
literal argument.

Identity forwarding and one-operation bodies are below the usefulness boundary.
Changed formal roles, operators, subtree shapes or more than one literal difference
produce explicit evaluated negatives under this policy. Unsupported native trees,
non-`u32` profiles and missing prerequisites remain unevaluated. Unknown calls and
writes cannot be removed to propose a helper over the remaining arithmetic.
Evidence, successful and negative native validation, and operation summaries are
borrowed and reused per endpoint within the report; no graph clone, re-admission
or routine hash is needed.

Each proposal contains exactly two members. A proposal for A/B and B/C does not
establish an A/B/C group: A/C needs its own comparison and policy result. This
consumer does not build transitive groups.

The reported pair denominator covers accepted strict pairs when matching is
complete. Nonadmitted captured owners remain in `unavailable_functions` with
original outcomes and reasons; potential members outside the accepted pair roster
are not counted as clean. They make helper coverage incomplete. Matching failure
or budget exhaustion leaves the full accepted-pair denominator unknown. Findings
can coexist with incomplete coverage. No requested consumer is clean merely
because its proposal list is empty.

Complete analysis returns zero by default, or one with `--fail-on-findings` when
there is a finding. Incomplete coverage or failed/budget-exhausted work returns
two, taking precedence over findings. Without this flag the helper report is
`not_requested`. Full and compact reports carry the same helper assessments;
text reports identify advisory pair proposals alongside source locations.

These are review suggestions. Integer overflow, call-site evaluation, naming,
visibility and API design still require human review. No automatic rewrite or
total equivalence is established. Persistent review, broader helper types and
other abstraction families remain migration work; legacy style/patterns,
thresholds, suppressions and review entries are unchanged.
