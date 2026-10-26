# Original fallible wire readers

Source-selected before admission assessment: complete regex-automata0.4.18,
`check_slice_len::<u8>` → `try_read_u32`, with the original `read_u32` and
`DeserializeError::buffer_too_small` dependencies. `selection.json` fixes original
spans/XXH3 checksums, complete context files, default/no-default library builds,
and the complete ten-variant error/dependency closure. Existing frozen inventories
and denominators remain unchanged. Admission is not delivered yet.

The first contract covers every byte-slice length, usize minimum and explicit
static message. The length helper returns BufferTooSmall iff `len < minimum`,
carrying the same string pointer/length and reading no element contents.
`try_read_u32` returns that error before decoding any short input; otherwise it
returns the native-endian first four bytes and consumed count4, independent of
trailing bytes. Any panic fails this fallible-reader runtime expectation.

Required connected proofs: concrete generic instance identity `T=u8`; fresh
unit/scalar-tuple success Results; callable guarded non-Copy error components;
same-error `?` from unit helper to tuple result; and saved slice/read/panic
composition with the existing native reader. A successful length check does not
implicitly grant a downstream panic-free summary. Prove the relationship or
retain the real conditional panic evidence. No name-based generic substitution,
blanket purity, missing-origin-as-empty or new hashing/cache key is justified.
The original nested generic helper also retains the ordinary source scanner's
deferred `rust.call-summaries` rejection at `slice.len()`. Raw type observations
must preserve that flag and diagnostic. Activation must independently prove the
whole source/HIR body and discharge that exact call restriction; changing the
original scan flag is not proof.

Tier2 (`try_read_u16/u128` and their existing readers) and tier3
(`read_version/read_endianness_check`) are frozen deferred selections. They are
runtime controls only until their whole compositions admit. Tier3 additionally
requires tuple-success propagation, exact scalar-field mismatch constructors,
literal static error origins and the original assertion effects. Short-input
BufferTooSmall errors must precede version/endianness mismatches.

Independent oracle: append code to the entire unchanged original crate and call
its private functions/error variants; check every original byte in the copied
crate before and after execution. Mathematical positional decoding supplies the
expected native values, rather than extraction output or native decode helpers.
The corpus includes empty/short/exact/trailing slices, four start offsets, every
first-byte value, three static messages, extreme length bounds, and explicit
version/endian successes and mismatches. Caller-supplied messages retain exact
pointer/length; validator-internal literals are checked by value, without assuming
separate source literals share addresses. **285,339 cases per context** are
specified before execution:3 constructor,17,820 length,230,400 reader and37,116
validator cases. Deferred tiers participating in the oracle confer no admission.

Source-only validation and coordinator-scheduled native execution:

```sh
python3 eval/coverage-development/wire-fallible-readers/oracle.py
python3 eval/coverage-development/wire-fallible-readers/oracle.py --run-native --output /tmp/wire-fallible-readers-oracle.json
```

The native command runs default/no-default × debug assertions on/off at opt-level0
with the pinned compiler,64-bit supported native host and unwind panics. Foreign
hosts require native execution; cross-target capture is a separate check.

Native aarch64 Linux execution passed all 285,339 cases in each of the four
contexts with zero mismatches. Retained output:
`/tmp/wire-fallible-readers-oracle.json`. This validates the source expectations;
it does not establish extraction, admission or matching support.

Integration adversaries must reject wrong width/endian, concrete substitution,
callee/build identity, rebound slice/message, moved precheck reads, changed
residual conversion, conflated error variants, omitted/flattened origins,
nonempty success reference origins, incomplete error variants and hidden Drop.
No general generic, unsafe, arbitrary callback, owned storage or unwind-cleanup
support is in the first contract. Matching quality and rewrite safety remain
separate from extraction coverage.

Current capture checkpoint: exact local u8 call receipts retain the complete
original-owner type overlay, including concrete len receiver/signature and ordered
builtin-dereference/shared-borrow adjustments. The original generic HIR and source
rejection are unchanged. Unit and `(u32,usize)` success/error observations grant
no existing ownership or helper permission; adjacent tuple shapes stay unsupported.
Complete original default/no-default captures preserve all selected function/build
identities and every admitted record (apart from transient evidence navigation):
198/2,582 and138/590, zero gains/losses. Excluded records gain error declaration
observations; previous declarations remain unchanged. No new runtime hashing or
serialized lookup keys. Evidence: `/tmp/rstyle-451-{default,no-default}-comparison.json`.
