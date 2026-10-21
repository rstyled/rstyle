# Original float kernels: acceptance contract v1

Status: Data-v3 3.63 implements the bounded float foundation. Diagnostic
composition admits; the selected original family remains blocked by module
ownership proof. Full-original capture has zero gains and zero losses (1/1131).

## Original selection and context

Unchanged libm **0.2.16** `math::k_sinf::k_sinf(f64)->f32` and
`math::k_cosf::k_cosf(f64)->f32`. The versioned
[selection](../../eval/coverage-development/float-kernels-selection.json) freezes
function/file/manifest/build-script/root-module hashes, constant bits, numerical
expectations and sample identities before admission measurement. No easier public
delegates replace these private kernels.

| Original source | Function bytes | Function SHA-256 |
| --- | --- | --- |
| `src/math/k_sinf.rs` | 955..1125 | `b95e9206a4193b0c1744468ff26cee36d160c48152bec1432074aee9b36551e9` |
| `src/math/k_cosf.rs` | 965..1120 | `9303ddcf967269bb7bfc8a4cabb99ec0e7d23461c55cd837602f472104731621` |

Observed context: complete original no_std lib/non-test, edition2021,
default/arch features, native aarch64 Linux, nightly-2026-09-30 commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`, debug/unoptimized, overflow checks,
unwind, `ENSURE_NO_PANIC` absent. Actual cfgs, package/unit/definition identities and invocation are retained in
the selection: `arch_enabled` present, `assert_no_panic` absent. The conditional
attribute is inactive. Separate module-owning `cfg_if!` invocations remain a blocker. Later no-default/x86 Linux/ARM macOS contexts need their
own evidence. Frozen sixty identities/denominators remain unchanged.

## Complete known blocker chain and bounded semantics

1. Independently prove width-distinct standalone f32/f64 interfaces/locals and
   original literals. Driver `supported_literal`, HIR normalization and concrete
   observation excluded floats at the baseline. Preserve exact bit strings, negative
   zero and spelling; no JSON floating numbers, nested floats or scalar() widening.
2. Prove all eight same-module f64 constants S1..S4/C0..C3 from original literal
   and unary-minus declarations, rustc evaluation and independently rounded bits.
   The baseline constant validator was integer/bool-only. Reuse exact source
   and unit authority; no general const expressions, imports or familiar names.
3. Preserve source-ordered binary64 multiplication/addition and saved operands:
   k_sinf has eight multiplications/four additions, k_cosf seven/four. Round at
   each operation; no reassociation, FMA contraction or integer overflow hazards.
4. Prove the final builtin f64→f32 narrowing separately: finite ties-to-even,
   signed infinity on overflow, subnormals/underflow and signed zero. No float↔int,
   bool/char conversions, algebraic APIs, environment flags or traps in this slice.
5. Verify complete source/cfg ownership, exact operand/type/constant attributes,
   widths, rounding sites, document policy and strict admission independently.
   Historical policies must reject. Matching stays structural under unchanged
   scoring; neither numerical equivalence nor rewrite safety follows.

Implementation separates float narrowing from integer casts and restricts
arithmetic hazard propagation to exact integer types. Assertion hazard handling
remains independent of the unit result type. Constant declaration name/type checks do not prove value equality;
independently parse original decimal/unary-minus source and compare width-specific
bits. Existing ordered operand flow can be reused without a new inventory.

Original capture confirms the expected build-script cfgs and inactive
conditional-attribute ownership. `ENSURE_NO_PANIC` presence, even empty, enables
`assert_no_panic`; capture cfgs rather than assuming environment defaults. No new cache,
analysis substrate or generalized source exception is justified by this contract.

## Pinned NaN authority

Pinned `core/src/primitive_docs.rs:1215` (full SHA-256 and compiler identity in the
selection) defines preferred, quieting and unchanged NaN propagation. Arithmetic
sign is nondeterministic; aarch64/x86_64 have no extra payloads. Narrowing drops
low payload bits; unchanged signaling propagation cannot turn into infinity.

For these single-input, finite-constant graphs, a generated NaN permits preferred
f32 NaN with either sign. A NaN input additionally permits either sign of its
51-bit payload shifted right29 bits, quieted or with its original quiet bit if
the result remains NaN. Every path includes arithmetic. No node can invent another
payload; unchanged propagation along an input-to-output path realizes every
surviving original candidate, and preferred/quieting are available at the final
arithmetic/cast. Stable shared intermediates introduce no further restriction.
This exact final-set proof is specific to these graphs, not arbitrary multiple
NaN inputs or additional targets. No canonical payload/sign is invented.

## Independent executable expectations and evidence

The stdlib-only [rational oracle](../../eval/coverage-development/float-kernels-oracle.py)
uses exact Fraction decimal parsing, IEEE normalization/ties-to-even and rounding
after each source operation, then binary32 narrowing. Production parsing must be
independent. Fourteen fixed primitive vectors check ties, subnormal boundaries,
overflow and signed zero; seven independently fixed NaN-set cases check payload
truncation and sign symmetry. The sample has106 directed inputs plus4096 frozen
SplitMix64 patterns; the unchanged random-byte digest is in the selection.

At ±0 both kernels return positive zero/+1 respectively. k_sinf(±infinity) returns
corresponding signed infinity; k_cosf(±infinity) returns a preferred NaN. Native
unchanged-whole-file harness checks pass all **4202 inputs**, with exact bits for
non-NaNs and exact allowed-set membership for NaNs. This is no approximation test
and does not substitute for complete original Cargo capture or extraction proof.
Log: `/tmp/rstyle-float-kernels-native-oracle.log`.

Run `python3 eval/coverage-development/float-kernels-oracle.py --native PATH`.
The native JSONL protocol is documented in the script. Parent harness
`/tmp/rstyle-float-kernels-oracle/main.rs` includes the unchanged original whole
files; no copied polynomial or std sin/cos supplies expected results.

Baseline full-original capture completes all assessment/enumeration/evaluation
ledgers: **1 admitted /1130 excluded**, both selected kernels excluded. Rejections
confirm float types, arithmetic, unresolved constant reads and integer-only casts;
no additional source-boundary rejection was observed. Raw report and compact
audit paths are retained in the selection. This baseline predates literal/constant HIR bit transport and its independent checks.

The implementation connects standalone float interfaces/literals, exact owned
constants, binary64 addition/multiplication, f32/f64 sign negation and f64→f32
narrowing. It checks source order, saved values, bit widths, NaN interpretation,
policy and ordinary codegen; historical 3.62 and backend overrides reject.
Diagnostic selection: 14 admitted /14 excluded, 91 complete comparisons across
three targets × std/no_std; these are composition tests, not original acceptance.

Full-original measurement preserves all 1131 identities/build contexts:
**1 admitted /1130 excluded → unchanged**, both selected kernels excluded with
`Orphan source file prevents closed selected-unit proof`. The existing census
retains140 files, including20 generic files and the aarch64 architecture module
owned through local `cfg_if!` invocations. The ordinary module walker cannot prove
these declarations. No buffer is omitted and no macro permission is invented.
Next prerequisite: bounded first-match cfg-if module ownership, exact local macro
definition/invocation linkage and inactive/unknown evidence (P22/P05). Until then,
this foundation advances the float blocker chain without completing the family.
Evidence: `/tmp/rstyle-float-kernels-delivery-capture.json`,
`/tmp/rstyle-float-natural-delta.json` (zero changes). Strict original matching
remains pending admission; diagnostic strict verification is a separate result.

Integrated validation: 955 tests pass /5 existing skips; doctests, fmt and
workspace all-target Clippy pass; rstyle0/3119, patterns0 unignored. Frozen60 and
all three reproduced package selections have zero admission changes against
3.62 (`run-bgyl78v2` versus `run-15rr9we7`). Native three-platform CI is separate
from the local three-target capture/composition checks and remains pending push.
