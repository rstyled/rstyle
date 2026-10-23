# Checked index construction

Separate source-first `regex-automata 0.4.18` family, in two delivery boundaries:
original `SmallIndex::new → TryFrom<usize>` with complete success/error helpers;
then original generated `PatternID::new` with exact `map`/`map_err` constructors.
Frozen inventories and previous Match construction selections stay unchanged.

`selection.json` fixes six ordinary functions, three generated member **template**
ranges, complete relevant declarations/impl/macro/invocations, full-file XXH3
context checks and independent expectations before admission measurement.
Templates are not fabricated emitted function identities. Original default and
no-default library contexts use pinned nightly, 64-bit ARM Linux initially;
x86 Linux/ARM macOS captures are distinct later gates.

`python3 eval/coverage-development/checked-index-construction/oracle.py` checks
source with system libxxhash XXH3, without building. The coordinator can schedule
`--run-native` (optionally `--context default` or `no-default`). It exercises
actual public SmallIndex and PatternID APIs on boundary/deterministic values:
x≤2147483646 yields exact x, otherwise exact attempted=x as u64. SmallIndex is
public through `util::primitives`; only its fields and local Usize trait are
private. A separate exact Usize trait/impl source copy supplies diagnostic
width/debug controls, not original private-helper acceptance. Debug formatting
checks are runtime controls, not an extraction contract for formatting effects.

Canonical capture is debug_assertions=true. `--debug-mode release-branch` or
`both` supplies separately labeled runtime controls at opt-level0; neither may
replace the original checked debug body. On chosen64-bit targets the actual
usize→u64 conversion must be independently proved total before expect's failure
branch can be eliminated. No width/general-conversion shortcut is authorized.

Before activation, resolve raw Usize/core TryFrom and MAX evidence, non-Copy
error no-drop/ownership, exact source/trait membership, generated callable/error
ownership, constructor callback selection and ordered variant transport. All
known blockers/unknowns and adversarial expectations are in the selection.
Strict full-context verification/admission, complete selected matching and
unchanged-selection gains/losses remain gates. `PatternID::must` and Match::must
retain reachable expect/formatting and other blockers beyond this family.
Native public-API oracle now passes8,202 cases (4,101 successes/4,101 errors)
per default/no-default and debug/release-branch context, zero mismatches. The
copied private Usize witness passes separately; its unused original trait members
produce dead-code diagnostics. These are runtime/source expectations, not
extraction admission. Evidence:`/tmp/rstyle-402-index-oracle.log`.

Data3.95 source increment reuses exhaustive Copy/source receipts for exact ordinary
nongeneric inherent owners with scalar arguments, shared receivers and scalar or
owning-product results. Both original `index_type_impls!` neighbors remain checked;
no generated callable or function-effect authority follows from a source receipt.
The new receipt binds original method/impl/declaration, complete interface,
source census, cfg/build and profile. Historical3.94 document downgrades reject.

Native ARM Linux no-default complete owners retain590 identical definitions:
103→109 admissions, zero losses. Of the six preselected ordinary functions,
`new_unchecked` and `as_usize` gain; the other four remain excluded. Four additional
roster gains are `as_u64`, `as_u32`, `as_i32` and `one_more`; these are incidental
outcomes, not retroactively frozen source-first targets. All six gains pass strict
original/replay verification, admission and identity matching (Accepted score1).
The caller `one_more → as_usize` now composes. Matching for the complete library
remains explicitly budget-incomplete; cross-target/default contexts are pending.
Evidence:`/tmp/rstyle-403-native-delta.json`, `/tmp/rstyle-403-strict.log`.

The checked constructor family remains incomplete. Its remaining chain is MAX
nominal constant evidence, real local Usize/debug core conversion dispatch,
non-Copy scalar-error no-drop/whole Result transport and connected summaries.
Generated PatternID additionally needs emitted callable/error ownership and
ordered map/map_err constructor transport. Separate unregistered proof leaves
for scalar errors and local trait dispatch are development work, not activation.

Final source gate preserves ordinary lexical alias/import rejection and adds
adversarial generated primitive/glob controls. All1,624 workspace tests plus
doctests pass, fmt/Clippy pass, style0 and patterns0 with15 unchanged ignores.
Strict replay checks all six identity pairs and completes15 original pairs;
frozen60 identities, denominators and outcomes are unchanged (24/29/7).
Evidence: `/tmp/rstyle-403-final-*`, `/tmp/rstyle-403-frozen-comparison.json`.
