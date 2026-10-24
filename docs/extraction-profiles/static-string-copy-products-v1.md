# Supplied static strings in Copy products

Data-v3 3.101 (`rust.static-string-copy-products`) connects constructors and
ordinary helper forwarding for concrete nongeneric Copy structs containing
supplied `&'static str` fields and supported complete Copy components. It adds no
allocation, string-content, enum-payload, Result-consumption or destructor model.

The private receipt requires independently resolved original declarations,
complete ordered fields, source-spelled immutable `&'static str`, positive
builtin Copy evidence for every nominal dependency, and exhaustive active
source closure. Unknown neighboring imports, derives, macros and implementations
remain barriers. The exact receipt supplies cleanup evidence to namespace
checking; an erased `Shared(Str)` token cannot mint it. Historical profiles retain
their original selection path and documents cannot acquire this capability.

Direct string parameters must also spell `&'static str` in the parsed original
function. Tuple/Option/nested-reference string formals remain excluded pending
recursive lifetime correspondence. Checked nominal formals and receiver borrows
retain their separately proved original field lifetimes. This restriction does
not interpret a receiver's lifetime as its string field's lifetime.

Existing aggregate lowering evaluates each operand once in source order.
Existing component origins and helper summaries preserve the supplied backing;
static lifetime never means empty origins or independent aliases. The independent
document verifier reconstructs constructor slots and reference origins.
Development tests check ordinary callers, saved references across rebinding,
multiple selected nominal declarations, strict replay and equivalent matching.
An independent native oracle checks pointer identity for equal-content distinct
static storage, byte lengths, the flag, and absence of drop. Altered lifetimes,
missing/forged Copy, unproved neighboring expansions, forged origins and policy
downgrades remain rejected.

The unchanged natural `time 0.3.47` selection is still a four-function target:
`ComponentRange::{unconditional,conditional}`, its `From` conversion into
`TryFromParsed`, and the inverse `TryFrom`. This increment advances the two
constructors' value chain; it does not deliver that family. The foreign generated
`time_core::convert` export footprint and remaining source closure must prove
before the constructors admit. Nominal enum/Result payload origins and exact
conversion-member summaries remain separate required links. See the
[execution record](../rust-support-execution.md) for measurements and validation.

Routine consistency remains cached XXH3. This activation adds no SHA computation,
serialized lookup key or new cache substrate.
