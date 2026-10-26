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
python3 eval/coverage-development/wire-arithmetic-errors/oracle.py
python3 eval/coverage-development/wire-arithmetic-errors/oracle.py --run-native --output /tmp/wire-arithmetic-oracle.json
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

Default-feature capture is **incomplete**, with no admission claim: the existing
concrete-observation depth/entry budget aborts at
`dfa::sparse::Transitions::<T>::try_state`. Its long body repeatedly observes the
expanded error tree. Investigate capture work/accounting before broadening this
context; do not increase budgets or convert incomplete evidence to unsupported or
admitted. The independent default runtime oracle still passes. Foreign native CI
is unrun. General payload storage/moves, cleanup/unwind, map_err callbacks, `?`,
callable arithmetic summaries and shl remain separate blockers.

Evidence: `/tmp/rstyle-448-{comparison.json,strict-comparison.json,nextest.log,
focused-final.log,clippy-final.log,style-final.json,patterns-final.json,
natural-final.json,default-final.json}`; unchanged runtime oracle:
`/tmp/rstyle-445-oracle.json`. No new SHA256, serialized lookup keys or whole-unit
indexing pass. The natural no-default run took27.18s/375,744KiB peak; the final
focused composition test took2.25s with binaries built. These are workload timings,
not controlled speedup measurements.
