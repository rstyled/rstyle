# Original float kernels: acceptance contract v1

Status: **design only; not activated**. P02.D source audit, 2026-10-21. No
build, capture or runtime oracle has run. P01 integrates shared scalar vocabulary
first for conflict avoidance; character semantics are not a float prerequisite.

## Frozen originals and proposed build

Unchanged libm **0.2.16**, registry root
`/home/dev/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16`.
Select the complete original `math::k_sinf::k_sinf(f64) -> f32` and
`math::k_cosf::k_cosf(f64) -> f32`. No easier public delegates replace them.
Function spans below exclude their preceding cfg_attr; whole-file identities
retain it and all constants.

| Source | Bytes | Function SHA-256 | Whole-file SHA-256 |
| --- | --- | --- | --- |
| src/math/k_sinf.rs | 955..1125 | b95e9206a4193b0c1744468ff26cee36d160c48152bec1432074aee9b36551e9 | cf0e8bbe8d73a6704b7f1f710f5b9d31f190f1c633e894d4f69c4e3bc359eba3 |
| src/math/k_cosf.rs | 965..1120 | 9303ddcf967269bb7bfc8a4cabb99ec0e7d23461c55cd837602f472104731621 | 8f76089d014e217b96c9eca520271e64d467dc0d6542bf313dd70eb96c040f7f |

Additional SHA-256 identities:

- Cargo.toml: `a2d94f9cbaa4a177bacde9737bfcc7b2ed9de5bddadafe507923a5b5748f9bd7`
- Cargo.toml.orig: `5cef83cc9b297dcf1b6853237371d839a8c3ece4adb0e7fe9f7ba8958f319b0e`
- build.rs: `85dfa93693f40e282ecf2d9a3115280d8f508f056063862add9e529b8143b71b`
- configure.rs: `884fa9c05c600d03450b15651665848e88b09cf602f86c843e55e5e1ee0a4772`
- src/lib.rs: `d565c5f29ae9f502edca78d536436e85bbc4b955595328a6cfd4a97c1dbbac9b`
- src/math/mod.rs: `c4ae2e62996ac9d7d6fdc0f7cb57ac0a815e639374b24068516ae3f092b0c59e`

Predeclare first Cargo context: complete original lib/non-test, edition 2021,
default features (`arch`), native aarch64-unknown-linux-gnu,
nightly-2026-09-30 / rustc `5c543b0b8c73c7b72bc8284ced4fb22ead15734d`,
debug/unoptimized, overflow checks, panic unwind; `ENSURE_NO_PANIC` absent and
`assert_no_panic` inactive. Crate is no_std. Capture actual manifest/package ID,
unit identity, invocation, build-script cfgs and definitions before evaluation;
these are **not yet observed**. Follow with no-default-features and x86 Linux / ARM
macOS contexts only when authority is established. Do not assume cfgs from a
proposed command or equate cross-target capture with native execution.

## Complete known blocker chain

1. Capture and independently verify width-distinct f32/f64 interface, bindings and
   literals. Current primitive/cast contracts exclude floats. Carry exact binary
   literal values, preserving negative zero; do not serialize float values as
   ordinary JSON numbers. No f16/f128, float integers, or char/bool casts here.
2. Prove exact same-module f64 constants S1..S4/C0..C3, including decimal rounding
   and source unary negation. This is P03 dependency evidence, not permission for
   arbitrary const expressions or familiar constant names. Independently parse
   decimal text to a rational and round to binary64; compare rustc observations.
3. Ordered binary64 multiply/add and intermediate locals: k_sinf has eight
   multiplications/four additions; k_cosf seven/four. Preserve grouping and every
   rounding boundary. No reassociation, contraction/FMA or real-number algebraic
   matching. Integer overflow/division/panic hazards must not attach to floats.
4. Exact builtin f64-to-f32 conversion with ordinary round-to-nearest,
   ties-to-even finite narrowing, overflow to signed infinity and underflow
   including signed zero. Prove captured dispatch/types; integer `as` semantics
   do not supply this contract.
5. Existing cfg source ownership must prove inactive no_panic attribute and
   complete original modules/constants. General macro permission is not required
   by the two bodies, but neighboring macros/build cfg cannot be silently omitted.
6. Independent document verification, strict import, profile/policy isolation and
   matching must retain widths, constant bits, operand order and rounding sites.
   Older policies cannot acquire float support through permissive type parsing.

Unknowns: exact HIR constant/literal transport, semantic authority needed for
rounding/subnormal behavior per target, allowed NaN outcomes, and whether current
selected source correspondence already handles these conditional attributes.
No capture means this list is the complete **known** chain, not a claim that all
activation failures are discovered. No added analysis substrate/cache is proposed.

## Independent oracle design and semantic bounds

Implement a test-only integer/rational IEEE evaluator, separate from producer and
verifier code. Decode finite binary64 bits to exact signed dyadics; parse decimal
constant spellings as exact rationals; implement normalization, ties-to-even,
subnormal rounding and overflow independently. Evaluate the explicitly ordered
source operation graph with rounding after each binary64 operation, then narrow
once to binary32. Keep signed-zero rules and infinity/NaN cases explicit. Cross
check primitive rounder cases against separately fixed bit vectors before using
it for the kernel graph. A native copy of the polynomial or std sin/cos is not
this semantic oracle, and producer/verifier agreement is insufficient.

Predeclare exact finite bit expectations, not tolerances. Cover both zeros,
minimum/maximum subnormals, normal/subnormal boundaries, neighbors of powers of
two, midpoint ties with even/odd significands, narrowing overflow thresholds,
intermediate overflow, cancellation and positive/negative inputs. At ±0,
k_cosf returns +1. The source-ordered k_sinf expression returns +0 at both ±0
under ties-to-even: its `-0 + +0` is +0. Do not replace this with an assumed
odd-function signed-zero rule. Check original execution against these independent
expectations before accepting them as validated runtime evidence.

For infinities and quiet/signaling NaNs, require the Rust-authorized result set;
NaN payload/sign need not be reproducible. Do not require a single invented
canonical NaN, equate NaN with an ordinary scalar, or claim floating environment
flags/traps are modeled. Until the pinned Rust/target allowed set is established,
NaN-sensitive matching claims remain unavailable. Include NaN/±infinity inputs
and classification expectations; these polynomials produce NaN from ±infinity.

Freeze a reproducible sample: directed boundary vectors plus 4096 deterministic
binary64 bit patterns (generator/seed committed with oracle). Operation budgets
are the exact two acyclic graphs above, at most four constants, four locals and
one narrowing each; no arbitrary precision growth budget is permission to skip
an operation. Support the full input type, not only the trigonometric reduction
interval. Optional approximation checks against high-precision sin/cos on
[-pi/4, pi/4] are separate numerical-quality evidence, not extraction semantics
or a tolerance chosen after observing errors.

Adversarial documents: altered constant bit/source/value, changed width, omitted
rounding node, swapped/reassociated operation, forged FMA, integer hazard on float,
wrong narrowing result and historical-policy downgrade must fail after ordinary
integrity seals are recomputed. Repeated identical fixture functions should match.
Source-altered constants/grouping may still match under existing clone scoring;
such a match establishes no semantic equivalence or rewrite safety. Do not
require original sine and cosine to match. Evaluate all selected pairs; freeze
interface-gate and identical-fixture expectations before measuring them.

## Readiness and ownership

**Source contract ready; implementation not ready** until the lead freezes float
value/operation vocabulary, NaN/target authority and constant proof seams after
P01 integration. Proposed D-exclusive leaves: dedicated `float_scalars` producer
and verifier modules, `variant_capture/float_kernels.rs`, and a test-only rational
oracle under eval/coverage-development. Lead owns shared driver HIR/type
registration, concrete/data types, eligibility, hazards/lowering, constant
resolution, policy/admission and roadmap. P03 constant proof may be delegated
separately only with one owner and an agreed interface. No edits to those shared
files have been made. The frozen sixty remains unchanged; these originals form a
separate versioned development selection. Natural acceptance, strict replay,
matching, performance and quality gates remain pending.
