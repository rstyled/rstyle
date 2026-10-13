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
Stored shared-reference fields and reference-bearing struct/tuple values require
the origin proof in `stored-reference-fields-v1.md`. Field-value origins include
the stored reference's referent projection, unlike field-storage borrows.
Lifetime-only nominal declarations/impls additionally require
`lifetime-nominals-v1.md`; type/const substitution remains unavailable.
The semantic correction in `field-value-context-v1.md` distinguishes aggregate
places from terminal value reads and required stored-reference loads. Consecutive
implicit dereferences lacking referent-storage read evidence remain unavailable.
The four read-only slice/str len/is_empty summaries are connected under
`read-only-lengths-v1.md` and `read-only-length-registry-v1.md`, initially for the
registered aarch64 pinned core build. Grouped explicit field-borrow receivers use
`borrow-receiver-grouping-v1.md`. Construction, explicit dereference, local/
temporary borrows, indexing/subslicing and other library operations remain
unavailable. Observation alone never authorizes an operation.

Data types use a deterministic compact JSON tagged tree for non-scalar interface
strings. Scalar tokens retain existing widths and signedness. Trees distinguish
shared reference/referent, tuple positions, array length, slice and str. Nominal
extension tokens retain opaque Cargo source, package/version/manifest, declaration
identities and ordered resolved field names/types; no
structural erasure or compiler debug strings are allowed. Lifetimes are excluded
from matching text but remain subject to original-source and origin verification.

For every formal and result, verify the exact original source spelling against
captured resolved trees. Literal array lengths only; unresolved aliases, type/const generic
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
The prospective reference-aware helper/method boundary is frozen separately in
`read-only-helper-calls-v1.md`; it remains unavailable until its read-location and
return-origin verification is connected. Existing scalar helper admission does
not authorize memory-reading callees.

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

Method receiver/signature observations are specified separately in
[receiver-capture-v1.md](receiver-capture-v1.md). They record compiler adjustment
order and concrete types without authorizing any additional method or library
summary. The four named length/emptiness calls now require the separately
registered core artifact/signature/effect authority and independent serialized
CALL checks; other library methods remain unavailable.

[receiver-verification-v1.md](receiver-verification-v1.md) independently checks
receiver inventories, source/HIR identity, concrete types and shared transitions
before real Cargo assessment and on both serialized frontend records. Current
Data handoffs require the inventory even when empty; missing or corrupt records
are incomplete execution. Existing registered scalar signatures are checked
against their authority; unregistered method signatures do not authorize calls.
