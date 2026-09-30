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
