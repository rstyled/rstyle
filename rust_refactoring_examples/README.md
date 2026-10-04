# Rust Refactoring Examples

This bundle contains four deliberately small Rust projects. Each project already uses some sensible reuse (traits and/or macros), but also contains a few related concerns that are implemented separately and could plausibly be unified under a stronger concept.

The intent is to give you code that is *not terrible*, but is uneven enough to support a realistic refactoring discussion.

## Projects

1. `telemetry_workspace/` — a Cargo workspace with a shared core crate plus file and HTTP-style agents.
2. `backup_sync/` — a small backup synchronizer with pluggable object stores.
3. `queue_worker/` — a tiny background-job runner with typed handlers and a registration macro.
4. `feature_gate/` — a feature-flag evaluator with multiple rule sources and audit output.

See `REFACTORING_CONCERNS.md` for the intentionally un-unified concepts.

## Verification note

The generation environment did not have `cargo` installed, so the projects could not be compiled in-place. The sources received a basic static/structural sanity pass and intentionally use only the Rust standard library plus local path dependencies in the workspace project.
