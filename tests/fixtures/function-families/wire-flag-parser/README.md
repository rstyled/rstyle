# Original wire flag parser

Separate development selection, fixed before implementation/admission evaluation:
regex-automata0.4.18 `dfa::dense::Flags::from_bytes` → delivered
`util::wire::try_read_u32`. Original default library context is active; unchanged
no-default is inactive because `dfa-search` gates the module. Historical
selections and denominators are unchanged.

Contract: lengths0..3 return the original `BufferTooSmall` error with message
`"flag bitset"` before decoding. Otherwise read the native-endian first4 bytes,
set the three boolean fields from bits0/1/2 and return consumed count4. Ignore
upper29bits/trailing bytes, preserve all input bytes and never panic.

The independent oracle appends test entry points to the complete original crate;
all original bytes are checked before/after execution. Expected flags use
division/remainder, independent of the production masks/shifts. Each default
debug-assertions context exercises82,368 cases: all8 flag combinations ×33 upper
patterns ×8 offsets ×13 lengths ×3 tail patterns;25,344 short errors and57,024
successes. Native execution is opt-in, debug assertions on/off at opt-level0:

```sh
python3 tests/fixtures/function-families/wire-flag-parser/oracle.py
python3 tests/fixtures/function-families/wire-flag-parser/oracle.py --run-native --output /tmp/wire-flag-parser-oracle.json
```

Data3.123 connects complete concrete aggregate Result capture, independently
proved builtin Copy/no-drop source ownership, exact tuple→aggregate-tuple
same-error `?`, immutable local Flags construction and the saved consumed
component. Complete graph verification retains static error roots, reader reads
and panic evidence; every original statement and operation is accounted for.
Selected impl/module/derive ownership remains mandatory; neighboring bodies are
not certified.
Reject swapped masks/fields, changed errors/guards, wrong consumed components,
hidden effects/Drop and omitted source/read/panic evidence. Runtime success grants
no admission, equivalence or rewrite authority. The only direct higher callers
are unsafe DFA deserializers and remain outside this bounded target.


Measured activation: default203→204/2,582 (only original Flags parser), unchanged
no-default143/590, zero losses. All historical selected pairs and the frozen60
persist; tests and quality gates pass. Full validation, timings and the remaining
caller frontier are recorded in [the execution record](https://github.com/rstyled/rstyle/blob/c35bd14c/docs/rust-support-execution.md).
