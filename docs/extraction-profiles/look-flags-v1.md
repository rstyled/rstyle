# Fieldless flags through verified helpers

Source-selected target: regex-automata 0.4.18 `util::look`, default features,
lib/non-test, edition 2021, pinned nightly; std/no_std and the existing three
64-bit target contexts are independent validation contexts. The thirteen target
functions are `Look::as_repr` and `LookSet::{contains, contains_anchor,
contains_anchor_haystack, contains_anchor_line, contains_anchor_lf,
contains_anchor_crlf, contains_word, contains_word_unicode, contains_word_ascii,
singleton, insert, remove}`. `LookSet::empty` is a prerequisite.

Before admission evaluation, source specifies all thirteen as in scope: each
variant represents its explicit single bit; membership tests that bit, grouped
queries test the documented masks, and insertion/removal return changed values
without modifying the input. A separate fixed development copy is
`eval/coverage-development/look-flags.rs`; the complete original crate and frozen
sixty retain their identities and denominators.

Known blocker chain: complete fieldless nominal observation → independent
source/discriminant and builtin Copy proof in the existing mixed unit → original
variant paths → enum-to-integer casts → checked helper transport → grouped
queries and returned value constructors. The existing scalar-payload enum model
cannot stand in for a fieldless enum; no fake payload, integer layout assumption,
new census or purity assertion is justified. Unknowns: further call/interface
composition gates become visible after complete observation. Audit them before
claiming the family complete.

Bounded contract: nonempty local nongeneric fieldless enums, builtin Copy proof,
default Rust representation, complete literal/shift discriminants with checked
arithmetic and implicit successors. No custom repr, aliases to unverified types,
external variants, cfg-erased variant subsets, mutable effects, payload binding,
custom derives, destructor behavior or general trait dispatch is added. Existing
scalar-payload enums retain their separate contract.

Independent expectations: all bit positions, all 18-bit sets and grouped masks;
zero/full/high unrelated bits; idempotent insertion/removal and original value
preservation; source/type/tag/discriminant disagreement must reject. Negative
cases include custom repr, generic/mixed/payload declarations, unresolved named
constant discriminants, overflow and incomplete inventories. `Look::reversed`,
`from_repr`, `as_char`, mutable setters and iterator construction remain outside
this declared family and must not gain support from a prefix of their bodies.
