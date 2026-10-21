# Original float kernels: acceptance contract v1

Status: Data-v3 3.64 completes `k_sinf`/`k_cosf`; 3.65 adds original `k_cos`;
3.66 adds original `k_sin` with exact i32-controlled float alternatives.
Complete-libm admission rises **6→7/1131**, zero losses on unchanged source/build
identities. Strict replay and validation evidence appear at each milestone below.

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
attribute is inactive. Local module-owning `cfg_if!` invocations now have checked
source/expansion ownership. Later no-default/x86 Linux/ARM macOS contexts need their
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
Initial diagnostic selection: 14 admitted /14 excluded, 91 complete comparisons across
three targets × std/no_std; these are composition tests, not original acceptance.

Data-v3 3.63 retained all140 original source files but could not prove21
cfg-if-owned files and their descendants; both originals remained excluded.
Data-v3 3.64 independently checks the complete vendored cfg-if definition,
textual binding, first-true cfg selection and every compiler expansion stage.
Two emitted external-module declarations close the original inventory; ordinary
descendants keep ordinary ownership. Extra/missing/forged records, changed or
shadowed macros, unsupported outputs and unknown effects fail closed.

The exact original call-free float leaf must independently exhaust every source
dependency before ancestor namespace restrictions can be narrowed. Only primitive
float values, initialized immutable locals, exact owned literal constants and
registered ordered operations qualify. Local aliases/globs/expansions, calls,
nominal values, references, hidden drops and uncovered prefixes/suffixes reject.
General nominal import restrictions and historical policies remain unchanged.

Complete unchanged original capture: **5 admitted /1126 excluded /1131**, four
gains and zero losses against3.63. Both selected kernels strict-verify; their
single comparison completes and rejects structurally. Incidental `dec_digit` and
`hex_digit` gains reuse existing scalar Option/range semantics: source-audited,
strict-verified and exhaustively runtime-checked over256 bytes each. All five
original admissions independently pass document/producer verification and strict
admission. Full-library matching remains explicitly budget-limited at zero pairs.

Validation:976 tests /5 existing skips; doctests/fmt/all-target Clippy pass;
rstyle0/3174 functions, patterns0 unignored, existing ignore ledger unchanged.
The4202-input original numerical oracle passes. Frozen60 stays23 admitted /30
excluded /7 inactive, preserving identities/denominators. Complete package rosters
retain syntax39/956, lz4_flex8/150 and time-core11/226, zero gains/losses. All58
package admissions strict-verify; six selected comparisons complete. Retained run
`run-qqq32cfl` compares against3.63 `run-bgyl78v2`. Native three-platform CI for
3.64 passes Linux x86, Linux ARM and macOS (run39643046602).

Retained evidence: `/tmp/rstyle-float-kernels-cfg-if-final-capture.json`,
`/tmp/rstyle-float-cfg-if-natural-delta.json`,
`/tmp/rstyle-float-cfg-if-original-{strict.log,matching.json}` and
`/tmp/rstyle-float-cfg-if-{native-oracle,digits-oracle}.log`.
## Original cosine tail (Data-v3 3.65)

[Selection](../../eval/coverage-development/float-cos-tail-selection.json): unchanged
libm0.2.16 `math::k_cos::k_cos(f64,f64)->f64`, bytes2723..2973, SHA-256
`241cccd334d65622668b01aba6c01735e209c6825500be4263fa81f62fbfd99a`.
Same complete-original Cargo context as above. Baseline3.64 rejects ordered
subtraction and falls back to the ancestor source-boundary rejection. Four exact
binary64 subtractions, six constants and the two distinct saved `w` bindings now
compose through the exhaustive dependency proof; no broader source exception.
The new capability is required by facts and document policy independently.

The [independent oracle](../../eval/coverage-development/float-cos-tail-oracle.py)
rounds exact rational arithmetic after every source operation. All **4352 inputs**
pass against the [native harness](../../eval/coverage-development/float-cos-tail-native.rs)
including the unchanged complete original file:256 directed pairs +4096 frozen
random pairs. Finite/signed-zero expectations are exact. Allowed NaN fractions
are preferred or either original NaN input's unchanged/quieted fraction, either
sign. Both raw inputs meet in the late `x*y`; each can propagate to the final
addition with compatible shared-intermediate choices. This exact set proof is
specific to this graph and pinned targets, not arbitrary multiple-input DAGs.

Diagnostic composition: eight admitted/four excluded,28 complete comparisons,
three targets × std/no_std. Resealed operator, rounding, contraction, effect,
operand-order, constant and saved-binding mutations reject;3.64 policy remains
closed. f32 subtraction, division, comparison, unknown calls and hidden Drop
remain excluded. The earlier float fixture now admits15/rejects13,105 comparisons.

Retained evidence: `/tmp/rstyle-float-cos-tail-{natural-delta.json,oracle.log}`;
complete capture `/tmp/rstyle-float-cos-tail-natural-capture.json` and
`/tmp/rstyle-float-cos-tail-original-{strict.log,matching.json}`. Full-library matching remains
budget-limited. Next candidate `k_sin` needs a checked i32 branch condition and
exhaustive float-result alternatives; no additional control transport is yet
justified. Borrowed results, Result/?, general macros, float comparisons and
broader ownership remain outside this contract.


3.65 gates: **980 tests /5 skips**, doctests/fmt/Clippy pass; **rstyle0/3175,
patterns0 unignored**, existing ignores unchanged. Frozen60 stays23/30/7;
complete syntax39/956, lz4_flex8/150 and time-core11/226 have no gains/losses.
Run `run-6qkc6ea9` versus3.64 `run-qqq32cfl`; exact source/build identities and
all denominators match. All64 original admissions strict-verify; nine selected
comparisons complete. Fresh replay logs: `/tmp/rstyle-float-cos-tail-frozen-strict.log`.
Native3.65 CI passes all three runners (run39646408115).

## Original sine tail (Data-v3 3.66)

[Selection](../../eval/coverage-development/float-sin-tail-selection.json): unchanged
libm0.2.16 `math::k_sin::k_sin(f64,f64,i32)->f64`, bytes2170..2458, SHA-256
`7f326ba8cb49bfb91a3a261892f34325a8c92550c2e3ab685362b40aa0d4e1df`.
The 3.65 ancestor-boundary rejection concealed the complete dependency chain:
i32 formal/literal equality, six coefficients, ordered arithmetic and two float
alternatives. Capability `rust.closed-scalar-control-dependencies` closes that chain.

The call-free source proof permits an immutable bare i32 formal compared by `==`
to an in-range nonnegative integer literal, either operand order. Independent HIR
checks bind its exact operator, operand types/spans, literal spelling and formal
identity through the signature pattern. Both equal-width float alternatives and
nested guards must pass the existing complete source/value/return verification.
Calls, hidden Drop, unsupported operations even in unreachable branches, primitive
shadowing and missing alternatives stay closed. Earlier source-proof grammars do
not gain this permission. Resealed guard/binding/source/merge/return attacks reject.

Complete original Cargo capture: **6→7/1131**, exactly `k_sin`, zero losses;
all source/build identities match. All seven libm admissions strict-verify and six
selected original comparisons complete (structurally rejected). Identical diagnostic
helper callers match; three-target × std/no_std composition passes. Full-library
matching remains explicitly incomplete under the zero-pair budget.

The [independent rational oracle](../../eval/coverage-development/float-sin-tail-oracle.py)
passes **5384 original native inputs**:1288 directed +4096 frozen random triples.
When iy==0, y is never evaluated and cannot contribute a NaN payload. Otherwise
raw x/y unchanged or quieted fractions, either sign, or preferred NaN are permitted
by this specific DAG and pinned authority. Signed zeros are checked independently;
no mathematical sine approximation or valid-input precondition is assumed.

Retained evidence: `/tmp/rstyle-float-sin-tail-{natural-delta.json,oracle.log,
original-strict.log,original-matching.json,tests.log,style.json,patterns.log}`.

3.66 gates: **986 tests /5 existing skips**, doctests/fmt/Clippy pass;
**rstyle0/3186, patterns0 unignored**, existing15 ignores unchanged. Frozen60
stays23/30/7; complete syntax39/956, lz4_flex8/150 and time-core11/226 retain
exact source/build identities and denominators, zero gains/losses. Run
`run-008n2qyy` versus3.65 `run-6qkc6ea9`. All **65 original admissions**
strict-verify; **12 selected comparisons** complete across the package replays.
Native3.66 CI pending;3.65 passes all three runners. Next shared ownership frontier
is recorded in [the execution record](../rust-support-execution.md#resume-order).
