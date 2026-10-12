# CCGraph bounded quality study v1 — preregistration

Question: does the integrated opt-in strict detector retrieve source-reviewed
whole-function structural edited copies and reject independently different
recipes, within this deliberately small selected source scope? Positive means
whole-function structural clone relatedness, not equivalence, safe refactoring,
common problem identity, matching names, or agreement with this implementation.
No abstraction consumer, mining, grouping, proposals or rewriting is authorized.

## Source labels and limits

Labels in manifest.json were authored and reviewed from source before any study
run, by Codex in a separate source-only pass by the same agent. This is not
independent human review or genuine blinding. Published profiles and integration
assertions were previously read; conceptual overlap is disclosed. No scores,
gates, extracted features or accepted pairs from these study inputs were read.
No unresolved reviewer disagreements; uncertain partial correspondence remains
ambiguous or unlabeled. These are model-authored labels subject to review bias.

Positive: documented whole-function edited-copy correspondence preserving the
operation/dependence/control recipe, including exact copies, formatting/comments,
alpha renaming/shadowing, name changes, operator/literal/branch outcome changes,
and primitive-width variants. E.g. `a & b` versus `a ^ b` can be structural clones
with different behavior. One-arm minimum versus maximum selection is positive.
Small unit-body expression addition is a minor structural edit and positive.
Negative: source review establishes a different recipe/dependence arrangement;
similar names, counts or shared utility purpose are insufficient. Sign extension
(cast then shift) versus inverse extension (shift then cast) is negative despite
similar source shape. Partial two-bound versus one-bound clamp is ambiguous.
Unknown correspondence has no binary label. No behavioral difference alone
makes a negative. Secondary attributes explicitly retain semantic changes,
function-name changes and unassessed equivalence/refactoring suitability.

## Frozen scope, inputs, splits and leakage

Exactly 37 selected source definitions, four disjoint exhaustive command scopes:
controlled-development 15, controlled-heldout 15, natural-development 3,
natural-heldout 4. All targets includes normal and test library compilations:
expected 30, 30, 6 and 8 detector functions in 4, 4, 2 and 2 units respectively.
The complete declared context universes are 435, 435, 15 and 28 pairs; each run
emits every unordered admitted pair, including cross-context pairs. Cross-scope
pairs are not evaluated. Quality uses only normal (test=false) contexts:
105, 105, 3 and 6 source pairs. Repeated compilation contexts remain in raw and
per-pair evidence but are not independent quality observations. One-to-one mapping
uses actual source filename, half-open byte span, package/target and test context.

All definitions/variants/module/package copies from a controlled family stay in
one split. Natural split is by project: pinned num-traits development and pinned
byteorder heldout. No pair-random split. Whole upstream files, selected byte
ranges, exact excerpts, licenses and adaptations are recorded. Select first three
clamp helpers and first four sign helpers by source order, before predictions.
No conditional exclusion or replacement after extraction. Convenience sampling
favors standalone helpers; excerpt coverage says nothing about whole projects.
All source digests, labels, scopes, split membership, build scaffolding/configs
and this protocol are frozen in freeze.json. Vendored rust-r0 contracts at
04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f remain unchanged; older snapshots
are not used. No known integration fixture is held-out evidence.

## Configurations, identities and execution

Primary: unchanged pinned defaults: size ratio 0.8, numerical 0.9, strings 0.5,
final 0.9, smaller_graph_overlap. Secondary: same thresholds, fixed experimental
cross_product_cosine. No calibration, ablation or threshold adjustment. Preserve
development for analysis and future separately scoped calibration. Build with
nightly-2026-09-30, locked debug binaries at detector source revision
06d7e8e81653aa0bd53147275555d774985c87dd. Its tree is identical to reviewed
merge baseline 6a81a3b3edbcb2a209e6ba72b350ccf1425f33c2. Study-only commits
must not change production source. Record binary hashes, compiler/Cargo versions,
profile/artifact hashes, git revision, inputs, settings and full report identities.

Use `cargo rstyle clones --format json --all-targets --locked --offline --jobs 2
--max-pairs 1000 --trace --matching-config CONFIG --manifest-path MANIFEST`.
Real compiler source capture → paired documents → external manifest verification
→ frontend verification → admit_produced → strict admission → projection → pairs.
Fresh command-owned isolated target directories; no study evidence cache or scans
of old artifacts. Smoke only the development scope. Then run both development
configurations, diagnose, freeze binary/config identities, and run each heldout
scope/configuration exactly once. Preserve every successful run, even unfavorable.
Infrastructure retry records cause and identities; never silently discard results.
A detector defect preserves the result and requires follow-up/new evaluation
version; original holdout is exposed. Only harness repairs may retain v1.

Per command budget: 180 seconds wall, 1000 exhaustive pairs, 2 jobs, 2 GiB maximum
reported process-tree RSS when available, 1 GiB fresh run directory. Measure wall
and command CPU/max-RSS via GNU time if available (its max-RSS is maximum child,
not simultaneous process-tree RSS); unavailable measures are null/disclosed, not
zero. Budget overflow invalidates the run. Compilation and matching are combined
if existing command offers no separate timing. Fresh recompilation stays visible.

## Metrics, denominators and targets decided before results

Populations and splits reported separately; no pooled headline. Canonical normal
contexts define quality. Report selected/observed/assessed/admitted/excluded/
unknown/failed functions and units with reasons; all context inventory counts;
positive/negative/ambiguous/unlabeled pair counts; evaluability by reason; TP FP
TN FN among binary-labeled evaluated compatible pairs. Ordinary gate rejection
is a negative prediction; extraction/admission exclusion and incompatible pairs
are non-evaluable. Failed arithmetic/resource/verification/capture/mapping/report
execution invalidates its scope and cannot become a negative prediction.

Precision=TP/(TP+FP), recall=TP/(TP+FN), FPR=FP/(FP+TN),
F1=2TP/(2TP+FP+FN); zero denominator is null. Positive admission coverage is
both-functions-admitted positives / ALL selected labeled positives. Positive
pair evaluability also reports incompatible losses separately. End-to-end yield
is accepted labeled positives / ALL selected labeled positives, including
extraction/admission losses. Unlabeled accepted pairs are not FP. Break down by
frozen edit stratum. Family/project dependence precludes binomial pair CIs;
report descriptive per-family/project support and rates, no independence-based
interval or real-world generalization. Macro per-scope metrics are not a headline.

Heldout primary acceptance (each population separately): at least 15 evaluable
positives, 30 evaluable negatives, 3 independent provenance/family groups with
evaluable binary labels; precision >=0.90, recall >=0.80, FPR <=0.10,
positive admission coverage >=0.80, end-to-end yield >=0.64. Targets prioritize
avoiding unrelated proposals (90% precision), retaining most edited copies (80%
recall), and visibly limiting coverage losses; 0.64 is 0.8*0.8, not a safety
claim. Minimum samples are modest descriptive evidence, not statistical proof.
Natural population deliberately small and may necessarily yield insufficient
evidence; this is a bounded feasibility sample, not justification to pool it with
constructed evidence. Minimum groups avoid declaring success from one recipe.
Targets apply only to default heldout; experimental target comparisons secondary.

Each target: insufficient evidence if minimum evaluable positives/negatives/groups
not met or its denominator undefined; otherwise met/not met by frozen inequality.
Coverage/yield also report observed numerical pass/fail irrespective of minimum
matching sample, explicitly distinct from quality acceptance. Study completion is
not target attainment. A valid insufficient or poor result remains published.

## Amendments, retention and harness exits

Preregistration distinct commit before first detector run. Changes to frozen
inputs/labels/splits/configs/targets invalidate v1 heldout; new version/new untouched
holdout required. Record harness-only amendments and tests, retaining original
outputs. Freeze evaluated binaries before heldout access; drift invalidates.

Commit protocol, digests, corpus/licenses, harness/tests, semantic per-function and
all emitted per-pair data, aggregates, representative traces, identities, timings
and report. Raw reports and documents retained in ignored study artifacts with
hash inventories; fresh target trees ignored/reproducible, not committed. Raw
artifact absolute paths are navigation evidence, not portable identities.
Harness exits: 0 valid target met; 1 valid target miss/insufficient evidence;
2 invalid/incomplete execution. Validation-only commands may return 0 independently
of target outcome. Never call a quality miss a compiler failure.

Next milestone decision after results: abstraction-consumer pilot only if both
populations justify it; coverage/correctness follow-up for exclusions/defects;
otherwise separately preregistered calibration with a new untouched holdout.
Do not implement the next milestone here.
