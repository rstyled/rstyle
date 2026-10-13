# Grouped shared-field borrow receivers, version 1

Implementation baseline: `062939b`, with the pending length implementation.
Local Data supplement only. The prospective length contract permits explicit
shared-field-borrow receivers, whose ordinary Rust method syntax requires grouping:
`(&value.text).len()`. Existing borrow proof incorrectly parses that HIR span as
an ungrouped Reference expression even though it retains the parentheses.

Permit balanced original parentheses around the already supported shared-field
borrow. Recover the Reference expression through the original parsed Paren nodes;
keep the exact original bytes, spans and HIR correspondence. Do not alter graphs,
labels or source before capture. The underlying borrow still requires an immutable
reference to exactly one verified formal field place, the existing non-reference-
bearing/drop-free boundary, exact referent/result types and verified formal origins.
No new field-base expression, dereference, temporary/local borrow, mutable/raw
reference, coercion or macro is authorized. No new matching interpretation.

Use the existing source/type/node budgets; no truncated grouping walk or partial
certification. Independent source inventory, field/place/borrow document checks
and the length receiver/summary/operand checks must all retain their obligations.
Test grouped str-field borrows in a real composed Cargo family and reject changed
mutability, place/type/origin facts and unsupported receiver adjustments. Historical
profiles and matching behavior remain unchanged; this is syntax ownership repair
for the explicitly registered Data composition, not source normalization.
