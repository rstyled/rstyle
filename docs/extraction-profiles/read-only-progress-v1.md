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

## Strict source-type correspondence

The `data-source-types-v1` contract now has a separate independent source verifier.
It checks nested scalar/tuple/shared/slice/array spellings, requires literal array
lengths, and fails named paths without scoped resolution evidence. Observational
validation still retains unsupported-to-prove compiler facts; it never substitutes
for this stricter check. Rust unit uses `()`, not the normalized token `unit`.

Validation: `cargo test --workspace --locked`, workspace clippy with all targets
and warnings denied, and fmt check passed. All 78 real-command tests ran. The
repository-built style command reported zero violations across 1444 functions;
patterns reported zero candidates. Inventory scripting tests (11) and study/harness
scripting tests (43) passed. Logs: `target/data-source-proof-*`.

This is not data admission. No new extraction or matching identity is registered;
no new natural coverage gain is claimed. Effect/origin proofs, data lowering,
independent document verification and the real source-to-pair data path remain
required.

## Connected nonnominal data slice

Implementation baseline: `a4514f4`; the data extraction/matching contract was
committed separately as `23f8853` before implementation. The opt-in `hir-data-v1`
path now carries concrete interface tokens through driver capture, source/type
verification, source-backed field lowering, serialized PDG/CFG manifests, producer
and independent data semantic verification, strict import, the separate matching
vocabulary, projection and exhaustive pairs. Historical profiles do not gain these
inputs. Shared formal roots retain alias uncertainty. Reference-valued outputs
retain all source origins and the verifier independently follows actual serialized
value/reaching-definition/alternative edges to check them.

The six-function temporary Cargo correctness family combines a scalar predicate,
shared tuple getters, field reads feeding verified predicate calls, and branching
between borrowed outputs. All six admit and project, and all 15 pairs receive
exhaustive decisions. On the same source the helper selection admits only the
scalar predicate. This is five newly admitted fixture functions, not five natural
inventory gains. The getter's independent numerical expectation is
`[0,2,4,0,0,0,0,4,2]`: four retained OTHER nodes, four data edges, two execution
edges, and distinct binding/field read keys. Tampering with locations, alias facts,
reference counts, matching identity, types or origin dependencies is rejected.

Nominal structs/effect proof, local reference correspondence, construction,
read-only library operations, indexing/subslicing and the broader coverage
inventory remain outstanding. The original full goal is not complete.

Validation for this connected slice: `cargo test --workspace --locked` passed
549 tests with zero failures/ignored tests, including all 81 real-command tests.
`cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
Repository-built `cargo rstyle --format json` reported zero violations across
1517 functions; `cargo rstyle patterns --format json` reported zero candidates.
The inventory (11) and study/harness (43) Python tests passed. A fresh five-function
Cargo smoke run also projected all five and decided all ten pairs; its ten paired
documents validated against both unchanged vendored closed schemas using
jsonschema 4.25.1. Logs/artifacts are under `target/data-admission-*` and the
per-invocation Cargo run directory. None of these fixture counts identifies the
historical user-observed 6/50 inventory.

Selected-unit nominal context slice (baseline `4dffdd5`): the data driver now
receives a bounded typed private argument containing the frozen Cargo package,
selected root and observed crate name. The source-to-pair handoff checks it against
the selected ledger and invocation. Data nominal observations preserve opaque
Cargo package source identity; historical helper observations omit the new field.
This context is a prerequisite, not drop/namespace certification or new nominal
admission. Closed source and independent resolution remain required.

Validation: three real data command tests passed; driver argument tests and PDG
context/namespace/concrete-type tests passed. Workspace clippy passed. Repository
style reported 1,524 functions and zero violations; the initial patterns run found one duplicated transport-budget check, subsequently
extracted into one helper and rechecked. Logs: `target/nominal-context-{real-final,driver,pdg,clippy}.log`,
`target/nominal-context-{style,patterns}.json`. Historical inventory was not rerun
or replaced in this slice.

Connected nominal slice (implementation baseline `35f10c9`): `hir-data-v1` now
admits concrete drop-free nominal parameters/shared receivers, nested formal field
reads and nominal shared outputs. Original rustc local buffers form a bounded
census, independently closed over ordinary modules. Lexical declarations and named
imports/aliases bind exact nominal identities and owning receiver impls. The
source boundary recursively rejects hidden trait impls and macros inside bodies;
it does not trust compiler destructor flags as a stand-alone drop-free proof.
Nominal tokens preserve namespace plus field identities/types and recursive native
width requirements. Independent serialized checks verify interface roles/types,
field locations, possible aliases and borrowed value origins.

The real five-function API fixture contains nested structs, an imported type alias,
shared receivers, aliased shared arguments, a shared identity output, same-named
types in distinct modules, and BOM/CRLF original sources. It admits/projects all
five and evaluates all ten distinct pairs. Independent getter expectations are
three distinct reads, five OTHER vertices, six data edges and three execution
edges: `[0,3,6,0,0,0,0,5,3]`. Census/context/source/receiver/output tampering and
hidden nested `Drop` cases fail closed. This is disclosed correctness evidence,
not natural coverage or a quality study.

Validation: `cargo test --workspace --locked` passed 559 tests (zero failures or
ignored tests), including all 83 real-command tests. Formatting and workspace
all-target clippy passed. Repository style: 1,565 functions, zero violations;
patterns: zero candidates. Inventory scripting: 11 tests passed; study/harness
scripting: 43 passed. Six nominal smoke documents passed the unchanged closed
vendored schemas. Logs: `target/nominal-api-workspace-final.log`,
`nominal-api-clippy-final.log`, `nominal-api-{style,patterns}-final.json`,
`nominal-api-{inventory,scripting}.log`; the earlier failed feature assertion and
warm-run diagnostic remain in `nominal-api-workspace.log`.

Remaining after the nominal slice: construction; local values/references and explicit borrows/dereferences;
read-only slice/string operations; reference-aware read-only method/cross-unit
summaries; assertions, constants/patterns and cyclic correspondence; broader frozen
natural applicability inventory/targets. The original whole goal is still active.
Historical selected definitions, denominators, R0/expanded evidence and vendored
contracts are unchanged. No thresholds were tuned or mining/grouping activated.

Connected shared-field borrow slice (implementation baseline `eb658c4`, contract
frozen in `f49c5eb`): original immutable field borrows now cross the real data
command and independent serialized verification. Nested address projections are
source-backed computations with exact formal locations, not invented field-value
loads. Borrow origins retain the projected input location without appending a
dereference. Shared roots conservatively may alias. Reference-bearing targets,
local/temporary/mutable/raw borrows, indexing, explicit dereferences and array-to-
slice coercions remain unavailable.

The eight-function fixture combines shared receivers, nested struct/tuple places,
conditional borrowed outputs and a verified scalar predicate call. All eight
functions admit/project and all 28 unordered pairs receive decisions. Independent
getter features are `[0,4,5,0,0,0,0,6,1]`; compatible scalar getters score exactly
`1/1`. A separate fixed-array/unsized-slice/str field-borrow fixture admits three
functions and decides all three pairs. Tampered source ranges/types, place roles,
effects, locations, origins and operand positions fail closed. Trace on/off
outcomes agree for both the earlier tuple family and the new borrow family.
These disclosed fixtures are correctness evidence, not natural coverage gains.

Validation: workspace tests passed 562 tests, zero failures/ignored, including all
86 real-command tests. After strengthening only the fixture with the predicate
call, all eight Data real-command tests passed again. Formatting and workspace
all-target clippy passed. Repository style reports 1,578 functions and zero
violations; patterns reports zero candidates. Inventory tests: 11 passed;
study/harness tests: 43 passed. The initial Python attempts lacked inventory
dependencies and the Cargo PATH respectively; both were rerun successfully.
A fresh six-function smoke run completed capture, assessment, enumeration and
evaluation, admitting all six; all twelve paired documents passed the unchanged
vendored schemas with jsonschema 4.25.1. Evidence is in
`target/shared-field-borrow-{workspace,data-final,clippy-final}.log`,
`shared-field-borrow-{inventory,scripting}-final.log`,
`shared-field-borrow-{style,patterns}.json`, and
`shared-field-borrow-smoke-final.json`.

Remaining: aggregate construction and local bindings; stored shared-reference
field origins and lifetime-only nominal declarations; read-only slice/string
operations; reference-aware method/cross-unit summaries; assertions, scalar
constants/patterns and cyclic correspondence; broader frozen natural inventory
and admission targets. Data admission as a whole is not complete. Historical
denominators and contracts remain unchanged.

Connected stored-reference slice (implementation baseline `9306ff3`, contract
frozen in `c3fef82`): non-generic drop-free structs now retain recursively named
shared-reference origins when passed or returned by value. Field reads load stored
references with an added referent projection; they remain distinct from borrowing
field storage. Serialized loaded origins are derived from the actual read access,
registered formal location and exact value type, then compared independently with
source-derived origins. Reference-bearing array/slice elements still lack element
origins: even mixed values with some supported reference fields fail closed before
lowering rather than returning a partial origin set.

The seven-function real Cargo family includes stored scalar/str/slice references,
shared receivers, nested reference-bearing intermediate values, by-value identity,
conditional outputs and conservative aliasing. All seven admit/project and all 21
pairs receive decisions. Independently specified field getter features remain
`[0,2,4,0,0,0,0,4,2]`. Separate negative workspaces prove reference-element gaps
and lifetime-generic nominal declarations remain exclusions, while scalar functions
remain admitted. Trace collection preserves semantic outcomes on fresh warm runs
for all three connected families. These are disclosed correctness fixtures, not
natural inventory coverage or detector quality evidence.

Validation: `cargo test --workspace --locked` passed 565 tests with zero failures
or ignored tests, including all 88 real-command tests. Formatting and workspace
all-target clippy passed. An initial clippy failure for a 53-line test helper was
fixed by extracting path expectations; the focused stored-reference command test
was rerun afterward. Repository style reports 1,581 functions and zero violations;
patterns reports zero candidates. Inventory tests: 11 passed; study/harness tests:
43 passed. A fresh seven-function smoke run decided all 21 pairs; all fourteen
paired documents validated against the unchanged vendored schemas with jsonschema
4.25.1. Logs: `target/stored-reference-fields-{workspace,real-final,clippy-final}.log`,
`stored-reference-fields-{inventory,scripting}.log`,
`stored-reference-fields-{style,patterns}.json`, and
`stored-reference-fields-smoke.json`.

Remaining prerequisites include lifetime-only nominal declarations, construction,
local bindings, resolved read-only library operations, reference-aware verified
calls and cross-unit summaries, assertions/constants/patterns, cyclic value proof,
and the preregistered broader natural inventory. Whole data admission remains
unfinished; historical reports, defaults and pinned artifacts remain unchanged.

Connected lifetime-only nominal slice (implementation baseline `6d30c00`, contract
frozen in `e34ebec`): Data capture now permits only concrete region arguments for
local nominal structs, instantiates each field with the exact resolved arguments,
and independently checks lifetime-only declarations, impl ownership and terminal
path argument arity. Type/const arguments, lifetime bounds and nominal where
clauses remain unavailable. Historical helper captures retain their no-generic
argument rule. Nominal/source identity and stored-reference/location proof remain
exact; matching omits region spelling without erasing nominal identity.

The eight-function family contains module aliases, lifetime-only shared receivers,
stored scalar/str/slice references, nested by-value values, conditional outputs,
and elided versus explicit static instantiations. All eight admit/project and all
28 pairs receive decisions. The two scalar-reference getters with distinct lifetime
spellings score exactly `1/1` through unchanged matching. R0, expanded and helper
profiles exclude this same family. Bounded lifetime impls, nominal where clauses,
type/const generic values and malformed source path arguments fail closed. A
129-lifetime instantiation demonstrates explicit incomplete capture under the
128-argument budget, with no assessed or scored remainder. Trace on/off outcomes
agree for all four Data families. This is correctness evidence, not natural
coverage or a detection-quality result.

Validation: `cargo test --workspace --locked` passed 570 tests, zero failures or
ignored tests, including all 91 real-command tests. Formatting and workspace
all-target clippy passed. Repository style reports 1,588 functions and zero
violations; patterns reports zero candidates. Inventory tests: 11 passed;
study/harness tests: 43 passed. A fresh BOM/CRLF Cargo smoke run admitted eight and
decided all 28 pairs; all sixteen paired documents validated against unchanged
vendored schemas with jsonschema 4.25.1. Initial test API/profile-token/method-name/
field-path assumptions and a clippy argument-count failure were corrected before
the final workspace run; earlier logs remain available. Final evidence:
`target/lifetime-nominals-workspace.log`, `lifetime-nominals-clippy-verified.log`,
`lifetime-nominals-{inventory,scripting}.log`,
`lifetime-nominals-{style,patterns}.json`, and `lifetime-nominals-smoke.json`.

Remaining: aggregate construction and local bindings, read-only library/slice
operations, reference-aware method/cross-unit summaries, assertions and ordinary
scalar constants/patterns, cyclic value correspondence, and a preregistered broader
natural inventory with measured targets. The conservative closed-unit nominal
boundary still rejects unsupported trait/expansion items; successful fixture
admission does not establish natural-crate majority coverage. Whole data admission
is still active and incomplete. Historical evidence and vendored contracts remain
unchanged; thresholds, grouping and mining were not activated.

Legacy observation separation follow-up (baseline `caad2d4`): permissive helper
type observations again accept their historical nongeneric where clauses and
qualified associated field spellings. These observations still authorize no Data
admission. Strict Data declaration/census/source-path checks separately reject
both unsupported cases. A new real-command regression checks capture completeness,
the helper's captured observation, and the excluded outcomes under both profiles.

Validation: workspace tests passed 571 tests, zero failures or ignored tests,
including all 92 real-command tests. Formatting and workspace all-target clippy
passed. Style reports zero violations across 1,588 functions; patterns reports
zero candidates. Logs: `target/lifetime-observation-legacy-{real,workspace,clippy}.log`
and `lifetime-observation-legacy-{style,patterns}.json`. Earlier inventory and
study/harness checks remain 11 and 43 passed; their scripts were not changed.

The prospective `read-only-lengths-v1.md` contract is frozen, but the four discovery
library calls remain excluded pending their full proof/registry implementation.
Field place-versus-value context is being audited before that next admission slice.
No broader data completion or natural coverage claim is made.

Connected field place/value correction (baseline `9305432`, contract frozen in
`9bbaf82`): nested by-value aggregate projections now compute places rather than
inventing whole-aggregate reads. Stored shared-reference field bases retain the
required pointer-value read, including beneath an explicit field borrow. Original
source-derived field types independently distinguish these cases; serialized read
roles, operands and reference origins remain checked. Consecutive implicit shared
dereferences remain explicitly unavailable pending a separate storage-read proof.
Earlier nested-getter feature vectors remain historical observations, superseded
by this correction: the getter now has `[0,3,5,0,0,0,0,5,2]`.

The seven-definition independent family admits six, excludes one unsupported
dereference, and decides all fifteen pairs. It combines owned aggregates, stored
references, lifetime receivers and tuple field borrows; forged pointer types and
removed/changed pointer reads fail verification. This is semantic correctness
evidence, not a measured natural coverage gain.

Validation: `cargo test --workspace --locked` passed 572 tests, zero failures or
ignored tests, including all 93 real-command tests. After extracting the common
address-operation constructor to resolve the patterns finding, all fourteen Data
real-command tests passed again. Workspace all-target clippy and final formatting
passed. Final style reports 1,592 functions with zero violations; patterns reports
zero candidates. The BOM/CRLF smoke admitted six and decided fifteen pairs; all
twelve paired documents passed the unchanged vendored schemas. Logs:
`target/field-value-context-workspace-final.log`,
`field-value-context-data-refactored.log`,
`field-value-context-clippy-refactored.log`,
`field-value-context-{style,patterns}-final.json`, and
`field-value-context-smoke.json`. Inventory/harness scripts were unchanged; their
last checks remain eleven and forty-three passed.

Read-only length summaries, reference-aware verified calls, aggregate construction,
slice indexing/subslicing, assertions/constants/patterns, cyclic value proof and
the broader preregistered inventory remain unfinished. No full Data completion,
majority natural coverage or detection-quality claim is made.

Connected method-receiver observations (baseline `335c60f`, prospective contract
frozen in `a894c2b`): Data capture now records exact method/receiver HIR identities,
source spans, ordered concrete receiver adjustments and instantiated callee
inputs/result. Builtin dereferences and shared/mutable borrows have closed kinds;
other conversions retain explicit stable unsupported reasons. All recorded types
use concrete observations and the existing finite budgets, with additional
method/adjustment budgets. No display/debug type rendering authorizes a call.
This is observational evidence, not a semantic summary or an admission flag.

Independent real-compiler expectations check all four slice/str length operations,
stored shared str fields, exact shared-reborrow transitions/signatures, array
coercion, unsupported String receivers and overloaded dereference. These calls
remain excluded; helper captures do not acquire the new observations. Before any
length admission, the library registry/core/build proof, independent source/type/
origin transition checks and independent serialized CALL/effect/value-edge checks
still need implementation. No fixture observation or discovered fingerprint is a
substitute for that proof.

Validation: `cargo test --workspace --locked` passed 574 tests, zero failures or
ignored tests, including all 95 real-command tests. Formatting and workspace
all-target clippy passed. Style reports 1,598 functions with zero violations;
patterns reports zero candidates. Inventory and study/harness suites passed eleven
and forty-three tests. A fresh BOM/CRLF Data smoke retained six admissions, one
explicit exclusion and all fifteen pair decisions; all twelve paired documents
validated against unchanged vendored schemas. An initial full run found two
mutation-budget style violations (93 other CLI tests passed); cohesive interface
and receiver-type capture phases resolved both before the successful full rerun.
No thresholds, suppressions or ignore records changed. Logs:
`target/receiver-capture-workspace-final.log`,
`receiver-capture-clippy-final.log`, `receiver-capture-{style,patterns}-final.json`,
`receiver-capture-{inventory,scripting,schemas}.log` and
`receiver-capture-smoke.json`; the initial failure log remains
`receiver-capture-workspace.log`.

Whole Data admission remains unfinished. The next prerequisite is independently
verified receiver/call inventories and a tested pinned library registry, followed
by reference-aware verified helper calls and the remaining compositional bundles
and preregistered natural inventory. Historical reports, old profiles, vendored
contracts and the strict matching algorithm remain unchanged.

Connected independent receiver verification (baseline `7ccfc10`, contracts frozen
in `5843889` and `1c37e83`): real Data handoffs now require the exact receiver
inventory before assessment. The verifier reconstructs HIR method membership,
original-source correspondence, exact receiver identity/span, known concrete
receiver/result types, shared-reference adjustment transitions and endpoints.
Malformed, missing, duplicate or extra inventories fail handoff with incomplete
execution. Unsupported transitions remain observations, never effect authority.
Existing registered scalar summaries independently establish formal input types;
those captured signatures must agree. Other method signatures remain observational
until a separate registered authority establishes them.

The paired-document verifier reads both serialized frontend inventories directly,
checks their transitions and compares their evidence with captured records without
invoking lowering or re-emission. Independent tampering cases cover missing and
duplicate records, changed identities/spans/types, reordered/forged transitions,
invalid unsupported reasons and mutable-borrow spoofing. A real admitted scalar
primitive family checks corrupted captured and serialized signatures; a real
driver shim checks four corrupt inventory variants cannot become successful
excluded-only runs. Exact-kind traversal is shared with the existing helper
inventory verifier; its order and filtering are unchanged. Length operations
remain excluded, pending pinned core/build registry and independent CALL/effect/
operand/origin verification. No new library admission is claimed here.

Validation: `cargo test --workspace --locked` passed 576 tests, zero failures or
ignored tests, including all 97 real-command tests. All eighteen Data real-command
tests passed before the final signature/helper refinements, and the full run
passed after them. Formatting and workspace all-target clippy passed. Final style
reports 1,609 functions with zero violations; patterns reports zero candidates.
Inventory and study/harness suites passed eleven and forty-three tests. A fresh
BOM/CRLF family retained three admissions and two explicit length exclusions,
decided all three pairs, and validated all six documents against unchanged
vendored schemas. Initial test Pair-cloning/map-mutation API assumptions, a test
helper line-limit failure, three spacing diagnostics and a duplicate inventory
traversal pattern were corrected before the final full run; no suppressions,
ignores or thresholds changed. Logs: `target/receiver-verification-workspace-final.log`,
`receiver-verification-clippy-verified.log`,
`receiver-verification-{style,patterns}-final.json`,
`receiver-verification-{inventory,scripting,schemas}.log` and
`receiver-verification-smoke.json`; earlier failure logs remain available.

Remaining: connect the tested pinned slice/string library summaries through
admission and serialized CALL verification, then reference-aware verified calls,
aggregate construction/bindings, indexing/subslicing/checked access, assertions,
constants/patterns, cyclic value proof and the broader preregistered natural
inventory. Whole Data admission remains active and incomplete. Historical reports,
legacy profiles and vendored contracts remain unchanged; no quality study,
threshold tuning, grouping or abstraction-mining detector behavior was enabled.

Connected pinned length/emptiness summaries (implementation baseline `062939b`,
core/signature/effect contract frozen there; grouped-borrow ownership contract
frozen in `7648026`): the four core slice/str len/is_empty operations now cross
the real capture, manifest, frontend, producer/import, projection and exhaustive
pair boundary. The initial authority is nightly-2026-09-30's official
aarch64-unknown-linux-gnu core build. Compiler-selected core artifact paths/sizes/
SHA-256 are captured once per selected invocation, before source diagnostic and
callee handoff resolution. Exact definition/build fingerprints bind a code-owned
registry whose semantics derive from reviewed pinned sources and independent
native empty/nonempty/zero-sized-element/UTF-8 byte-length expectations. Missing
or corrupt evidence and byte-budget failures are incomplete execution. Consistent
unregistered builds remain unavailable. Data compiler commits also must match
the frozen Cargo compiler identity. Other profiles acquire no library artifacts.

Data method dispatch uses verified concrete tokens, and the registry checks exact
instantiated input/result types, allowed shared reborrows, source invocation and
formal origins. CALLs remain source operations; no inlining, normalization, buffer
read/write or panic/exit edge is invented. Serialized checks inspect actual CALL
membership, callee, type, effects, summary/core/build/receiver evidence and operand
value edges independently of producer re-emission. Existing scalar summary wire
records omit the new optional library payload and retain their historical fields.

The fifteen-function independent BOM/CRLF family combines stored slice/str fields,
lifetime receivers, explicit grouped field borrows, borrowed outputs, a verified
scalar-helper call, a scalar loop and conditional reference outputs. All fifteen
admit and all 105 pairs receive decisions; trace on/off warm-run decisions agree.
The simple length graph independently has features `[0,2,3,0,0,0,1,3,1]`. The four
previously excluded discovery functions now admit and reach all six pairs; their
original exclusion report is retained. These are disclosed correctness fixtures,
not frozen natural-inventory gains or unseen detector-quality evidence.

Negative compositions keep array coercion, mutable/extra shared receivers, owned
String, indexing and unsummarized reference-producing chains excluded. A same-
spelled user method receives no builtin treatment; its otherwise supported field
getter remains independently admissible. R0, expanded and helper modes continue
to exclude the broader family. Forged core/definition/signature/operand/effect/
summary facts, missing/duplicate CALL evidence and modified compiler commits fail
verification; real driver corruption yields incomplete handoff. Unknown methods,
reference-aware local method/helper calls and other library operations remain
unavailable.

Validation: `cargo test --workspace --locked` passed 581 tests, zero failures or
ignored tests, including all 100 real-command tests. Formatting and workspace
all-target clippy passed. Style reports 1,638 functions with zero violations;
patterns reports zero candidates. Inventory and study/harness suites passed eleven
and forty-three tests. The fresh BOM/CRLF Cargo smoke admitted all fifteen and
decided all 105 pairs; all thirty paired documents validated against unchanged
vendored schemas. Logs: `target/length-library-workspace.log`,
`length-library-clippy-final.log`, `length-library-{style,patterns}-final.json`,
`length-library-{inventory,scripting,native,schemas}.log` and
`length-library-smoke.json`. Initial source-diagnostic capture ordering, policy-
tagged width evidence, grouped borrow parsing and style/clippy assumptions were
corrected before the full successful run; failure logs remain available.
No thresholds, suppressions, ignore entries or vendored artifacts changed.

## Memory-free helpers with shared inputs

The continuation at `37360b1` froze `read-only-helper-calls-v1.md` before expanding
the call boundary. Recursive Data type-tree native-width checks now propagate
through verified helper dependencies; malformed tokens fail verification and
historical scalar interpretation remains unchanged. Data direct-call dispatch
uses canonical child/result tokens, as registered metadata methods already do.
Shared inputs with scalar/unit results can consequently cross the existing
memory-free, nonpanicking, returning, same-unit acyclic helper boundary.

A six-function real Cargo family composes slice/str metadata with qualified paths,
an import alias and a three-level helper chain. All six functions admit and all
fifteen pairs receive decisions; fresh trace and warm untraced decisions agree.
Transitive manifest/source/signature/effect tampering fails producer verification.
A separate mixture admits its field getter and shared identity body but excludes
the memory-reading caller, borrowed-result caller and recursive caller; its single
admitted-function pair receives a decision. These are correctness fixtures only.
Memory-reading callees, borrowed call results, inherent-method helper dispatch,
generics and cross-unit dependencies remain unavailable. The next borrowed-result
contract is specified prospectively without claiming implementation admission.

Validation: `cargo test --workspace --locked` passed 585 tests, zero failed/ignored,
including all 102 real-command tests. Inventory and harness suites passed eleven
and forty-three tests. Workspace fmt and all-target clippy passed. Style reports
1,638 functions and zero violations; patterns
reports zero candidates. Logs are `target/data-shared-helpers-{workspace,inventory,
scripting,style,patterns,real,negative}.log` (style/patterns use `.json`).

## Whole shared-formal helper results

The continuation at `e82d2a8` connects `rstyle.helper.shared-return/1` under the
previously frozen bounded result contract. Summaries derive a complete sorted
return-origin set from verified callee RETURN evidence. Source and serialized
graph verification substitute only selected actual argument origins, preserving
argument position, nested stored-field origins and possible shared aliases. The
historical scalar/unit summary omits the new optional `reference_outputs` field.
Data helper dependencies additionally require exact selected-unit context,
compiler invocation and semantic build identity. Neither memory locations nor
projected/aggregate reference-result components acquire callee summary admission.

Registered metadata source correspondence now checks exact source invocation,
resolved receiver/signature and pinned library/build authority before linking.
It does not certify frontend effects or reference origins; those proofs remain
mandatory after dependencies are attached. This repairs the earlier diagnostic
ordering problem for `identity(value).is_empty()` without erasing the call.

The ten-function family includes ordinary unbounded named-lifetime helpers,
multi-level borrowed-result calls, selected and
swapped argument origins, branching between possibly aliasing shared inputs,
stored-field actual arguments and metadata on a borrowed helper result. All ten
admit and all 45 pairs receive decisions, with trace/warm invariance. Independently
specified simple helper features are `[0,2,3,0,0,0,1,3,1]`. Editing the callee's
returned formal changes dependent manifests and caller return origins in a fresh
run, which again evaluates every pair. Summary output removal/emptiness, wrong
parameter/path/identity, changed compiler invocation/target, forged CALL origins
and swapped graph operand positions fail verification. The seven-function negative
mixture admits four bodies and evaluates all six pairs while excluding a
memory-reading caller, recursion and an aggregate reference-result caller.

Validation: `cargo test --workspace --locked` passed 586 tests, zero failed/ignored,
including all 103 real-command tests. Style reports 1,649 functions and zero
violations; patterns reports zero candidates. Inventory and scripting suites
passed eleven and forty-three tests. A fresh BOM/CRLF real command admits all ten,
evaluates all 45 pairs and marks each execution stage complete. All twenty paired
documents pass unchanged pinned schemas. Logs are under
`target/data-borrowed-helpers-*`. The initial fixture lifetime error and source
diagnostic ordering failure are retained in the earlier real-command logs.
Workspace fmt and all-target clippy passed. After the full suite, the selector
fixture switched from static inputs to ordinary unbounded named lifetimes; its
complete real-command test, including trace invariance, projection, tampering and
callee-edit invalidation, passed again. The pinned compiler resolves those
lifetimes as late-bound without an early-bound generic substitution. A separate
two-function named-lifetime discovery run also admitted both and evaluated their
pair; it remains disclosed fixture evidence, not a natural-inventory gain.
These are disclosed correctness fixtures, not natural coverage or quality gains.

The historical primitive registry is now a byte-identical code-owned artifact at
`crates/rstyle-pdg/contracts/primitive-summaries-v1.json`; its existing docs JSON
remains an unchanged historical snapshot. The loader no longer compiles a docs
path into the implementation. A frozen SHA-256 test preserves the exact original
bytes (`d0b0b8a8f43f26584cedeff2e3abfab9ba1b5bd6e2c4304c00aa2cce11e42504`).
Producer/imported-matching regressions and the real primitive-summary/R0 exclusion
test passed after relocation. Style and patterns remain at zero; fmt and workspace
all-target clippy passed. Logs: `target/code-owned-primitive-registry-*`.

The separate `hir-data-v2` extraction and `rstyle.ccgraph.data-calls/1.0.0`
matching registration are connected without widening Data-v1. The real five-body
registration fixture admits the same four getter/metadata/identity/borrowed bodies
under both selections, excludes its memory-reading caller and evaluates all six
pairs. Independent projection expectations count CALL read keys only under the
new matching registration, retaining distinct binding/location keys and collapsing
repeated keys. Those graph-stage fixtures do not certify memory-reading callers;
summary derivation and exact location substitution remain unavailable pending
implementation. The contract is `data-calls-v1.md`.

Ubuntu CI exposed the original aarch64-only library authority. The separately
verified x86_64 authority now binds the official target's exact core hash,
definition fingerprints and rmeta/rlib digests. Cross-target real capture admits
the four metadata operations (six pairs) and the full disclosed fifteen-function
BOM/CRLF family (105 pairs). Warm/trace-off decisions agree. All thirty family
documents validate unchanged pinned schemas. No native CI test was disabled;
native x86_64 CI still needs to confirm the push.

Combined validation passes 591 workspace tests including all 104 real-command
tests, with zero failed/ignored. Formatting and workspace all-target Clippy pass;
style reports 1,656 functions and zero violations, patterns zero candidates.
Inventory and harness scripting tests pass eleven and forty-three tests.
Logs: `target/data-v2-registration-*` and `target/x86-core-*`.

Whole Data admission remains incomplete. Outstanding work includes memory-reading
and inherent-method verified calls across modules/packages, aggregate construction/local reference
bindings, element reads/subslicing/checked Option, assertion provenance, scalar
constants/patterns, merges outside versus inside loop cycles, scoped nominal effect
proof and the broader preregistered contemporary-crate inventory with measured
targets. Historical reports and the unidentified user-observed 6/50 remain distinct
from these fixtures. No mining/grouping/calibration or quality study was enabled.

## Same-unit source-backed memory-reading direct calls

Implementation baseline: `383e477` on `ccgraph/read-only-api-coverage`, with a
clean working tree before this slice. The preceding x86_64 registry and Data-v2
registration checkpoint passed both Ubuntu x86_64 and ARM jobs in run
`38471863840`. This supplements the earlier pending CI record above.

Only Data-v2 derives `rstyle.helper.read-only-body/1` from independently verified
callee documents. Each read occurrence preserves its operation/access identity,
formal position, ordered resolved path and exact value type. Actual shared
referent origins replace exactly the leading callee shared-reference projection;
stored-reference dereferences, alternative origins and possible aliases remain.
Source-backed CALLs retain argument edges and exact substituted read accesses;
no callee graph is inlined or projected into the caller. Independent serialized
checks reconstruct locations and CALL effects and reject binding READs claiming
referent accesses. Existing Data-v1 still excludes these callers.

The twenty-function family reaches all 190 pairs, composing nested getters,
multi-level wrappers, aliases and qualified paths, multiple/identical shared
inputs, repeated reads, borrowed field arguments, branching memory-free shared
results, stored references, unit helpers, scalar casts/loops and metadata calls.
Independent expectations specify the wrapper's nine features and exact read
multiplicity/location paths. Tampering checks cover CALL accesses, locations,
summary occurrences and dependency manifests/source/signatures/effects. Warm
trace-off runs preserve pair decisions; callee edits change dependent manifests
and exact caller locations. A separate mixed family retains explicit exclusions
for by-value and borrowed-result memory-reading callees, recursion, potentially
panicking arithmetic and nonreturning callees.

Native ARM and x86_64 three-function direct-call chains each admit all three
functions and enumerate/evaluate all three pairs with complete capture and
assessment. All six documents per target validate unchanged vendored schemas.
These are connected coverage/correctness fixtures, not the historical or broader
natural applicability inventory and not a detection-quality result.

The explicit read-occurrence budget is 4096 per derived summary and substituted
CALL, alongside existing depth-eight and eight-MiB dependency limits. Exhaustion
fails capture rather than admitting truncated evidence. Read-budget-specific
real-command stress coverage remains an obligation of the broader resource audit.
Inherent methods, cross-unit linking and the other outstanding Data bundles above
remain unavailable or unfinished. Whole Data admission is not complete.

Validation: `cargo test --workspace --locked` passes 594 tests, including all 106
real-command tests, with zero failed/ignored. The final enriched family test also
passes independently. `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` pass. Style checks
report 1,679 functions and zero violations; patterns report zero candidates.
The documented `uv` inventory command passes eleven tests; scripting passes
forty-three with Cargo on PATH. Initial Python attempts without dependencies/PATH
are preserved in the logs. An initial workspace attempt was invalidated when an
overlapping targeted Cargo build replaced its live orchestration executable;
the isolated final rerun passes all five affected tests and the complete suite.
Logs and retained schema reports are under `target/data-read-calls-*`.
