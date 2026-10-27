# Resolved scalar constant expressions

Data-v3 extraction 3.3.0 adds `rust.resolved-scalar-constants`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Expression reads may resolve same-unit named
constants and nongeneric inherent associated constants with primitive integer,
bool or unit types. Imports and aliases of the constant name are supported.
This is a partial V12 slice; constant patterns and range endpoints are separate.

Capture evaluates only referenced constants through rustc's cached const evaluator.
Initializers may contain computations, including const-function calls. Rustc owns
initializer execution: the independent checker does not implement a second const
interpreter. The existing path-resolution description stores the definition name,
original declaration, normalized scalar type and canonical value. No additional
source snapshots, provenance tables or caches are introduced.

Independent admission checks the expression shape, resolved type, declaration
name and primitive type annotation, canonical value and integer bounds. Native
integers require target pointer width. Lowering creates a typed `constant.value`
computation without a runtime memory read or binding symbol. The resolution record
participates in captured function evidence, so edits to a referenced constant
change the dependent function evidence even when its expression source is stable.
Existing helper admission and dependency validation continue to apply.

Nonlocal definitions, trait associated constants, generic definitions, expanded
declarations, statics, floats, aggregates and references remain unavailable.
Source type aliases and qualified primitive annotations are outside the declaration
checker. Existing arithmetic panic and helper restrictions still apply.

Validation covers signed and unsigned extremes, bool/unit, imports, inherent
constants, constant edits through helper callers, earlier-profile exclusion,
unsupported statics/generics, malformed values and declaration/type mismatches.
Natural coverage is measured separately using the unchanged frozen inventory.
