# HIR R0 coverage ledger

Authority: compat `04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f`, vendored unchanged
in `vendor-compat/rust-r0`. Targets are `compat.rust.hir-r0/0.1.0`, PDG `0.1.0`
and CFG `0.2.0`. Landed imported-contract hardening is `a9da3eb`, an ancestor
of this branch; its strict adapter and validation tests remain in use. Later
root-policy transport changes (`314e91b`, `820bd39`, `9c9d324`) are also retained.

## Verified implementation and bounded claims

The opt-in driver captures pre-expansion owned-body source, resolved HIR,
types, built-in dispatch, lexical bindings, exact original UTF-8 bytes (including
CRLF normalization reversal), compiler identity, target layout and settings.
Compiler-private dependencies remain in the driver. Compiler-independent
lowering, dataflow, dominance, value lowering and serialization live in
`rstyle-pdg`. Legacy defaults and matching algorithms are unchanged.

All 23 pinned cases run through the pinned real rustc. The 16 accepted cases
compare exact operation inventories/spans, CFG transfers, dependence multisets,
binding/definition inventories, specified read/IN/OUT definition sets,
literal/operator/role attributes, and merge guard/frontier expectations.
Both documents validate against the pinned schemas, their exact serialized
bytes validate against external manifests, and accepted documents cross
`admit_produced` and the real strict imported boundary. Seven negative cases
check the independently specified eligibility and diagnostic spans.

Capability support is bounded to these independently exercised cases and the
additional tests below. The fixture pack is not exhaustive certification of
all integer widths/operators or of deferred Rust constructs.

| Capability | Verified scope |
| --- | --- |
| scalar interface | integer/bool/unit parameters, unused inputs, return metadata; aliases and mutable parameters additionally exercised |
| local bindings/writes | initialization, shadowing, direct writes, RHS-first primitive compound updates, unit binding definitions |
| scalar computation | source literals, saved read snapshots, tested built-in scalar dispatch and operand positions |
| blocks/returns | source tails/closing braces, explicit returns, unreachable supported suffixes and both-arm returns |
| branching/short circuit | both outcomes, saved left boolean, returning arms, expression merges and nested availability/frontiers |
| reaching definitions | exact fixture read sets and specified per-binding kill/gen IN/OUT sets |
| control/execution | exact reachable postdominance and nearest dominating operation edge multisets |
| deferred constructs | fixture calls, macros, nested items, unnamed/reference interfaces and unit comparisons; additional negative cases below |

## Pinned wire contract versus workspace transport

The pinned PDG schema **requires** policies on edges and known reference counts,
and uses an array of profile-specific conformance assessments. The later workspace
transport uses a root profile and a single assessment. `producer::pinned` emits
the authoritative pinned wire representation and checks every supplied policy
for agreement before converting back to existing transport models. Required
policies cannot be missing. CFG operation provenance also carries the pinned
policy in its permitted extension. No schema or admission requirement is weakened.

## Exceptional flow and evidence limits

Only bounded normal-flow extraction is assessed. Panic/cleanup transfers are
omitted. Arithmetic assessment uses resolved widths, captured overflow settings,
checked constant evaluation and reaching-definition snapshots; each relevant
reachable operation records its checked operands and safety result in a
namespaced extension. The two arithmetic proof fixtures discharge their local
constant bounds. Possible hazards yield partial CFG evidence and specific
limitations. The minimum-i32 literal negation exception is tested.

Unknown constants, including values too large for the current signed-i128 proof
engine, conservatively retain limitations. These are not rewrite-safety or
complete Rust execution claims. Conditional body source without complete
per-source resolution is conservatively unknown, even when rustc removed it;
known unsupported constructs remain ineligible. No supported remainder is emitted.
Missing original source/signature evidence fails extraction explicitly rather
than dropping a function into an eligible empty result.

The handoff verifier recomputes expected normal-flow evidence from captured
compiler facts. Its analyses share producer routines; the exact fixture oracle
is separate and never calls those routines to generate expected facts. Serialized
value-site validation independently enumerates normal paths and recursively
checks availability, frontiers and snapshots. Broader independent semantic
oracles remain a future coverage obligation.

## Additional regression and adversarial tests

`crates/rstyle-driver/tests/hir_extraction.rs` exercises:

- UTF-8 identifiers/comments, a leading BOM/prefix, CRLF, original-byte spans.
- cfg-disabled calls/literals and unresolved removed paths; compiler failures.
- unreachable closures/casts, nested items, async, macros, generic/tuple interfaces.
- aliases, mutable parameters, same/nested shadowing, unit writes/discards,
  both-arm returns and signed-minimum literal negation.
- independent straight-line inventories/operand positions for every eager
  integer operator on i32, addition across all 12 integer types, eager boolean
  operators, all 10 integer compound updates, signed unary negation and
  representative integer/boolean unary not operations.
- repeated serialization determinism under identical captured facts.
- ID renaming and document-array reordering compared to the unchanged baseline
  using known operation/binding correspondence, followed by strict admission.
- wrong manifests, source digests, settings, missing/extra edges/correspondence,
  mismatched/missing wire policies, false capability applicability, definition
  ownership/reaching sets, guards/frontiers and future-predicate authorization.
- actual opt-in driver output through serialization, manifest validation and
  strict admission; rejected functions produce assessments without graph pairs.

## Remaining coverage obligations

Exhaustive operator/type combinations, cross-target builds, all semantic-setting
combinations, complete adversarial definition-set coverage, and a separately
implemented general dependence oracle are not claimed as verified by this initial
pack. The conservative conditional-source evidence gate needs finer resolved
correspondence before cfg-annotated supported source can receive certification.
All-target rstyle also reports test/example violations, including the IO-heavy
compiler harness; default production rstyle is checked separately. No suppressions
or thresholds were changed.

At the producer milestone, source-to-pair orchestration had not started; its
subsequent integration is recorded below. Miner activation, the bounded quality
study, R1, exceptional flow implementation and matching-profile changes remain
outside this work.

## Validation commands and outcomes

- `cargo test -p rstyle-driver --test hir_extraction --locked`: 16 tests passed,
  including all 23 pinned cases (16 exact accepted cases and seven rejections).
- `cargo test --workspace --locked`: passed, including imported admission,
  legacy behavior and the workspace's default style self-test.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo rstyle --format json`: zero production violations.
- `cargo rstyle patterns --format json --fail-on-candidates`: passed with zero
  candidates after extracting shared eligibility traversal and optional graph
  record collection. No suppressions, ignore entries, or threshold changes
  were added.

## Pinned case results

Each row is exercised by the actual frontend and the independent fixture harness.

| Case | Result | Exact source identity |
| --- | --- | --- |
| straightline-integer-input-and-tail | passed exact extraction/schema/admission | `sha256:b5ea3ee7b934b92de243704c845be5e33602f25909a73d509a63ee98926fc7bf` |
| two-arm-reassignment-reaching-set | passed exact extraction/schema/admission | `sha256:64647ff54e9eeaa3f5a74566951bf83412f0f4804fe54fd15138344013c6ce47` |
| short-circuit-and-saved-left | passed exact extraction/schema/admission | `sha256:18a069dc5f46744d06c63c09565b609b332d1a1b5e494a591540ee96d19cc9d5` |
| short-circuit-or-outcomes | passed exact extraction/schema/admission | `sha256:fc9aec7f1ae63f8f3f0aa6340bf5694f245a92f0781d6351237ca7eae3d4a646` |
| early-return-controls-continuation | passed exact extraction/schema/admission | `sha256:4101b865c8a04130768be6409566ec88e7321e66af72d8c423e9a5ac1f742e72` |
| returning-arm-does-not-execute-continuation-declaration | passed exact extraction/schema/admission | `sha256:184714834f2e7a9d2624b329bbe8314d8935b6c2a7aa304721173a80dd322358` |
| if-value-effectful-declaration-site | passed exact extraction/schema/admission | `sha256:e1453e908393cdb99e3d898735c826795a4922d8d2dfd04fdcb3763f7db1d6fa` |
| saved-left-value-before-branching-operand | passed exact extraction/schema/admission | `sha256:17e9d46e1568a3506e30141942330b6d8856726cad12206eaf4816d955a2b3af` |
| nested-if-merges-share-site-with-distinct-availability | passed exact extraction/schema/admission | `sha256:de1b0919218cd72ca8d47996c1cd5c67f3da5d683319ce8cce7068d6ea716623` |
| rhs-first-primitive-compound-write | passed exact extraction/schema/admission | `sha256:fb756ae61c97ce8337efa7f09117d53dd669f713ff02e1d6414754cc2ca9683c` |
| ordinary-binary-old-value-snapshot | passed exact extraction/schema/admission | `sha256:23d35ac8bfcfd0a0a828659767887c16591e568c7ce170f58c3168c7203ca582` |
| parameter-shadow-initializer-reads-old-binding | passed exact extraction/schema/admission | `sha256:69ae96b3dd0149d2f6161493860735acee50241ff27d2eb4b3c097d27bf9c227` |
| empty-unit-body-retains-unused-interface | passed exact extraction/schema/admission | `sha256:d43610f9a2305442c2f11415cdbd0197570660a73b00b1e7bf34929068c07e4d` |
| unit-binding-retains-definition-not-value-operands | passed exact extraction/schema/admission | `sha256:ccbfcb46b35e7f2fdfe55da6e08bb72185acbdf609f2654bf4a49cc34176c194` |
| explicit-unit-literal-return | passed exact extraction/schema/admission | `sha256:659440c6fa903ed945787d923029d7e0e14270fe8b7a24127e52f964dded40a9` |
| unreachable-supported-source-preserved | passed exact extraction/schema/admission | `sha256:7ffcbf92e62fb3852b56e39378b86ebf16c1d15d6d51f76d850665147284eb42` |
| unsupported-unit-comparison-not-builtin | passed rejection | `sha256:b737f403497b6edd37211806b9cdf71836dab2c6a27e330d916f949d45281cff` |
| unsupported-direct-call | passed rejection | `sha256:6188228d385e56a65b828bf3c8045f8f75b4c6dca4697cb48d73c466c0e5dd23` |
| unsupported-call-after-return | passed rejection | `sha256:0b71e167737e91ea9bc35800df1b5d5c1447f9da615b9b544511d8d45cafa278` |
| unsupported-short-circuit-macro | passed rejection | `sha256:cc285b6793c98b2c01f7330a4015053b7ea0638dcabbfaad7fa32144a108cabe` |
| unsupported-nested-item-owner | passed rejection | `sha256:a2a1b388d978d42d8cbc840dd2a07cfa72dd2ef5de02a52f1650ed89441b194a` |
| unsupported-unnamed-parameter | passed rejection | `sha256:bddd05278a459732a1ead8cd34454cf55dd5398d478125294fd6adf8402f7e71` |
| unsupported-reference-interface | passed rejection | `sha256:27edccddb7ea5560998d297506f6e8115211d99b8c887b57d11fb11f08cb78b1` |

## HIR extraction naming and native transport refactor

Implementation names use HIR extraction: `rstyle-extraction-fixtures`,
`HirFnFacts`, `HirNode`, `HirDiagnostic`, and `HirEligibility`. The opt-in
frontend uses `RSTYLE_HIR_FACTS_FILE` and `RSTYLE_HIR_DOCUMENT_DIR` exclusively.
The normative profile identifier remains `compat.rust.hir-r0/0.1.0`.

PDG serialization and decoding now use the native record-policy model directly;
the pinned-wire translation module is removed. Native transport schema tests
use the pinned PDG schema, and imported validation rejects mismatched edge,
reference-count, capability, and provenance policies. Resolved-type extensions
require known string facts; the legacy raw-string accommodation is removed.
The older vendored PDG example is no longer a native transport test input;
independently constructed native documents and actual frontend outputs cover
schema roundtrips against the authoritative schema.

Refactor verification: 437 workspace tests passed, including the 23 pinned
extraction cases. Formatting, all-target clippy, workspace build, and default
rstyle checks passed. Pattern review retained the two existing low-score hints
without suppressions or ignore entries.

## Pattern cleanup

The two remaining parameterization suggestions now share explicit traversal
helpers. `Outcome::check_each` checks every parameter or binding and combines
all diagnostics without an intermediate result vector. `collect_present`
collects optional graph records for imported read symbols and producer
definitions, preserving order and multiplicity. Domain-specific checks and
record construction remain at their original call sites.

The default style check reports zero violations, and the default pattern check
with `--fail-on-candidates` reports zero candidates. No new suppressions or
ignore entries were introduced.

## Opt-in source-to-pair integration

`cargo rstyle clones` now connects captured HIR/source/build facts to exact
serialized-document verification, `admit_produced`, strict imported admission,
conservative projection and exhaustive unordered pair decisions. See
[the Cargo workflow](docs/source-to-pair.md) for selection, identity, evidence,
reporting, resource budgets and exit behavior. Legacy defaults remain unchanged.

Runs use fresh Cargo target directories with incremental compilation disabled,
independent compiler-invocation artifact destinations, Cargo unit-graph planning,
completion ledgers and byte-digest inventories. No evidence cache or prior-run
artifact discovery is used. Source navigation captures real filenames and
file-relative spans; matching Name uses the unqualified source identifier. Global
keys preserve compilation contexts and distinct identical-source definitions.

Real-command temporary-workspace tests exercise supported/unsupported mixtures,
independent feature and gate expectations, small/unit/unused-input functions,
reachable merges, package/module/raw-identifier identity, concurrent compilers and
commands, warm recapture, edits/deletion, stale evidence, selection/build-feature
changes, required-feature targets, failed compilation, missing/corrupt artifacts,
manifest mismatch, exact threshold lexemes, overlap/cosine, explicit resource and
arithmetic failure, deterministic semantic results across compiler scheduling,
traces and fail-on-pairs behavior. Literal edits exercise the deliberately ignored
matching labels; these assertions are not an empirical quality study. Legacy style
and patterns retain their behavior with inherited HIR settings. Conditional source
with incomplete resolution remains unknown, with its evidence gap visible.

Validation on the pinned compiler:

- `cargo test --workspace --locked`: 459 tests passed, including real compiler tests.
- `cargo test -p rstyle-driver --test hir_extraction --locked`: 18 tests passed.
- `cargo test -p rstyle-ccgraph --locked`: 44 imported matching tests passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- Default production style: zero violations; legacy patterns: zero candidates on
  the integration binary, without changing thresholds, suppressions or ignore entries.

Remaining obligations: independently exhaustive cross-target/operator/settings
coverage and the producer obligations listed above remain unclaimed. Compiler
contexts that cannot be distinguished by the initial completion-ledger identity
fail explicitly; they are not silently deduplicated. OS termination cannot publish
a completed report, and interrupted directories are never reused. The bounded
quality study, calibration, held-out evaluation and abstraction mining remain
out of scope. Freeze labeling rules, splits and acceptance targets before that
separate study begins after this integration lands.

## Opt-in local coverage expansions

The pinned R0 ledger above remains historical R0 evidence. Local expansions use
`rstyle.rust.hir-coverage`, not compat approval or an R0 scope correction. See
[the versioned local contracts](docs/extraction-profiles/coverage-v1.md).

| Historical evidence token | Local version | Added capability |
| --- | --- | --- |
| `hir-const-v1` | 0.1.0 | ordinary scalar const bodies |
| `hir-casts-v1` | 0.2.0 | resolved built-in integer casts |
| `hir-match-v1` | 0.3.0 | 1–16 literal/wildcard scalar arms, final wildcard |
| `hir-guards-v1` | 0.4.0 | ordered pure scalar guards |
| `hir-conditional-v1` | 0.5.0 | original owned-source/HIR inventories |
| `hir-loops-v1` | 0.6.0 | local scalar cycles and divergence-sensitive dependence |
| `hir-calls-v1` | 0.7.0 | four pinned primitive instantiations |

Cargo now exposes only `r0` and `hir-expanded-v1`. The latter selects the complete
0.7.0 bundle, including every capability above, and preserves its `hir-calls-v1`
wire token. Intermediate identities remain registered for historical admission and
independent staged regression fixtures; they cannot be selected through Cargo.
R0 defaults and exclusions are unchanged. Matching
remains pinned strict ccgraph with exhaustive unordered pairs, exact thresholds,
overlap defaults and explicit experimental cosine. Local policies and semantic
build contexts must match for comparison; mixed R0/local comparisons fail the
pair compatibility gate. Const qualification alone adds no matching label.

The frozen 60-definition eligibility inventory spans unmodified library targets
of lz4_flex 0.14.0, regex-syntax 0.8.11 and time-core 0.1.9. It measures selected
excerpts, not whole-package coverage. Baseline admitted 0; the final local bundle
admits 5. Conditional correspondence accounts for four measured admissions and
named primitive summaries for one additional admission. Const/cast support was
necessary for some of these functions but its earlier slices alone admitted none.
Loops add no measured admissions in this sample. See
[the inventory report](eval/hir-coverage/REPORT.md) for the measured outcomes, limits and a command to regenerate complete
function assessments, blocker combinations and serialized-document checks.

Removed operations without independent resolution remain unknown. The initial
loop policy excludes expression-value merges in loop-containing functions.
Primitive summaries are limited to the pinned aarch64 compiler/core identities
and i32/i64 unsigned_abs plus u32/u64 wrapping_mul. Nonliteral match patterns,
macros, arbitrary calls, aggregates, references, generics and iterator for remain
excluded. Unsupported unreachable source still prevents certification.

The original ccgraph-v1/v2 studies remain unchanged historical evidence. Their
exposed examples are disclosed regressions. These coverage measurements and
correctness fixtures establish no post-expansion heldout pair quality. A new
preregistered quality study should use untouched project/family holdouts before
making broader quality claims; no such study or HIR abstraction-mining workflow was activated here.

The separate opt-in `hir-data-v1` now has a connected nonnominal data slice:
shared scalar/tuple/array/slice/str interfaces, formal tuple field reads, and
source-verified shared outputs selected by supported branching. A composed
six-function Cargo fixture admits and projects all six functions and enumerates
all 15 pairs, including field reads feeding verified scalar helpers. This is
correctness evidence only. The next connected slice admits concrete nominal
shared-receiver getters, nested field reads and named imports/aliases under a
closed-unit original-source/drop-free proof. Its five-function real Cargo fixture
reaches all ten pairs. Shared field borrows now retain address projections and
conditional borrowed outputs through verified source-to-pair evaluation.
Stored shared-reference fields and by-value reference-bearing struct/tuple
values also preserve nested nominal origins; their seven-function fixture reaches
all 21 pairs. Lifetime-only nominal declarations and inherent impls additionally
reach all 28 pairs in an eight-function module/alias/API fixture. Reference-bearing
array elements, type/const generics and bounded lifetime impls remain unavailable.
The versioned field-context correction now preserves aggregate places and actual
stored-reference loads; extra implicit dereferences lacking read evidence fail
closed. Corrected feature measurements are recorded separately from earlier
disclosed fixture results.
Construction, local reference
correspondence and read-only slice/string operations remain unfinished; the broader
data goal and natural-coverage targets are not achieved. See the versioned local
data contract and progress record under `docs/extraction-profiles`.
