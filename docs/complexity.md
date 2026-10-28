# Source-level complexity

`cargo rstyle complexity` ranks functions by cyclomatic complexity (CC) or
cognitive complexity (COG). These are inspection signals, not proofs of safety.

```sh
scripts/style.sh complexity --sort cognitive --top 20
cargo style complexity --sort cyclomatic --format json --top 100
```

The ordinary `cargo rstyle --format json` report also includes every selected
function under `functions`. Each function has its identity, source location, and
`complexity`: `policy_version`, `cyclomatic`, `cognitive`, and `contributions`.
Each contribution records its semantic kind, line, column, nesting, and the
increment for each score. CC has a base of one; the evidence accounts for all
other CC points and every COG point. Locations are one-based.

The ranked view sorts by the selected score, then the other score, then path,
line, column, and name. `--top` limits displayed functions; `total`, `measured`,
and `unavailable` still cover the full selected input. The command exits 2 if
any selected function lacks complexity data. Old facts deserialize as
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

## Optional limits

Scores are always available for inspection. Limits are disabled by default.
To enforce them, add a policy to `rstyle.toml`:

```toml
[complexity]
max_cyclomatic = 10
max_cognitive = 15
```

`cargo rstyle` reports scores strictly above the limits as `cyclomatic_complexity`
and `cognitive_complexity`. Findings use the existing source evidence, fix groups,
and exit status 1. `cargo rstyle complexity` remains an inspection command and
does not fail on style thresholds. A required but missing measurement is an
operational error, not a clean check.

Suppress a deliberate exception with the existing reason-bearing syntax:

```rust
// rstyle-allow(cyclomatic_complexity): protocol state dispatcher
```

Suppression removes the threshold finding, not the function's measured scores.
Do not raise limits or split functions solely to make these signals smaller.
