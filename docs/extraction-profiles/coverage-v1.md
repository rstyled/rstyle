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
