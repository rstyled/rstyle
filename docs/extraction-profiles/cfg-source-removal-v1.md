# Conditional source removal and inactive definitions

Data-v3 extraction 3.7.0 adds `rust.cfg-source-removal`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Original function-body operations guarded by
`cfg` or nested `cfg_attr` may be independently proven absent using the actual
compiler cfg set already captured in build evidence. Supported predicates are
flags, string key/value settings (including features and target settings),
`all`, `any` and unary `not`. Unsupported predicate forms never justify removal;
they remain subject to ordinary source/HIR correspondence.

The existing source inventory records one `cfg_inactive` entry at the original
removed root span, then omits its descendants from active-operation expectations.
This applies to blocks/expressions, locals, macro statements, nested items and
match arms. The verifier recomputes the predicate from original attributes and
captured settings, checks the complete correspondence record, and rejects any
captured HIR inside a supposedly inactive span. A missing HIR node alone cannot
create an inactivity claim. No frontend field, source snapshot, provenance table
or cache is added.

Pre-expansion diagnostics wholly inside proven inactive roots are removed from
the active function assessment. This deliberately advances the earlier boundary:
even a deferred construct in such a root does not execute or require resolution
in that build. Without an independent false guard, unsupported original source
continues to be excluded and unmatched operations remain unknown. The independent
field-origin verifier uses the same cfg semantics when collecting active reads,
shadowing and mutations, so removed fields/locals do not invalidate active roots.
Earlier Cargo selections retain their previous treatment of removed source.

Graphs still come exclusively from resolved active HIR; no removed operation is
lowered or reinterpreted. Source capture now derives diagnostics from the same
correspondence inventory instead of parsing the function twice. Diagnostic profile
selection is evaluated once per inventory rather than once per entry.

Data-v3 extraction 3.8.0 adds `rust.cfg-inactive-definitions`. The existing
original-file inventory evaluates each function's attributes and traversed enclosing
file, inline module, impl, trait, expression, local and match-arm scopes against the
actual compiler session cfg set. Default methods and nested functions retain their
exact source ownership. Inactivity propagates through removed ancestors and is
restored for siblings. Unknown predicates alone never establish inactivity.

An independently false guard changes the existing inventory `presence` value to
`inactive_by_cfg`; it must have no exact active HIR correspondence. No guard stack,
per-definition cfg copy or provenance table is serialized. The frozen assessment
matches original path, byte span, name, source bytes/digests and observed compilation
context. Invalid or contradictory claims fail. It reports an explicit `inactive`
outcome and counts proven inactive contexts without shrinking the frozen selection.
Mixed contexts require every active context to be admitted; all-inactive selections
are inactive, never admissions. Missing proof still produces unknown.

This advances S03–S04 but does not read external source files that the compiler never
loaded, inspect definitions inside macro tokens, resolve arbitrary attribute
expansion or implement a general removed-source resolver. Runtime `cfg!` macros
remain separate from source-removing attributes. Existing interface, data-origin,
helper and control-flow restrictions continue to apply. Earlier extraction
selections preserve their previous absence treatment.

Validation covers feature-on/off builds, nested predicates/attributes, removed
blocks/locals/macros/items/arms, shared-tuple field composition, earlier-profile
behavior and contradictory cfg/HIR rejection after regenerating inventory evidence.
The unchanged frozen inventory measures natural admissions separately.
