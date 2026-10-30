Original regex-automata0.4.18 `SmallIndex::try_from` for u16/u32/u64/usize.
`selection.json` fixes the original functions, complete impl/source contexts,
blocker chain and source-first expectations. Existing frozen selections are intact.

Data3.117 joins exact source/structural/call input evidence with the existing
local selected-impl, associated Error, no-drop and complete helper proofs.
U16/U64 by-value local routes retain their whole trait/impl rosters. Pinned core
u16→u64 From and 64-bit u64→usize TryFrom/expect retain exact implementation,
substitution, original operand/message and target authority. The reviewed core
source makes that TryFrom always Ok; this grants no permission to discard an
arbitrary conversion error. Historical policies gain no new capability.

Contract: input≤2,147,483,646 returns the same numeric index; larger input returns
its exact attempted u64 value. Every u16 succeeds; high u64 bits cannot truncate.
Signed/generic/dynamic inputs and unreviewed targets remain outside this contract.
Generated PatternID/StateID TryFrom wrappers still need independent trait-impl
expansion/associated Error ownership and connected Result map/map_err callbacks.
Their public runtime oracle checks grant no extraction permission.

Native no-default Cargo roster:118→123/590, only the three added conversions and
U16/U64 as_usize helpers; zero losses. Original identities and build settings
match after normalizing capture navigation/producer revision. Capture/assessment
and all7,503 library comparisons complete. All four selected originals strictly
verify/admit with serialized replay; identical replay pairs Accepted score1.
All six cross-input original comparisons complete and Rejected; these distinct
input domains have no behavioral-equivalence expectation. Matching thresholds
and labels are unchanged.

The independent original public-API oracle passes270,438 checks per context,
default/no-default × debug true/false, zero mismatches. Core implementation pins
were separately captured on Linux ARM/Linux x86/macOS ARM; native runtime and
natural admission were tested on Linux ARM. Foreign native CI is not claimed.
The complete positive Cargo fixture takes2.46s with the prebuilt driver (one
sample, no speedup claim). No new runtime hashing or serialized lookup keys;
source/archive semantic pins retain existing identities.

Evidence: `/tmp/rstyle-442-{comparison.json,oracle.json,selected-strict-summary.json,
all-strict.log,composition.log}`. Integrated gates and frozen inventory results
are recorded in `docs/rust-support-execution.md`.
