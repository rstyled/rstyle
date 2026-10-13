# Data source type correspondence, version 1

This contract supplements the observational concrete type contract. It does not
register extraction or authorize admission under any existing profile.

The independent source verifier accepts primitive integer/boolean/unit spellings,
parenthesized/grouped types, tuples, shared references (lifetime spelling is
ignored), slices, str, and fixed arrays with literal integer lengths. It checks
width, signedness, arity, mutability and length against the resolved type tree.

A named path requires separately supplied resolution evidence. The resolver must
bind the exact source path in its owning declaration and compilation context to
the complete resolved type tree. A matching final path segment, declaration name,
or compiler display string is insufficient. Missing resolution fails closed.
The verifier never evaluates const expressions, follows aliases by spelling,
accepts generic path arguments, or treats observational success as source proof.

Every recursively nested struct field is checked from its exact declared source
bytes. The existing concrete type depth/entry/array/field/source budgets apply
before recursion. Raw pointers, mutable references, trait objects, inferred types,
function pointers and nonliteral array lengths are unavailable. This establishes
source/type correspondence only; drop, immutable referent, origins, effects,
locations and producer/document verification remain separate obligations.
