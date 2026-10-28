# Saved-run reevaluation

Reevaluate a specific completed capture without invoking Cargo or the compiler:

```sh
cargo rstyle clones --replay-run /absolute/path/to/run-directory \
  --extraction-profile hir-data-v3 --source-rules \
  --analysis-config analysis.toml --format json --report-detail compact
```

Use the original extraction profile. Source-rule policy, matching policy and
consumer requests can change; source/signature rules, guard advice and helper
proposals use the captured evidence. Original source files and manifests are not
required for standalone replay. This does not update the saved run to current
source. Reports retain the original compiler identity, expected compilation
units, build contexts, source snapshots and unit inventory.

The replay directory must retain its plan, expected Cargo unit graph, successful
final Cargo completion log, compiler-unit ledgers and original evidence bundles.
The loader reconciles selected roots and artifacts against ledgers before strict
bundle import. Missing, changed or incomplete evidence fails instead of becoming
a fresh capture. Check and test units remain separate even when Cargo reports
identical root metadata. Replay has no implicit latest-run selection.

Dependency captures also require their dependency-closure inventory and original
compiler metadata artifacts. Their existing association and strict-import checks
still apply; missing closure data cannot fall back to standalone replay.

Replay rejects manifest/package/all-target selection and build-option flags,
including `--locked` and `--offline`: these would imply a new compilation request.
`scope.selection_origin` is `saved_run`; request-level packages, all-targets and
manifest fields are null because the original command-line selection is not
persisted. Effective original selection is recorded in `expected_targets`,
`intended_primary_units`, `intended_dependency_units` and
`expected_compilation_units`. `scope.build_options` describes the current
consumer request; `build_options_origin` states this distinction.

Exit behavior is unchanged: complete requested findings optionally exit one via
`--fail-on-findings`; failed or incomplete requested analysis exits two. Replay
does not make unavailable or generated-source evidence complete. Reports and
policies are separate from legacy review/suppression files. This workflow adds
no mutation/effect analysis and retires no legacy capability.
