# Retire expanded evidence

The shared pipeline no longer needs a migration contract that reconstructs
nested helper proof trees. The common CLI test helper previously enabled
`--expanded-evidence` for every clones run, including fresh/warm/edit and trace
comparisons. Its expensive export and standalone re-verification were not the
ordinary shared analysis path.

The CLI option, application exporter, expanded report-facts variant, library
`expanded_frontends` implementation and mutable helper-capture adapter are
removed. Driver document publication always emits shared tables and their seal;
it no longer has a selectable standalone publication runtime. Facts-only driver
capture remains a raw observation output with no nested helper proofs. The
replay's storage report now counts shared bytes/records only. Earlier expanded
storage comparisons remain historical measurements, not a supported export API.

The removed report fields are `artifact_prefix` and `document_manifest`.
`unit_evidence` now includes the exact strong unit identity alongside the bundle
path and function handle. Source navigation, typed pair serialization, strict
bundle verification, byte sealing and graph interchange validation remain.
No graph schema, bundle version, hash meaning, extraction capability, matching
threshold or rule configuration is changed. Old callers must move records into
`UnitStore::capture` and borrow its analysis views instead of mutating an array
of facts to attach proofs. The removed CLI option fails during argument parsing.

Tests now resolve paired documents through strictly loaded shared stores. Large
family loops retain one owning load/analysis context and borrow each function's
records. Test-only copies are made when deliberately mutating documents. Caller
links and shared helper summaries are corrupted in the bundle rather than in
nested per-call JSON. Raw frontend edits recompute the exact unit identity and
update paired references before strict import, ensuring rejection tests reach
actual evidence checks instead of stopping at an intentionally stale hash.
Every mutated bundle is rejected in ordinary strict import and when import is
called inside a generated-policy scope. Untampered baselines must load first.
Byte-edit tests retain exact seal verification. Transitive-edit assertions now
check changed unit identity and derived graph evidence while retaining an
unchanged raw caller tree. Strict/generated serialization, source checks,
argument/read occurrence order, native width, all pair counts and decisions,
fresh/warm/edit runs and tampering categories remain covered.

The obsolete expansion-budget test is removed with the runtime it exercised.
A regression rejects the retired CLI flag before capture. Driver handoff
coverage verifies shared byte sealing, strict serialized-pair verification with
a resolver, admitted and rejected roots, and absence of expanded files. The
driver's matching dependency is now test-only.

## Validation

Validated on the pinned nightly-2026-09-30 aarch64 toolchain with locked
workspace dependencies, an 8-CPU quota and 16 GiB memory. Commands ran
sequentially; build/setup time is separate from nextest's reported test duration.

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `CARGO_TARGET_DIR=target timeout 600 ./scripts/test.sh --nextest`: 642 passed,
  zero skipped, 109.905 seconds; workspace doctests passed. No test exceeded
  the 60-second slow threshold. The inherent and read-helper family tests took
  33.885 and 32.915 seconds respectively.
- `./scripts/style.sh --format json`: 1,970 functions, zero violations.
- `./scripts/style.sh patterns --fail-on-candidates --show-ignored`: zero
  actionable candidates, 15 existing reviewed ignored candidates.
- `cargo build -p rstyle-pdg --example helper_replay --locked`, followed by a
  120-second bounded generated storage replay of the retained pre-helper facts:
  completed with 21 functions, 21 unique graphs, 20 unique helpers and 2,985,484
  shared bundle bytes. This is a stage replay, not a complete source-to-pair run.

Logs are retained locally at `/tmp/rstyle-retire-final-clippy.log`,
`/tmp/rstyle-retire-final-nextest.log`, `/tmp/rstyle-retire-final-style.json`,
`/tmp/rstyle-retire-final-patterns.log` and `/tmp/rstyle-retire-replay.jsonl`.
The observed suite duration is not a controlled speedup measurement: repeated
alternating before/after samples were not collected. The structural reduction
is removal of expanded-proof production and its standalone test re-verification;
shared storage size and record counts on the retained replay remain unchanged.
