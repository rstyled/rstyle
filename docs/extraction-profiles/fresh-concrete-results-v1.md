# Fresh concrete Results

Data-v3 3.78 adds whole fresh `Result<(), E>` construction and helper forwarding
for an exact source-owned, nongeneric `struct E(())`. `E` need not be Copy.
The [original selection](../../tests/fixtures/function-families/scalar-result-next-selection.json)
freezes regex-automata 0.4.18's `UnicodeWordBoundaryError::check`, its cfg-selected
`is_word_char::check`, and the disabled-feature error constructor across three
feature contexts and three pinned targets. Independently specified outcomes are
Ok with Unicode tables enabled and Err otherwise; no selected function panics.

The error's declaration identity and unit field are checked independently of
capture. A complete active source closure must exclude a possible Drop impl,
including aliases, imports, nested items and relevant macro expansions. Unknown
ownership or expansions remain unavailable. Source-only standard macro contracts
carry exact pinned source digests; they grant no function-effect permission.
A private checked witness permits bypassing the coarse unresolved-expansion
barrier only after this closure proof. Proof caching belongs to an immutable
unit environment and is keyed by the full cfg set and selected nominal identity.

Both Result type children and exact core constructor definitions are verified.
The graph records the variant and exactly one saved payload, evaluated once
before wrapping. Strict replay checks complete constructor inventories, source
operand correspondence, types, effects, policy and ordered operands. Unit payload
edges are permitted only in the exact success constructor slot.

This contract permits fresh constructors and complete helper results forwarded
whole. It grants no Copy, general move/drop, Result parameter/local storage,
payload extraction, match, Result-valued conditionals or `?` permission. Normal and
unwind cleanup of arbitrary owners remain separate work. Existing profiles do
not acquire this capability.

Real-Cargo tests include alpha peers and transitive callers, rejected storage and
Drop owners, and resealed mutations of constructor authority, type, tag, payload
order, effects and policy. Native unchanged-library execution supplies an oracle
independent of producer/verifier agreement. Cross-target capture and strict
replay are distinct from native execution.

Data-v3 3.79 extends this boundary through the original
[LookSet::available chain](../../tests/fixtures/function-families/result-propagation-selection.json)
with [same-error propagation](same-error-result-propagation-v1.md). Its independent
native oracle covers 4,719,096 cases across three feature contexts. The 3.78
contract above remains the historical construction/whole-forwarding boundary.
