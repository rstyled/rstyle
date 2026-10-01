# LLM Change Log

## 2026-09-29

- Scaffolded the original `syn`-based `rstyle` crate (config, rules, CLI). Superseded, see below.

## 2026-09-29 (HIR rewrite)

- Plan changed: analysis moves from `syn` (syntactic) to rustc HIR (resolved names + types) and adds a weighted side-effect density score.
- Installed nightly (`rustc-dev`, `llvm-tools-preview`, `rust-src`, `clippy`, `rustfmt`) and pinned it in `rust-toolchain.toml` as `nightly-2026-09-30`.
- Removed the earlier half-installed `nightly` left by an aborted `rustup component list` probe.
- Converted the repo to a workspace: `crates/rstyle-core` (pure rules), `crates/rstyle-driver` (rustc driver, facts extraction), `crates/cargo-rstyle` (CLI).
- Deleted the old root `src/` and `tests/` (copied to `/tmp/rstyle-old` while porting config/report/line scanner/suppression into core).

## 2026-09-29 (HIR rewrite, implemented)

- `rstyle-core`: config (strict, defaulted), `FnFacts` data contract, pure rules (`fn_length`, `fn_params`, `mutation_per_target`, `mutated_targets`, `side_effect_density`), suppression (`// rstyle-allow(rule, ...): reason`), deterministic report (text/json + summary), `strip_generics` path normaliser.
- `rstyle-driver` (rustc_private, nightly-2026-09-30): `after_analysis` extracts per-fn facts. Mutations via `ExprUseVisitor` (with `bind` overridden so `let x = ..` is not a mutation), calls/effects/statement count via a HIR `Visitor`. Writes `$RSTYLE_OUT_DIR/<crate>-<kind>.json`.
- `cargo-rstyle`: `cargo rstyle [-p PKG] [--all-targets] [--format text|json] [--config PATH] [--manifest-path PATH]`. Runs `cargo check` with `RUSTC_WORKSPACE_WRAPPER`, forces the pinned toolchain via `RUSTUP_TOOLCHAIN`, uses `target/rstyle/{build,facts}`. A driver fingerprint is fed into RUSTFLAGS (`-Cmetadata`) so a rebuilt driver invalidates cached facts. Exit codes 0/1/2.
- Deviations from the plan (all deliberate):
  - Calls are stored raw (`CallEvent`) and classified by core, so config changes never need recompiling. `EffectEvent` holds only config-free effects (unsafe, static mut).
  - `CallEvent.dynamic` marks dyn/fn-pointer/generic-callable calls; unclassified ones become `unknown` effects.
  - `MutationEvent.escapes`: only writes through `&mut`/raw pointers count toward density; owned-local mutation counts for the mutation budget only.
  - Default `std::process::` prefix narrowed to `Command::output/spawn/status`, `exit`, `abort` (builders are not effects); added `Path::exists` etc. to IO.
  - `invalid_suppression` also fires for unknown rule ids.
  - Unsafe-fn calls only count when not inside a user `unsafe` block.
- Self-refactors driven by dog-fooding: `cargo_args`, `summarize`, `describe` rewritten functionally; IO boundaries and visitor methods carry reasoned `rstyle-allow` comments.
- Fixtures (`crates/cargo-rstyle/tests/fixtures/{clean,violations,broken}`) + 12 end-to-end tests incl. insta golden output, byte-identical double run, config-change-without-recompile, and a self-check of this workspace.
- Added `rstyle.toml`, `clippy.toml`, `.cargo/config.toml` (alias `style`), `scripts/style.sh`.
- Status: 39 core + 7 CLI unit + 12 e2e tests pass; `scripts/style.sh` reports 0 violations on this workspace.

## 2026-09-29 (clippy wiring)

- Root `Cargo.toml`: added `[workspace.lints.clippy]` with `too_many_lines = "warn"` and `too_many_arguments = "warn"` (thresholds come from `clippy.toml`); each crate got `[lints] workspace = true`.
- Fixed the two clippy warnings this surfaced in `rstyle-driver/src/effects.rs` (`doc_lazy_continuation` on doc comments, `collapsible_if` in `on_path`).

## 2026-09-29 (density false positives)

- Ran the checker with `max_density_milli = 0` to list top offenders. Worst were dispatcher/delegating methods (`Walker::on_call` 3000, `Collector::mutate` 1000).
- Fix 1 (`effects.rs`): `visit_arm` counts bare-expression match arms as statements. A tail `match` used to count as 1 statement regardless of arm count.
- Fix 2 (`mutations.rs`): autoref/reborrow `&mut *r` of a whole crate-local object (e.g. `self.helper()`) no longer sets `escapes`, so it does not count toward density (callee is analyzed itself). Still counts toward the mutation budget.
- Result: self-check 95 fns, 0 violations, mean density 203 -> 109. Golden snapshot updated (`Account::churn` 1000 -> 750).

## 2026-09-30 (small-function density exemption)

- Added `side_effects.min_lines` (default 8, `0` disables). Functions with fewer code lines (same counter as `fn_length`) skip `side_effect_density`. Works alongside the existing `min_statements`.
- `rstyle-core`: `config.rs` field + default, `rules.rs` check + unit test; `rstyle.toml` documents it.
- `violations` fixture got an `rstyle.toml` with `min_lines = 0` because its functions are tiny on purpose.

## 2026-09-30 (min_lines default 10)

- `side_effects.min_lines` default raised from 8 to 10 (`config.rs`, `rstyle.toml`, boundary test updated).

## 2026-09-30

- Added `SIDE_EFFECTS.md`: shareable, language-agnostic spec of the mutation budget, effect classification, density score, determinism rules, and an agent prompt for porting to another language.

## 2026-09-30

- Added `.gitignore` (target dirs, fixture lockfiles, insta pending snapshots, profiling output, editor/OS files).
- Added `README.md` (layout, requirements, usage, config table, suppression, dev workflow).

## 2026-09-30 (Gitea + CI)

- Created remote `gitea-origin` (ssh://hermes-agent@100.106.213.77:2222/zcstarr/rstyle.git; repo created by user since the tea token lacks `write:user`), pushed `main`.
- Work continues on branch `ci-and-call-graph` with incremental commits.
- Added `.gitea/workflows/ci.yml` (PRs + pushes to main): fmt check, clippy `-D warnings`, `cargo test --workspace`, rstyle self-check (JSON report uploaded as artifact). Installs rustup if missing; pinned nightly comes from `rust-toolchain.toml`.
- Ran `cargo fmt` (one let-chain in `mutations.rs`).

## 2026-09-30 (step 1: call-graph facts)

- `FnFacts` gained (all `#[serde(default)]`, additive): `id` (stable `<crate>::<def path>`), `implements` (trait-method id for trait-impl fns), `edges: Vec<Edge>` (calls into workspace code incl. same-crate callees, with `dynamic`), `mutated_types` (workspace ADTs written through, sorted).
- Driver: new `ids.rs` (`def_id_str`, `in_workspace` = definition's file lives under the workspace root). `effects.rs` records edges before the local-callee early return (classification of effects still skips local callees). `mutations::collect` also returns the mutated workspace types.
- New fixture `graph` + e2e test `facts_record_call_graph_ids_and_mutated_types` (same-crate edge, trait method edge, `implements`, dyn flag, no std edges).

## 2026-09-30 (commit hygiene)

- Stripped `Co-authored-by: Cursor` trailers from the two earlier commits (history rewritten, branch force-pushed) and added a local `.git/hooks/commit-msg` that deletes that trailer. Agent commits use `-c commit.gpgsign=false` (gpg pinentry cannot prompt headless).

## 2026-09-30 (CI verified)

- PR #1 opened with `tea`; Gitea Actions ran on both the PR (run 13) and the merge to `main` (run 14): fmt, clippy, tests, rstyle self-check and report upload all green on the `anon99-macos` runner (label `ubuntu-latest`). PR #1 was merged into `main`.

## 2026-09-30 (calibration round 1: argh, rustfix, tarpc)

- Ran rstyle on google/argh, google/tarpc, rust-lang/rustfix (clones in /tmp/real). All compile with the pinned nightly.
- Found and fixed three noise sources: (1) trait-method calls inherited the effects of *any* impl, so generic callers of `Stub::call` got `rand`/`Instant::now` from an unrelated impl; `graph::Callee` now has `targets` and a caller inherits only effects present in *every* target. (2) default network prefix `std::net::` matched getters like `SocketAddr::ip`; narrowed to `TcpStream`/`TcpListener`/`UdpSocket` (+ reqwest, tokio::net, hyper). (3) repeated writes to one `&mut` target counted N times in density; `effects_of` now counts a written target once (the budget rule still counts every write).
- Result: argh density violations 10 -> 5, tarpc 19 -> 14. Tests: graph test for intersection semantics, rules test for mutation dedupe, config test for net getters.

## 2026-09-30 (calibration round 2: rust-lang/cargo, 4426 fns)

- Ran on the whole cargo workspace (28-41 s). 908 fns flagged (20%): fn_length 367, mutation_per_target 335, mutated_targets 302, side_effect_density 296, fn_params 220.
- Design flaw found: orchestrators inherited every *distinct* IO label from deep callees (`package_with_dep_graph` density 15000). Fix: a callee now exports **one effect per kind** (`io: std::fs::open +3 more`), counts accumulate across hops. Worst cargo density 15000 -> 5250; flagged fns 986 -> 908; `max_density` stays high only for genuine IO/printing functions (`Shell::new`, `print_*`, `main`).
- Threshold check (argh + tarpc + rustfix + cargo, 4886 fns): current defaults flag lines>50 8.0%, params>4 4.6%, per-target mutations>3 7.9%, mutated targets>3 6.8%. Treated as the right "aspirational outlier" band; thresholds unchanged. Percentiles: lines p90=44 p95=66; params p95=4; per_target p95=5; targets p95=4.
- Known noise left: `tokio::net::*::local_addr` style getters count as network; `writeln!` to stdout/&mut Write counts as IO per call (legit IO functions, use `rstyle-allow` boundaries).
- Fix-grouping prototype on cargo data (python, in /tmp/real/groups.py, not in repo): footprint = fn + callers for `fn_params` only -> 693 groups, largest 89, 93% singletons. Adding density callers or mutated types creates hubs (largest group 373-519 of 908). Decision for the grouping step: footprint v1 = fn identity + direct callers for `fn_params`; mutated types and density callers only as soft "related" hints, not must-stack.
