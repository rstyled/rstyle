# Rust producer capability backlog

Status: implementation backlog for
[compat.rust.hir-r0/0.1.0](rust-extraction-profile.md).
All entries are currently **unimplemented for the new profile**. Legacy `rstyle`
code may extract related information; this registry makes no claim that it satisfies
these contracts. Fixture requirements below are designs, not executed tests.

Stable capability IDs should be referenced by future implementation checks and
structured diagnostics. This document does not implement those checks. Preserve IDs
when refining scope; new semantics may require profile versions or new IDs.

## Milestone R0 — agreed first supported subset

The producer is HIR-first and source-facing. Construct CFGs plus binding/def/use
facts, then derive PDGs. Operation extraction, CFG validity, dependence analysis,
projection and matching are separate acceptance boundaries.

| Capability ID | Target scope | Required distinguishing fixtures |
| --- | --- | --- |
| `rust.scalar-interface` | Primitive integer, boolean and unit inputs/outputs; ordered typed interface evidence | Empty input list, unit output, multiple parameters, rejection of unsupported types |
| `rust.local-bindings` | Local declarations, parameter/local reads, scope identity and shadowing | Same-spelling distinct bindings, nested scope, distinct definition IDs |
| `rust.local-writes` | Direct local assignment and compound assignment | Read after reassignment, repeated writes, compound read/write, unsupported indirect write |
| `rust.scalar-computation` | Built-in supported scalar operations/comparisons; no overloaded operators | Repeated operands, operand order, operator identity, normal-flow panic limitations |
| `rust.blocks` | Blocks, tail expressions and supported scalar expression results | Tail versus discarded result, nested block value, source/provenance mappings |
| `rust.branching` | `if`/`else`, branch outcomes, reaching-definition joins | Assignment in one/both arms, expression-valued branch, no invented definition |
| `rust.short-circuit` | Explicit conditional CFG paths for `&&` and `||` | Skipped RHS, nested conditions, no unconditional RHS execution |
| `rust.returns` | Explicit/early returns and implicit tail return under exit policy | Early return bypasses later operation, multiple returns share normal exit, unreachable source retained but excluded from matching |
| `rust.control-dependence` | CFG/postdominance-derived dependence for supported acyclic normal flow | Early-return condition controls subsequent operation; branch join not naively controlled by syntactic nesting |
| `rust.execution-dependence` | Nearest-dominating-operation prerequisites on the reachable acyclic normal-flow CFG; no full dominance closure | Straight-line independent operations, branch alternatives/join, short-circuit path, return bypass; operation correspondence/boundary fixtures required; loops need a separate policy |

### R0 strict eligibility

- Arbitrary calls, unsupported types/operations, references/dereferences and
  memory-backed writes are not supported in R0.
- Encountered unsupported constructs produce actionable capability diagnostics and
  make the function ineligible for conformant matching, rather than being deleted.
- Rejected constructs must include loops, `match`, `for`, closures, macros, async,
  unsafe and other deferred constructs until their applicable capabilities pass.
- Preserve evidence for unreachable source. The R0 profile requires strict
  scanning of unsupported constructs even in unreachable/short-circuited source;
  do not certify a supported remainder.
- Built-in integer operations may panic. Normal-flow extraction does not establish
  complete exceptional flow. Record relevant CFG/PDG limitations truthfully and
  check conformance against explicitly normal-flow requirements, not a claim of
  complete language-level execution coverage.
- Function eligibility must not depend on matching size/score thresholds. Report
  extraction exclusions separately from matching-policy exclusions.
- Each capability requires positive and negative fixtures, semantic validation,
  source mappings and visible unknown/unsupported evidence before implementation
  status can advance to verified.

## Milestone R1 — loops and bounded matching

| Capability ID | Deferred scope | Dependencies / acceptance fixtures |
| --- | --- | --- |
| `rust.loops` | `while`, `loop`, `break`, `continue`, loop-carried definitions | R0 CFG/def-use; zero iterations, back edges, break/continue, nested loops; no legacy graph/refinement caps |
| `rust.match` | Explicitly bounded scalar pattern/guard subset, to specify | Pattern binding and outcome policies; arm joins, unsupported patterns/guards, evaluation order |
| `rust.divergence` | Nontermination-sensitive dependence policy | Cyclic CFGs without fabricated executable exit edges; postdominance policy and terminating/nonterminating alternatives |

## Expansion ladder — calls and memory

| Capability ID | Deferred scope | Dependencies / acceptance fixtures |
| --- | --- | --- |
| `rust.call-summaries` | Selected calls with explicit, tested summaries | Argument/result dependencies, mutation/panic/nonreturn evidence; absent summary is not purity |
| `rust.abstract-memory` | Abstract locations, field/index accesses, indirect writes | Location identity, conservative alias sets, read/write dependencies and visible uncertainty |
| `rust.conservative-call-effects` | Calls interacting with abstract memory and mutable references | Abstract memory; may-write/reaching-definition effects, unknown targets/effects, conservatism limitations |
| `rust.precise-aliasing` | Progressively more precise alias analysis and summaries | Conservative baseline and declared supported scope; alias/nonalias and join cases; no universal exactness claim |
| `rust.for-iteration` | Iterator-based `for` | Call/trait-resolution and iteration policies; iterator effects, loop-carried state, desugaring provenance |

## Additional explicit backlog

| Capability ID | Deferred scope | Required policy / fixture direction |
| --- | --- | --- |
| `rust.closures` | Closure bodies/captures and invocation | Capture ordering, bindings, memory effects and closure identity |
| `rust.macros` | Expanded operations and generated functions | Source versus expansion identity; provenance and eligibility, no invented source spans |
| `rust.async` | Async operations and suspension | Suspension/resumption flow and source-facing lowering policy |
| `rust.unsafe` | Unsafe operations | Explicit operation, alias and unsupported-evidence policies |
| `rust.unwinding-cleanup` | Panic/unwinding, destructors and cleanup | Exceptional exits, drop effects and source/synthetic provenance; CFG schema extensions where necessary |
| `rust.nonreturning-calls` | Diverging calls and process termination | Resolution evidence, no fabricated fallthrough, termination versus divergence |
| `rust.unspecified-order` | All-orders overapproximation if required | Preserve every permitted ordering; disclose added paths; otherwise reject |
| `rust.additional-types` | Composite, floating-point, reference and other interface/value types | Type normalization, operation dispatch, layout/memory/drop boundaries; stage separately |

## Status discipline

Track each capability through `unimplemented`, `in_progress`, and `verified` with
implementation/test references and the profile version. Partial implementations
must describe their remaining boundary. No status advances merely because the
schema accepts emitted JSON or the code compiles.

Scoring, imported-graph fixtures and detector conformance can progress independently
of these frontend milestones. See the [matching profile](ccgraph-profile.md),
[extraction profile](rust-extraction-profile.md), [decision record](ccgraph-decisions.md)
and [schema README](README.md). The hand-authored
[extraction expectations](fixtures/rust-r0-extraction.json) are not compiler/analyzer
results or a complete certification suite; more fixtures remain required.

R0 deliberately also defers receiver syntax, function-level generic parameters,
non-Rust ABI/variadic signatures, uninitialized/destructured/wildcard bindings,
casts, nested items, explicit source borrows, globals and unit comparisons using
library traits. Unit values remain supported; all unit-typed value-operand edges are
omitted deterministically while unit binding reaching-definition edges remain.
See the extraction profile for the full closed supported-construct list.
