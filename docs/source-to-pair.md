# Opt-in Cargo source-to-pair integration

Build the two binaries using the repository's pinned toolchain:

```sh
cargo build -p cargo-rstyle -p rstyle-driver --locked
PATH="$PWD/target/debug:$PATH" cargo rstyle clones --format json
PATH="$PWD/target/debug:$PATH" cargo rstyle clones -p example --all-targets --trace
```

`clones` is independent of default style analysis and legacy `patterns`. It uses
actual captured HIR R0 evidence, paired serialized documents, external document
manifests, producer verification, `admit_produced`, strict imported admission and
the existing strict pair pipeline. It never uses the legacy miner, grouping,
normalization, candidate retrieval or ignore ledger. Existing defaults remain
unchanged. Accepted pairs are similarity decisions, not proof of equivalence,
purity or safe refactoring. No quality study or abstraction mining is activated.

## Scope and build settings

The default selection is the workspace's ordinary Rust library/binary targets.
`-p/--package` is repeatable. `--all-targets` includes Cargo test, example and bench
targets. `--manifest-path` selects a workspace. Build options are `--features`,
`--all-features`, `--no-default-features`, `--target`, `--release`, `--jobs`,
`--locked`, and `--offline`. Cargo resolves required-feature targets; disabled
ones have no expected compilation unit. An empty active compilation selection is
reported as incomplete, never as a successful missing-artifact run.

The embedded `rust-toolchain.toml` selects `nightly-2026-09-30`. The driver is
resolved next to `cargo-rstyle`, or through the existing explicit `RSTYLE_DRIVER`
override. The plan records the SHA-256 of both binaries, exact compiler and Cargo
versions, workspace target selection and build options. Every captured function
records producer revision, compiler commit, target/layout, semantic settings,
active cfg/features and the compiler arguments. Cargo's unit graph records resolved
features, build profiles, targets, modes and dependency units before compilation.

The wrapper passes the selected profile as the private driver argument
`--rstyle-extraction-profile=<registered-token>` before the rustc path. The driver
validates it once, removes it from the compiler arguments and carries the typed
profile through source scanning and HIR capture. The recorded rustc invocation
contains only compiler arguments. `RSTYLE_HIR_EXTRACTION_PROFILE` does not select
a profile; inherited values are ignored. Unregistered, duplicate or misplaced
private selection arguments fail before compilation. A standalone driver without
that argument retains its default R0 capture and historical evidence shape.
Only primary compilation units selected as Cargo unit-graph roots receive
extraction destinations. Probes, build scripts and dependency compilation units
compile without HIR capture, including selected packages compiled as build
dependencies. Compiler mode and resolved features must match the planned roots.

Strict and legacy subprocesses remove inherited `RSTYLE_*` settings before
selecting their own mode. Strict extraction also clears external rustc wrappers
and selects the pinned compiler. Normal Cargo configuration and Rust flags remain
build inputs and their effective compiler arguments are recorded. Incremental
compilation is disabled, including an explicit failure if Rust flags attempt to
re-enable it: the pinned producer cannot certify the pre-expansion scan under
incremental compilation.
Select features through Cargo options; manually injecting additional
`--cfg feature=...` flags cannot satisfy the planned feature inventory and fails
capture completeness.

## Freshness, concurrency and completion

Each command creates a unique directory under `<target>/rstyle/clones/run-*`.
That directory owns a fresh Cargo target directory: every warm run recompiles and
recaptures evidence. There is **no evidence cache**, and old run directories are
never searched for successful inputs. This initial strategy costs build time and
disk space, particularly for workspaces with dependencies. Completed run directories
can be removed when their reports/evidence are no longer needed.

Each wrapped compiler invocation creates its own directory. `started.json` records
its selection and arguments. Only after rustc succeeds and all declared artifacts
are present does the wrapper publish a compilation-unit ledger using `create_new`.
Duplicate or ambiguous ledger identities fail visibly. The ledger inventories
facts, per-function assessments, graphs and manifests with exact byte digests.
No two compiler processes write the same artifact files.

Successful Cargo compiler-artifact messages identify the ledgers to consume. The
observed selected-target inventory must agree in multiplicity with Cargo's primary
unit inventory. The complete raw unit graph also contains dependency/build-script units;
those are build evidence, outside the captured matching scope. Failed Cargo runs
are never handed to matching, even if some compiler invocations succeeded. Missing
ledgers/artifacts and digest mismatches fail closed. Unrelated files and prior runs
are never substituted for missing evidence.

## Identity and source navigation

A function is one source definition **in one compilation context**. The same source
function compiled normally and as a test target is intentionally two distinct
functions. Pairs across those contexts are included when both are admitted. There
is no source-digest deduplication or display-name grouping.

The deterministic global function key is SHA-256 of a JSON identity tuple:
package identity, target name/kind, test context, actual source filename,
file-relative function byte span, qualified path, captured semantic/build facts,
and compiler invocation. Only the fresh run-directory prefix is replaced by
`<run>` in invocation paths; it is not a matching normalization. The source digest
alone is never an identity. Identical definitions in separate modules or packages
remain distinct. Duplicate keys fail the run rather than merging functions.

Captured `frontend_options.source_file` is the real absolute frontend filename;
`fn_span` is a half-open UTF-8 byte interval in that file. `source.bytes` contains
the exact original function bytes, including CRLF/BOM normalization reversal, and
`source.digest` identifies them. Native graph source records carry the real
filename; graph spans remain function-relative. Add `fn_span.start` to obtain
file-relative offsets. Graph-local
node/binding IDs stay local; document keys are resolved within their compilation
unit's manifest namespace. `artifact_prefix` and the observed unit directories
locate each exact serialized document and its correspondence extensions.

`source_identifier` captures the exact unqualified identifier token spelling,
removing raw identifier syntax in the Name feature and preserving Unicode spelling
rather than using rustc's normalized symbol. The qualified path remains navigation
metadata. Identity and navigation do not enter graph labels, depth or scoring.

## Configuration and decisions

Use `--matching-config FILE` for a JSON object containing only the pinned allowed
keys: `minimum_size_ratio`, `numerical_threshold`, `string_threshold`,
`final_threshold`, and `scoring`. Legacy `--config` and `--ignore-file` are errors
for `clones`. For example:

```json
{
  "string_threshold": 0.50000000000000001,
  "scoring": "cross_product_cosine"
}
```

Decimal strings and numeric lexemes are preserved as exact rationals; they do not
pass through f64. `1.00000000000000001` is invalid. The default is the pinned
`smaller_graph_overlap`; `cross_product_cosine` is explicitly experimental.
Effective thresholds, scoring and gate outcomes are recorded. No thresholds are
calibrated here. All existing projection, features, sequential gates, directed WL,
weights and multiplicity rules are retained.

Every unordered distinct admitted-function pair is evaluated in sorted-key order.
There are no self/reverse duplicates, ranking filters or hidden caps. Optional
`--max-pairs N` is an execution budget: if exhaustive enumeration requires more
pairs, the command fails incomplete before enumerating any. It never selects a
partial candidate set. Arithmetic failures mark evaluation incomplete. Failed or
incompatible pairs have a null decision and an explicit `evaluation_status`,
separate from ordinary gate rejection. Later gates remain not evaluated after an
earlier rejection/failure.

## Reports and exit codes

`--format json` prints machine-readable evidence; the default text output prints
counts, completeness indicators and the report path. Every captured run writes
`report.json`. The report separately records:

- `capture_complete`: Cargo succeeded and the expected selected compilation units
  and their artifacts were observed.
- `assessment_complete`: captured function classification/admission finished
  without missing, corrupt, ambiguous or unverifiable evidence.
- `pair_enumeration_complete`: all required admitted-function pairs were emitted.
- `pair_evaluation_complete`: capture, assessment and all pair evaluations succeeded.

Functions retain admitted, excluded, unknown or failed outcomes and reasons.
Unsupported/unknown source is never silently removed from coverage. Original
frontend facts, source bytes/navigation, document identities, bounded normal-flow
limitations and capability diagnostics remain available. Matching records retain
projection sizes/categories, nine features, interfaces, distinct-read reference
counts, diameters, exact components and gates. `--trace` additionally includes
node/edge projection dispositions, WL round histograms and per-node shared-label
partitions. Trace collection uses the existing scoring transitions and does not
change decisions.

An entirely unsupported workspace can have complete assessment and zero admitted
pairs, with its exclusions visible. Compiler failure or missing expected evidence
cannot become a complete empty success. Configuration errors before capture print
an error to stderr; execution failures during capture/admission produce an
incomplete JSON/text report when report storage is available.

Exit codes:

- **0**: complete run, including complete unsupported/unknown assessments and
  zero admitted pairs. Accepted pairs do not change this default.
- **1**: complete run containing an accepted pair when
  `--fail-on-accepted-pairs` is explicitly enabled.
- **2**: configuration, compilation, capture, verification, resource, arithmetic
  or reporting failure. Failure takes precedence over accepted-pair exit status.

R0 limitations, including incomplete conditional-source evidence, remain fail
closed. A compilation unit with two otherwise indistinguishable ledger contexts
is rejected explicitly rather than silently collapsing it. No broader Rust subset,
exceptional-flow support, quality claim, retrieval or abstraction proposal is added.
After this integration lands, the bounded independently labeled quality study is
separate work: freeze labeling rules, development/held-out splits and acceptance
targets before calibration or held-out evaluation.

## Explicit extraction expansion selection

Use `cargo rstyle clones --extraction-profile hir-expanded-v1` to select all
currently verified capabilities in [the coverage ledger](../HIR_EXTRACTION_COVERAGE.md).
Omitting the option selects R0. `style` and `patterns` retain their legacy paths;
inherited `RSTYLE_*` values cannot select an extraction policy. No evidence cache
or old-directory discovery is introduced.

Reports record extraction selection, cumulative capabilities, registered policy,
call-summary identities, effective unchanged matching configuration and all
function exclusion/unknown/failure reasons. Conditional selections add original
owned-source inventories and whole-function presence in loaded source files.
Capture, assessment, enumeration and pair-evaluation completeness remain distinct.
Trace mode records existing traces without changing decisions.

Every scored local function crosses real compiler capture, paired serialized
documents and exact manifests, frontend-evidence verification, registered producer
verification, strict imported admission and unchanged projection. Every distinct
unordered admitted pair is enumerated; incompatibilities are recorded at the
validation gate rather than filtered to inflate successful pair counts. Mixed
local policy versions, R0/local policies or incompatible semantic build settings
cannot be compared. Failed compilation, missing/corrupt artifacts and explicit
resource/arithmetic failures remain failed/incomplete evidence.
# Local helper implementation slice

`cargo rstyle clones --extraction-profile hir-helpers-v1 --locked --format json`
selects the separately registered `rstyle.rust.hir-helpers/0.1.0` scalar contract.
It supports verified acyclic direct calls within one captured compilation unit,
including modules and resolved aliases. See
[helpers-v1.md](extraction-profiles/helpers-v1.md) for the evidence, panic,
argument-type and resource boundaries. Calls remain source operations and pairs
remain exhaustive under the unchanged pinned scalar matching algorithm.

This interim command does **not** complete the compositional data expansion.
Assertions, additional patterns/constants, concrete read-only APIs and cross-unit
dependencies remain unavailable under it. The planned `hir-data-v1` command and
its separate matching extension are not registered or selectable yet. Defaults,
R0 and hir-expanded-v1 retain their historical semantics.
