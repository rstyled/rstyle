# Original checked integer-widening readers

Source-first development selection, separate from the frozen sixty:
regex-automata0.4.18 `wire::{try_read_u32_as_usize,try_read_u16_as_usize}` in
unchanged default/no-default library contexts. Data3.125 excludes both wrappers.
No admission gain is claimed by this contract or oracle.

On selected64-bit hosts, fewer than four/two bytes returns BufferTooSmall with
the original static message pointer/length. Otherwise the first four/two bytes,
interpreted in native endian order, give `Ok((usize(value),4/2))`. Tails cannot
change the result; inputs remain unchanged. Short checked errors remain distinct
from raw decoder panics. The independent encoder uses arithmetic, not production
decoding operations. The whole original crate is copied with append-only probes;
all original files, neighboring declarations and private bodies are retained.

The corpus has246,134 cases/context:131,072 exhaustive u16 reader/wrapper checks,
114,480 boundary/one-hot/fixed-sample full reads,576 short errors and6 constructor
controls. Offsets, tails and equal strings in distinct static storage exercise
saved input/count/message expectations. Default/no-default ×debug assertions
on/off runs use opt-level0, overflow checks and unwind panics.

```sh
python3 eval/coverage-development/wire-usize-readers/oracle.py
python3 eval/coverage-development/wire-usize-readers/oracle.py --run-native --output /tmp/wire-usize-readers-oracle.json
```

First connected activation target: the u32 wrapper. Its complete chain is
`try_read_u32` → outer Result `and_then` tuple binding → selected lossless
u32→usize conversion → `map` capturing saved count → `map_err` capturing the
original static message → `DeserializeError::invalid_usize`. The checked reader,
decoder and constructors already admit. Missing pieces are nested callback HIR,
scoped selected-call receipts, exact environment/signature and intermediate-result
evidence, complete guarded source/graph/component origins and callable summaries.
Capturing closures must not be flattened into the root call roster. The outer
environment is created even on a short reader error; its body runs only on Ok.
Both inner environments exist only inside that success branch. A proved total
conversion does not authorize omitting its error callback source or cleanup proof.

The u16 follow-on has separate prerequisites: its checked reader still has narrow
u32/four-byte restrictions, and its conversion selects blanket TryFrom→Into→From
with an uninhabited error (`Infallible` aliases `!` on this compiler). The u32 path
selects a concrete implementation with TryFromIntError. Do not conflate these
error vocabularies or infer blanket obligations from a matching type spelling.

Required negatives include altered tuple order/count/captured message, rebinding,
extra callback work, changed implementations/substitutions/error types, target
width/core identity changes, wrong byte order/width, hidden cleanup, missing
scoped calls and CFG bypasses. Runtime agreement supplies no extraction authority.
Strict admission and complete selected matching remain activation requirements;
general stored/mutable closures and generic trait resolution remain outside this
bounded family.

Data3.126 prerequisite: real Cargo capture retains all three original u32 callback
environments and separates their call inventories. The mapped conversion Result
has a genuine usize/count tuple and opaque error; all standalone/nested value
uses remain closed. Source grammar and scoped-selector adversaries are tested,
but these leaves grant no admission authority. Exact expected call-roster proof,
guarded graph/component flow, error-discard proof and callable summaries remain
the next connected delivery. Both wrappers still exclude in both contexts.
