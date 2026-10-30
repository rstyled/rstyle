# Compiler regression tests

`cli.rs` is the integration-test entry point. Its modules are named for current
features; the former `migration_*` suites are ordinary regression coverage.
Shared algorithms also have focused unit tests in `rstyle-core` and `rstyle-pdg`.
Shared source selections, oracles and advisory cases live under
[`tests/fixtures/`](../../../tests/fixtures/README.md); recorded evaluations stay
under `eval/`.

| Modules | Coverage |
| --- | --- |
| `compiler_advisory` | Compiler collection, authored/generated scope, build contexts, owner reuse, numeric evaluation, root style, patterns, baselines and saved reviews |
| `effects`, `effect_composition`, `density_review` | Effects, propagation, density and individual reviews |
| `mutations`, `mutation_branches`, `mutation_loops`, `mutation_policy`, `mutation_review` | Writes, control flow, exclusions, suppressions and review outcomes |
| `rules`, `source_baseline`, `length_review`, `parameter_review`, `spacing_review` | Source rules, missing data, source ownership and baselines/reviews |
| `parameter_plans`, `planning`, `guard_plans`, `guards` | Parameter stacking, caller scope and guard advice |
| `capabilities`, `helpers`, `dispatch`, `differences`, `abstraction_review` | Advisory proposals, negative controls and review lifecycle |
| `replay`, `review`, `output` | Saved evidence, unavailable/stale findings and report formats |

Run all checks with the same bounded compiler concurrency as CI:

```sh
CARGO_TARGET_DIR=target ./scripts/test.sh --nextest
```

Run one feature with a prebuilt driver:

```sh
CARGO_TARGET_DIR=target cargo build --locked -p rstyle-driver
RSTYLE_TEST_DRIVER="$PWD/target/debug/rstyle-driver" CARGO_TARGET_DIR=target \
  cargo nextest run -p cargo-rstyle --test cli --locked --profile ci \
  -E 'test(compiler_advisory::demanded_diagnostic_owners)'
```

Use `--test-threads 1` when comparing aggregate test cost. Nextest runs each test
in a separate process, so a static fixture cache would not share captures across
tests. Related live/replay assertions instead share one local fixture within a
scenario. Independent scenarios retain separate temporary workspaces.

The large-owner scenario keeps 4,101 functions to exercise the 4,096-payload
boundary. It exports a late finding, checks baseline resolution and review pruning
independently, removes the late measurement, changes the compilation context and
replays the original finding without source or tools. The over-budget positive
case stays separate. These checks must not be replaced with smaller fixtures or
missing-as-zero expectations.

Other consolidated coverage lives at:

- `compiler_advisory::try_statements`: authored counts, direct IO and source-free replay from one capture.
- `compiler_advisory::complexity`: construct-level scores and policy replay from one capture.
- `compiler_advisory::style_routing`: root projection and repeated-run stability from the same workspace.
- `compiler_advisory::patterns::metadata_payload`: default authored scope and offline replay, alongside graph/payload contracts.
- `compiler_advisory::patterns::routing`: repository review filtering and unchanged ledger bytes on live capture and replay.
- `src/cli_tests.rs`: argument validation, including removed selectors and root/subcommand separation, without compiling a fixture.

Historical receipts under `eval/` retain their original test names and timings;
the directory renames do not rewrite recorded evidence.

The large-owner group was measured locally with the serial command above:

| Cost | Before consolidation | After consolidation |
| --- | ---: | ---: |
| Live compiler captures | 9 | 4 |
| Test processes | 5 | 2 |
| Nextest execution time | 87.468 s | 52.336 s |

That is about 40% less execution time for this group, excluding Cargo build time.
The default CI profile still runs four test processes concurrently, so this is
an aggregate-cost measurement, not a claim of 40% faster whole-workspace CI.
No cross-test cache, hash, suppression or smaller resource boundary was added.
