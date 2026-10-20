# Nested-array byte membership

Data-v3 3.46.0 (/61) connects original regex-automata 0.4.18 default-feature
library `ByteSet::empty` and `ByteSet::contains`, including constructor/getter
composition. Matching remains `rstyle.ccgraph.data-calls/1.0.0`; scoring and
thresholds are unchanged. This contract does not activate all ByteSet methods.

## Source-selected target and independent expectations

Selection preceded activation, from the complete original 2582-function capture
and source in `src/util/alphabet.rs`:

- `util::alphabet::ByteSet::empty`, bytes 28176..28257,
  SHA256 `95c66f772604235239ec20e74f5dd282c0f1066a69db322361fac0893858e4a0`.
- `util::alphabet::ByteSet::contains`, bytes 28903..29079,
  SHA256 `3a795ab4f5e2bb7ef972473f37ebcf6e37569964e4b95a147986a2a57307c19d`.

Both were excluded at 3.45. The owner contains nested builtin-derived Copy
`BitSet([u128; 2])`. Its other derives, custom Debug implementation, neighbors,
module imports and original build context remain present. No declarations or
original bodies were substituted to obtain admission.

Empty contains no byte. Each of 256 singleton sets contains exactly its selected
byte, including boundaries 127/128 and 255. Dense patterns and the full set obey
a flat Boolean membership reference. Constructor/getter composition returns false
for every byte. Source arithmetic gives bucket 0..1 and bit 0..127, but those
expectations alone did not supply a pipeline range certificate.

`eval/coverage-development/byte-membership-oracle.py` checks both identities,
copies the complete original crate, appends only a test module, and runs locked
default-feature Cargo tests. All three tests pass on ARM Linux: empty queries,
all 65536 singleton/query combinations, dense patterns and all-bits-set queries.
This is independent runtime evidence, not native execution on other targets or
permission to rewrite code.

## Complete blocker chain and bounded support

The original chain required mixed-unit Copy proof for fixed scalar arrays,
concrete u8-to-usize From, nested field/array reads, index and shift bounds,
and nonpanicking helper composition. Addressing the array rejection alone still
left the getter excluded; adding From admitted the leaf with a partial CFG but
left the caller excluded. Complete bounds and helper evidence were necessary.

`rust.copy-array-fields` admits fixed scalar-array fields within the existing
selected nominal proof. Exact source/type/length and positive builtin Copy
authority remain required for each selected owner and nested declaration. This
does not grant Copy to non-Copy mixed-unit owners or authorize destructors,
reference-valued arrays, slices, arbitrary generic fields or custom derives.
Old document policies cannot acquire this permission, including nested owners.

`rust.usize-from-u8` captures and verifies the exact selected core implementation
for lossless `usize::from(u8)`. The reviewed pinned `core/src/convert/num.rs`
contains explicit `impl_from!(u8 => usize)` and lossless `small as Self` with
range assertions. The three existing 64-bit target registries bind its exact
implementation identity: Linux ARM64, Linux x86_64 and Apple ARM64. Source,
trait/implementation, signature, input/result types and core authority must
agree. Existing u16-to-u32 support remains distinct. This is a named conversion
summary, not permission for custom From, arbitrary widening or narrowing.

`rust.bounded-unsigned-reads` derives ephemeral unsigned upper bounds over the
existing lowered graph and reaching definitions. Resolved unsigned types supply
complete fallback ranges. Known nonnegative constants, constant nonzero
division/remainder, verified lossless conversion and snapshots may narrow them.
Every reaching definition must have an available bound before a read narrows;
missing evidence falls back to the full type range, never zero. No arbitrary
arithmetic, path-sensitive condition inference or alias independence is assumed.
Graphs without shifts or array-value indices allocate no range state.

An array-value index can enter a helper only when its exact evaluated index
upper bound is less than the independently checked array length. Shifts require
a count below the resolved operand width. Other operations retain existing
exceptional-flow assessment. Proof results reuse existing checked-operation
records and require the registered document policy. Full producer verification
rederives them; missing or mismatched proofs do not grant helper permission.

Actual getter documents read `self.bits` and `self.bits.0` as complete nested
struct/array values through existing shared struct-field locations. The index
then operates on the array value. Existing exact owned-receiver snapshots
transport these values without a dynamic formal-element selector or new helper
summary format. Dynamic shared index helpers, subslicing/unwrap helpers, mutable
updates, array equality, iterators/closures, Result/Try serialization, general
macros, unsafe/FFI and concurrency remain outside this contract. The existing
acyclic, returning, nonpanicking helper boundary remains enforced.

No serialized range inventory, source copy, cache or helper-summary identity was
added. Historical reports and frozen selections are retained unchanged.

## Activation and verification

On the unchanged complete original library, all 2582 names/files/spans/source
digests/compilation contexts agree with the 3.45 roster. Features, cfgs, target
and compiler commit agree. Admissions rise **59 to 66: seven gains, zero losses**.
Both selected methods gain; five incidental gains are ByteClasses::empty,
write_to_len, get, alphabet_len and ByteSet::write_to_len. They do not redefine
the original selection or certify every incidental function as a total helper.
Capture and assessment complete, with zero unknowns/failures. The explicit
zero-pair budget leaves whole-library enumeration/evaluation incomplete.

The immutable fifteen-function development corpus `byte-membership.rs` fixes
five intended complete positives: constructor, getter, empty caller, word caller
and saved-owner caller. Five partial leaves and five exclusions remain explicit.
Overall admissions rise **4 to 10, with no losses**: all five intended positives
plus one explicitly partial unbounded getter gain. Unknown array indices,
rebinding, wider merges, narrower shifts and possibly-zero divisors retain
partial evidence; their unsupported callers remain excluded. Generated Clone
methods remain excluded. All 45 development comparisons complete, including
all ten pairs among the intended complete functions.

Final independent strict unit import, document/producer verification and
admission pass for **all 66 natural admissions plus five development functions**.
The selected two natural and five development graphs have complete CFG evidence
and complete all **21 comparisons: two Accepted, 19 Rejected**, no diagnostics,
under unchanged defaults. Different caller/helper graph shapes need not meet
the similarity threshold. Neither these decisions nor extraction support imply
equivalence, rewrite safety, abstraction activation or held-out detector quality.

All 867 workspace tests pass with five existing skips. Six permanent integration
tests cover complete composition, source/effect/read-summary corruption, exact
conversion identity corruption, policy downgrades, Copy/drop/reference-array
boundaries and adversarial range cases. Capture/strict checks pass on all three
installed pinned CI targets; this does not execute native macOS binaries.
All-target Clippy with warnings denied, doctests, formatting and whitespace
checks pass. Rstyle has **zero violations across 2733 functions** and patterns
**zero candidates**. No suppression or threshold was added.

Final frozen replay `run-gl3pfp6d` preserves all 60 selected identities and
denominators: **20 admitted /33 excluded /7 inactive**, zero gains/losses.
Context counts and source blocker combinations are unchanged. Full package
rosters retain 150/7 admitted for lz4_flex, 956/20 for regex-syntax and
226/8 for time-core, with identical source identities and compilation contexts
(after temporary-root normalization), and no outcome changes. All 70 documents
verify.

## Bounded performance evidence

Quiescent comparisons against isolated 69b201d (3.45) use one warmup and seven
alternating pairs per workload. Byte-window control median is .89/.89 seconds
(baseline/current ranges .88–.94/.86–.90), median RSS 122872/122808 KiB. Cyclic
control median is .63/.65 seconds (+.02 seconds, 3.2%; ranges .61–.64/.63–.65),
RSS 111684/111360 KiB. All fourteen retained control matching reports are
identical. The small cyclic median shift remains part of the result.

Activated membership costs .51/.68 seconds (ranges .49–.52/.66–.71), RSS
116316/118772 KiB, while adding six admissions and 39 comparisons (six to 45).
A single original-crate compiler capture is 8m01s/8m00s; this does not measure the
whole assessment stage or replace controlled repeated benchmarks. These bounded
measurements establish no broad performance guarantee.
