# Original ID readers and error constructors

Separate source-first development selection: regex-automata0.4.18
`wire::{read_state_id,try_read_state_id,read_pattern_id}` and
`DeserializeError::{state_id_error,pattern_id_error}`, original default/no-default
library contexts. Complete generated declarations, error variants, neighboring
source and build identities stay frozen; earlier selections are unchanged.

Independent contract on selected64-bit hosts: mathematical native-endian first4
bytes produce x. For x≤2147483646 return the matching ID and consumed4; larger
values return the exact StateID/PatternID error with attempted=u64(x). Supplied
`what` retains its original pointer/length. Direct readers panic for lengths0..3;
the checked reader instead returns BufferTooSmall before decoding. Inputs remain
unchanged and trailing bytes are ignored. Unchecked ID readers are outside scope.

The append-only whole-original-crate oracle covers323,154 cases/context:
319,680 full reads,1,152 checked short errors,2,304 expected direct-reader panics
and18 constructor controls. It exercises boundaries, every one-hot bit,256 fixed
samples, offsets, tails and two equal strings with distinct static storage.
Expected bytes use arithmetic, not production native-decoding APIs. No extraction
or admission authority follows from runtime agreement.

```sh
python3 tests/fixtures/function-families/wire-id-readers/oracle.py
python3 tests/fixtures/function-families/wire-id-readers/oracle.py --run-native --output /tmp/wire-id-readers-oracle.json
```

Native execution is coordinator-scheduled: default/no-default ×debug assertions
on/off, opt-level0, overflow checks and unwind panics. Reuse existing XXH3 source
and whole-copy checks; no production sources are omitted or replaced.

Data3.125 connects all three original safe readers to the Data3.124 owned-error
constructors. The two direct readers publish read/panic summaries. The checked
wrapper admits as a complete root but publishes no callable summary: the current
flat conditions cannot combine its precheck with the callee's nested Result tag.

The contract now includes exact generated SIZE ownership, independently checked
Copy/no-drop nominal ID tuple results, saved prefix reads and bounds, an immediate
Err-only callback with immutable static capture, one-use owned error transfer and
nominal-to-tuple same-error `?`. Wrong errors/messages/counts/guards, hidden Drop,
changed saved values, missing effects and CFG bypasses remain rejected. Captured
shapes alone never establish ownership, source authority or cleanup.

Unchanged selections: default206→210/2,582; no-default145→148/590; zero losses.
Default additionally admits `Special::write_to_len`: its exact source is
`8 * StateID::SIZE`, independently expected32. Existing admitted records and
nominal declaration receipts remain unchanged; excluded functions can retain
additional concrete declarations without gaining admission.

Stored mutable callbacks, cursor updates, nested guarded callable outputs,
loop-heavy sparse validation and unsafe DFA loading remain open. In particular,
`Special::from_bytes` also needs its captured mutable reader, tuple binding and
`?`, aggregate construction, `validate` and assertion/helper composition. These
readers do not certify that caller. See the
[execution record](https://github.com/rstyled/rstyle/blob/c35bd14c/docs/rust-support-execution.md) for completed gates
and retained evidence.
