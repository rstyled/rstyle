# Original Accel parser family

Source-frozen development selection: regex-automata0.4.18
`Accel::from_slice` → `Accel::from_bytes` → `Accel::from_bytes_unchecked`.
`selection.json` fixes original byte spans, XXH3 checksums, complete error owners,
constants and context files before implementing the family. Default and separate
`dfa-search`-only library builds are in scope; no-default remains inactive.
Historical inventories and denominators are unchanged.

Contract: slices shorter than4 return BufferTooSmall `"accelerator"`, including
short slices whose first byte is invalid. Otherwise first-byte values4..255 return
Generic `"accelerator bytes cannot have length more than 3"`. Valid inputs return
exactly their first4 bytes followed by4 zero bytes; trailing bytes are irrelevant.
The array validator follows the same first-byte rule. The unchecked constructor
preserves all4 input bytes, including invalid first bytes, and adds4 zero bytes.
All inputs remain unchanged and no selected operation panics.

The complete chain needs fresh non-Copy/no-drop aggregate ownership (`Accel`
derives **Clone, not Copy**), exact source constants, usize min/prefix slicing,
saved slice/array values, slice-array `map_err`/`?` with independently proved
TryFromSliceError ownership, aggregate Result success and tagged callable
forwarding. Existing conversion observations and error constructors alone do not
complete it. Hidden Drop, fields, neighboring items and derive/source ownership
must remain checked. Further capture restrictions are unknown until activation.
Higher callers need loops/chunks or backing-storage/raw-pointer contracts and
remain outside this selection.

The independent oracle appends probes to the complete original crate, checks
all original bytes before/after execution and invokes original bodies. Division
by4 specifies validity independently of the production comparison. Separate
probes inspect all8 output bytes and original error variants/messages. Each active
build/debug-assertions context has164,352 cases: exhaustive first-byte domain,
every byte value independently in each payload position, short/exact/trailing
lengths,4 offsets and3 tail patterns, plus checked/unchecked array controls.
This is not an exhaustive Cartesian product of all32-bit input words. Fixed
category counts are stored in the selection and checked independently by Python
and the native runner.

```sh
python3 tests/fixtures/function-families/accel-parser/oracle.py
python3 tests/fixtures/function-families/accel-parser/oracle.py --run-native --output /tmp/accel-parser-oracle.json
```

Native aarch64 Linux oracle:657,408 cases/zero mismatches across default and
dfa-search-only × debug assertions on/off, opt-level0, overflow checks and unwind.
Evidence: `/tmp/rstyle-462-accel-oracle.json`. Foreign native execution and the
checked-parser integration remain pending. Reject
changed bounds, byte order, padding, error precedence/messages, captures, extra
callback work and hidden Drop. The oracle alone grants no extraction permission.

Additional source-audited constructor control before Data3.130 admission checks:
`Accel::new` (same original file, line404) constructs exactly eight zero bytes.
It is active with default `dfa-build`, inactive in the other selected contexts.
It has the same Clone-only owner and a constant-length repeat expression;
acceptance also depends on the existing repeat/constant proof. Its classification
is checked separately without changing the three parser target identities.


Data3.130 delivers direct fresh construction/return only: the original unchecked
constructor admits in the default build. The whole owner must be nongeneric,
source-proved no-drop, with bounded builtin scalar or scalar-array fields.
Owned storage, parameters, updates, discards and helper transport remain denied;
no Copy authority follows. `Accel::new` remains excluded by its named repeat count.
Default capture also admits the independently audited scalar-only direct constructor
`meta::error::RetryFailError::from_offset`; no-default remains unchanged.

The next complete target is `Accel::from_bytes`: aggregate Result success,
fresh-success cleanup and exact `Ok(verified_constructor(bytes))` roles need a
separate constructor callable proof. Existing u8-to-usize conversion authority
already suffices. The original `from_slice` additionally needs checked prefix/min,
slice-array conversion/error propagation and tagged forwarding. These are pending.
