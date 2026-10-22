# Checked borrowed Result parser flow

Data-v3 3.81 connects the source-selected `lz4_flex 0.11.6`
`block::uncompressed_size` chain. The
[development selection](../../eval/coverage-development/uncompressed-size/selection.json)
fixes its original source, complete error declaration and nine build contexts.
The frozen sixty-function inventory remains separate.

The bounded contract composes these operations in one function:

- Original `slice.get(..end)` saves receiver then endpoint exactly once. Some
  contains the saved prefix iff `end <= saved_receiver_length`; this checks
  metadata and does not read an element or introduce a panic.
- Eager `ok_or` evaluates its fresh, source-qualified unit error even on Some.
  The complete scalar enum and owning source closure independently prove no drop.
  The error is not assumed Copy. Named scalar variants are represented so a unit
  variant never stands in for an incomplete declaration.
- Exact core Result `?` inspects the saved tag. Its Err branch moves the same
  error once into the output Result and returns; its Ok branch retains the saved
  shared slice. Input `Result<&[u8], E>` cannot be forwarded whole as output
  `Result<(usize, &[u8]), E>`. Selected Try, FromResidual and reflexive From
  implementations, substitutions and compiler sources must agree.
- Borrowed four-byte conversion retains the length check and unwrap panic path.
  Little-endian u32 conversion uses the exact four observed bytes. A suffix view
  checks `start <= length` and retains its saved base and endpoint origins.
- The returned tuple preserves the suffix reference as component one. Failure
  has no successful reference roots only under its explicit verified contract.

Result/error parameters, saved owned Result/error locals, arbitrary error
conversion, arbitrary payload moves, destructors, mutation and general residual
traits remain outside this contract. Historical policies gain no new permission.
Builtin test markers, Debug imports/derive and prelude From require exact bounded
source-ownership proof; neighboring items and macro-generated shadows are checked.

Independent expectations come from the original library oracle: short input is
ExpectedAnotherByte, otherwise length is positional little-endian arithmetic and
rest is the exact borrowed suffix, including an empty suffix. Native ARM Linux
execution checks 5,787 cases in each of three feature contexts. Cross-target
capture is separate from native execution.

Real Cargo composition tests include distinct alpha peers, receiver and endpoint
rebinding, an ordinary enum cast inside the endpoint, strict document/producer
replay and matching. Adversaries change protocol witnesses, payload guards, bounds,
saved operands, ownership consumers and resealed bundles. Drop-bearing or
reference-bearing errors, stored Results and unsupported inclusive views must
remain unavailable. Original-crate admission deltas and integrated gate results
are recorded in the root roadmap.
