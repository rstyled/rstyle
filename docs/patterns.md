# Pattern mining

`cargo rstyle patterns` looks for *latent shared abstractions*: code written as parallel copies that
could be one trait, generic function or parameter.

```text
cargo rstyle patterns [--format text|json] [--all-targets] [--min-score N] [--top N]
                      [--hide-implements TEXT]...
```

```text
#1 trait_method  id 40b2ac243d79  score 1.95  [support 3, lift 0.85, holes 2, coverage 1.00]
  observation: 3 functions have aligned dependence-graph regions; recorded substitutions: T0 = Dog | Cat; M0 = Dog::bark | Cat::meow
  inference: Cat, Dog are used interchangeably through methods of identical signature: an implicit interface.
  possible refactor: trait Shared { fn method(&self) -> String; } implemented by Cat, Dog; make the sites generic over it
  sites:
    src/lib.rs:77  emit_dogs
    src/lib.rs:83  emit_cats
    src/lib.rs:89  emit_cats_each
```

It runs across the selected workspace. Authored functions are selected by default, and generated
signatures stay visible in coverage. `--compiler-pattern-scope captured` selects everything captured
instead. Reports keep compilation contexts, unavailable inputs and the saved run directory, and
incomplete input exits 2. Output is deterministic.

To analyze a saved run again without source files or compiler tools, use `--replay-run PATH`. See
[saved runs](style-checks.md#saved-runs). To add runtime bounds to the report, use
[`--complexity`](runtime-bounds.md).

## How candidates are found

1. **Signature groups.** Methods of different types with the same normalized signature, such as
   `fn(&Self) -> String`. Rare signatures score high and common getters score near zero. This is a
   blocking key, not a finding.
2. **Dependence-graph candidates.** Each function gets a dependence graph of data, control and
   operand-position edges. `for`, `?` and macros are single nodes, and call nodes are labeled by the
   callee's signature class, not its name. A Weisfeiler-Lehman kernel clusters functions with the same
   structure, an alignment step produces the *holes* (`T0 = Dog | Cat`, `M0 = Dog::bark | Cat::meow`),
   and method holes are joined back to the signature groups. `for` loops and `Iterator::for_each`
   closures count as the same iteration, and a `match` whose arms call analogous methods is reported as
   `enum_dispatch`.

Each candidate has:

- a `kind`: `trait_method`, `capability_set`, `enum_dispatch`, `generic_fn` or `parameterize`
- a score with its parts: `support`, `lift`, `holes` and `coverage`
- an observation, an inference and a sketch of a possible refactor
- every site with `path:line`, and the functions that fill the holes
- counter-evidence, such as an effect mismatch or a common signature

Patterns whose implementations all already implement one trait method are listed under `suppressed`,
not proposed.

The approach was inspired by comparing functions through approximate matching of their program
dependence graphs, as in
[CCGraph: A PDG-based Code Clone Detector with Approximate Graph Matching](https://yinxingxue.github.io/papers/ase2020_CCGraph%20A%20PDG%20based%20Code%20Clone%20Detector%20With%20Approximate%20Graph%20Matching.pdf)
(ASE 2020), with a Weisfeiler-Lehman graph kernel for the similarity. It isn't a reimplementation.

## Reading the score

```text
score = (ln(support) + lift) * coverage
```

The result is divided by `holes / 3` when there are more than 3 holes, and halved on an effect
mismatch. It's in natural-log units, and it's a ranking, not a probability.

| Part | Meaning |
| --- | --- |
| `support` | Functions in the cluster |
| `lift` | How rare the shared signature is: `ln(methods in the workspace / methods sharing it)`. About 0 for getters, with up to +50% when the types share the method name. |
| `coverage` | Mean share of each body that aligns |
| `holes` | Number of differing columns |

`generic_fn` is halved, `parameterize` is divided by 3, and both have lift 0, so compare scores within
a kind. For the trait kinds, a score under about 0.7 or the counter-evidence "signature is common in
this codebase" is noise. These cutoffs come from the formula and from reading output, and aren't
calibrated on labeled data.

## The ignore ledger

Every candidate prints an `id`. After you review one and decide not to merge it, dismiss it:

```text
cargo rstyle ignore <id> [--allow-growth] [--reason TEXT]   # writes rstyle-ignore.toml
cargo rstyle patterns --show-ignored                        # list what the ledger hid
cargo rstyle patterns --no-ignore                           # one-off full view
cargo rstyle patterns --fail-on-candidates                  # CI gate: exit 1 if any candidate is left
cargo rstyle ignore --status [<id>]                         # where each entry's functions are now
cargo rstyle ignore --all-targets --prune [--dry-run]       # drop entries that match nothing
```

`rstyle-ignore.toml`, at the workspace root or named by `--ignore-file`, is meant to be committed, like
a lockfile. It's a final filter on the reported candidates: mining never reads it, and `--top` and
`--min-score` apply after it. Delete the file or an entry to re-review. Comments directly above an entry
or inside it are kept when `ignore` rewrites the file, so notes like "discussed in PR 12" survive.
Other comments are dropped. `--reason` is the structured version.

**What a dismissal covers.** It holds while the candidate stays the same: the same kind, the same
functions, and the same dependence-graph shape at each site. Moving a function or adding an impl block
above it doesn't matter, but editing its structure does.

**Growth.** If two more functions join a dismissed candidate, it's a new candidate and resurfaces.
`--allow-growth` records the members instead, and the candidate stays hidden as long as they're all
still in it.

**Things to watch for:**

- Pass `ignore` the same clustering flags as `patterns`, since they decide which candidates and ids
  exist.
- Shape comes from the dependence graph, so edits inside closures, or edits that only swap one callee
  for another of the same signature, aren't seen.
- Pruning is manual, and needs complete full-workspace inputs captured with `--all-targets`. Saved
  replay uses the selection it recorded. Stale entries are harmless, and just never match.
- `--status` reports absent members as unobserved within each compilation population, rather than
  assuming they were deleted.

To record a review against a saved run instead of the ledger, use `--dismiss-compiler-pattern UNIT ID`
with `--replay-run`, `--review-file` and `--review-reason`.

To keep a function out of mining altogether, put `// rstyle-allow(abstraction): reason` on the line
above it.

## Experiments

These are off by default:

| Option | Effect |
| --- | --- |
| `--normalize all` or `--normalize ctor,pred,case,exit,counter,cmp,try` | Canonicalize graphs before comparing them |
| `--pair-coverage N` | Also cluster pairs by alignment coverage, even below the similarity threshold |
| `--max-holes N` | Raise the `parameterize` hole cap (default 3) |
| `--single-call-guard` | Admit single-call bodies that do more than forward their parameters |
| `--threshold N`, `--min-coverage N` | Move the clustering thresholds (defaults 600 and 600) |
| `--hide-implements TEXT` | Evaluation oracle: forget trait links whose method id contains `TEXT` (`*` hides all). Hide a trait with several impls and check that the miner proposes it back. |

## Known limits

- Analysis is function-level only.
- Closure bodies are opaque, except `for_each`.
- A cluster mixing already-trait and inherent impls is reported as a candidate, rather than as
  "implement the existing trait".
- The size floor is 4 nodes and 1 call, so small functions never cluster. On competitive-programming
  code, that's more than half of edited-copy pairs.
- The defaults were picked on a few real crates and a labeled example set, not tuned on a large corpus.
