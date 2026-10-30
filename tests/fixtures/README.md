# Shared regression fixtures

These inputs are consumed by Rust regression tests across workspace crates.
They are not extra Cargo workspace members or a production analysis stage.

| Directory | Contents and consumers |
| --- | --- |
| [`function-families/`](function-families/README.md) | Source examples, original-source selections and independent oracles used by cargo-rstyle, rstyle-driver, rstyle-pdg and the line-tail example. Companion Python/native oracle tools stay beside their selections so relative imports and source provenance remain available. |
| `advisory/` | Source-authored cases for effects, mutation, dispatch, planning, helper proposals, capabilities and review persistence. Consumed by cargo-rstyle integration tests and rstyle-pdg dispatch tests. |

Function-family documentation includes the original development history. Moving
these inputs does not change their expectations, recorded source identities or
proof requirements. The backtrack-match oracle uses a local snapshot of the
frozen HIR selection inventory, independent of the evaluation directory.

Current saved-report evaluation tools live under
[`eval/`](../../eval/README.md). Completed studies are available in Git history.
Guard and HIR const regression sources have
independent copies beside their respective suites in `crates/`. Historical
receipts retain their original paths; current tests do not depend on `eval/`.

Run regression tests with the normal workspace command:

```sh
CARGO_TARGET_DIR=target ./scripts/test.sh --nextest
```
