# Same-error Result propagation

Data-v3 3.79 connects the original
[LookSet::available chain](../../eval/coverage-development/result-propagation-selection.json)
to fresh `Result<(), E>` helpers. The success type is unit; `E` remains an exact
source-owned nongeneric `struct E(())` with independently verified absence of
Drop. Fresh Result method results can also be forwarded whole. Both extensions
require the new policy; earlier profiles retain their boundaries.

The original `?` and complete operand must correspond to retained compiler HIR.
Supplemental observations identify the concrete function-item substitutions,
selected Try and FromResidual implementations, reflexive From implementation,
and exact ControlFlow variants. Compiler, target, core identities and reviewed
source digests are pinned independently on the three supported targets. The
pinned compiler uses `Result<!, E>` as its residual. Generated HIR stays in the
evidence; permission does not extend to arbitrary ControlFlow or Never values.

The graph evaluates the original operand once and saves its complete Result.
A borrowed tag inspection selects two exclusive paths: Err returns that same
whole Result through the proved identity conversion; Ok consumes the saved
Result and yields unit. No source payload binding or general move permission is
inferred. Independent document checks verify the saved value, exact consumers,
effects, guard reachability, return-to-exit transfer and absence of fallthrough.
Guard checks share CFG adjacency and operation indexes within a function instead
of rescanning every edge at every visited vertex.

Cargo tests exercise original helpers, alpha peers, method forwarding, sequential
and nested `?`. Stored Results, other success types, Option propagation and
nonidentity error conversions stay excluded. Mutations target implementation
authority, substitutions, bindings, operand evaluation, consumers, effects,
guards, exits and historical policies. Graph attacks call the independent
semantic verifier directly before strict checks, so stale seals cannot explain
their semantic rejection.

General Result payload matching, references in errors, owned storage, cleanup,
custom Try/residual conversion and iterator state remain separate work. Native
unchanged-library execution supplies the independent bitmask oracle; cross-target
Cargo capture, strict replay and matching do not claim native execution there.
