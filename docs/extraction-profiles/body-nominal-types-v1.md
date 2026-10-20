# Nominal types used only in function bodies

Data-v3 extraction 3.25.0 adds `rust.body-nominal-types`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Supported concrete struct types used inside a
function no longer need a corresponding type in that function's signature.
Scalar-returning construction/projection, immutable local values, imported
constructor aliases, tuple/unit structs, nested construction, conditional values,
reference-bearing updates and `Self` construction compose within the existing
closed nominal source boundary.

Capture already observes concrete body tokens and original struct declarations.
Previously it discarded declaration entries absent from interface observations.
It now retains entries needed by successfully observed body types, using the same
declaration-origin records and original file ranges. Unused entries from incomplete
type observations are still removed. Callable parameter/result observations are
unchanged. No additional frontend record, provenance format, source snapshot or
retained cache is introduced.

A temporary declaration inventory reconstructs nominal type trees from existing
canonical body tokens and exact original declaration ranges. Field names, order,
source spelling and recursively observed component types are checked. Repeated
observations of one nominal type must agree. Existing type depth, entry, field and
source-size budgets still apply; arrays are not expanded by element count.
The temporary inventory uses the existing type-observation validation shape; its
added entries are never callable parameters or serialized interface changes.

The same inventory feeds Cargo package binding and independent lexical resolution.
Existing original-file checks, exact struct ranges, declaration digests, selected
package/unit identity, source census, lexical imports, field-type spelling and
nominal owner checks remain required. Constructors, field projections, locals,
reference origins and graph operands keep their existing semantic verification.
Functions whose declaration origins are empty or already covered by the signature
keep the path that avoids body reconstruction. Earlier profiles retain their
signature-only declaration boundary.

The composed Cargo fixture uses scalar/shared-only signatures with imported aliases,
nested and tuple constructors, immutable locals, updates, conditional values,
reference projections, scalar projections, a scalar helper using body-only construction, unused unit locals and `Self` construction
inside an inherent implementation. All admitted graphs pass independent document
and strict producer verification, and equivalent functions reach accepted matching.
Changed declaration identities, ranges and body field layouts are rejected.
An earlier fixture's body-only reference constructor now admits. Data-v2 retains
its construction/projection exclusions.

This does not admit arbitrary mixed-content crates. Existing restrictions on enums,
traits, type/const generics, derives/macros, conditional modules and ownership/drop
remain. Local storage borrows and mutable reference-bearing locals remain excluded.
Static associated helper calls still lack the shared-receiver ownership contract
used by existing inherent helper certification; admitting an associated function as
a root does not establish that call boundary. S09 remains Partial. Validation and
frozen natural inventory measurements are recorded in the roadmap continuation log.
