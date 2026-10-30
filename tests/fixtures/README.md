# Shared regression fixtures

These inputs are consumed by Rust regression tests across workspace crates.
They are not extra Cargo workspace members or a production analysis stage.

| Directory | Contents and consumers |
| --- | --- |
| [`function-families/`](function-families/README.md) | Source examples, original-source selections and independent oracles used by cargo-rstyle, rstyle-driver, rstyle-pdg and the line-tail example. Companion Python/native oracle tools stay beside their selections so relative imports and source provenance remain available. |
| `advisory/` | Source-authored cases for effects, mutation, dispatch, planning, helper proposals, capabilities and review persistence. Consumed by cargo-rstyle integration tests and rstyle-pdg dispatch tests. |

Function-family documentation includes the original development history. Moving
these inputs does not change their expectations, recorded source identities or
proof requirements. One relative reference to the frozen HIR inventory changes
to reach its existing location under `eval/`.

Recorded study results and saved-report evaluation tools remain under
[`eval/`](../../eval/README.md). Its README also identifies two study corpora
intentionally shared with regression tests. Historical receipts retain their
original paths; use the locations above for current test inputs.

Run regression tests with the normal workspace command:

```sh
CARGO_TARGET_DIR=target ./scripts/test.sh --nextest
```
