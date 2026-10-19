# Conditional source removal within captured functions

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

This advances S03–S04 but does not establish whole-function inactivity, enclosing
module/impl guard proofs, arbitrary attribute expansion or a general removed-source
resolver. Uncaptured definitions in the frozen selection remain unknown. Runtime
`cfg!` macros are separate from source-removing attributes. Existing interface,
data-origin, helper and control-flow restrictions continue to apply.

Validation covers feature-on/off builds, nested predicates/attributes, removed
blocks/locals/macros/items/arms, shared-tuple field composition, earlier-profile
behavior and contradictory cfg/HIR rejection after regenerating inventory evidence.
The unchanged frozen inventory measures natural admissions separately.
