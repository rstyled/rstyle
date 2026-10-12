# Local staged HIR coverage contracts

These contracts are local rstyle specifications. They do not amend or claim
approval from compat 04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f. Its unchanged
artifacts in vendor-compat/rust-r0 remain the authority for R0. Matching remains
compat.ccgraph.strict/0.1.0, including exact thresholds and exhaustive pairs.

The staged local identity is rstyle.rust.hir-coverage. Versions are immutable,
cumulative selections: 0.1.0 const bodies, 0.2.0 integer casts, 0.3.0 scalar
literal/wildcard match, 0.4.0 bounded guards, 0.5.0 conditional-source evidence,
0.6.0 scalar loops/divergence, 0.7.0 named primitive summaries. A version is
registered only when its producer, document verifier and strict admission checks
are implemented and independently tested. A planned version is not admission
permission. CLI selection and reporting must distinguish implemented from planned.

Base syntax, interfaces, bindings, snapshots, source-operation correspondence,
acyclic data/control/execution dependence and omitted exceptional flow follow
R0 unless a slice specifies otherwise. Existing compat.rust.hir-r0 namespaced
extensions retain their defined data vocabulary; a namespace is not a document
conformance claim. Every document/record/provenance policy must use its actual
selected local identity. Source/build/profile evidence must survive serialization
and exact manifest, frontend, producer and imported verification. Unknown or
unsupported source anywhere in the owned body rejects certification of a remainder.

Within one explicitly selected version and identical compatible semantic build
conditions, base-only and expanded functions may be compared: their common
category/dependence vocabulary has unchanged meaning. There is no cross-version
or mixed R0/local-policy admission in a single run. Existing target-width and
build compatibility checks must still pass. Constness is frontend evidence,
never a matching label. No new operation can bypass projection or compatibility.

## 0.1.0 — ordinary const function bodies

Capability rust.const-function-bodies. The pinned text explicitly excludes const
blocks but does not settle the meaning of "ordinary" for const signatures. No
external authoritative clarification is available here; therefore this is a local
expansion, not a claimed R0 correction. Analyze the ordinary source-facing body
of a safe synchronous const fn with integer/bool/unit interface using the same
supported base constructs as a non-const fn. Preserve signature.is_const, exact
signature/body spans and build settings. Do not execute compile-time evaluation,
substitute folded values, strip syntax, admit globals, nested const items or
const blocks. Const qualification adds no operation, binding, transfer, dependence
or matching label. Base unsupported constructs remain unsupported inside const fn.

Positive fixtures: scalar integer, bool and unit bodies, branching/writes/returns,
and equivalent non-const operation graphs. Negative fixtures: casts/calls/const
blocks/nested items/unreachable unsupported source. Adversarial fixtures: altered
constness/signature/source identity/build/profile and exact serialized manifests.
Historical lz4/time excerpts are disclosed regressions, never new heldouts.

## Subsequent slice obligations (unregistered until implemented)

Integer casts use one computation operation with operator as, operand at position
0 and normalized operand/result widths, retaining original expr_cast and source
span. Rust integer casts truncate low bits when narrowing, sign-extend signed
widening, zero-extend unsigned widening, and reinterpret same-width signedness.
Target-native widths require captured target layout. Float/pointer/reference/trait
conversions are excluded. Projection is OTHER; new nodes/dependencies can change
scores. Independent inventories cover width/sign boundaries, nesting, writes,
branches/returns and tampered resolved/correspondence facts.

Scalar match evaluates its integer/bool scrutinee once. Each literal decision and
wildcard selection retains match source operations, ordered outcomes and joins;
no source if normalization. Only literal/wildcard patterns are in the first stage.
Guards later use base scalar expressions; pattern failure skips guards and guard
failure goes to later arms. No destructuring/reference/or/binding/range patterns
are implicitly enabled. Arm-local ordinary lets/writes and expression merges
follow base binding/snapshot rules. Fixtures must specify exact transfers,
reaching definitions, merges and dependence, including early returns, repeated
patterns/guard fallthrough and unsupported unreachable arms. Macros stay excluded.

Conditional-source certification requires complete original-owned-source and
resolved-HIR correspondence under recorded cfg/features. Removed source cannot
be typed from token spelling. Independently resolved supported removed operations
may be certified; unresolved names/types/operations remain unknown. Unsupported
removed calls/macros/items always reject. Whole-function conditional absence must
be reported separately. Tests cover nested conditions, features, edits and missing
or inconsistent resolution evidence.

Loop registration requires a cyclic policy before code: fixed-point kill/gen
reaching definitions; local scalar state; while/loop with unlabelled, valueless
break/continue owned by the innermost loop. No iterator for or fabricated executable
exit. Dominance uses the real entry. Divergence-sensitive postdominance must account
for infinite paths (including mixed termination/divergence); any analysis-only
virtual sink must be explicitly separate from CFG execution. Value availability
and merges require fixed-point semantic checks, not acyclic path enumeration or
iteration caps. Nonconvergence/resource/overflow is failed/incomplete evidence.
Independent fixtures include zero/many iterations, snapshots, nested exits,
returns, divergence and tampered back edges/definition sets. Until justified,
cyclic admission remains excluded rather than reusing the R0 proof.

Call registration follows the frozen eligibility inventory. Each named versioned
summary pins resolved callee/type/signature, argument order/dependencies, scalar
result, memory-free effects, panic/overflow/settings, returning behavior and
provenance. Missing/mismatched facts reject. Names alone never resolve calls.
Call projects to STATE under the unchanged matching profile; no new WL labels.
Fixtures distinguish same-spelled user methods, neighbors, wrong versions/types,
settings and tampered effect/result/provenance. No arbitrary inlining or legacy
miner participation is permitted.

## Implemented cast wire facts

`hir-casts-v1` selects local version 0.2.0 and includes 0.1.0. Both documents
carry `rstyle.hir-coverage:integer-casts`, an exact inventory of computation-node
IDs, operand IDs/types, result types, target pointer width, source span and
`rust-integer-as/1` semantics. A cast is nonpanicking; unknown cast constants
remain unknown in the conservative arithmetic proof engine. This may leave a
later arithmetic panic bound unproved, never falsely discharged.

## Match wire/flow specification before registration

The initial bounded match subset requires a final unguarded wildcard, at most
16 arms, integer/bool scrutinee and result types in the base scalar domain.
Earlier arms have one literal or (in the guard slice) a guarded wildcard. No
bindings/or/ranges/paths/destructuring are admitted as patterns, including forms
that rustc lowers into related HIR. Negated integer literals are specified
individually as literal patterns; no arbitrary const-expression pattern support.

Evaluate the scrutinee once, keeping its snapshot. Each literal arm emits one
source-backed CONTROL operation at its pattern span: original kind pat_literal,
role match_pattern:<ordinal>, construct match.literal, constant literal evidence,
resolved pattern type retained in frontend evidence, and saved scrutinee value at
operand 0. True selects the arm, false proceeds to the next selector. A wildcard
emits a source-backed unconditional CONTROL selection, original kind pat_wild,
construct match.wildcard, then its body (or its guard when enabled). No source
if/else label is used. Guard evaluation consumes only a matched pattern path;
guard CONTROL consumes its already-evaluated bool at operand 0, construct
match.guard. Failed-pattern and failed-guard frontiers join before the next arm,
which is lowered exactly once. Earlier writes cannot cause scrutinee reevaluation.

Guards are limited to memory-free literals, binding reads, built-in base
unary/binary/short-circuit expressions, expression-only blocks and if expressions.
Declarations, assignments, returns, casts, calls and other constructs in guards
are excluded in this bounded version. Arm bodies retain ordinary base/selected
cast semantics, local declarations/writes, returns and nested supported branching.
All arms are scanned, including unreachable arms.

A match expression merge is analysis-synthetic, with one alternative per normally
completing nonunit arm, outcome arm:<ordinal> and operand position equal to that
ordinal. Its wire value site carries the namespaced selection kind
`rstyle.hir-coverage:selection-kind = ordered-match/1`. Alternative guards are
DNF over actual boolean selectors/guards, partition normally completed paths,
and retain exact completion frontiers and value snapshots. They do not pretend
one boolean predicate alone identifies a guarded arm. Base if/short-circuit merge
validation is unchanged. Nonunit single-completion matches reuse the saved value;
unit matches have no value-operand merge. Binding reaching definitions remain
fixed-point kill/gen over the actual acyclic CFG; data/control/execution rules
are unchanged. New CONTROL/merge nodes legitimately change matching features.

## Conditional correspondence implementation boundary

Local 0.5.0 independently reparses original owned function bytes solely to check
source inventory (never to extract matching graphs). Each semantic source entry
carries its exact byte span/kind and a unique resolved HIR reference, together
with source digest and captured cfg/features. The verifier recomputes this
inventory and correspondence rather than trusting source_scan_complete. Match
patterns retain distinct pattern entries; grouping/block wrappers add no runtime
operations. A unique original body-span match recovers pre-expansion scanning for
function headers with documentation/attributes; macro-generated source is excluded.

A conditional statement whose source entries all occur in active resolved HIR
can be complete. A removed operation has no independent resolution in this
implementation, even if its spelling is an integer literal; it remains unknown.
No standalone compilation under invented cfg environments is used as evidence.
Unsupported removed constructs are still rejected by the strict pre-expansion
scan. This deliberately defers admission of cfg-disabled supported statements;
resolution under removed lexical/type environments is a remaining obligation.
Whole-function absence stays visible in the frozen source inventory; active
functions retain actual cfg/features and file-relative identity. This is an
eligibility improvement, not complete conditional-compilation coverage.

## Initial cyclic slice contract (0.6.0)

The initial cycle slice is intentionally narrower than all scalar Rust loops.
It permits source while/loop, local scalar state, unlabelled valueless break and
continue resolved to the innermost loop, nested loops and early return. Iterator
for, labels, value-carrying break and nonunit conditional/short-circuit/match
value merges anywhere in a loop-containing function remain excluded in this
version. This excludes loop-epoch-sensitive value merges, rather than treating
acyclic guard histories as valid across iterations. Ordinary scalar snapshots
within each iteration and unit control branching are supported. Condition
operations are reevaluated on every while back edge; loop headers and break/
continue retain distinct source operations. A while false edge and actual break
frontiers reach the next operation; an infinite loop adds no fallthrough edge.

Reaching definitions use monotone finite kill/gen fixed points over actual CFG
edges, including zero-iteration, back-edge and continue paths. Dominance is the
entry-rooted greatest fixed point on the finite reachable vertex set. For each
candidate operation z, divergence-sensitive postdominance is the *least* fixed
point starting with {z}, adding a vertex only when it has nonempty successors
and all successors are already in the set. Thus every maximal finite or infinite
path from that vertex must encounter z. Exitless cycles that can avoid z never
receive fabricated exit reachability. Mutual postdominance in unavoidable cycles
is permitted; no acyclic immediate-postdominator chain is followed. An outcome
edge x->y induces control dependence on each real operation in post(y) excluding
strict postdominators of x. Self control dependence is retained when justified.
Execution dependence remains the nearest strict entry dominator. These meanings
remain control/data/execution under unchanged matching category rules.

The serialized cycle verifier independently computes CFG dominance and kill/gen
fixed points. Value operands must be formal inputs or strictly dominating real
operations; merges in loop-containing functions are excluded. Each read's
reaching-definition edges must agree exactly with the computed definition set.
Convergence follows finite monotone sets; there are no path/iteration samples or
silent caps. Explicit resource/arithmetic failures must remain failed evidence.
Normal exit boundaries may be unreachable; only actual returns/fallthrough have
execution transfers. Cycle policies and actual loop annotations must be recorded
in both documents and checked against captured source/HIR/build facts.

## Frozen primitive-summary choice

Source review of the frozen inventory identifies both i32::unsigned_abs followed
by u32::wrapping_mul and i64::unsigned_abs followed by u64::wrapping_mul in the
selected time-core is_leap_year definitions. Freeze only these four instantiations
for the initial summary set, before admitting or inspecting detector predictions.
No other primitive method, free call, user method or trait dispatch is enabled.

Summary identities are rstyle.primitive.unsigned-abs/1 (i32->u32, i64->u64) and
rstyle.primitive.wrapping-mul/1 (u32,u32->u32, u64,u64->u64). Resolve exact core
inherent callee paths on pinned rustc commit 5c543b0b8c73c7b72bc8284ced4fb22ead15734d;
record that compiler/source provenance. Receiver and then arguments evaluate in
Rust order and feed result operands 0 and 1 as applicable. Both operations are
memory-free, nonmutating, always returning and nonpanicking under either overflow
setting. unsigned_abs returns the magnitude including signed MIN; wrapping_mul
returns low width bits modulo 2^width. No call is inferred from spelling alone.
The call is a source-backed CALL/STATE node, retaining original expr_method_call,
callee, method form, resolved argument/result types and summary facts. Effects,
returning behavior, callee/type/summary/commit and settings must match exactly.

Source inventory correspondence permits exact spans or surrounding-source-
parentheses/1: a HIR span may enclose the parsed semantic span only with balanced
parenthesis tokens and whitespace surrounding it. This handles rustc retaining
parenthesized spans. The original HIR span and source bytes remain unchanged;
no computation, cast, read, label or dependency is normalized or removed.
