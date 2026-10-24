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

Data3.93 connects the complete selected function through formal memory locations,
ordered loads/stores, cyclic RAW/WAR/WAW dependencies, and explicit checked-failure
post-state. Strict producer/document verification and original/replay matching pass
on the fresh native full 150-definition owner. Frozen identities and denominators
are unchanged: 24 admitted / 29 excluded / 7 inactive, gain 1 / loss 0;
`backtrack_match` is the only changed outcome. Full lz4 owner: 9 admitted / 141
excluded (previously 8 / 142). Evidence: `target/hir-coverage/run-ng8slil8` and
`/tmp/rstyle-399-{frozen-comparison.json,natural-strict.log}`.

The native oracle passes 48,856 cases with zero mismatches. Its distinct valid
cursors do not establish captured disjointness: conservative aliasing permits
checked failure after the first store. Strict corruption tests independently
require 46 memory edges, four roots, ten alias pairs and six checked exits,
including that post-store failure. Historical profiles remain closed.

Fresh full-owner captures, strict replay/admission and selected matching pass in
all three declared target contexts (`target/backtrack-match-399/`). Native
foreign-target execution remains pending; cross-target capture does not prove it. Stateful helper
export, local reference storage, reborrows, plain stores, owned cleanup and public
compressor callers remain unsupported. This completes one bounded mutable-formal
family, not general memory or ownership coverage.
