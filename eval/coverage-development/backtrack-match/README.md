This corpus freezes the original safe `lz4_flex 0.14.0::block::compress::backtrack_match`
before another admission measurement. The fixed sixty and their identities remain
unchanged. `selection.json` copies the existing selected source, span and historical
identity verbatim; the harness computes no digest. The installed 0.11.6 crate is
not a substitute.

Run the source check, then optionally the small native witness:

```sh
python3 eval/coverage-development/backtrack-match/oracle.py
python3 eval/coverage-development/backtrack-match/oracle.py --run
```

`--source-root PATH` selects another retained copy of **0.14.0** and still requires
exact frozen bytes at `[8505,9088)`. Native compilation uses the pinned nightly,
debug assertions, overflow checks and unwind. It compiles one standalone source
file, without Cargo or dependencies. The private original function is inaccessible
through the unchanged public API, so the harness copies its complete unchanged
function bytes into a wrapper. This is diagnostic private-body execution, **not**
unchanged full-crate execution, Cargo capture, strict replay, admission or matching.
It does not exercise neighboring items, cfg selection or package imports. Those
remain obligations of complete original capture.

The independent specification finds the maximal equal suffix of the two initial
slice prefixes, bounded by `candidate` and `cur - literal_start`, then subtracts
that suffix length from both initial cursors. It does not mirror the mutable loop.
It separately specifies guard short circuit and ordered bounds failures. Both bad
indices are tested with distinct length/index panic messages to distinguish the
first failing access. Panic payload wording is a pinned-toolchain diagnostic
observation; this is not a general Rust guarantee about panic text.

The witness checks exact normal and panic cursor post-state, zero writes on first
mismatch, boundary/excess/maximum cursors, empty and offset views, identical and
overlapping immutable backings, and unchanged bytes. The mutable cursors are always
distinct local objects. It never creates aliased mutable references or uses unsafe.
A normal first comparison makes all later decreasing indices valid; therefore any
bounds panic in this particular source occurs before any store. This source fact
does not establish a general exceptional-memory or unwind-cleanup model.

The historical bundle in `selection.json` retains complete original HIR but lacks
mutable-location evidence. Fresh captures now independently verify the five-formal
roster, eight ordered scalar accesses and ten conservative reference-root alias
pairs under the [observation contract](../../../docs/extraction-profiles/mutable-scalar-formals-v1.md).
Mutable interface observations remain Unsupported; neither these records nor the
existing cyclic normal CFG establish location-aware mutable state or admission.
Next connect loads/stores to cyclic memory dependencies and normal/failure
post-state. Stateful helper export and public compressor behavior remain outside
this increment.

The oracle's distinct valid cursors do not establish captured disjointness. In the
conservative abstract alias model, the first cursor store can change the second
cursor before its compound load; checked subtraction could then fail after a
store. Preserve that ordering and failure post-state rather than applying the
distinct-cursor oracle's pre-store panic conclusion to all abstract aliases.

Validated capture: `target/hir-coverage/run-gdz69t4m` (native default features).
The strict readiness probe verifies the original 150-definition owner and selected
unchanged function; it remains excluded. Frozen sixty remain 23 admitted / 30
excluded / 7 inactive, gain 0 / loss 0. Native oracle: 48,856 cases, zero mismatches.
Other declared target captures remain pending for connected memory activation.

Neither source checks nor this oracle currently claim a new admission. Future
positive expectations are the complete selected safe function in each of the
three declared default-feature build contexts. Unsupported unsafe siblings,
reordered accesses/stores, stale loop states, omitted writes and merged roots must
remain negative; these are production corruption-test obligations, not assertions
that the present oracle independently tests serialized evidence.
