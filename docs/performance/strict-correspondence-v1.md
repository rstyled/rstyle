# Strict source-operation correspondence

The strict verifier now indexes expected nodes by the complete original
correspondence predicate and tracks claimed IDs in a set. Keys borrow typed
fields, including source-operation metadata; lookup uses exact equality. No JSON
lookup keys, cryptographic hashes, integrity changes or admission changes are
introduced. Existing ambiguity, uniqueness and overwritten-name handling remain.
Output renaming remains deterministic regardless of hash-map iteration order.

This removes two quadratic scans. Expected-node lookup is average linear;
ordered result/claimed-ID bookkeeping is O(n log n), plus key-content costs.

Isolated debug benchmark, baseline `65354c1`, same fixture nodes in reversed
order, prebuilt binaries, ARM Linux, pinned nightly, no concurrent builds/tests:

| Nodes | Baseline median ms | Indexed median ms |
| --- | ---: | ---: |
| 256 | 3.892 | 0.555 |
| 1,024 | 59.498 | 2.305 |
| 4,096 | 948.006 | 9.631 |
| 8,192 | 3894.739 | 19.543 |

Each size has an untimed correctness/warmup run and three alternating samples.
Both implementations return identical complete renaming maps. This measures
correspondence only and does not establish whole-test or extraction speedup.
Harness/source and raw samples: `/tmp/rstyle-393-correspondence-bench/` and
`/tmp/rstyle-393-correspondence-bench-final.log`. Regression tests cover every original
key discriminator, ambiguous candidates, repeated claims, and 4,096 reversed
renamed nodes. Historical source identities and transport XXH3 remain unchanged.
