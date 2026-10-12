# CCGraph v2.1 coverage/correctness follow-up

The bounded study completed, but its primary admission and oracle-completion
targets were **not met**. Only one of three heldout source-expected supported
definitions was admitted. No natural labeled pair was evaluable. These results
justify a contract-scope/correctness follow-up, not an abstraction-consumer pilot
or threshold calibration.

Preregistration `d626e7c` predates every detector run. Version 2.1's packaging-only
repair was preregistered at `b35df30`, after a failed smoke compilation and before
any predictions. The reserved wrapper name `macro` became `r#macro`; function
bytes, labels, split, targets and configuration did not change. Original freeze
and invalid run remain preserved. Development results and binary/config freeze
were committed at `b4c643e` before opening heldout. Same-agent source-only review
is disclosed; there was no independent human review or genuine blinding.

## Identities and unchanged production

Reviewed merge: `6a81a3b3edbcb2a209e6ba72b350ccf1425f33c2`. Initial HEAD:
`3d17e78dce130893164af333eb449b2ce5545153`, clean tree; changes since the reviewed
merge concerned v1 study infrastructure/results only. The feature branch is
`ccgraph/coverage-correctness-followup`. Production trees/settings remain identical
to `06d7e8e81653aa0bd53147275555d774985c87dd`, itself identical in these paths to
the reviewed merge. No detector, matching profile, threshold, support boundary,
legacy default, grouping, rewriting or abstraction implementation changed.

Study execution HEAD: smoke/development `b35df30`; heldout `b4c643e` (full IDs in
per-scope results). Producer embedded build revision is
`c22d1751538f01aee98e873c70f4d5188ba56308`; its build/source settings and exact
compiler arguments are retained in representative-traces.json. Binary/config
snapshots include both executable SHA256s, compiler and Cargo identities, harness
and amendment digests, and protocol freeze identity. Producer binary:
`d7689f1b7f8b1c7145e941f9d9af594929bb658c544292dc18298b109bea9f4a`;
orchestrator: `7c2e24c86dc729e8942f570c9a9a3537d5d1a5486f3142ca9a8ecaf0895c01fb`.
Compiler: nightly-2026-09-30, rustc commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`, aarch64-unknown-linux-gnu.
Contract: `04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f`, unchanged vendored rust-r0
artifacts. Primary default overlap and fixed secondary cosine results are stored
separately; effective thresholds remained 4/5, 9/10, 1/2, 9/10.

## Scope, coverage and completeness

Fifteen selected source definitions: nine natural excerpts from four pinned
projects, six constructed coverage controls. Every selected body/signature was
preserved; no rejected definition was dropped. All project versions and contexts
stay in one split. Selection sought scalar source, including historical releases;
it is highly purposeful, not a repository survey, and cannot estimate whole-project
or contemporary Rust coverage. Upstream files, source bytes, excerpt spans,
licenses and every packaging adaptation are committed with SHA256 identities.

Per configuration: 10 selected, observed and assessed compilation units; 30
selected function contexts, all observed and assessed; five additional excluded
compiler-generated test mains, retained separately. Every capture/assessment/pair
completeness flag is true. There are 99 declared selected-context pairs across
five disjoint scopes and 129 observed-function pairs including generated mains.
Seven admitted-context pairs were emitted and evaluated exhaustively, zero
incompatible or failed pairs. Excluded/unknown contexts were not offered to
matching. No cross-project pairs were evaluated. Repeated test compilations stay
in actual output but do not increase canonical source quality sample size.

| Scope / split | Selected source definitions | Selected contexts | Admitted / excluded / unknown contexts | Declared context pairs | Emitted/evaluated pairs |
|---|---:|---:|---:|---:|---:|
| lz4 / development | 2 | 4 | 0 / 4 / 0 | 6 | 0 |
| lexical / development | 2 | 4 | 0 / 4 / 0 | 6 | 0 |
| regex / heldout | 2 | 4 | 2 / 2 / 0 | 6 | 1 |
| time / heldout | 3 | 6 | 0 / 6 / 0 | 15 | 0 |
| audit / development | 6 | 12 | 4 / 4 / 4 | 66 | 6 |

Default and cosine have the same inventory/outcomes. Natural source admission is
1/9 selected definitions (1/4 source-expected supported); development 0/4 selected
and 0/1 expected supported, heldout 1/5 selected and 1/3 expected supported.
These are excerpt denominators, not project coverage. Controlled admission is
2/6 definitions; two unsupported definitions excluded and two conditional-source
definitions unknown as expected. No selected source failed assessment.

## Frozen target outcomes and quality denominators

| Primary bounded target | Result / denominator | Frozen minimum | Outcome |
|---|---:|---|---|
| Natural heldout supported admission, 100% | 1/3 | 3 definitions, 2 projects | Not met |
| Natural heldout bounded oracle completion/agreement, 100% | 1/3 | 3 definitions, 2 projects | Not met |
| Natural heldout unsupported exclusions, 100% | 2/2 | 2 definitions | Met |
| Development pure unknown controls, 100% | 2/2 | 2 controls | Met |

Oracle agreement among actually admitted natural functions is 1/1; two heldout
oracles were unavailable because admission failed. The 1/3 target therefore
measures the preregistered completion-and-agreement requirement, not two erroneous
emitted graphs. Controlled admitted oracles agree 2/2. All results agree across
normal/test contexts and configurations. The checks are narrow inventories of
formal inputs, reads, returns, controls/merges and operators. They do not replace
independent complete dependence-graph correctness proof or the real producer's
serialized-document, frontend and strict-admission verification.

Canonical source pair quality, separately for both scoring variants:

| Population / split | Source pairs | Positive / negative / ambiguous / unlabeled | Evaluable labeled | Non-evaluable | TP / FP / TN / FN | Positive admission / end-to-end yield |
|---|---:|---:|---:|---:|---|---|
| Natural development | 2 | 2 / 0 / 0 / 0 | 0 | 2 excluded | 0 / 0 / 0 / 0 | 0/2 / 0/2 |
| Natural heldout | 4 | 1 / 3 / 0 / 0 | 0 | 4 excluded | 0 / 0 / 0 / 0 | 0/1 / 0/1 |
| Controlled development | 15 | 0 / 0 / 0 / 15 | 0 | 14 | 0 / 0 / 0 / 0 | Undefined / undefined |

Fourteen control pairs are non-evaluable: five exclusion-only, five unknown-only,
four combined exclusion/unknown. The remaining unlabeled control pair is
evaluated and rejected. No canonical unlabeled accepted pairs. All precision,
recall, false-positive rate and F1 metrics are undefined (`null`), not zero.
Positive end-to-end denominators include every selected labeled positive lost
to extraction/admission; yield zero is an actual coverage/yield observation, not
matching rejection. Secondary quality targets are insufficient evidence: the
frozen minimum is 20 evaluable positives, 20 negatives, two provenance groups.
No inferential intervals are used: historical variants and repeated compilation
contexts are dependent, and four projects cannot support a broad quality claim.
Per-project and source-edit strata are in summary.json and per-scope outputs.
There is no pooled constructed/natural headline.

## Error taxonomy and representative evidence

1. **Packaging/infrastructure failure:** first smoke compilation rejected the
   reserved module keyword before capture. It is invalid, never a negative
   prediction; raw report/cost and amendment are preserved. Corrected smoke passed.
2. **Contract-scope discrepancy:** historical lz4 and both historical time helpers
   are rejected solely as “Const evaluation context is outside R0.” The pinned
   profile explicitly rejects const blocks, not const function signatures; source
   review expected these safe scalar function bodies to qualify. `eligibility.rs`
   rejects `signature.is_const` unconditionally. This identifies an unresolved
   scope interpretation requiring authoritative clarification, not proof that
   the producer violates a settled const-function requirement. Frozen expectations
   remain unchanged and fail. Constness was not removed to improve coverage.
3. **Expected unsupported coverage:** current lz4 has casts; current time has casts
   and methods; lexical uses match; regex scalar table uses match and a macro.
   Full original functions stay selected, with capability/reason evidence.
4. **Expected unknown coverage:** pure disabled lets have unavailable complete
   conditional-source resolution evidence; they are unknown, not gate rejections.
5. **Ordinary gate rejection:** scalar/unit control graphs retain 5/2 nodes, giving
   size ratio 2/5 < 4/5. Numerical, strings and WL are NotEvaluated for that pair.
   The pair is unlabeled; it is not a true negative quality observation.
6. **Repeated-context acceptance:** regex's same source compiled normally and for
   libtest yields one accepted detector pair, not an independent clone discovery.
   Projection retains seven OTHER nodes, one distinct read binding, diameter four;
   numerical vectors are identical, Name/IntroPara/ReturnPara similarities each
   equal one, and all weighted WL rounds match (depth four, weights 5,4,3,2,1).
   Both configurations score one. Scalars and unit controls have analogous
   repeated-context acceptances. Three of seven actual pairs are accepted,
   four rejected; none is an evaluable labeled canonical quality pair.

Operators/literals remain extraction evidence but are intentionally omitted from
base WL labels; source identities and variable names do not become clone labels.
This sample provides no causal estimate of name changes: different-name lexical
copies never reach matching. Name/interface evidence for the repeated regex
context is exact, not independent evidence for different functions. No empirical
false positive/negative or name-driven disagreement can be diagnosed when no
labeled canonical pair is evaluable; the report does not invent such traces.
Full canonical functions, admitted CFG/PDG documents and selected stages are
committed in representative-traces.json; every retained raw artifact is linked
through evidence-index.json and independently rechecked against saved aggregates.

## Cost, validation and next milestone

Ten successful study commands took 103.88 seconds combined wall time, with
sampled concurrent process-group peak RSS at most 226.12 MiB and 2,801,851 bytes
in their fresh command run directories. Successful smoke adds 10.89 seconds;
failed smoke cost is separately retained. Every command used a fresh isolated
capture/build directory. Command reporting does not separate compilation/capture
and matching time; combined costs and GNU time CPU/RSS observations are recorded.
The harness's 50ms RSS sampling can miss short spikes. No budget was exceeded.
Ignored target/artifacts directories and binaries stay local; committed digests
support verification, reproduction regenerates machine-specific builds and paths.

Validation: workspace 459 tests; explicit CLI 55; producer/extraction 18 including
23 pinned cases; imported matching 44; Python 22. All passed, with no ignored or
failed Rust tests. Formatting, pinned clippy and build passed. `cargo rstyle`
reported 1208 functions, zero violations; requested legacy pattern hygiene found
zero candidates. It supplies no study predictions or abstraction work. Exact
commands, exit status and log hashes are in results/validation.json. The complete
heldout, natural-source and real-compiler obligations were executed. `summarize`
exits 1 for valid missed primary targets, not compiler failure; raw-evidence
packaging exits 0 after digest/semantic verification. No production repairs made.

Next milestone: resolve const-function scope with the pinned contract authority
and test that interpretation explicitly. If a prediction-affecting repair is
separately authorized, preserve v2 results and use a new preregistration and new
untouched project holdout. Independently verify prospective inputs compile and
have enough contract-eligible natural positive/negative pairs before freezing that
study, without consulting predictions. Current evidence does not justify an
abstraction-consumer pilot or calibration: the limiting factor is eligibility
coverage and a scope discrepancy, not established threshold errors. The exposed
v2 holdout must never be presented as unseen after any repair. No next milestone
is implemented here.
