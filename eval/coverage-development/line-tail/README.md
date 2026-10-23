# Original Line tail selection

This separate development selection freezes base64ct **1.7.3**
`decoder::Line::slice_tail → len` before admission evaluation. Existing selections
and the frozen sixty retain their membership. The complete original method must
preserve checked subtraction, scalar Result success payload, same-error early
return, checked suffix lookup and the original borrowed field origin.

`selection.json` retains exact original byte ranges, complete Line declaration,
complete inherent/default impls, lexical imports, the entire error module and root
declarations. Historical decoder/root/manifest identities and the already selected
`len` identity are reused from `base64ct-std-import-selection.json`. No digest or
lookup-key format is introduced. The source checker compares the newly frozen
bytes directly; it does not recompute historical whole-file identities or certify
the complete original Cargo census.

The source-derived expectation is **Err(InvalidLength) exactly when nbytes exceeds
the remaining length**. Every valid result has the requested length and the exact
suffix bytes and backing pointer, including empty/full tails. `len` reports the
current shared field length, the receiver remains unchanged, and the returned
slice can outlive its temporary Line owner. The negative cases include oversized
and extreme usize requests, the other Error variant, wrong offsets/origins, missing
payload guards and changed compiler/source authority.

The methods are private. `oracle.py --run-native` therefore assembles the exact
original declaration and two methods into a small native witness and imports Error
from the unchanged full original dependency. The lexical import path is adapted
for this witness. It does **not** execute the private methods through an unchanged
public API, prove whole original-crate admission, or discharge original source
ownership/import/derive obligations. Original Decoder callers require additional
loops and mutation and are outside this selection.

The oracle checks 1,209,654 tail calls per default/std context: all tails of lengths
0–512, four invalid requests per view, three backing offsets, three byte patterns,
and 4,617 checks that returned slices outlive temporary Line values. Every ordinary
query checks length before/after and exact receiver pointer/length. No extractor
output supplies expected values. Each Cargo run has a 120-second default timeout.
Native execution certifies only its host. Future original acceptance requires
six unchanged whole-library contexts (default/std across the three pinned targets),
strict replay and twelve selected same-context identity comparisons. No admission,
matching or runtime result is recorded in this initial freeze.

Run the read-only source check without building:

```sh
python3 eval/coverage-development/line-tail/oracle.py
```

The coordinator can run the native witness after other builds/gates finish:

```sh
python3 eval/coverage-development/line-tail/oracle.py --run-native \
  --target-dir /tmp/rstyle-line-tail-native-target \
  --output /tmp/rstyle-line-tail-native-oracle.json
```

Native checkpoint (2026-10-23): the assembled witness passes default/std on pinned
ARM Linux, each with1,186,569 valid,18,468 invalid and4,617 lifetime queries and
zero mismatches. Evidence: `/tmp/rstyle-387-line-tail-oracle.json`. This does not
establish unchanged whole-crate capture, strict admission or matching.

Data3.88 connects the complete original `slice_tail → len` chain. The new pieces
are eager `Option<usize>::ok_or` with the actual fieldless Copy Error, same-error
`usize` success extraction into a borrowed Result, and checked suffix access.
The error's source/derive/import proof grants Copy; no no-drop record is invented.
`RangeFrom<usize>` remains non-Copy and is permitted only as the fresh checked
access carrier. Receiver/start/error values are saved in source order; suffix
success requires start ≤ saved length and preserves backing pointer plus start,
including the empty end pointer. Neither suffix construction nor scalar success
extraction reads slice contents. General residual conversions remain unsupported.

Compared with committed baseline `c0f4e51`, each of the six unchanged contexts
moves from1/2 to2/2 selected admissions: one gain, zero losses. Full original
rosters remain141 default/150 std; their only admission gain is `slice_tail`.
Reports and baseline deltas are retained under `/tmp/rstyle-388-line-*` and
`/tmp/rstyle-388-baseline-line-*`. Captures used zero pair budget; full-library
matching is incomplete. The dedicated acceptance probe separately strictly
imports the complete original bundle, checks both selected source identities and
all frozen source obligations, admits both methods, evaluates both identity
comparisons, and rejects eight resealed semantic attacks:

```sh
cargo build --locked -p rstyle-ccgraph --example line_tail_activation
target/debug/examples/line_tail_activation REPORT.json
```

The connected Cargo regression also compares an independently renamed complete
method and checks non-Copy errors, custom error conversion and unsupported scalar
payloads. A wrapper returning this borrowed Result remains excluded: its helper
summary requires a separate supported contract. Original public Decoder callers
still require mutation, loops and additional dispatch/owned-storage support.
