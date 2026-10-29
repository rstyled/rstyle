# Source-level complexity

`cargo rstyle` reports style findings, fix groups, cyclomatic complexity (CC),
and cognitive complexity (COG) together. These are inspection signals, not
proofs of safety.

```sh
scripts/style.sh
cargo style --format json
```

The text report lists functions with findings, ranked by COG and then CC, alongside
all diagnostics and fix groups. The `cargo rstyle --format json` report includes
those functions under `functions`. Each function has its identity, source
location, and `complexity`: `policy_version`, `cyclomatic`, `cognitive`, and
`contributions`.
Each contribution records its semantic kind, line, column, nesting, and the
increment for each score. CC has a base of one; the evidence accounts for all
other CC points and every COG point. Locations are one-based.

Ties use path, line, column, and name. Clean functions are omitted; functions with
missing measurements remain visible. The summary counts all analyzed functions,
including `complexity_measured` and `complexity_unavailable`. The command exits 2 if any selected function lacks
complexity data, while retaining other available findings. Old facts deserialize as
`complexity: null`, never as a measured zero.

## Shared policy, version 1

The pure `rstyle-core::complexity` module owns the counting rules. A frontend
supplies a tree of semantic `Node` values with `Kind`, source location, and
source-ordered children. It does not supply score weights or compute nesting
costs. The model has no Rust compiler types or language-specific syntax names.
Other frontends can map their own syntax to the same model and reuse the counter,
evidence, ranking, thresholds, and shared tests.

| Construct | CC increment | COG increment | Child nesting |
| --- | --- | --- | --- |
| Function entry | 1 | 0 | Starts at 0 |
| Branch, loop, selection guard | 1 | 1 + nesting | +1 |
| Else-if | 1 | 1 | +1 |
| Else | 0 | 1 | +1 |
| Selection | Number of alternatives minus 1, minimum 0 | 1 + nesting | +1 in alternatives |
| Short-circuit operator | 1 per operator | 1 per contiguous run of the same operator | Unchanged |
| Error propagation | 1 | 0 | Unchanged |
| Labeled jump | 0 | 1 | Unchanged |
| Direct recursive call | 0 | 1 per call | Unchanged |
| Closure | 0 | 0 | +1 |
| Sequence/group | 0 | 0 | Unchanged |

Else-if and else nodes are siblings of the initial branch: a chain does not
acquire nesting merely because a parser represents its next clause inside the
previous clause. A selection's scrutinee stays at the surrounding nesting;
alternatives share one additional nesting level. Guards add their own decision.
Alternative patterns within one arm do not add paths. Boolean groups contain
source-ordered operator nodes; parentheses preserve the operator sequence, while
an intervening expression such as a function call starts its own boolean group.

This is a versioned, source-level policy inspired by the same distinction as
gstyle's CC and COG. It is not a count of optimized machine-code paths or a claim
of exact score parity across different languages. In particular, an unconditional
loop counts once, selection arms contribute `N - 1`, and propagation has no
cognitive cost. A future change to counting semantics needs a policy version
change and corresponding shared fixtures.

## Rust mapping

The driver walks the active AST after macro expansion and cfg selection, before
HIR lowering. This retains the written `for`, `?`, and async constructs and avoids
counting their compiler-generated matches and loops.

- `if` and `if let` map to branch; `let ... else` maps to one branch.
- `while`, `while let`, `for`, and `loop` each map to one loop.
- `match` maps to selection; each arm is one alternative and each guard is a guard.
- `&&` and `||` map to short-circuit operators; bitwise operators do not count.
- `?` maps to error propagation. Ordinary `return`, `break`, and `continue` add no
  points; labeled `break` and `continue` map to jumps.
- Closures and explicit async/gen blocks add nesting within their owning function.
  An async function itself adds no extra nesting. `.await` adds no points.
- Resolved direct recursive calls add COG points. Indirect recursion through
  another function, a function pointer, or dynamic dispatch is not inferred.
- Nested named functions have their own records and do not contribute to their
  enclosing function. Anonymous closures remain part of their owner, as in gstyle.
- Macro-generated functions and macro-expanded expression bodies are outside this
  source-level metric. A macro invocation is opaque; its expansion does not add
  hidden branches. Functions disabled by cfg are absent from the selected build.

## Limits

Both limits default to 12. A score strictly above either limit is a finding;
exactly 12 is allowed. To change the limits, set them in `rstyle.toml`:

```toml
[complexity]
max_cyclomatic = 12
max_cognitive = 12
```

This workspace temporarily sets cyclomatic to 40 and cognitive to 32, the current
maximum scores. Follow-up refactoring PRs can lower these limits toward 12 while
keeping the self-check clean. The tool defaults remain 12 for both rules.

`cargo rstyle` reports scores strictly above the limits as `cyclomatic_complexity`
and `cognitive_complexity`. Findings use the existing source evidence, fix groups,
and exit status 1. A missing measurement produces exit status 2, with unavailable
counts in the same report. Missing data takes precedence over findings when
selecting the exit status.

Suppress a deliberate exception with the existing reason-bearing syntax:

```rust
// rstyle-allow(cyclomatic_complexity): protocol state dispatcher
```

Suppression removes the threshold finding. Measurement still covers all functions;
only functions with remaining findings or missing measurements appear in the report.
Lower rollout limits through refactors that make control flow easier to follow.
