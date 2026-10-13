# Local data extraction and matching, version 1

Opt-in selection: `cargo rstyle clones --extraction-profile hir-data-v1`.
Extraction identity: `rstyle.rust.hir-data/1.0.0`.
Matching identity: `rstyle.ccgraph.data/1.0.0`.
These identities are separate from pinned compat, historical expanded, and helper
profiles. Registration must fail closed for any capability lacking its complete
proof. The existing compositional-data draft remains the broader delivery scope.

The first connected slice covers concrete scalar/tuple/fixed-array values, shared
parameters and shared outputs, scalar field projections on formal tuples, and
branching between shared inputs. Concrete nominal struct parameters, shared
receivers and nested formal field getters require the complete closed-source,
lexical declaration and drop-free proof registered in `nominal-data-v1.md`.
Shared borrows of verified formal field places are registered in
`shared-field-borrows-v1.md`; they preserve address operations without value loads.
Construction, dereference, local/temporary borrows, slicing and library operations
remain unavailable until their complete obligations are implemented. No successful observation authorizes these operations.

Data types use a deterministic compact JSON tagged tree for non-scalar interface
strings. Scalar tokens retain existing widths and signedness. Trees distinguish
shared reference/referent, tuple positions, array length, slice and str. Nominal
extension tokens retain opaque Cargo source, package/version/manifest, declaration
identities and ordered resolved field names/types; no
structural erasure or compiler debug strings are allowed. Lifetimes are excluded
from matching text but remain subject to original-source and origin verification.

For every formal and result, verify the exact original source spelling against
captured resolved trees. Literal array lengths only; unresolved aliases, generic
paths, mutable references, raw pointers and arbitrary const evaluation fail closed.
The finite grammar of scalars, tuples, literal arrays and shared references proves
its own drop-free immutable referent boundary recursively; nominal trees do not.
Every expression and binding retains its concrete resolved type evidence.

Source-backed tuple/struct field reads evaluate their base once, retain its value
operand, and name the exact formal location and ordered shared/tuple/nominal-field projections.
All shared formal roots may alias, including self aliases. There are no memory
writes, no claim of disjointness and no invented memory-summary node. Copying a
shared reference is a binding read, not a read of its referent. Borrowed outputs
must retain verified origins in shared formals, including all branch alternatives.
Unknown calls and unresolved reference origins exclude the entire function.

CFG, scalar reaching definitions and dependence use the existing analysis.
No exceptional transfers or executable exits are fabricated. Existing arithmetic
panic limitations remain explicit. Cyclic value merges remain unavailable until
an independently specified epoch-aware proof exists.

Matching retains the existing projection labels, nine numerical features,
comparison algorithm, sequential gates, directed WL, depth, weights, multiplicity
and exact scoring. Read counts distinguish scalar binding keys from location keys
(root formal binding plus exact path); a shared-reference binding read is counted
by its binding key. Memory summaries remain excluded. Pair compatibility requires
equal extraction/matching identities and compatible known overflow/debug-assertion/
panic settings; native widths anywhere in type trees require equal known widths.
Nominal identity is never aliased between packages or declarations.

Every source-to-pair function requires exact artifact/manifest verification,
independent frontend source checks, producer and independent serialized data
semantic checks, strict import and projection. Every distinct unordered admitted
pair is enumerated. Failed capture, arithmetic, budgets or verification are explicit
incomplete execution. This slice has no natural coverage or detector-quality claim.

The type-token text budget is 64 KiB per token. Capture must fail explicitly when
serialization exceeds it, even if the recursive observation tree fits its entry
budget. Reference-bearing by-value arrays lack element-origin evidence and remain
unavailable. The initial implementation also retains the blanket exclusion of
conditional value merges in loop-containing functions; separating merges outside
cycles from epoch-sensitive merges is an outstanding part of the broader goal.

The nominal source census is bounded to 256 local files and 8 MiB of original
bytes per selected unit. Capture resource failures abort the unit explicitly.
Nominal proof independently closes ordinary inline/external modules and rejects
trait impls, derive/attribute expansions, macros (including invocations in bodies),
unsafe item forms, conditional/path module attributes and unresolved ownership.
This first nominal boundary is deliberately conservative. Such items may exclude
nominal functions without excluding independently supported scalar functions in
the same unit. Construction and local reference operations remain unavailable. Formal field
borrows follow their separately specified source-place contract.
