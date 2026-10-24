# Resolved scalar constant patterns

Data-v3 extraction 3.4.0 adds `rust.scalar-constant-patterns`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Match arms may use same-unit integer/bool
constants, imported names and nongeneric inherent associated constants. They
compose with literal/range alternatives and scalar guards. Constant range bounds
are separate work. The final unguarded wildcard and existing arm-count, type,
control-flow and helper restrictions still apply.

Capture uses `pat_constant` with the existing scalar-constant resolution record
and its canonical value as the selector literal. Rustc resolves names and evaluates
initializers under the [constant-expression contract](scalar-constants-v1.md).
Independent admission checks declaration/name/type/value consistency, requires
the selector literal to equal the resolved value, and checks the scrutinee type.
It does not re-execute initializers. Nonlocal/generic/trait constants, floats,
aggregates and payload/binding patterns remain outside this slice.

Original-source inventory records qualified paths as constant patterns. Bare
identifiers are syntactically ambiguous between a binding and a constant, so the
inventory requires a corresponding `pat_binding` or `pat_constant` at the original
span. Semantic match admission then rejects bindings. OR alternatives are checked
individually. Earlier profile selections retain their existing inventory behavior.

Lowering uses one saved scrutinee and one `match.constant` selector for a standalone
constant arm. Mixed OR arms retain `match.or-patterns` with kind/value descriptors;
guard and body execution are not duplicated per alternative. Constant selection
has no runtime memory read. Definition edits change captured dependent evidence.
No frontend fields, provenance tables, source snapshots or caches are added.

Validation exercises imports, inherent constants, signed/bool/u128 values,
alternatives/guards, dependency edits, unsupported nonlocal constants and constant
range bounds, earlier-profile exclusion, and inconsistent selector/type rejection.
The unchanged frozen inventory measures natural coverage independently.
