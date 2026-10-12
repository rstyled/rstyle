# CCGraph v2 coverage/correctness follow-up — preregistration

Question: does the unchanged source-to-pair path correctly capture and admit
purposefully selected natural scalar source in the published Rust HIR R0 scope,
while preserving explicit exclusions and unknown evidence? Pair retrieval is a
secondary descriptive question. This follows v1's coverage failure; v1 results
and integration fixtures are exposed regression knowledge, never unseen evidence.
No calibration or abstraction consumer is part of this study.

## Independent source expectations

Codex authored labels and eligibility expectations from exact source, then
performed a second source-only review of the packaged excerpts and upstream
bytes. Same-agent review is not independent human review or genuine blinding.
No study detector scores, gates, features or decisions were available during
selection, labeling or review. Published contracts and existing regression tests
were read. There were no unresolved adjudications. Coverage controls are unlabeled
for clone quality rather than assigned artificial negative labels.

Positive means whole-function structural edited-copy relatedness, independently
of behavior, names or acceptance. lz4's historical bound expression and current
cast variant share a whole-body recipe with changed arithmetic width; lexical's
two identical match tables differ in name; time's two historical predicates are
exact copies. time's current multiply/bitmask predicate has a different recipe
and is negative against each old predicate. regex's byte-prefix predicate and
match-based scalar table are negative. Purpose, names or category counts alone
cannot establish relatedness. Partial correspondence would be ambiguous; there
are no such selected pairs. Behavior and safe refactoring are unassessed; casts
and the new leap-year algorithm are disclosed source changes.

Expected eligibility follows extraction profile section 1, not predictions.
All five natural scalar definitions use primitive interfaces and allowed scalar
operations. Casts, calls, match and macros remain unsupported, even when pure.
Two pure cfg-disabled let controls expect unknown evidence: their removed source
has no unsupported construct but cannot be fully resolved in HIR. These controls
are conceptually exposed by the integration regression; they are development
checks. No difficult selected function may be removed or rewritten into R0.

oracles.json supplies narrow independent operation inventories for every expected
admitted definition: formal inputs, reads, returns, controls/merges for lazy logic,
and explicit operator multisets. It intentionally does not claim complete PDG
or CFG correctness, dominance, spans or edge validation beyond the existing
serialized-document verification. Oracle disagreement is a correctness result,
not automatically an execution failure; missing/tampered evidence is invalid.
No oracle may be adapted to observed producer output.

## Frozen corpus and scopes

Exactly 15 source functions in five exhaustive command scopes: lz4 (2), lexical
(2), regex (2), time (3), audit (6). Each is one standalone library, compiled
normally and as a test library, two selected units per scope. Selected context
functions: 4,4,4,6,12; declared unordered context universes: 6,6,6,15,66.
Source quality universes: 1,1,1,3,15 (21 total). Every source pair is labeled,
including unlabeled controls. Compiler-generated test main is retained separately.
Every admitted context pair must be emitted, including repeated and cross-context
pairs; there are no cross-command/project pairs. Normal-library contexts alone
are canonical quality observations. Repeated test compilations do not increase
sample size. Full actual output is retained without deduplication.

Development: lz4, lexical, audit. Heldout: regex, time. Entire project histories,
families, module copies and compilation contexts stay in one split. Four natural
projects are new relative to v1; only heldout results are unopened until the
binary/config freeze after development. No random pair split or version leakage.

Provenance pins repository, revision, whole file, excerpt byte spans/digests,
licenses and scaffolding. Bodies/signatures are unchanged; attributes and external
documentation are omitted and recorded. Historical release selection specifically
seeks allowed scalar source, strongly biases coverage, and is not a survey or
whole-project coverage estimate. Preliminary source review also examined json,
miniz_oxide, md5, RustCrypto hashes, aho-corasick, jpeg-decoder and fast-float-rust;
none was substituted after detector results. Upstream generated lexical tables
are natural upstream source, not independent hand-authored clone discoveries.
Natural and controlled populations are never pooled for a quality headline.

## Configurations, targets and denominators

Primary unchanged pinned defaults: size 4/5, numerical 9/10, strings 1/2, final
9/10, smaller_graph_overlap. Secondary fixed cross_product_cosine changes only
scoring. Both splits use both fixed configurations, with no calibration.

Primary natural heldout admission coverage target: 100% of source-expected
admitted functions, minimum three definitions in two projects. Primary bounded
oracle agreement: 100% of those definitions, same minimum. Correct unsupported
exclusion target: 100%, minimum two natural heldout excluded definitions. Unknown
controls: 100% outcome agreement, minimum two development controls, a regression
check only. These strict targets are justified because every selected supported
construct is explicitly required and every unsupported construct explicitly
forbidden. They assess this bounded corpus, not a population guarantee.
A mismatch fails; insufficient definitions/projects yields insufficient evidence.
No numerical target is lowered after results. Admission coverage denominator
includes all five selected natural source-expected supported definitions, split
and project reported separately. Audit controls reported separately.

Secondary quality targets remain precision >= .95, recall >= .80, false-positive
rate <= .05, F1 >= .85, positive admission coverage >= .80 and end-to-end positive
yield >= .70. At least 20 evaluable positives and 20 evaluable negatives in two
provenance groups per population/split are required: this study cannot satisfy
that minimum, deliberately, and must report insufficient evidence. Pair quality
is not the rationale for this coverage follow-up. TP/FP/TN/FN use evaluable labeled
canonical pairs only. Unsupported, unknown and incompatible are non-evaluable,
not false negatives. End-to-end positive yield divides accepted canonical labeled
positives by ALL selected canonical labeled positives, including admission loss.
Ambiguous/unlabeled accepted pairs are counted without calling them FP. Undefined
ratios are null. Project/stratum results are reported. No pooled headline and no
independent-pair confidence intervals: historical pairs and test contexts are
correlated; project-level counts/ranges and explicit small-sample limits suffice.

## Execution, amendments and retention

Preregistration commit and SHA256 freeze precede every study run. Freeze includes
protocol, source/label/provenance/oracle/configuration bytes, pinned artifacts and
reused evaluator. Source revision must remain production-tree-identical to
06d7e8e81653aa0bd53147275555d774985c87dd (reviewed merge 6a81a3b). Since that
merge HEAD contains only v1 study infrastructure/reports. Actual study git HEAD,
producer embedded revision, executable hashes, compiler/Cargo identities and
settings are recorded. Finish Rust validation/build before copying isolated
binaries. Smoke uses development audit only; then development, diagnose, freeze
binary/config snapshot, heldout once per configuration. No successful result may
be overwritten or unfavorable run discarded. Fresh command run directories only.

Per command budgets: 180 wall seconds, 2 GiB sampled process-group RSS, 1 GiB
fresh run disk, 1000 pairs, two Cargo jobs. Captures are offline/locked/all-targets,
debug profile, edition 2021, no dependencies. Compilation and matching combined
time and fresh compilation cost stay visible (no separate command phase timer).
Incomplete capture, ambiguous/missing navigation, malformed/missing/duplicate
pairs, verification/arithmetic errors or budget failures invalidate execution.
Unsupported/unknown/incompatible outcomes do not invalidate complete runs.
A documented infrastructure retry preserves original failure and identities.
Prediction-affecting detector defects are follow-up work, never repaired here.
Any post-freeze source/label/config/target change creates a new version and
invalidates this holdout's unseen status. Harness-only fixes require dated,
committed amendments and tests, preserving original invalid output.

Exit 0: valid target check meets primary coverage/correctness targets. Exit 1:
valid target check misses targets or insufficient evidence. Exit 2: invalid or
incomplete execution. Execution-only commands exit 0 on completeness, never
assert quality success. Secondary insufficient quality evidence remains explicit.
Raw reports, paired documents, manifests, compilation trees and isolated binaries
are locally retained under ignored artifacts/ or target/. Committed digest
inventories link them to compact machine-readable results and selected traces;
reproduction regenerates machine-specific paths, not byte-identical builds.
