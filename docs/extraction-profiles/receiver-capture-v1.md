# Resolved method receiver capture, version 1

Implementation baseline: `335c60f`. This prospective Data-only observational
supplement prepares `read-only-lengths-v1`; it does not authorize any new call.
Existing helper, expanded and R0 records and decisions remain unchanged.

For every original HIR method call capture its exact call and receiver identities,
ordered receiver adjustments, concrete unadjusted/adjusted receiver types, and
resolved instantiated function inputs/result. Use the registered concrete type
model, including explicit unsupported observations; never Rust debug strings as
type identity. Each adjustment has a closed semantic kind: builtin dereference,
shared borrow, mutable borrow, or unsupported. Unsupported conversions retain a
stable reason (overloaded/pin dereference, raw/pin borrow, pointer coercion,
generic reborrow or never coercion), and cannot become library-summary authority.

Records are source/HIR backed and per-invocation. Preserve adjustment order and
unsupported records rather than filtering to recognizable names. Concrete type
capture budgets apply to every recorded type; at most 128 adjustments per receiver
and 4096 method records per function, with explicit incomplete capture on budget
exhaustion. Missing/duplicate compiler correspondence is an evidence failure.
The existing resolved callee evidence supplies definition/core identity; no name
or caller purity claim authorizes a method.

Before length admission, independently check exact source call inventory,
receiver origin/type correspondence, allowed adjustment transitions, instantiated
signature, resolved library/core/build identity and serialized CALL semantics.
This observational capture does not assert effects, aliases, bounds or returning
behavior, and introduces no graph node or scoring change. The later registry must
reject all unsupported transitions, including array-to-slice coercion, additional
shared dereferences and owned/overloaded receivers. Shared references may alias.

Real compiler fixtures must check all four length definitions, shared reborrow
order, stored-field receivers, array coercion, overloaded receivers and legacy
absence. Mutated/missing facts must fail the future admission verifier; until it
exists these calls remain excluded. No historical inventory conclusion changes.
