# Original fallible wire readers

Source-selected before admission assessment: complete regex-automata0.4.18,
`check_slice_len::<u8>` → `try_read_u32`, with the original `read_u32` and
`DeserializeError::buffer_too_small` dependencies. `selection.json` fixes original
spans/XXH3 checksums, complete context files, default/no-default library builds,
and the complete ten-variant error/dependency closure. Existing frozen inventories
and denominators remain unchanged. Data3.122 connects the tier1 reader through
its checked concrete length-helper instance and both tier3 validators; tier2
remains deferred.

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

Tier2 (`try_read_u16/u128` and their existing readers) remains a frozen deferred
selection and runtime control. Tier3 (`read_version/read_endianness_check`)
additionally requires tuple-success propagation, exact scalar-field mismatch constructors,
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
python3 tests/fixtures/function-families/wire-fallible-readers/oracle.py
python3 tests/fixtures/function-families/wire-fallible-readers/oracle.py --run-native --output /tmp/wire-fallible-readers-oracle.json
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

Historical Data3.120 capture checkpoint: exact local u8 call receipts retain the complete
original-owner type overlay, including concrete len receiver/signature and ordered
builtin-dereference/shared-borrow adjustments. The original generic HIR and source
rejection are unchanged. Unit and `(u32,usize)` success/error observations grant
no existing ownership or helper permission; adjacent tuple shapes stay unsupported.
Complete original default/no-default captures preserve all selected function/build
identities and every admitted record (apart from transient evidence navigation):
198/2,582 and138/590, zero gains/losses. Excluded records gain error declaration
observations; previous declarations remain unchanged. No new runtime hashing or
serialized lookup keys. Evidence: `/tmp/rstyle-451-{default,no-default}-comparison.json`.


Data3.121 connected boundary: the entire original `try_read_u32` now admits in
both selected Cargo contexts, using a separately verified concrete length helper.
The original generic definition remains excluded and every original function
entry is retained. Complete receipt inventories include opaque unsupported loops;
only a retained exact HIR call can authorize linking. Missing or altered receipts,
source diagnostics, substitutions or owner evidence cannot certify a prefix.
Native byte reads and their real conditional panic evidence remain explicit.
The reader itself has no callable summary yet.

Fresh unchanged selections: default198→199/2,582 and no-default138→139/590, sole
gain `util::wire::try_read_u32`, zero losses; every earlier admitted record is
unchanged apart from evidence navigation. Independent source oracle passes285,339
cases in each of four native aarch64 Linux contexts. Foreign native CI remains
unrun. New adversaries cover receipt completeness, unsupported-loop boundaries,
exact guards/origins/scopes and whole reader graph semantics.

Activation recaptures the reverse local caller closure in O(V+E), retaining
unchanged graph/helper tables and rebuilding proof caches. Cases with affected
existing roots/helpers or private projections fall back to complete recapture.
Cargo composition verifies all original facts and prior PDG/CFG bytes survive.
No new runtime hashing or serialized lookup keys. Alternating equal-work warmed
no-default repeats preserve all590 outcomes/139 admissions:36.75–38.40s before,
29.84–30.26s after; peak405.6–405.7→382.7–383.0MiB. These local observations do
not predict other workloads. Evidence: `/tmp/rstyle-452-perf-summary.json`.

Next complete family: callable reader → original `read_version` and
`read_endianness_check`. Shared prerequisites are tagged-output/read/panic summary
composition; tuple→scalar same-error propagation; scalar tuple declaration with
saved component positions; source-backed escaping static literal roots; ordered
two-scalar mismatch constructors; and original assertion effects. Preserve the
short-input error before either mismatch, expected/found order and consumed4.
Broader widths, unsafe/stateful deserializers and allocation/drop remain deferred.

Frozen reproduction `target/hir-coverage/run-mhlxtnst` preserves all60 identities,
outcomes/reasons/blockers/denominators and118 Data documents:24 admitted/29 excluded/
7 inactive. R0 and expanded profiles are also unchanged. Comparison:
`/tmp/rstyle-452-frozen-comparison.json`. Strict original/serialized replay covers
all338 admitted originals across the two natural contexts:676 imports,338 accepted
self-comparisons and29,292 complete cross-pairs, with all28,956 old pairs unchanged.

Data3.122 connects the original reader → `read_version/read_endianness_check`
family. Checked no-drop tagged results can carry read/panic summaries; exact
reader-tuple→usize same-error `?`, immutable saved scalar tuple components,
source-backed static error roots and ordered two-u32 mismatch constructors
compose through the original active assertion. Assertion lexical closure requires
its exact retained standard expansion and recursively checks every argument;
hidden items/macros and historical policies retain rejection. No general move,
destructor, generic residual conversion or validator callable summary is implied.

Both unchanged selections gain exactly those two validators and two constructors:
default199→203/2,582; no-default139→143/590, zero losses. The original generic
length helper and wider readers remain excluded. The next audited safe consumer
is default-only `dfa::dense::Flags::from_bytes`: aggregate-success ownership,
tuple→aggregate propagation and complete struct/tuple construction remain open.
Direct validator callers are unsafe DFA deserializers with additional table,
pointer, cursor and ownership blockers; they are outside this contract.
