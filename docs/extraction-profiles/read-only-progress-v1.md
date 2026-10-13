# Read-only API implementation continuation

Implementation baseline: `2d67b260edfbf3cd66641a4baa90981aa74983de`
(merged `origin/main`, clean working tree). Branch:
`ccgraph/read-only-api-coverage`. The merged scalar helper, inventory repair,
concrete type observation and explicit driver-selection changes are retained.
Pinned compat authority remains `04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f`.
No vendored artifact, existing profile or matching threshold changes are authorized.

The next prerequisite is independent validation of the concrete field type
boundary. A field declaration must agree recursively with resolved tuples,
arrays, shared references, slices and str, not only direct scalar spellings.
Mutable references, raw pointers, executable array-length expressions and
unsupported type syntax cannot establish this boundary. Qualified nominal
paths require separate captured definition resolution; terminal display names
alone do not prove identity. Existing observations remain observations and
cannot authorize data admission.

Subsequent slices must bind formal referents and resolved field paths, verify
reads and borrowed-output origins, then connect data lowering and a separately
registered matching extension through the real Cargo command. The draft
compositional-data contract remains unavailable for admission until those
boundaries and independent serialized verification exist.
