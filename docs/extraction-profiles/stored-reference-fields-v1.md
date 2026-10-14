# Stored shared-reference fields, version 1

Implementation baseline: `9306ff3`. This supplement extends only the separately
registered local `hir-data-v1` extraction and matching contracts. It does not
change pinned compat, historical scalar profiles, scoring or thresholds.
The later `lifetime-nominals-v1.md` local supplement supersedes only the initial
lifetime-parametric declaration exclusion below; stored-reference origin rules
and reference-bearing element exclusions remain unchanged.

The first slice supports non-generic concrete drop-free structs and tuples with
supported stored shared references, passed by value or through shared inputs.
Original field reads may return stored references or intermediate supported
struct/tuple values containing them. Ordinary supported conditional outputs retain
all alternative origins. Existing nominal source census, exact lexical type and
field identities, immutable referent boundary, compiler lifetime checks and build
compatibility remain required. Lifetime-parametric declarations remain unavailable
in this slice; source reference fields can use explicit `'static` lifetimes.

Loading a stored reference differs from borrowing field storage. A field read
retains its actual read effect and one base value operand. Its returned reference
origin is the field location followed by SharedReferent. Reference-bearing struct
values carry the union of recursively enumerated nominal field paths; tuple
values use ordered tuple projections. Stop enumeration at each shared reference:
the origin identifies that reference's referent, not every nested reference that
could subsequently be read through it. No disjointness is inferred from distinct
arguments or fields. All shared-bearing formal roots may alias, including self.

Independent frontend checks derive paths from verified formal types and field
observations. Independent serialized checks recover loaded-reference origins from
the actual read access, registered location evidence and exact serialized value
type; supplied origin annotations alone cannot certify the graph. Existing
serialized field verification separately checks that location against the original
source and resolved field identity. Return/alternative dependence paths must agree
with both proofs. There are no writes, destructor effects, added panic edges or
inlining. Projection remains OTHER with actual read reference counts and exact
nominal/shared-referent interface identity; lifetime spelling remains omitted.

Reference-bearing arrays/slices by value need element-origin evidence and remain
unavailable. Construction, local reference rebinding, mutable/interior-mutable
referents, arbitrary generic/trait dispatch, recursive type expansion and cyclic
reference-value correspondence remain unavailable. Taking `&` of a reference-
bearing field is still excluded by the separate borrow-place contract. Existing
finite type/source/field budgets apply without truncation. Missing, corrupt,
duplicate or inconsistent origin/location/type evidence fails closed. Real Cargo
fixtures must verify stored scalar/str/slice references, nested intermediate
values, conditional outputs, aliasing and tampered origins before admission.
