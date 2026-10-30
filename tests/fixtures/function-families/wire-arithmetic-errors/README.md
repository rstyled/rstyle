# Original checked arithmetic error chain

Source-selected before activation: unchanged regex-automata0.4.18
`util::wire::{add,mul}` → `DeserializeError::arithmetic_overflow`.
`selection.json` freezes original functions, build contexts, complete files,
all ten error variants/15 fields and both generated error children. Frozen
historical selections remain untouched.

Data3.119 supports the complete chain in the native no-default Cargo context.
Checked usize addition/multiplication returns the mathematical value when it fits,
otherwise the exact ArithmeticOverflow variant retaining the static `what` pointer
and length. No panic is permitted. Constructor summaries retain the exact
wrapper/variant/field/reference route; arithmetic retains empty Ok origins and a
None-guarded Err route from formal2. Every graph operation, saved operand, payload
guard, consumer and return is checked. The non-Copy error's no-drop proof covers
all ten variants and five nominal dependencies, including unused variants and both
generated children. Checked imports/reexports terminate at exact source receipts;
these receipts grant no Copy, arbitrary macro, derive or callable authority.

The bounded constructor contract is one unchanged explicit static-string formal,
one fresh named variant field and its unary wrapper. Source review confirms four
additional unchanged constructors (`generic`, `buffer_too_small`, `invalid_usize`,
`label_mismatch`) fit it. They are measured incidental gains, not additions to the
three-function frozen development selection. add/mul consume a constructor summary;
this does not yet make add/mul callable helpers or support propagation through `?`.

Independent oracle: u128 mathematical sum/product, all256² small pairs,
15² boundary pairs and8,192 deterministic full-width pairs, three static messages,
both operations and three direct helper checks. Original errors are matched against
the private variant and exact string pointer/length; panic fails. **443,721 checks
per context**, zero mismatches in default/no-default × debug assertions on/off.
The append-only harness checks every copied original byte. Expected values never
come from production extraction. These prior runtime results remain applicable to
the unchanged originals; they do not grant capture/admission authority.

```sh
python3 tests/fixtures/function-families/wire-arithmetic-errors/oracle.py
python3 tests/fixtures/function-families/wire-arithmetic-errors/oracle.py --run-native --output /tmp/wire-arithmetic-oracle.json
```

Measured no-default admissions: **131→138/590**, seven gains/zero losses, identical
source/build selections. Capture and assessment complete; zero-pair roster matching
is explicitly incomplete. Separate strict verification imports all138 originals
and replays (276 imports), checks138 original/replay comparisons at Accepted1,
and completes9,453 cross-original comparisons. All131 prior verification records
and8,515 pair records persist exactly.

Independent semantic expectation distinguishes add(2,3)=5 from mul(2,3)=6.
The structural matcher nevertheless accepts add/mul at score1; this fails an
expectation of semantic separation and establishes no equivalence or rewrite
safety. Both constructor/arithmetic comparisons reject. Extraction support and
detector quality remain separate claims.

Validation: full2,177-test suite plus doctests passes; final routing refactor has
17 focused tests plus doctests passing. Multiline signature and import/reexport
composition run through real Cargo and strict replay. Fifteen resealed graph
attacks reject changed static/component origins, missing annotations, flattened
origins, changed None guards, nonempty Ok routes and removed branches. Both direct
and reexported hidden Drop variants exclude every selected function; incomplete,
changed or reordered type inventories and the historical3.118 downgrade reject.
Final fmt/Clippy clean, style0/7,256 and patterns0/15 unchanged ignores.

At Data3.119, default-feature capture completed after function-local reuse of completed
root type observations/tokens:2,582 functions/110 admissions, independently checked
selected source/build contexts. At that checkpoint, the three selected originals remained **excluded**:
cleanup source proof follows imported alloc::vec in dfa::dense into an unknown
external root before consulting existing pinned library authority. This was the
next source-boundary blocker, not a missing capture or admitted error chain.
The formerly failing counter reached4,097 entries at depth1; caching avoids repeated
work without changing limits. Genuine oversized trees and repeated interface
inventories still fail incomplete; nested char remains unsupported after a root
char observation. No-default all590 function records,138 strict replay checks and
9,453 pair records remain identical. Full2,181 tests/doctests, fmt/Clippy, style0
and patterns0 pass. Timings are not controlled speedup measurements.
Foreign native CI is unrun. General payload storage/moves, cleanup/unwind,
map_err callbacks and callable arithmetic summaries remain separate. Audit found
all direct arithmetic callers need unsafe or stateful iterator support; prioritize
`check_slice_len::<u8>` → `try_read_u32` instead, requiring concrete generic identity,
unit/tuple success Results, guarded callable error origins, saved read/panic evidence
and same-error propagation. Expand to other widths and validators after it connects.

Evidence: `/tmp/rstyle-448-{comparison.json,strict-comparison.json,nextest.log,
focused-final.log,clippy-final.log,style-final.json,patterns-final.json,
natural-final.json,default-final.json}`; unchanged runtime oracle:
`/tmp/rstyle-445-oracle.json`. No new SHA256, serialized lookup keys or whole-unit
indexing pass. The natural no-default run took27.18s/375,744KiB peak; the final
focused composition test took2.25s with binaries built. These are workload timings,
not controlled speedup measurements.

Capture reuse evidence: `/tmp/rstyle-449-{comparison.json,strict-comparison.json,
nextest.log,clippy.log,style.json,patterns.json,default-before.json,
default-after.json,no-default.json}`. Exact rustc Ty keys remain function-local;
recursive capture never reuses root-only permission, budget failures never become
cached Unsupported values, and per-use constant/environment evidence stays intact.


Data3.120 removes that default-build blocker through unique pinned alloc macro
imports and native std::thread_local routing. Both use existing complete cleanup
argument/initializer walkers; no macro body receives function-effect authority.
The unchanged three originals now admit in both contexts. Default110→198/2,582
includes88 source-audited gains/zero losses;51 already admitted without defaults.
No-default138/590 and every compact function record remain identical. New Cargo
composition0.94s; shadowing, globs, hidden Drop, missing/private/changed exports,
missing native TLS and historical3.119 policies reject. Further validation and
reader prerequisites remain in the [execution record](https://github.com/rstyled/rstyle/blob/c35bd14c/docs/rust-support-execution.md).
