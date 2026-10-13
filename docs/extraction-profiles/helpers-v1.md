# Verified local scalar helper contract, implementation slice 1

Identity: `rstyle.rust.hir-helpers/0.1.0`. Opt-in selection: `hir-helpers-v1`.
This independent scalar contract includes the frozen hir-expanded-v1 capabilities
and adds direct concrete scalar calls within one captured compilation unit.
It uses pinned compat.ccgraph.strict matching with its existing scalar interface
vocabulary. It does not register the planned data contract or broader matching
extension. R0 and expanded identities and semantics remain unchanged.

Capture all function owners before dependency resolution. Resolve direct free
function paths, qualified paths and aliases by compiler definition identity;
callee names alone never authorize effects. Receiver/method, generic and indirect
dispatch are excluded. Callee arguments must be resolved integer/bool values;
unit parameters are initially excluded at call sites because mixed unit argument
position evidence needs a separate representation. Zero-argument calls and unit
results are supported. Evaluate each argument in source order once. Preserve
each nonunit argument at its actual operand position and keep one source-backed
CALL with Direct form, resolved callee and signature. Unit results carry no value
operand. No inlining, normalization or callee graph participates in projection.

A helper dependency records `rstyle.helper.source-body/1`, derived signature,
argument order, call-input/return-graph dependence, mutation, memory, panic and
returning facts. These fields are recomputed only after the callee boundary
verifies; caller-provided purity or summary flags cannot authorize a call.
It includes exact captured callee frontend facts, exact serialized
CFG/PDG and their manifest. Their producer and document boundaries must verify.
The callee's resolved definition identity, signature and semantic build settings
must agree with the call. Callee source and transitive evidence are retained, so
edits invalidate dependent manifests. Summary effects are derived by verifying
the bounded scalar callee rather than accepting supplied purity/effect flags.
Only verified memory-free, nonpanicking acyclic returning callee flow is summarized.
Scalar local assignments have no effect on caller bindings. Recursion, loops in
callees, panicking/partial callee flow, unresolved or unsupported source and absent
dependencies remain exclusions. Every argument contributes its actual call input
dependence, even if a callee does not use it. This does not claim behavioral
equivalence or return-value sensitivity to every argument.

Dependencies outside the current captured compilation unit remain unsupported;
cross-package handoff requires a separate post-capture artifact phase that binds
all selected units before deriving summaries. No compiler scheduling order or
familiar-name fallback resolves that gap. Existing four registered primitive
instantiations retain their independently tested portable contracts.

Current budgets: depth at most eight nested helper dependencies and at most
8 MiB encoded per-call evidence. Budget exhaustion must be an explicit incomplete
execution, never an ordinary match rejection. Recursive dependency records cannot
be manufactured as cyclic JSON. No path/loop iteration cap certifies callee flow.

Both document inventories retain dependency/source correspondence. Verification
must independently check resolved signatures, dependency manifests, callee
normal-flow evidence, source-backed CALL inventory and operand edges rather than
only comparing re-emitted caller bytes. Missing/corrupt callee documents, summary
facts or frontend provenance fail closed.

The existing projection maps calls to STATE and all other nodes by the pinned
table. Reference counting remains distinct retained scalar read bindings. Exact
interface serialization, all nine features, numerical/string gates, WL depth,
weights, multiplicity and overlap/cosine arithmetic are unchanged. Pairs require
the same registered scalar policy and compatible overflow/debug-assertion/panic
settings; native-width operations/interfaces, including transitive callee
operations hidden behind fixed-width caller signatures, require equal known pointer widths.
No R0/local or mixed-version pair bypass is permitted.

This is an implementation slice, not completion of compositional data coverage.
Assertions, broader patterns/constants, data/reference/slice operations and the
cross-unit summary phase remain unregistered under this identity. Validation and
implementation limitations must be reported before claiming the slice available.

## Slice validation

The real Cargo fixture admits all six definitions in the scalar predicate,
qualified/aliased wrappers, multilevel wrapper and conditional unit-helper family,
and evaluates all 15 distinct pairs. An independently specified wrapper projection
has coordinates `[0,2,3,0,0,0,1,3,1]`. Another complete family verifies two same-named
module definitions, multiple arguments and a branch-valued argument. Shuffling
captured owners produces identical dependency facts. Trace on/off preserves
function projections and pair identities/decisions. These fixtures are correctness
evidence, not a natural-crate coverage or quality result.

Adversarial checks reject changed source, signature, manifest and effect summary;
source edits alter transitive wrapper evidence and document manifests. Recursive,
unsupported, panicking and nonreturning callees remain exclusions. Exceeding the
dependency budget fails capture and pair evaluation explicitly.

Validation at this slice: `cargo test --workspace --locked` passed 503 tests across
24 suites, with none ignored; the subsequent five focused `clones_helpers` real
Cargo tests passed, including two added tests. `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
`cargo rstyle --format json` reported zero violations and
`cargo rstyle patterns --format json` reported zero candidates. Inventory scripting
tests passed 11 tests using the reproduction protocol's pinned Python dependencies.
These commands use the pinned compiler, with Cargo available through
`PATH=/home/dev/.cargo/bin:$PATH` in this environment.

Final branch validation after the concrete observation prerequisite passed 511
workspace tests in 25 suites. The repository-built command, selected with
`PATH="$PWD/target/debug:/home/dev/.cargo/bin:$PATH"`, also admitted the documented
six-function correctness family and evaluated all 15 pairs. An independent Python
JSON Schema validator checked all 12 resulting documents against the unchanged
vendored schemas and checked their SHA-256 entries in the external manifests.
This six-function fixture is distinct from the unidentified user-observed 6/50
inventory. No natural-crate admission target or detection-quality claim follows.
