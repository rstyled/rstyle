# Field reads through shared-reference locals

Data-v3 extraction 3.21.0 adds `rust.shared-local-field-reads`, composing with
[immutable shared-reference locals](shared-reference-locals-v1.md). Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. A shared-reference local with one supported formal
referent can serve as a field base. Tuple/struct members, grouping, nested field
chains, shared-reference field results and existing supported formal-field borrows
compose with the alias. This does not create local storage locations or authorize
borrowing a local value's storage.

Capture follows existing immutable binding identities and original initializers.
Supported initializers are formal references, earlier aliases, stored reference
fields, supported formal-field borrows, blocks and conditionals whose alternatives
have the same referent. Exact shared/referent type equality is required at each
step. A stored reference field contributes its referent projection; a borrowed
formal field already names the referent and contributes no synthetic dereference.
Consecutive implicit dereferences retain their existing evidence limitation.

The independent source verifier begins with the original field syntax, checks its
local binding identity and reconstructs the existing reference origins. Exactly one
origin is required. It walks the original formal concrete type along that origin,
checks the local shared-reference type, then resolves the original source members
against concrete declaration identities and positions. The resulting location must
match capture, including the compiler field position/owner and result type. Ordinary
formal roots retain their existing source mutation/shadowing checks and capture path.

Lowering uses the existing field operation, formal memory location and one immediate
base operand. The base is the current local binding read, with its existing reaching
definition and original initializer dependence; no initializer is reevaluated by the
field operation. Nested places and stored reference loads preserve their existing
roles. No new observation format, producer operation, provenance, source snapshot,
origin annotation or retained cache is introduced.

The real Cargo fixture checks aliases and chains, grouping, shadowing and saved
aliases, nested fields, stored references, formal-field borrows, same-origin
conditional initializers, block-local aliases, reference-valued results, borrows
through aliases and unit fields. Every admitted pair passes independent document
and strict producer verification. Equivalent functions reach accepted evaluated
matching with score 1. Wrong same-typed formal roots in captured and serialized
locations fail verification. The borrowed-alias location explicitly retains the
original formal referent, inner field and result field without another dereference.
Data-v2 keeps its local-field exclusions.

Multiple distinct possible origins require a read model with alternative locations;
the existing single-location representation does not admit them. Mutable aliases and
reference reassignment, reference-bearing aggregate locals, helper-result field
bases, explicit/unavailable implicit dereferences and borrows of local/temporary
storage remain unfinished. D12 stays Partial and D13 remains separate. Frozen
inventory and final validation are recorded in the roadmap continuation log; the
full roadmap remains active.
