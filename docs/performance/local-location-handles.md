# Pair-local location handles

Base: `3f6e7639e19c0858323effc852d6fe8ad797b0a7`; candidate is the commit
introducing this document. This is the location-identity increment in the
[full implementation plan](hash-and-matching-plan.md), not completion of build,
source, unit, transport or matching work.

`data_documents::formal_location` previously constructed JSON for a root/path,
serialized it into a temporary byte vector and SHA-256 hashed the vector on each
call. Location construction now goes through `LocationContext`, borrowing one
immutable function view and owning a `LocationTable` for one paired graph.
`LocationKey` contains the formal parameter index, exact source root symbol and
complete ordered `Vec<Projection>`, including nominal identity, field position,
field name and each explicit shared dereference. A `BTreeMap` interns complete
keys into private-constructor `LocationId(u32)` handles. Equal keys reuse a
handle; contradictory types for the same key fail. Distinct formal inputs remain
distinct even when aliasing is possible. Every read occurrence and operand
position remains in its ordered access vector.

No lookup hash, digest, JSON identity tree or identity byte vector participates
in interning. The real location record remains serialized for the PDG transport.
Helper argument checks now visit checked typed read occurrences without creating
location IDs, MemoryLocation records or JSON that would immediately be discarded.
Type/origin and explicit 4096-entry occurrence-budget checks run for every visit.
Helper location verification reconstructs each call's expected effects once,
using the same result for ordered access validation and inventory accumulation.

## Format, scope and trust

The abstraction advances from `rstyle.formal-field/1` to
`rstyle.formal-field/2`. IDs export as `location:local:N`; N is a pair-local dense
index. The location evidence extension retains parameter, root_symbol, ordered
path and type. IDs have no content-fingerprint or cross-pair meaning. They may
change after source edits or construction order changes. No persistent cache
may reuse a location solely because its exported integer/string matches.

Import decodes a typed key table, requiring canonical in-range dense IDs, no
duplicate IDs or duplicate complete keys and the version 2 abstraction. The
frozen imported table cannot create a missing handle. Source-derived field and
helper locations must resolve by exact key and type, with the existing complete
inventory, origin, signature, read/effect and source correspondence checks.
The table owns its lookup namespace; no operation accepts a foreign bare ID.

Strict producer replay translates actual to reconstructed IDs by complete typed
key and type equality, then performs the existing full normalized document
comparison. `location_id` and place-location annotations participate in that
reference translation. Conflicting reference namespaces fail. Consistent local
renumbering is now supported, without making arbitrary location metadata valid.
Old content-hash IDs/version 1 locations are rejected by this contract; there is
no permanent dual runtime. Pinned standalone graph schemas remain unchanged
because their abstraction field already carries the versioned interpretation.

Retired guarantee: a location string no longer fingerprints its root/path, nor
is it stable outside its owning graph. Computational source/graph associations
remain independently verified; the numeric handle grants no verification
capability. This change does not cross strict/generated policy boundaries.

## Regression coverage

- Complete-key tests distinguish roots, root symbols and projection order, reuse
  equal keys, and reject contradictory types.
- Imported-table tests cover changed numbering, a foreign/missing key despite a
  matching local integer in another table, duplicate keys/IDs, sparse/out-of-range
  handles and the retired version.
- The complete read/helper CLI family renumbers multiple locations, updates all
  references and reverses record order. Both data validation and strict producer
  validation must succeed; duplicate handles must fail both validators.
- Existing field, origin, alias, call operand/effect and source-edit tampering
  regressions remain enabled. No ignores, thresholds or assertions are removed.

## Measurements and validation

Matched prebuilt tools at the base above and this candidate used
`scripts/measure_library_resolution.py` with `/tmp/rstyle-before-location` and
`target` respectively. Linux aarch64, eight CPU quota, 16 GiB limit, pinned
nightly-2026-09-30, locked dependencies, one command at a time. Analyzer debug
and release profiles use the same ordinary fixture check settings. Each workload
has one warmup, three ordinary samples per trace mode and a separate diagnostic
run; three strict replays follow. Builds and tests completed before measurement.
Before and after phases ran sequentially, so machine drift is a limitation.
Time resolution is 0.01 s. [Raw samples](local-location-samples.json) retain all
156 commands, RSS and report sizes, including warmups and diagnostics. The
harness's retired library fingerprint counters are zero in both revisions and
are not location operation counts.

Ordinary wall medians in seconds (before → after):

| Analyzer | Workload | Plain | Trace |
| --- | --- | --- | --- |
| Debug | Minimal, 2 functions/1 pair | 0.23 → 0.24 | 0.23 → 0.23 |
| Debug | Read, 20 functions/190 pairs | 1.59 → 1.58 | 1.61 → 1.57 |
| Debug | Inherent, 21 functions/210 pairs | 2.22 → 2.21 | 2.23 → 2.21 |
| Release | Minimal | 0.21 → 0.21 | 0.21 → 0.20 |
| Release | Read | 0.38 → 0.34 | 0.37 → 0.36 |
| Release | Inherent | 0.42 → 0.41 | 0.41 → 0.42 |

Strict replay medians, debug minimal/read/inherent: 0.01/0.65/0.98 →
0.01/0.64/0.94 s; release: 0.00/0.07/0.11 → 0.00/0.07/0.09 s. Zero is
below timer resolution. Maximum RSS across all modes and iterations changed
from 114040 to 114048 KiB for debug read and 121124 to 121160 KiB for debug
inherent; release read/inherent changed 109304 → 109392 and 116348 → 116512 KiB.
No memory improvement is established. Timing gains are small and mixed; they
are not a suite-wide speedup claim. All runs retained identical named pair
scores, stage outcomes, decisions, admissions and completeness, including
cross-profile parity. Strict imports completed for every captured bundle.

Structurally, each former formal-location identity construction loses one JSON
identity tree, byte-vector serialization and SHA invocation; argument-only
validation loses discarded wire location construction. These are per-operation
counts established by the implementation, not measured global allocation totals.

Validation: formatting check; warning-free workspace/all-target Clippy; full
`CARGO_TARGET_DIR=target ./scripts/test.sh --nextest` (647 passed, zero skipped,
workspace doctests passed); 24 Python tests passed. Focused location unit tests
and complete helper-family renumbering/strict-import tests passed. Both style
gates passed with zero violations and zero actionable candidates; 15 existing
reviewed ignores remain unchanged. No thresholds, ignores or suppressions were
added. The full implementation plan remains active.
