# Original checked integer-widening readers

Source-first development selection, separate from the frozen sixty:
regex-automata0.4.18 `wire::{try_read_u32_as_usize,try_read_u16_as_usize}` in
unchanged default/no-default library contexts. Data3.125 excludes both wrappers.
The independent oracle grants no extraction authority; measured activation follows below.

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
decoder and constructors already admit. The connected proof requires nested callback HIR,
scoped selected-call receipts, exact environment/signature and intermediate-result
evidence, and complete guarded source/graph/component origins.
Capturing closures must not be flattened into the root call roster. The outer
environment is created even on a short reader error; its body runs only on Ok.
Both inner environments exist only inside that success branch. A proved total
conversion does not authorize omitting its error callback source or cleanup proof.

The u16 follow-on has separate prerequisites: Data3.127 still restricted its
checked reader to u32/four-byte proof, and its conversion selects blanket TryFrom→Into→From
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

Data3.127 activates the complete original u32 wrapper. It proves all three
immutable environments and scoped core selections, saved decoded/count/message
components, read bounds and ordering, distinct reader/conversion errors, and the
original no-drop error owner. The pinned u32→usize conversion is total on the
selected64-bit targets; its unreachable error branch remains represented and
verified, including Copy/no-drop wildcard discard. No opaque error is treated
as unit or as an unguarded shared reference. Strict serialized replay and
independent document verification reject source, capture, selection, guard,
order, origin and historical-policy mutations; the unchanged duplicate must
match completely at score1.

The verified read/panic/tagged summary is published, but a whole-result forwarding
caller remains excluded: ordinary reference-output summaries cannot contain a
guarded error message. A safe follow-on must retain that tagged boundary and
callee scope while substituting arguments and effects. General callbacks,
blanket conversions, moves/drop and the u16 chain remain outside this contract.

Acceptance459: unchanged default210→211/2,582 and no-default148→149/590,
exactly the original u32 wrapper, zero lost admissions or changed prior admitted
records. All211/149 selections pass two strict imports and score1 self replay;
complete22,155/11,026 cross pairs preserve every prior pair result. Frozen60 and
historical profiles are unchanged. Native aarch64 Linux oracle984,536 cases,
zero mismatches; macOS/x86 definition capture is not native CI execution.

Data3.128 connects the original checked `try_read_u16` dependency, with a closed
width descriptor binding the result, both sizes, prefix, copy/unwrap and native
decoder. Its separate propagation identity/capability preserves the u32 contract.
The independent graph verifier now also rejects entry bypass, repeated reads and
changed exit kinds for both widths. Wrong bounds/count/helper/endianness remain
excluded. Capturing a u16 tuple interface alone grants no reader-body authority.
The widening wrapper remains excluded pending actual blanket selection closure,
never-error carriers and complete connected callback proof.

Acceptance460: default211→212/2,582 and no-default149→150/590, exactly
`try_read_u16`, zero losses. All212/150 strict selections and33,541 cross pairs
pass with earlier pair results preserved; frozen60/historical profiles unchanged.
The original-source oracle repeats984,536 cases with zero mismatches.
