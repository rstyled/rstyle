# Scalar Result / Option if-let frontier audit

2026-10-21, source audit only. **No independently ready bounded original found
in the four current natural packages.** No admission outcomes were consulted,
production files edited, builds or captures run. This is a named dependency
artifact, not delivered support.

The smallest explicit scalar/scalar Result original found is regex-syntax
0.8.11 `src/unicode.rs::SimpleCaseFolder::get`, line 198:

```rust
fn get(&self, c: char) -> Result<usize, usize> {
    self.table.binary_search_by_key(&c, |&(c1, _)| c1)
}
```

Frozen source identity (SHA-256): file
`b2084dcbd4331501b9a895fd7e7575d93ff96eb661c6e6adbc8c66bb72685cde`;
function bytes `[7134,7247)`
`a02173c29e763205346313473af6e714a1e84b058b09db84089a54e3a1ef1c4d`;
normalized registry manifest
`10c3f14629c0900ac452dd6d55a62aab561bbabbdf5774a802c8d223933cfa3b`.
Proposed original context: package regex-syntax@0.8.11, edition 2021,
default-feature library/non-test, nightly-2026-09-30,
aarch64-unknown-linux-gnu, Cargo debug. Exact captured unit identity is unknown.

Known chain: character scalar proof (P01) → stored-reference owner/table and
component origins (P06/P07) → borrowed char receiver and closure
`|&(c1, _)| c1` destructuring/capture → exact binary_search_by_key concrete
protocol/dispatch (P12/P13) → scalar Result transport (P08). This is not a
construction-only Result leaf. Unknown: exact selected core instance and its
transitive implementation requirements. The contract cannot be activated by
allowing its Result type alone. Independently fixed semantics: for sorted unique
table keys, Ok(i) exactly identifies an equal key; Err(i) lies in `[0,len]` and
all earlier keys are less, all later keys greater. Test empty/singleton tables,
first/last matches, insertion before/between/after, Unicode boundaries and valid
scalar gap. Do not invent ordering preconditions for arbitrary table values;
freeze original table invariants and the actual selected instance first.

The other explicit scalar/scalar Result original is
`hir/literal.rs::PreferenceTrie::insert`: mutable Vec-backed trie storage,
nonzero indices, binary search, insertion/allocation, iterator and mutation
protocols dominate; P14/P15/P16 precede its complete activation.

The short unit-result candidate regex-automata
`util/look.rs::is_word_char::check` returns
`Result<(), UnicodeWordBoundaryError>`. Its error is a nominal non-Copy
`struct UnicodeWordBoundaryError(())`, **not** unit `()`. Default selected
branches return Ok(()); disabled-word-boundary branches return a constructed
error through `new`. This is a plausible later concrete nominal-error family,
but requires explicit source/derive ownership, reference-free error validity and
Result nominal-component proof. `LookSet::available` additionally needs `?`.
Do not relabel that as scalar/unit Result support or omit the error branch.

Option if-let in these packages is generally on mutable state, container values,
nominal PatternID, callback results or cyclic iterator flow. The smallest scalar
owner example located, `nfa/thompson/builder.rs::check_size_limit`, consumes
`self.size_limit: Option<usize>` but its complete body depends on `Builder`
(non-Copy state), `memory_usage`, nominal BuildError construction and Result
transport. P06 owner/field proof and nominal error semantics are real prerequisites;
syntax support alone would not complete the original.

Recommendation: keep P08.B blocked on a **source-ready acceptance target**, and
promote P01/P06 or an independently frozen concrete nominal-error contract.
If expanding beyond the four packages, freeze a separate development natural
selection before admission measurement. Do not implement scalar Result vocabulary
or Option if-let solely to turn constructed fixtures green. No exclusive
production write assignment or shared interface change is requested by this audit.
