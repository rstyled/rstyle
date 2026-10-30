# Linear product Result composition (Data3.90)

Capability: `rust.linear-product-result-flow`. This joins the existing fresh
reference-product Result boundary to same-unit guarded helper summaries, exact
core `?` propagation and bounded affine local ownership. Acceptance target:
base64ct1.7.3 `LineReader::new_wrapped → new_unwrapped`, unchanged in six contexts
specified by `tests/fixtures/function-families/line-reader-wrapped/selection.json`.

The helper summary retains its complete verified saved Result value and return
graph, including scalar/Option fields and the Err alternative. Component origins
are relative to the Ok product; a missing component never means Err. Only an
independently checked Err path has empty origins. Error ownership remains the
existing actual Copy-error proof; the product retains its separate no-drop proof.

Propagation requires exact selected Try, FromResidual and identity From bodies,
complete generated branch/arm/return evidence, and identical concrete normalized
product/error types. Declared and generated lifetime spellings remain distinct;
region-erased type equality does not replace backing-origin/lifetime evidence.

One initialized mutable product local may receive ordered direct Copy scalar or
scalar-Option field updates, followed by exactly one whole move into Ok. The
complete original statement, HIR-use and binding inventories prove no aliases,
escapes, partial moves, intervening owner consumption, reference-field updates or
unsupported merges. The lowered update consumes the saved old aggregate and RHS
and defines a complete replacement in the exact original owner. Independent
verification checks the field slot, operands, owner, guards and graph inventory;
unchanged reference components retain their exact origins.

Static helpers on exact lifetime-only local impls additionally capture their
original selected-owner association under this capability. This supplies no
general type/const-generic, trait, destructor, heap or foreign-call permission.
Routine transport seals remain XXH3; no new hash operation or serialized cache key.

Six unchanged default/std selections on ARM Linux, x86 Linux and ARM macOS each
gain1/lose0 (0/1→1/1), with original rosters141/150 unchanged. Six strict owning
replays, six selected identity comparisons and108 resealed attacks pass. Native
ARM Linux original-source oracle:64,638 cases/context, zero mismatches. The Cargo
fixture checks equivalent wrappers, repeated Some/None updates and ownership escape;
strict import/matching pass. Final suite:1,497 tests/five existing skips, doctests,
fmt/Clippy; rstyle0/4,843, patterns0/15 unchanged ignores. Composition1.646s in the
suite; no controlled speedup claim. Frozen60 assessment/source/build parity and116
schema checks pass (23/30/7); this adds no frozen strict replay or native x86/macOS
execution claim. Full-library matching stays incomplete. Evidence:`/tmp/rstyle-390-*`.
