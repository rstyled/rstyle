# Nominal data proof, version 1

Implementation baseline: `e6eca47`, clean `ccgraph/read-only-api-coverage`.
This is a registered local supplement to `hir-data-v1`, not pinned compat/R0 or
historical helper admission. The broader compositional-data goal remains intact.

The driver receives an explicit typed selected-unit context before rustc arguments:
Cargo's frozen package snapshot, root source path and observed crate name. No
RSTYLE environment variable selects or authorizes this proof. The handoff checks
that context against the frozen plan and exact observed compiler invocation before
producer/imported admission. Context transport has a 64 KiB budget; failure is
incomplete capture, never an excluded or certified remainder.

Nominal identity additionally retains the opaque Cargo package source identity.
Name/version/manifest/declaration spelling alone cannot collapse two separate
package namespaces. Historical observational nominal records omit this new field.
Data records require it. Reference lifetime text remains excluded from matching;
nominal identity, scalar widths, array lengths and field identities are preserved.

Capture the compiler's original buffers for local unit source files, including
leading BOM and CRLF offsets. A closed source census binds each absolute path,
exact bytes/digest, selected root and compilation context. Budget: 256 files,
8 MiB combined original bytes. Independently follow ordinary inline/external
modules from the selected root; an external module must have exactly one captured
ordinary source candidate. Missing, duplicate, orphan, escaping or ambiguous source
files cannot prove a closed unit. Path overrides and conditional module selection
need separately resolved presence evidence; until then they are unavailable.

Build a lexical source namespace from the closed census. Resolve concrete struct
paths through their actual modules and named imports/aliases; no last-segment or
structural-name heuristic is sufficient. Bind each captured declaration to its
exact original item, source span and nominal digest. Check each field spelling
recursively against its resolved tree, including nested nominal paths. Type aliases,
generic types, arbitrary const lengths and unresolved dispatch remain unavailable.
Shared receivers require exact ownership in the source impl, not a `Self` name
heuristic or interval containment over arbitrary nested definitions.

The first independently verified drop-free boundary accepts concrete structs with
recursively supported scalar/tuple/literal-array/shared/slice/str fields. The closed
unit must contain no trait impls, derive expansions, item macros, unsafe item forms
or custom lang-item declarations that could conceal unrepresented ownership/drop
behavior. Inherent impls are allowed. Functions within ordinary inherent impls may
use shared receivers and read nested formal fields. This conservative availability
restriction is explicit; lifting it requires complete resolved trait/expansion
provenance, not trusting a supplied purity or `no_drop` flag. Drop-bearing,
interior-mutable, raw-pointer, mutable-reference and external/generic values stay
excluded. The compiler's destructor query is additional capture evidence, never
this independent proof's substitute.

Nominal field reads retain existing formal-location semantics and alias uncertainty.
Serialized verification must independently validate namespace/source/drop proofs,
interface tokens, receiver ownership, exact field reads and borrowed origins.
Matching remains the separately registered data extension, with the same algorithm,
features, gates, thresholds and scores. No memory-summary or source normalization
is introduced. Nominal structs are not admitted until this complete connected path
passes independent and real Cargo tests.
