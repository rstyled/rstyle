# Original size-prefixed block-header parser

This separate development selection freezes unchanged `lz4_flex 0.11.6`
`block::uncompressed_size` before admission evaluation. It does not modify the
frozen `lz4_flex 0.14.0` inventory or its denominator. Source hashes, exact spans,
complete error declaration, contexts and the full known blocker chain are in
[selection.json](selection.json).

For fewer than four bytes, the parser returns `ExpectedAnotherByte`. Otherwise it
returns the four-byte little-endian unsigned length and the exact borrowed suffix.
An empty suffix is valid; arbitrary trailing data is accepted. There is no
allocation or decompression in this function.

The delivered Data3.81 contract connects noDrop error ownership and Result transport:
`DecompressError` has one named two-field variant and four unit variants, derives
only Debug, and must never be certified Copy. Its complete declaration and
selected owning namespace must independently exclude destructor effects, including
relevant aliases, impls, macros and neighboring items. Existing borrowed enum
shape support does not authorize owned error transfer.

`ok_or` eagerly evaluates its error even on Some, then drops that unused value.
The subsequent `?` has input `Result<&[u8], E>` and output
`Result<(usize, &[u8]), E>`. Whole-input forwarding is type-invalid. Verify the
actual selected Try, FromResidual and reflexive From instances, saved operand,
error transfer/reconstruction, success guard and precise shared-success origins.
This requires a bounded non-Copy payload transfer proof; unchanged error type alone
is insufficient. Do not erase the later conversion/unwrap checks merely because
successful `get(..4)` should imply four bytes without independent length evidence.

The connected leaves are range get, borrowed array conversion of length4,
little-endian byte interpretation, suffix views and reference-bearing tuple Result
construction/return. Each belongs to this same complete parser acceptance target.
Exact source/module proof and compiler identities are verified in all nine selected
contexts. Downstream decompression callers remain outside this target because
they allocate, mutate and loop. General payload moves, stored Results and arbitrary
error conversions remain unsupported; see the [bounded contract](../../../docs/extraction-profiles/checked-borrowed-result-flow-v1.md).

The prepared oracle calls the unchanged public library. Its expected length uses
independent positional arithmetic, and it checks suffix bytes, length and pointer
identity. Cases include short inputs, all byte values in each header lane,
boundary lengths, varying suffixes and offset input slices. Native execution is
separate from extraction, cross-target replay, strict admission and matching.
Native ARM Linux execution now passes5,787 cases per context (default, no-default,
safe-decode-no-default), with zero mismatches. All nine selected cross-target
extraction instances were excluded at the Data3.80 baseline and admit at Data3.81:
nine contextual gains, zero losses. Source/build identities and full rosters are
unchanged. Per target, default admissions rise8→9/144, no-default5→6/91 and
safe-decode-no-default5→6/96. Strict replay verifies all63 contextual admissions
and nine selected mirror comparisons; distinct alpha peers are tested separately.

```sh
python3 eval/coverage-development/uncompressed-size/oracle.py --verify-only
python3 eval/coverage-development/uncompressed-size/oracle.py --output /tmp/rstyle-uncompressed-size-oracle.json
```
