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

## Formal field-location observations, version 1

Identity: `rstyle.rust.formal-field-observation/1`. Transport is the optional
`formal_field_observations` frontend option for already excluded concrete
interfaces under helper capture. This identity is not an extraction or matching
profile and does not admit any function.

The captured syntax is a named formal parameter or shared receiver followed by
one or more resolved struct/tuple fields. Shared-reference autodereferences are
recorded in the path; overloaded dereference, local aliases/rebindings, indexing,
raw/mutable pointers and unavailable field/type evidence are unsupported. Each
field operation retains its exact HIR identity, source span/snippet, formal
parameter position, declaring nominal identity (or concrete tuple position),
field position/name, and resolved output type. Nested field operations are retained
individually in base-before-projection order; branch alternatives remain in source
order without claiming that every branch executes. All captured shared formal roots may alias; no disjointness is asserted.
Unsupported formal types leave the overall referent/alias boundary unavailable.
A captured location is a candidate observation, not a verified origin certificate.

An independent source parser verifies the complete owned field inventory, exact
navigation, source member names, root names and unambiguous formal binding
correspondence. Source-local shadowing, assignments to formal roots, mutable borrows of roots,
and arbitrary macros prevent a formal-only proof. Each path is
checked against captured concrete interface trees and compiler-resolved field
positions/owners, preserving scalar widths and reference referents. Missing,
duplicate, corrupt or unsupported observations fail verification. Original
BOM/CRLF offsets remain function-relative; no normalized source is used.

These facts do not yet establish aggregate construction, borrowed-output origin,
read effects through arbitrary calls or document/projection evidence. CFG and
value/dependence lowering remains the existing classifier boundary, so data
functions remain excluded. The observation budget is 4096 field operations and
8 MiB serialized evidence; exhaustion fails capture explicitly. Unsupported
syntax retains a reason and cannot yield a partial certified inventory.

## Validation of the location prerequisite

`cargo test --workspace --locked` passed 528 tests in 25 suites with none ignored.
This includes real Cargo capture of receiver/nested fields, shared reference
chains, stored shared fields, concrete tuple paths, Unicode source navigation,
BOM/CRLF sources and same-owner versus nested-owner inventories. Negative cases
retain unsupported local aliases, shadowing, formal rebindings, mutable borrows,
macro-hidden rebindings, overloaded dereference and cfg-removed source fields.
Independent checks reject source/identity/type/path/order/alias tampering.
Separate real Cargo tests exceed the operation and serialized-byte budgets and
require incomplete capture, assessment and pair evaluation.

Formatting and workspace Clippy with warnings denied passed. Repository-built
`cargo rstyle --format json` reported zero violations across 1414 functions;
`cargo rstyle patterns --format json` reported zero candidates. The pinned Python
inventory tests passed 11 tests and scripting tests passed 43 tests. The first
plain-Python scripting invocation lacked Cargo on PATH and failed; rerunning with
Cargo available and the pinned Python dependency environment passed.

An initial expanded negative fixture used incompatible elided lifetimes and
failed real compilation. Giving its reassignment inputs explicit static lifetimes
made the fixture valid Rust; the final full regression run passed. Failed logs
remain under target alongside the successful validation logs.

There are no new data admissions or coverage measurements in this prerequisite.
All data interfaces in the real command fixtures remain excluded. Nominal
package/build binding, complete effect/drop-free verification, borrowed-output
origin/value correspondence, data document lowering and independently verified
matching-extension admission remain implementation obligations, not external
schema-authority blockers. Assertions, broader scalar patterns/constants,
cross-unit summaries, slice/string operations and the preregistered broader
inventory also remain outstanding under the original compositional scope.

## Original declaration origins

`rstyle.rust.declaration-origin/1` is a separate observational evidence identity,
serialized as `concrete_declaration_origins` in frontend options. Existing concrete
type observation semantics and extraction/matching profiles are unchanged. The
inventory contains exactly one origin per distinct nominal definition path in
successfully captured interface trees, including nested fields. Unsupported trees
do not leave partially captured declarations in this inventory. Conflicting
observations of one definition path fail capture.

Each origin carries the existing nominal observation, compiler source-file name,
exact original compiler-file bytes and digest, and the original file-relative
struct-item byte interval. Capture uses the compiler buffer and normalization map,
not a filesystem reread. The independent verifier checks complete inventory,
uniqueness, nominal agreement, file digest, exact declaration bytes, and an exact
struct-item interval independently parsed from the full source file. A matching
string inside a comment or string literal is insufficient. Source files with
unparseable or unmatched declaration boundaries fail capture; no interval
containment fallback is used. Serialized origins have an 8 MiB per-function
budget; exhaustion is an explicit capture failure.

This evidence does **not** authenticate a source-file name against Cargo's planned
unit inventory, bind the manifest digest to exact manifest payload/package source,
resolve named aliases, or certify destructor/interior-mutability/effect facts.
It is an original-byte location prerequisite, not a package identity or data
admission certificate. These remaining checks must be connected before a data
profile can admit functions. No additional pairs or coverage gains are claimed.

Validation for this origin slice: `cargo test --workspace --locked` passed 529
tests with zero failed or ignored, including all 76 real-command tests.
`cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
Repository-built `cargo rstyle --format json` reported 1423 functions and zero
violations; `cargo rstyle patterns --format json` reported zero candidates.
The independent origin fixture covers original BOM/CRLF offsets and rejects
changed identities, missing/duplicate entries, file digests, ranges, nominal
observations and declaration text placed inside a comment. The real receiver
fixture verifies the serialized origins retrieved from the command report.

Initial validation exposed a removed-BOM offset mismatch and an independent
parser first-line BOM discrepancy; capture now restores the prefix and the
verifier adds its exact byte length to spans parsed without it. The corrected
real fixture and full workspace run pass. Initial Clippy/style violations were
resolved by extracting cohesive capture and test helpers. Failed and successful
logs remain under `target/declaration-origin-*`. Inventory and study scripts were
unchanged; their previously recorded validation was not rerun for this slice.

## Selected package and compilation-unit binding

The next slice starts from `1eb5772` with a clean working tree and freezes its
contract in `02eec13` before implementation. See
[nominal-unit-binding-v1.md](nominal-unit-binding-v1.md) for the versioned authority
and negative boundary. No extraction profile, matching profile or vendored
artifact was broadened.

Opt-in helper capture now freezes exact selected manifest payloads and Cargo's
resolved package IDs/names/versions in the plan. The observed target ledger must
agree with that snapshot. After exact artifact verification and frontend
invocation checks, the handoff independently verifies serialized declaration
origins against the selected package and checks consistent original file bytes
across the whole captured unit. Function intervals in those files must match
exact original function bytes and digests. Source names are absolute; outside-root
and contradictory observations fail handoff explicitly. No source reread, evidence
cache or compiler-order dependence was introduced.

Function reports include `rstyle.rust.nominal-unit-binding/1` for observed concrete
interfaces, with selected package identity, manifest digest, declaration identities
and source navigation, and an exact build-evidence digest. Empty declaration
inventories may accompany unsupported interface observations, including generated
test harness functions; they certify no types. Scalar/default paths omit this
field, and default/expanded target reports omit the optional package snapshot.
The build digest identifies captured evidence, not matching compatibility.
Plan construction is pure, with initialization, package snapshot and publication
IO kept in separate helpers.

Independent checks cover identical nominal/manifest spellings in distinct package
roots, changed names/versions/digests, relative/outside-root paths, conflicting
compiler-file bytes and exact function ranges including BOM/CRLF. Real command
checks reject missing/orphan origins, altered package snapshots and source ranges,
and contradictory serialized file evidence. Two selected packages with inherited
versions compile in four library/test contexts concurrently. The fixture retains
all ten captured functions: eight instances of original source definitions and
two generated test mains. Four scalar instances retain existing admission; four
field getters and two generated mains remain excluded. Pair enumeration and
execution are complete under the existing matching contract.

A repeated-command fixture changes the manifest version and scalar field type,
then selects a release build. It verifies fresh invocation directories, changed
package/manifest/source/build identities, the new unsigned field type and effective
debug-assertion setting. The read-only getter remains excluded in both runs.
These are correctness fixtures, not natural-crate coverage measurements.

Final validation: `cargo test --workspace --locked` passed 535 tests, including
all 78 real-command tests, with zero failed or ignored. Formatting and workspace
Clippy with warnings denied passed. Repository-built `cargo rstyle --format json`
reported zero violations across 1438 functions; `cargo rstyle patterns --format
json` reported zero candidates. The pinned Python inventory suite passed 11 tests
and the scripting suites passed 43 tests. Logs are retained under
`target/nominal-binding-*`, including initial style diagnostics and fixture
expectation failures. The initial context fixture assumed eight total captured
functions and omitted the two generated mains; its corrected expectations retain
and explicitly classify both. Initial IO-density diagnostics were resolved by
separating plan construction from capture IO, without suppressions or tuning.

This unit binding is now connected to the serialized real-command handoff, but
it still issues no data admission certificate. Complete type/drop-free/effect
proofs, borrowed-output correspondence, data document lowering and registered
matching-extension admission remain implementation obligations. The original
assertion, pattern/constant, cross-unit summary, slice/string, loop-composition and
broader preregistered inventory work also remains outstanding. No new data
admissions or natural coverage gains are claimed.
