# Rust support: dependency-ordered parallel delivery

## Mission and baseline

Complete the remaining families in [the tracked feature roadmap](../rust-feature-coverage.md) through approximately four coordinated workstreams. Plan the whole backlog; execute bounded, source-selected families from its dependency-ready frontier. Keep incremental commits and a usable, green integrated system.

This is an executable handoff plan, not an implementation or a claim of an optimal schedule. Optimize expected elapsed time to independently verified natural-code support, accounting for dependencies, uncertainty, integration and validation cost. Replan from evidence.

Planning baseline: **`f71eff3`, Data-v3 `3.59.0`**, observed 2026-10-20. HEAD advanced during planning; recheck it before acting. Do not implement already-delivered work:

- Scalar Options, bounded cyclic merges, Copy nominal construction/updates and the earlier byte-reader families already exist within their documented bounds.
- `3.58` delivered nested borrowed owners, getters, six line-boundary methods and five direct ASCII predicates.
- `3.59` completed the declared ASCII family, including negating/repeated indexed-read helper composition. See [the current contract](extraction-profiles/indexed-read-helpers-v1.md); the earlier ASCII contract's remaining blockers describe 3.58.

Latest **recorded, not rerun for this plan**: 929 workspace tests pass /5 existing skips; rstyle 0 violations /3014 functions; patterns 0 candidates. Native CI for 3.59 is recorded as pending. Frozen inventory: 23 admitted /30 excluded /7 inactive out of 60. Complete package admissions: regex-automata 98/2582, regex-syntax 23/956, lz4_flex 8/150, time-core 11/226. These are reference points, not immutable future expected counts.

The checklist has 117 rows: 30 Covered, 38 Partial and 49 Missing. Rows differ in granularity: do not convert them into a percentage of Rust supported. All 87 Partial/Missing rows have a route below; preserve the 30 Covered rows and their residual limitations.

Paths below are relative to the `rstyle/` repository root. The tracked root `rust-feature-coverage.md` is authoritative; the similarly named untracked `docs/rust-feature-coverage.md` is not automatically current.

## Operating contract

1. **Support families, not isolated node kinds.** Accept an unchanged original function and its required helper chain. An individual feature row can require multiple family deliveries.
2. **Use primary evidence.** Original source, exact build context, raw HIR/unit artifacts, implementation and executable semantic expectations outrank historical summaries.
3. **Preserve distinctions.** Unsupported, unknown, inactive, failed and incomplete are not interchangeable. A missing observation is not proof of inactivity. Never emit a supported remainder as a whole-function result.
4. **Preserve semantic boundaries.** Do not invent purity, alias independence, totality, valid-input preconditions or cleanup behavior. Current partial normal-flow support is not full unwind/drop support.
5. **No green-by-weakening.** Do not change scoring/thresholds, add ignores/suppressions, disable tests, expand budgets to conceal a regression, or weaken import/admission checks. Historical policies must not acquire new capabilities.
6. **No architecture project by default.** Reuse existing unit links, stores, summaries and source proof. Require evidence before adding transport, caches, registries, dependencies or a new analysis substrate.
7. **Only task-owned changes enter commits.** Preserve existing dirty/untracked work. Do not create branches or worktrees unless explicitly authorized. This plan authorizes focused incremental commits during execution, not committing someone else's work.

## 1. Bootstrap once, then maintain a rolling frontier

The lead must do these before dispatching production edits:

- Record HEAD, status, compiler/toolchain, target, build profile/features, available test targets and verification policy. Inventory existing changes without reverting them.
- Read the latest roadmap continuation and relevant contracts. Check whether intervening commits already resolved proposed tasks.
- Establish a baseline with the gates in section 6. Record pre-existing failures; distinguish them from changes introduced by this work. Use a supported runner for pinned historical inventories.
- Start a compact execution record containing: package ID/submilestone, roadmap rows, original source/build identities, blocker DAG, owner/write set, estimated effort/uncertainty, status, commits and retained evidence paths. Use this document plus concise family contracts; do not build a scheduling tool.
- For the first wave, freeze evidence interfaces and exclusive shared-file ownership. Do not begin four incompatible edits to types, origins or eligibility.

Task states: **needs source audit → ready → implementing → awaiting integration → validating → delivered**, or **blocked** with a named prerequisite. Documentation of a blocker is not delivered support.

A family becomes ready only when it has an original acceptance target, a complete *known* dependency chain, independent positive/negative expectations, a bounded semantic contract and an implementable write assignment. Unresolved questions must be named; readiness is not proof that no hidden blocker exists.

## 2. Complete roadmap routing

Each package is a queue of separately reviewable submilestones, not one large commit. IDs P01–P22 are planning keys, not extraction capabilities. Repeated roadmap IDs intentionally identify cross-package seams.

| Package | Remaining roadmap rows | Ordered bounded deliveries and prerequisites |
| --- | --- | --- |
| **P01 Character scalars** | V10, V11; character domains in F04, F06 | Interfaces/literals/escapes and comparisons → literal/OR matching → ranges and one source-required ASCII operation. Separately add source-required builtin bool/char casts with exact rustc-authorized conversions. Preserve Unicode scalar validity and the surrogate gap; char is not interchangeable with u32. No dependency on general Result or generics for a scalar leaf. |
| **P02 Floating-point scalars** | V09, V11; float constants in V12 | Define width/bit/NaN/signed-zero semantics → literal/arithmetic/comparison flow → width conversions/constants → integer conversions separately. Do not route floats through integer overflow/division hazards. Production type-vocabulary edits follow P01 integration for conflict avoidance, not a semantic char prerequisite. |
| **P03 Constants and aliases** | V12, V13, V16; S11 | Exact same-unit alias chains and evaluated scalar constants → nongeneric arithmetic array/repeat lengths and const blocks → bounded reference-free aggregate constants. Use rustc evaluation with independent source/type/value proof. Trait/generic/nonlocal forms wait for the relevant P11/P12 proof; reference-bearing aggregates need P06/P07. |
| **P04 Bindings and never results** | V14, V15, V17; F07 | Initialized Copy destructuring and wildcard evaluation → scalar definite assignment as a separate increment. Reaching-definition union alone does not prove every-path initialization. Separately establish never-result/no-fallthrough interfaces; nonreturning calls require P09 continuation semantics. Non-Copy binding/discard needs P15. |
| **P05 Source and declaration ownership** | S02, S03, S04, S05, S09, S10, S11; X03; D02, D04, D10 | Residual selected path/cfg module and lexical import proof → conditional signatures/unloaded inactive inputs → lifetime-only callable impls and selected owner fields → ordinary nested items. Existing cfg, Copy, local import, direct-alias and nested const-table exceptions are controls, not new work. Subsets are independently deliverable; do not make all of P05 a universal prerequisite. |
| **P06 Read-only helper composition** | D02, D04, D09, D10, D11, D12, D14; C06, C07, C11, C12; X02, X06 | Single projected/stored shared-reference results with exact reads and origins → by-value component read substitution → reference-bearing aggregate helper results. Separately extend proven builtin shared dereference/receiver chains. Selected lifetime/owner cases consume P05 proof; local receiver storage consumes P14. Do not redo 3.59 indexed-read summaries. |
| **P07 Sequences and element origins** | D11, D12, D15, D16, D18 | Full/suffix shared scalar views → inclusive bounds separately → reference-free aggregate element reads → per-element reference-array construction/selection. Preserve saved index/base values, bounds and conditional origins. Borrowed view callers may consume P06; a union of all element origins is insufficient. Str views need explicit UTF-8 boundary proof; mutable views consume P14. |
| **P08 Checked values and pattern flow** | F04, F05, F06, F07, F08, F13; D17, D19 | Saved existing checked consumption → concrete scalar/unit Result construction/tag/payload flow. Independently add scalar Option if-let → let-else → while-let/let chains as separate contracts. Extend finite Copy enum/product patterns, unit/named/multiple payloads and explicit discriminant/tag proof separately. Concrete Option `?` and same-error Result `?` require exact residual/return evidence, not a general trait solver or user-facing if-let first. Reference/slice patterns consume P06/P07; effectful guards need their actual effect contracts. General payloads/moves/generic enums consume the relevant P06/P12/P15 proof. |
| **P09 Loop and callee flow** | F10, F11, F12; C14 | Exact labelled exits and scalar exit values as separate increments → nonrecursive scalar looping-callee summaries → conditional repeated-read summaries. Preserve divergence independently of panic/return. Pinned concrete for-protocol lowering consumes P13's protocol contract; recursion needs an explicit SCC/fixed-point design, not a relaxed acyclic gate. |
| **P10 Registered operations and assertions** | C08, C10, F14; X05 | Add only source-required concrete methods/conversions/assertion forms, one checked semantic contract at a time. Exact selected implementation, source order, failure behavior and compiler/target authority remain required. Custom formatting/trait comparisons consume P12; no generic familiar-name permission. |
| **P11 Cross-unit linking** | C13; X06 | Selected-unit scalar linking → transitive dependency identities/invalidation → existing read-only effects across units. First capture the required unit closure and exact instance/build relationships; removing `local` or invocation checks is not a linker. Can progress independently of general ownership and traits. |
| **P12 Traits and instances** | T01, T02, T03, T04, T05, T06, T09 | Concrete local override/default-body selection; separately decide concrete-instance versus symbolic-generic analysis. Deliver bounded type/const/lifetime substitutions → supported obligations/projections → blanket/supertrait/GAT cases. Pure overloaded operations can use exact concrete dispatch before general generics; Index/Deref results consume origin/receiver proof. |
| **P13 Callables, iterators and abstraction types** | C15, C16, C17; T07, T08, T10; F12 | Saved function items/bounded target flow → proven stored closure environments → one persisted concrete iterator state/protocol → bounded adapters/consumers/for handoff. Borrowed dyn dispatch and exact opaque hidden types are distinct slices. General protocols/binders consume P12; mutable/consuming captures consume P14/P15. A pinned protocol need not wait for every trait feature. |
| **P14 Local storage and mutation** | D13, D20, D21; X02 | Initialized local storage identity/liveness and nonescaping shared borrow → formal mutable/reborrow/read/write chain → field/index/deref writes and memory reaching definitions. Prove address/RHS order, alias uncertainty and justified strong/weak updates. Temporary scopes, escapes and local receiver borrows are later explicit increments. |
| **P15 Moves, drop and exceptional cleanup** | D23, D24, F15 | Whole moves/initialized state → partial moves → normal/early-return cleanup → explicit drop → unwind cleanup/catching separately. Requires storage/validity and path-specific obligations. Existing possible-panic annotations are not cleanup semantics. Coordinate nonreturning/cyclic flow with P09. |
| **P16 Containers and persistent/shared state** | D22, D25, D26 | Independently allow proved immutable static roots; then bounded allocation-owning APIs, growth/invalidation, mutable globals, Cell, RefCell, Rc, Arc and TLS as separate slices. Owning containers consume P14/P15 where needed; synchronized sharing consumes P20. Safe API summaries need not wait for arbitrary unsafe bodies. |
| **P17 Observable effects and ABI contracts** | C18, U03, U04 | Explicit ordered resource effects → one contracted external call; scalar non-Rust ABI separately. Pure pinned intrinsics may use P10; memory/clobber operations need memory/effect proof. Macro provenance and underlying IO semantics are distinct requirements. |
| **P18 Unsafe and representation** | U01, U02, U03, U04; X05 | One bounded raw operation with independently justified preconditions → layout/initialization/alignment/provenance → unions, arithmetic/transmutation or platform effects separately. Preserve unknown preconditions; pointer-bearing FFI consumes this proof. Do not assume accepting an unsafe block supplies any of it. |
| **P19 Suspension** | A01, A02, A03 | Concrete source-to-state suspension/resumption → saved captures/yield/completion → address stability/pinning → cancellation cleanup. Consume P15 where values have cleanup obligations; use targeted evidence feasibility work rather than rewriting the pipeline by default. |
| **P20 Concurrency** | A04; D25, D26 | One explicit synchronization/memory-order protocol → transfer/spawn/join, channels and locks separately. Requires relevant shared effects/ownership, not universal completion of async or unsafe. Sequential dependence edges are not a concurrency model. |
| **P21 Applicability and assurance** | X02, X03, X05, X06, X10, X11, X12 | Carry location/source/build/invalidation obligations with each family. Freeze additional source-only applicability targets separately from the historical sixty. Run a genuinely held-out quality study after support exists; calibration cannot use held-out results. Equivalence/purity/rewrite safety is a separately contracted assurance goal, never implied by admission or clone scores; do not build a rewrite system as an incidental coverage task. |
| **P22 Expansion and generated-source ownership** | S06, S07, S08; S09, X03 | One local declarative item-expansion contract with invocation/definition/output ownership and hygiene → separately one active derive/attribute context. Literal in-package includes can proceed independently once immutable input/owner linkage is proved; build-generated files additionally need producer/input invalidation. Existing builtin Copy/assertion exceptions are not general macro support. Located audit seed: regex-automata `src/util/primitives.rs::index_type_impls!`; generated error declarations are only one prerequisite of checked readers, not a complete unlock. |

### Important hard edges versus scheduling preferences

- Guarded Result payload flow precedes `?` on that Result shape; existing scalar Option permits a separate Option-only path.
- Component correspondence precedes reference-bearing aggregate/array selection; storage identity/liveness precedes local borrowing and lifetime-sensitive callable storage.
- A captured compatible dependency closure precedes cross-unit admission.
- Exact iterator state/yield/cleanup proof precedes a pinned `for` consumer. General trait machinery is not universally prerequisite to a pinned concrete protocol.
- Moves/drop-sensitive operations require ownership/cleanup proof; current Copy/read-only permission cannot substitute.
- Source and build obligations apply to the selected chain. Do not wait for all macros, modules or generics when the selected source requires none of them.
- Character-before-float integration, read-only-before-mutable work and the later-tier order below are scheduling recommendations. Promote a demonstrably independent ready family when evidence justifies it.

## 3. Four-agent first wave

Use four subagents plus the parent as coordinator/integrator; the parent should not duplicate their implementation. These are initial assignments, not four permanent architecture silos. B/C/D have explicit source-audit gates because planning did not establish a cheap original positive for each. Do not invent one to fill a slot.

| Agent | Initial bounded assignment | First acceptance target / readiness gate | Exclusive implementation focus |
| --- | --- | --- | --- |
| **A — character leaf** | P01: char interfaces, literal decoding, scalar comparisons and literal/OR/wildcard matching needed by the original leaf. | Located original: regex-syntax **0.8.11**, `src/lib.rs::is_meta_character(char)`. Freeze exact source/build identity and independently enumerated truth table before admission measurement. Calls/Unicode APIs are outside this slice. | Driver char type/literal observations; dedicated PDG character semantics/checks; family-specific tests/oracle. No float activation. Shared type, eligibility and policy edits go through the lead. |
| **B — saved checked consumption** | P08: consume a saved existing checked-sub Option with exact saved value, tag/payload and possible-panic semantics; pinned conversion Result save is a subsequent increment. | Existing regression seeds: `eval/coverage-development/end-of-input.rs::saved_result` and `tests/variant_capture/end_of_input/boundaries.rs` under cargo-rstyle. **First locate/freeze the natural caller**, or return its complete blocker chain before activation. Saved slice-get/constructed scalar Options already work; do not reimplement them. | `crates/rstyle-pdg/src/bitset_cardinality.rs`, its implementation/verifier subtree and dedicated saved-consumption tests. No general Result vocabulary, pattern capture or helper-origin changes in this increment. |
| **C — borrowed-result composition** | P06: a single shared result from a projected formal field or stored-reference load, preserving read effects separately from return origins; getter → wrapper → consumer. | **First select a tractable original chain.** Located `regex-automata 0.4.18 src/util/search.rs::Input::{haystack,get_span,start,end}` is an audit candidate, not a promise: lifetime-only impls, reference-bearing owner fields and Anchored variants are extra blockers. If those dominate, record dependencies and select a smaller original chain. | `helper_references.rs`, `helper_views.rs`, associated helper result checks and dedicated tests. Preserve 3.59 helper-index scopes; do not change sequence locations or payload semantics. |
| **D — source ownership unlock** | P05: prove one residual in-package path/cfg-selected module family, identifying the smallest shared source-ownership blocker that gates useful real code. | No natural positive was frozen during planning. **Locate and freeze one before implementation.** Existing path-module rejection tests are controls, not proof of natural support. Audit real cfg, loaded buffers, path ownership, shadowing and orphan/ambiguous files. | `nominal_boundary/census.rs`, selected module/import ownership and dedicated source tests. Do not alter general macro/derive semantics or change original module paths to obtain admission. |

#### Dispatch protocol

1. Each agent first returns its exact target, known blocker graph, scoped contract and proposed file write set. A can proceed after its target audit; the others need not delay it.
2. The lead approves ready tasks, resolves shared interfaces and records shared-file patch ownership. If a proposed seam changes types/locations/effects, decide its policy/import/matching consequences before implementation.
3. Agent B or C discovering a hard dependency on D must not independently patch D's source proof. D may reprioritize to the demonstrated shared unlock; document the change. Other independent work continues.
4. If a source audit finds the assignment unsuitable, return a bounded artifact immediately and pull another ready family. No open-ended browsing, ungrounded activation or unrelated feature implementation to keep busy.
5. D may prepare P02's already-located float acceptance contract/oracle after the source audit if its implementation is blocked. Original libm **0.2.16** `src/math/{k_sinf,k_cosf}.rs` require float arithmetic **plus constants and f64→f32 conversion**; all are part of that chain. `fdim`/`fabs` delegate to generic implementations and are not simpler substitutes.
6. After a foundation-only milestone with no natural gain, reassess the chain before stacking another. Useful foundations are allowed, but must name the concrete downstream acceptance target.

## 4. Rolling execution order after the first wave

These are readiness windows, not global barriers or fixed-duration sprints. Finished agents pull ready work; independent later tasks may start early. Never wait for an entire broad package to finish when a bounded prerequisite is sufficient.

| Window | Scalar / resolution frontier | Payload / flow frontier | Data / calls frontier | Source / foundations frontier |
| --- | --- | --- | --- | --- |
| **1** | P01 character leaf | P08 saved checked consumption | P06 single borrowed-result chain | P05 demonstrated source unlock |
| **2** | Character range/ASCII caller, then P02 float chain with agreed vocabulary; P03 constants as required | Concrete scalar/unit Results; Option if-let and let-else as separate slices | P07 full/suffix scalar views, aggregate elements and component-specific results | P03 alias chains; P05 conditional signatures, callable lifetime-only impls, nested items |
| **3** | P12 concrete trait override/default; bounded instance identity/substitution decisions | P09 labelled/value exits and scalar looping helpers; pinned concrete `?` when its evidence is ready | P14 nonescaping local storage; P11 scalar cross-unit linking | P05/P21 source/compatibility gaps demonstrated by selected targets |
| **4** | P12 bounds/projections/operators and further generic/const/lifetime slices | P13 saved callables, concrete iterator state/consumer; P09 for handoff | P14 mutable-formal read/write and reborrow chain | P22 bounded declarative expansion ownership, then independently justified derive/include/generated inputs |
| **5** | P12/P13 dynamic/opaque/binder slices as prerequisites permit | P15 moves and path-specific cleanup, then unwind | P16 owning containers, persistent state and interior mutability in distinct slices | P17 observable-resource/ABI contracts; P18 bounded provenance/layout work |
| **6+** | Remaining generic/dispatch/source boundaries | P19 suspension and cancellation | P20 synchronization and shared-state compositions | Broader P17/P18 platform contracts; P21 held-out assurance milestones |

For each frontier choice, report: **which complete originals become possible, which later tasks it unlocks, uncertainty/effort, expected validation cost, and conflicts avoided**. Prefer shared prerequisites when their downstream value is evidenced. Do not use a fabricated numeric benefit score or maximize agent utilization at the expense of the critical path.

P04 binding/never, P10 APIs/assertions and P21 compatibility/applicability work are continuously eligible side frontiers when a selected original requires them. They are not forgotten merely because the window table abbreviates them.

## 5. Ownership, interfaces and incremental commits

### Shared-file integration

The lead owns edits to shared registration/semantic choke points unless explicitly handed off to exactly one agent:

- Driver `hir_extraction/hir.rs`, `source.rs`, shared concrete-type transport and module/test registration.
- PDG `eligibility.rs`, `data_types.rs`, `concrete_types.rs`, `data_frontend.rs`, `source_evidence.rs`, `data_origins.rs`, `field_locations.rs`, `data_documents.rs` and shared origin checks.
- PDG `producer/lower.rs`, `producer/hazards.rs`, `extraction_profile.rs`, `lib.rs`; CCGraph admission/compatibility/projection changes.
- Shared CLI test registration, root roadmap and final delivery evidence.

Feature-specific leaf modules, tests and oracles should have one owner. The list above is a coordination rule, not permission to split a semantic change into unverifiable pieces. Small interface changes can be handed off serially. Review both compile-time interface compatibility and semantic assumptions about snapshots, guards, origins, effects, panic/divergence and build identity.

### Commit rhythm

- **Contract/oracle:** freeze original source/build identities and independently specified outcomes; production behavior unchanged.
- **Necessary evidence/proof:** add tested observation or verification prerequisites; leave unsupported paths closed until connected. Do not make this an endless sequence of capture-only commits.
- **Connected activation:** commit the coherent producer, independent verifier, policy gating and composition tests together when appropriate.
- **Integration/evidence:** record completed gates, measured natural/frozen deltas and remaining limitations. Update only affected roadmap rows with the narrow demonstrated boundary.

Use explicit paths when staging; never `git add .` in the existing dirty tree. One coordinator commits in a shared checkout, after reviewing the exact diff. Agents return owned diffs, checks and proposed commit boundaries; they do not concurrently stage/commit. Shared global formatting must run against a quiescent checkpoint, not reformat another agent's in-flight changes.

A focused worker commit need not rerun every full natural capture. It must compile and pass its relevant focused tests, retain fail-closed behavior and identify pending integration gates. Only a fully gated checkpoint is called green/delivered. If integration fails, fix/revert only this task's changes; preserve unrelated work.

## 6. Validation and resource budget

### Development loop

Start with the smallest affected Rust tests and real-Cargo composition fixture. Use independent source-level expectations, not output copied from the new producer. Producer/verifier agreement can share a bug; a second agent's approval is review, not an oracle.

Important negative cases depend on the family: changed value/type/source authority, wrong saved operand, removed payload guard, alias alternatives, later argument rebinding, wrong read selector/call scope, invalid index, stale loop value, incorrect exit target, mutated dependency/build context, and historical policy downgrade. Recompute ordinary integrity seals in tampering tests so rejection actually exercises semantic checks.

### Integrated checkpoint gates

Run from the repository root with a stable tree/binary set:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_TARGET_DIR=target ./scripts/test.sh --nextest
CARGO_TARGET_DIR=target ./scripts/style.sh --format json
CARGO_TARGET_DIR=target ./scripts/style.sh patterns --fail-on-candidates --show-ignored
git --no-pager diff --check
```

The existing nextest wrapper prebuilds the driver and runs doctests separately. If nextest or a required toolchain/target is unavailable, report the missing prerequisite; do not silently substitute a smaller check and call the gate passed.

Require zero rstyle violations, zero **unignored** pattern candidates, no new ignores/suppressions, formatting clean and Clippy with warnings denied. Existing ignored entries remain unchanged and visible. Full gates run at integrated family checkpoints and final handoff, not concurrently after every leaf edit.

### Family activation evidence

- Fresh real Cargo capture of the **unchanged complete original context**; a copied minimal fixture is a diagnostic tool, not natural-crate acceptance.
- Independent document/producer verification and **strict** import/admission of retained bundles. The normal CLI's `trusted-generated` policy alone is not this evidence.
- Complete selected pair enumeration/evaluation; matching acceptance or rejection must meet independently fixed expectations, not require every admitted pair to match.
- Independent runtime/value/order/bounds/panic expectations and applicable compiler/target/std/no_std/settings checks. Native CI execution and cross-target capture are different claims.
- Same selected original identities/build contexts before and after; inspect unexpected gains as well as losses. Historical frozen inventory remains unchanged in membership/denominator, not necessarily in admission counts.
- Additional natural targets live in separate versioned selections with predeclared expectations. Do not replace the frozen sixty or call development fixtures holdouts.
- Keep budget-limited full-library matching explicitly incomplete; a zero-pair roster assessment does not validate matching completeness.

### Resource control

- One lead schedules heavyweight captures, strict owning-unit replays, performance runs and full gates. Default to one large natural capture at a time: recent regex-automata runs are roughly 15 minutes and 6 GiB, based on recorded measurements, not a guarantee.
- Do not overwrite/rebuild driver or CLI executables while tests/captures use them. Use stable binary snapshots and separate build directories when running independent code revisions; otherwise serialize builds/captures. Native subagents share the working tree, so separate directories alone do not isolate source revisions.
- Bound commands with timeouts. Report timeout, memory pressure and arithmetic/resource incompleteness, not success. Choose runnable small controls for iteration instead of enlarging budgets to finish a pathological case.
- For performance claims, compare identical workload, compiler/dependency locks, runner, build profile, cache state and concurrency. Use warmup plus alternating repeated baseline/current samples; report elapsed ranges, peak memory and supported-work counts separately. A larger admitted set is not an equal-work speed regression or speedup.
- The pinned historical inventory has host/compiler requirements. Inspect `eval/hir-coverage/PROTOCOL.md`, `README.md` and `reproduce.py`; use the supported runner instead of rewriting a Linux context to fit macOS. If unavailable, retain that acceptance item as pending and run only clearly labelled local checks.

## 7. Completion and handoff

A submilestone is delivered when its original acceptance chain passes section 6, negatives remain appropriately unavailable, evidence is retained, and the roadmap describes exactly what became supported. A broad row becomes Covered only if its stated scope is actually met; refine broad scopes explicitly rather than silently shrinking them.

Report after each integrated wave:

1. Baseline/current revisions and incremental commits.
2. Delivered original families and roadmap submilestones; exact gains/losses on unchanged selections.
3. Test/oracle/strict-admission/selected-matching results, all quality gates, pending native/host checks and bounded timings.
4. Newly discovered blockers and updated hard versus tentative dependencies.
5. The next four concrete assignments with exclusive write sets and readiness gates.

Continue across the roadmap through this rolling process. If the session ends, leave the dependency frontier, task states, evidence paths and next commands actionable. Do not describe broad Rust coverage, held-out detector quality or rewrite safety as complete merely because more functions admit.

## Start instruction for the executing agent

Read this plan and the latest tracked roadmap. Reconcile HEAD and user changes first. Establish the baseline and dispatch the four assignments in section 3 with their source-audit gates and disjoint ownership. Implement ready tasks, make focused incremental commits through the coordinator, run the integrated gates, then update the frontier and continue. Do not stop at a planning-only response unless execution is blocked by genuinely unavailable evidence, tooling or authorization.
