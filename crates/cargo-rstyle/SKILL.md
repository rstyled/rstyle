---
name: rstyle
description: Run `cargo rstyle` to find Rust functions that are too long, take too many params, mutate too much, or have high side-effect density, then fix them in independent groups; also `cargo rstyle patterns` finds parallel code that should be one trait, generic fn or helper. Use when asked to check/enforce Rust style, reduce function complexity, clean up rstyle violations, or find abstractions to extract.
---

# rstyle

Deterministic Rust style analysis on rustc HIR. Same input gives byte-identical output.

## Run

```sh
cargo rstyle --format json            # machine-readable; prefer this
cargo rstyle -p <pkg> --all-targets   # one package, include tests/examples/benches
```

Other flags: `--config PATH` (default `<workspace root>/rstyle.toml`), `--manifest-path PATH`.

Exit codes: `0` clean, `1` violations found (not an error), `2` tool/config/compile error (message on stderr; fix the build first).

## Read the output

JSON top level: `diagnostics`, `summary`, `fix_groups`.

- `diagnostics[]`: `path`, `line`, `column`, `rule_id`, `message`, `actual`, `limit`, `details`.
- Rule ids: `fn_length`, `fn_params`, `fn_spacing`, `mutation_per_target`, `mutated_targets`, `side_effect_density`, `invalid_suppression`.
- `fix_groups[]`: violations whose fixes overlap. Different groups never conflict.

## Fix workflow

1. Run `cargo rstyle --format json`.
2. Pick one fix group. Groups are independent: one PR/commit per group, or work them in parallel.
3. Inside a group, fix callees first (listed order), then callers.
4. For `fn_params`, use the group's `plan`:
   - `signature` steps are re-signed functions, callee first.
   - `call sites` steps are callers that must follow.
   - `pass-through` params are only forwarded; drop them from the caller.
   - `bundle` is a params-struct candidate (params forwarded together at 2+ sites).
5. Re-run. Fixes change the call graph, so regroup after each change.
6. Run the project's tests. Do not stop at a clean rstyle run.

## Fix hints per rule

- `fn_length`: extract cohesive blocks into named helpers. Do not just compress lines.
- `fn_spacing`: add one blank line between logical phases (bindings, work, result). Do not split a fn just to dodge it.
- `fn_params`: introduce a params struct or move state into a type. Do not hide arguments in tuples.
- `mutation_per_target` / `mutated_targets`: return new values instead of writing through `&mut`; split functions by the thing they mutate.
- `side_effect_density`: separate pure computation from IO/network/time/random/unsafe. Keep effects in a thin outer function.

## Suppression (last resort)

Line directly above the function, reason required:

```rust
// rstyle-allow(fn_params, fn_length): FFI shim mirrors the C signature
```

A missing reason or an unknown rule id yields `invalid_suppression`. Never suppress to make the run pass without being asked; prefer refactoring. Do not edit `rstyle.toml` thresholds to hide violations unless asked.

## Find abstractions to extract (`patterns`)

```sh
cargo rstyle patterns --format json [--all-targets] [--min-score 1.0] [--top 20]
```

JSON: `signature_groups[]` (methods of different types with one normalized signature; a blocking key, noisy), `candidates[]` (ranked), `suppressed[]` (already behind one trait).

Candidate fields: `kind`, `score_milli` and `breakdown` (`support`, `lift_milli`, `holes`, `coverage_milli`), `observation` (what differs: `T0 = Dog | Cat; M0 = Dog::bark | Cat::meow`; `F` function, `L` literal holes too), `inference`, `possible_refactor` (sketch, names are placeholders), `sites[]` (callers, `path`/`line`), `definitions[]` (functions filling the method holes), `counter_evidence[]` (read it before acting).

Kinds:
- `trait_method`: one method hole joined to a signature shared by several types. Extract a trait.
- `capability_set`: two or more method holes, the types are used through a set of methods. Extract a trait with those methods (check every type really has them: `definitions[]` should list each type).
- `enum_dispatch`: a `match` whose arms call analogous methods. Extract a trait and dispatch through it.
- `generic_fn`: same body over several workspace types. One generic fn.
- `parameterize`: same body differing only in callees, literals or foreign types. One helper taking those as parameters.

Score (natural-log units, a ranking and not a probability): `(ln(support) + lift) * coverage`, divided by `holes / 3` when holes > 3, halved on effect mismatch. `lift` is how rare the shared signature is (`ln(methods in workspace / methods sharing it)`, about 0 for getters). `generic_fn` is halved and `parameterize` divided by 3, with lift 0, so compare scores within a kind: those two stay below about 1 even when good. For the trait kinds, a score under about 0.7, or counter-evidence "signature is common in this codebase", is noise. `capability_set` loses half its score to 6 holes (types times methods) without being weak; read it anyway.

Workflow: take the top candidates, open each `sites` entry, confirm the parallel bodies really mean the same thing, then extract the trait or generic fn. A candidate is a hint, not a verdict: low `lift_milli` (common signature) or effect-mismatch counter-evidence means be skeptical, and support 2 is thin evidence. Opt out with `// rstyle-allow(abstraction): reason` above the function. Oracle for evaluation: `--hide-implements Trait` pretends the trait is not implemented. Output is deterministic; facts are cached in `target/rstyle/facts-sem` (separate from style facts).

Defaults (WL similarity 600, alignment coverage 600, 3 holes max for `parameterize`) are the evaluated setting. Opt-in experiments, not recommended unless asked: `--normalize all|ctor,pred,case,exit,counter,cmp,try` (canonicalize graphs; on CodeNet it removes about one unmatched node per pair and did not change what gets flagged, `case` does nearly all the work), `--pair-coverage N`, `--max-holes N`, `--single-call-guard`, `--threshold N`, `--min-coverage N` (more recall of strangers' spellings at the price of more false positives; `--pair-coverage 650 --max-holes 4` on CodeNet: about 20 extra cross-problem flags per 350k pairs).

## Requirements

Needs the pinned nightly from the project's `rust-toolchain.toml` (installed by rustup automatically) and `rstyle-driver` installed next to the `cargo-rstyle` binary.
