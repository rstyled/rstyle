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
python3 eval/coverage-development/wire-id-readers/oracle.py
python3 eval/coverage-development/wire-id-readers/oracle.py --run-native --output /tmp/wire-id-readers-oracle.json
```

Native execution is coordinator-scheduled: default/no-default ×debug assertions
on/off, opt-level0, overflow checks and unwind panics. Reuse existing XXH3 source
and whole-copy checks; no production sources are omitted or replaced.

Data3.124 admits both original error constructors and publishes distinct callable
summaries: exact generated-error provenance, whole no-drop proof, one-use move
receipts and parameter1 static-message routing. Neither empty reference origins
nor a matching type token supplies move authority. Default204→206/2,582 and
no-default143→145/590, zero losses. All three readers remain excluded.

Existing decoder source-body helper summaries and complete generated error no-drop
proofs are dependencies. Remaining connected work: ordinary generated caller/
associated SIZE ownership; nominal tuple Results; saved prefix reads/panics;
capturing map_err with exact Err-only invocation; same-error nominal→tuple
propagation; callable reader and checked wrapper composition.
Generated caller receipts and aggregate ownership interactions remain unknown
until proved. Reject hidden Drop, changed variants/limits/actuals, lost pointers,
callback-order changes and omitted panic/read/guard evidence. Stored mutable
callbacks, loop-heavy sparse validation and unsafe DFA loading stay deferred.
