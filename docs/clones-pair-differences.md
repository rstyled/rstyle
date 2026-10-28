# Native pair explanations

Request ordered native differences in accepted strict pairs with:

```sh
cargo rstyle --format json clones --extraction-profile hir-data-v3 --locked --explain-pairs --report-detail compact
```

`pair_explanations` is separate from strict matching and local source rules. Its
population is accepted strict pairs. It reports execution, coverage, expected,
assessed and complete pair counts, reasons and per-pair results. An incomplete
matching run leaves the full accepted-pair denominator unknown. Unrequested
explanations perform no native comparison work. Requested incomplete explanation
coverage exits two, even if strict matching itself completed.

The closed contract supports scalar formal reads, scalar literals, builtin unary
and binary expressions, and tail-only blocks. Both interfaces and full native
expression rosters must be supported. Calls, branching, statements, opaque work,
duplicate identities and unsupported types cannot become a clean partial tree.
Native-width expressions require known equal target widths. Each requested
endpoint is bounded and validated once per report, including negative outcomes;
comparison retains borrowed evidence rather than cloning graphs or re-admitting
functions. Bounds are 64 parameters, 512 combined formal/node identities and
64 levels of tree depth; exhaustion is explicit.

Correspondence preserves operand positions and repeated occurrence identities.
Holes identify literal, formal-role, operation or subtree-shape differences.
A swapped subtraction preserves both ordered formal-role holes. Holes expose
compact HIR/span/type/value/operator references; full native trees remain in the
owning evidence bundles. Endpoint keys join the existing source-located
`accepted_pairs` records. Full and compact reports carry identical explanations;
text reports summarize coverage and hole counts.

Complete coverage means the bounded tree comparison completed. An empty hole
list means no differences under this contract. Neither result proves semantic
equivalence, usefulness of a helper, purity, or safety of a rewrite. Strict
matching decisions and scores are unchanged. Opt-in [helper proposals](clones-helper-proposals.md) consume this evidence
under a separate usefulness policy. Persistent review remains migration work.
