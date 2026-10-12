# Bounded CCGraph quality study v1

The study executed completely; it does not establish natural-source detector
quality or justify an abstraction-consumer pilot. Constructed heldout targets
were met, while natural-source targets have insufficient evidence. Recommend a
separately specified coverage/correctness follow-up before considering a pilot.
No detector, matching profile, extraction boundary, threshold or legacy default
changed. No next milestone was implemented.

## Identity and independence

Complete preregistration commit: 25e23fa, before first study command; frozen
protocol/input identity e1a9b49d94a5c507a9d6d1bf7b0b860f36df05469f9c66e06b0b53c22dd92124.
Initial partial preregistration 400165b was completed after fixing a license-path
fetch failure, before any detector run. Evaluated production source revision
06d7e8e81653aa0bd53147275555d774985c87dd has the identical full source tree
f72a7a6976bc8faf4402f8cf7d61ea9063d8fbad to reviewed merge baseline
6a81a3b3edbcb2a209e6ba72b350ccf1425f33c2. HEAD initially differed only in
merge ancestry, not source. Baseline object recovered over HTTPS after SSH host-key
failure. Initial worktree clean; branch ccgraph/bounded-quality-study created.

Actual final producer build metadata is
3dc1c5bfb1815d58325a0a98bb1fe47b43d874b0+dirty, with only study changes dirty;
production tree/settings were checked unchanged. Do not mistake producer metadata
for the baseline git revision. Exact executable hashes, compiler/Cargo identities,
source/config/contract identities and build scope are in
results/development-binary-freeze.json and results/evidence-index.json. The pinned
compiler is nightly-2026-09-30; compat authority is
04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f, using unchanged vendor-compat/rust-r0.
The older vendor snapshot was not used. Final heldout study commit was 8763e8b.

Source-only labeling and second review pass were performed by the same Codex
agent, with no study detector output. This is not independent human review or
genuine blinding. Existing integration examples/profile were read beforehand;
new examples have conceptual overlaps, disclosed in the protocol. No existing
integration outcome was presented as unseen quality evidence. Model-authored
rationales and limited whole-function edited-copy rules are a material limitation.

## Quality, coverage and yield

Default overlap primary; fixed experimental cosine secondary. Results are
identical in this corpus under both configurations; this does not imply scoring
variants agree elsewhere. Report populations/splits separately. Canonical normal
compilation contexts are quality observations; test compilations are retained
without being counted as independent observations.

| Population / split | Source pairs P/N/A/U | Evaluable P/N | TP/FP/TN/FN | Precision / recall / FPR / F1 | Positive admission / end-to-end yield |
| --- | --- | --- | --- | --- | --- |
| Controlled development | 22/82/0/1 | 22/56 | 19/0/56/3 | 1 / .864 / 0 / .927 | 1 / .864 |
| Controlled heldout | 22/82/0/1 | 22/56 | 22/0/56/0 | 1 / 1 / 0 / 1 | 1 / 1 |
| Natural development | 1/0/2/0 | 0/0 | 0/0/0/0 | undefined / undefined / undefined / undefined | 0 / 0 |
| Natural heldout | 2/4/0/0 | 0/0 | 0/0/0/0 | undefined / undefined / undefined / undefined | 0 / 0 |

P/N/A/U are positive/negative/ambiguous/unlabeled. Each controlled scope has 27
non-evaluable source pairs (26 negative, one unlabeled) due to excluded functions.
Natural development's three pairs and natural heldout's six pairs are excluded.
Natural admission/yield denominators include ALL selected positives (1 and 2),
so zero yield is defined even though matching recall is undefined. No excluded
pair is FN or TN; no incompatible or failed pairs occurred in final valid runs.
No ambiguous/unlabeled accepted pairs occurred. No FP occurred; none can be traced.
By-stratum and overlapping per-family/project results are in results/summary.json.

| Scope | Selected / observed units | Selected source contexts / observed source contexts | Admitted / excluded source contexts | Extra excluded generated functions | Declared context pairs / emitted / evaluated |
| --- | --- | --- | --- | --- | --- |
| Controlled development | 4/4 | 30/30 | 26/4 | 2 | 435/325/325 |
| Controlled heldout | 4/4 | 30/30 | 26/4 | 2 | 435/325/325 |
| Natural development | 2/2 | 6/6 | 0/6 | 1 | 15/0/0 |
| Natural heldout | 2/2 | 8/8 | 0/8 | 1 | 28/0/0 |

All observed units assessed (4,4,2,2 respectively). All observed functions assessed: 32, 32, 7, 9 respectively. Selected source
functions per scope: 15,15,3,4 (37 distinct source definitions). Unknown/failed
function counts were zero in final runs. Generated libtest main functions have no
selected source definition and are separately retained as excluded. The full
observed-function universes are 496,496,21,36, distinct from frozen selected
universes. Every admitted unordered context pair is retained; no cross-scope
pairs were run or claimed. Same-source module/package/test copies are distinct
reported keys. Repeated contexts are not independent quality samples.

Each controlled scope's unsupported method-call and conditional-source sentinel
is excluded. The intended unknown sentinel also contains a known unsupported
cfg-disabled call; the combined classification is excluded, with the conditional
resolution evidence gap preserved. This study therefore does not empirically
exercise a purely unknown classification, despite including an unknown-evidence
sentinel. It was not replaced after seeing that result. Unknown accounting is
unit-tested; broader unknown coverage remains a follow-up, not a passing claim.
Natural num-traits excerpts retain generics/macros/overloaded primitive-unknown
interfaces; byteorder excerpts retain casts. All remain outside R0. No excerpt
was rewritten for admission. Whole upstream revisions, paths, bytes, selected
ranges, adaptations and MIT/Apache-2.0 or MIT/Unlicense notices are committed.
Convenience selection of adjacent standalone helpers assesses excerpts only.

## Frozen acceptance outcomes and uncertainty

Controlled default heldout: 22 evaluable positives, 56 negatives and 3 independent
recipe families satisfy frozen minima 15/30/3. Precision >=.90, recall >=.80,
FPR <=.10, positive admission >=.80 and yield >=.64 are all **met**.
Natural default heldout: 0 evaluable positives/negatives/groups; all acceptance
targets are **insufficient evidence**. Its descriptive numerical admission/yield
values fail .80/.64; do not call that successful quality acceptance. The original
natural selected size was already too small for minimum quality evidence; this
was explicitly a bounded feasibility sample. Targets were not lowered. Secondary
cosine has the same descriptive outcomes and is not a primary target decision.

Variant pairs share derivation histories; negative pairs span overlapping families.
There are only three constructed families per split and one natural project per
split. Pair-based binomial intervals would overstate independence, so results
are descriptive counts/rates with per-group support, without such intervals.
No broad Rust, real-world quality, equivalence or refactoring safety is established.
No calibration was performed; development remains available for future separately
scoped work. Holdout is now exposed and cannot be reused as unseen evidence.

## Contract correctness and representative explanations

Regression conformance is separate from quality. Pinned profiles, schemas,
fixtures, source-to-pair implementation, producer verification, strict admission,
projection, nine features, sequential gates and overlap/cosine scoring were
inspected. Existing independent extraction/imported fixtures and real compiler
regressions pass. No contradictory contract defect was found in this bounded
inspection; this is not exhaustive certification. Producer handoff recomputes
using shared producer analyses; independent fixture oracles and serialized path
checks remain important limits on the strength of verification claims.

Representative raw stage/projection/WL partition evidence is in
results/representative-traces.json:

- TP shadow_base/shadow_rename: identical 12-node projected recipe; Name=4/9,
  both interfaces=1, string mean=22/27. Numerical cosine is 1, nine WL rounds
  overlap completely at depth 8, final=1. Lexical alpha renaming/shadowing preserves
  the source recipe; navigation names are not identity or initial graph labels.
- TP short_base/short_semantic: changing &&/|| changes behavior but remains a
  documented structural copy. Eleven retained nodes, identical numerical counts,
  full WL overlap at depth 4. Operator and branch-outcome facts are intentionally
  omitted from base labels. This is not behavioral equivalence.
- TN short_base/merge_base: both have 11 projected nodes. Numerical dot=221,
  squared norms=228 and 220 passes .90; interface/string mean approximately .772
  passes .50 despite different primitive interfaces and names. Shared label counts
  fall from 10 at round zero to 4,1,0,0; weighted overlap=23/55, below .90. Directed
  control/data/execution arrangement distinguishes these unrelated source recipes.
- FN development empty_base/empty_semantic: adding explicit unit tail is a small
  source edited-copy positive. Projection produces 2 versus 3 nodes; size ratio
  2/3 fails fixed 4/5. Later numerical/string/WL stages remain unevaluated.

Matching label disagreement is not contract nonconformance. No ignored semantic
fact was added to make a negative reject. Regression fixtures and synthetic
harness reports were never substitutes for these real-command runs.

## Completeness, failures, cost and validation

All eight final preregistered runs completed capture, assessment, enumeration and
evaluation; exact inventories verified. Heldout scopes/configurations executed
once each after development diagnosis and committed binary/configuration freeze.
Earlier invalid smoke mapping and natural-development binary drift were preserved;
successful original controlled runs (including FN) remain in prior-development.
AMENDMENTS.md explains the harness-only entrypoint repair and infrastructure
restart. No unfavorable successful result was discarded or tuned away.

Eight final runs used 91.783 s combined wall time and 94.13 s command user+system
CPU. Each cost 10.405–12.945 s fresh compile/capture/matching; these costs are not
cached-away. Sampled concurrent process-group peak RSS ranged 126.6–248.3 MB,
with 50 ms sampling (brief peaks can be missed); GNU time maximum-child RSS is
separately recorded, not mistaken for simultaneous aggregate memory. Fresh run
directories occupied approximately .46–5.44 MB each. All budgets passed (180 s,
2 GiB sampled RSS, 1 GiB run disk, 1000 exhaustive pairs, 2 jobs). Existing reporting
has no phase timers, so compilation/capture cannot be separated from matching.
All attempts, including infrastructure failures, smoke and repeated development,
are inventoried with actual cost in results/attempts.json. Validation/build costs
were not separately measured and are not included in these study timing totals.

Validation: workspace 459 tests, CLI/orchestration real-command suite 55,
producer/extraction 18 (all 23 pinned cases), imported matching 44, Python harness
15, all passed with zero ignored tests. Formatting and pinned all-target clippy
passed. Default cargo rstyle: 1208 production functions, zero violations. Requested
legacy patterns hygiene: zero candidates. Neither legacy output supplied study
predictions. Exact commands/results are in README.md and results/validation.json.
No required compiler/source/dependency obligation remains unavailable. Python/GNU
time were installed for the harness; pinned compiler was already installed.

`python3 scripts/ccgraph_study.py summarize` returns **1**: valid completed study,
insufficient natural quality evidence. Invalid execution is **2**; all targets met
would be **0**. Execution-only subcommands return 0 independently of quality.
Machine-readable results retain all pairs, exclusions, null undefined metrics,
strata, family/project support, identities and cost. Raw bulky HIR/documents/run
artifacts remain ignored and retained locally, identified by committed digests;
no arbitrary target directories are committed. See README.md for reproduction.

## Next milestone recommendation

A coverage/correctness follow-up is warranted: characterize representative
naturally occurring R0-compatible source and the observed generic/macro/cast and
conditional-evidence exclusions, with independently reviewed labels and a new
untouched project holdout. Specify its scope separately; no Rust support expansion
is authorized by this study. The constructed result alone cannot justify an
abstraction consumer. No threshold calibration is supported by these natural
exclusions; any later calibration needs its own preregistration and untouched
holdout. Stop here without implementing that work.
