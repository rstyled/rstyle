# Byte membership through nested fixed arrays (source-selected target)

Select original regex-automata 0.4.18 default-feature library functions in
`src/util/alphabet.rs`, from the complete 2582-function capture. Selection is
from source semantics before attempting activation:

- `util::alphabet::ByteSet::empty`, bytes 28176..28257,
  SHA256 `95c66f772604235239ec20e74f5dd282c0f1066a69db322361fac0893858e4a0`.
- `util::alphabet::ByteSet::contains`, bytes 28903..29079,
  SHA256 `3a795ab4f5e2bb7ef972473f37ebcf6e37569964e4b95a147986a2a57307c19d`.

Both are excluded under Data-v3 3.45. The original owner is a builtin-derived
Copy struct containing a nested builtin-derived Copy `BitSet([u128; 2])`.
Retain its other derives, custom Debug impl, neighbors and actual module imports.
Do not replace the original declarations with scalar wrappers for activation.

Independent expectations: empty contains no byte. For every one of 256 possible
singleton sets, exactly the selected byte is contained, including 127/128 and
255. Arbitrary dense patterns and the all-bits-set value obey a flat Boolean
membership reference. A development caller `ByteSet::empty().contains(byte)`
should complete the constructor/read composition and always return false.
Input bytes are u8, so bucket division by 128 selects 0 or 1 and remainder is
0 through 127. These are semantic expectations, not existing range certificates.

## Known blocker chain and unknowns

1. The selected mixed-unit Copy proof admits scalar/nested struct fields only;
   the fixed scalar array is rejected before its ownership proof completes.
   Recursively checked scalar arrays must preserve exact length/element and
   positive builtin Copy authority. Reference, slice, mutable, non-Copy or
   unknown nominal fields must not inherit permission.
2. `contains` calls `usize::from(bucket)`. Current scalar From support is only
   concrete u16-to-u32, so its otherwise supported source syntax does not supply
   the required selected u8-to-usize core implementation summary. Any extension
   needs exact implementation identity and pointer-width authority; a custom
   From or narrowing conversion remains excluded.
3. Nested struct-field/array-index reads must compose through the complete
   function, including exact location, base/index evaluation order, finite
   array length and bounds. Existing separate index tests are insufficient.
4. Division, remainder and shift are total on the declared inputs, but current
   partial-operation analysis may not prove their ranges. A by-reference helper
   caller may therefore still reject even after the two leaf functions admit.
   Investigate this before claiming the family complete; never remove panic
   effects or substitute a supported prefix without justified range evidence.
5. The by-value constructor temporary autoref must compose with the getter's
   dynamic array location and helper summary. Existing Copy snapshot support
   permits struct-field-only scalar reads, so dynamic array reads can expose an
   additional summary boundary. This remains an explicit unknown pending
   full production and independent document verification.

The 3.45 temporary Cargo probe reproduces both reported leaf blockers and rejects
`empty_contains` through unsupported helpers. It is diagnostic evidence, not a
new frozen inventory or a gain. The source-selected two functions and their
original build contexts stay fixed regardless of later results.

## Bounded support and validation plan

Support fixed scalar arrays inside independently checked Copy nominal owners,
read-only byte membership, concrete lossless u8-to-usize conversion, and the
constructor/getter caller composition only with complete bounds/effects proof.
Defer mutable set updates, iterators/closures, array equality, serialization,
Result/Try, arbitrary generics and broad macro support. Their neighboring source
must remain present and inspected under the existing source contract.

Require independent positive and negative tests: singleton/dense/all-byte runtime
oracle; exact array length/type and Copy proof; custom From/changed implementation;
wrong index/base snapshots; adversarial byte boundaries; potentially out-of-bounds
or oversized shifts that must preserve partiality; corruption of helper/location
summaries; complete real Cargo capture, document/producer verification, strict
admission and matching. Keep unsupported, unknown, inactive and incomplete
separate. Report both selected and incidental new/lost admissions on unchanged
identities and replay the frozen 60. Compare unchanged runtime/memory controls
without adding parallel provenance/cache records. Rstyle and patterns must stay
at zero; do not change thresholds to force similarity acceptance.

`eval/coverage-development/byte-membership-oracle.py` checks the two identities,
copies the complete original crate, appends only a test module and runs locked
original default-feature tests. It specifies semantic results, not extraction
support, rewrite safety or detector quality. No admission gain is claimed here.

Validation on the current ARM Linux host: all three original-crate oracle tests
pass (empty queries, 65536 singleton/query pairs, dense patterns and full set).
No original function, declaration or Cargo feature was replaced. Native runtime
execution on other hosts and pipeline admission are not established by this run.
Rust production code is unchanged from 66f7af4, whose 861-test, zero-style and
zero-pattern validations remain the applicable implementation evidence.
