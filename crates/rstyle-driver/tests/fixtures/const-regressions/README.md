# Historical const regressions

`lz4.rs` and `time.rs` are independent copies of
`eval/ccgraph-v2/corpus/{lz4,time}/sample/src/lib.rs` at commit `1d1cb4cf`.
The study provenance remains under `eval/ccgraph-v2/`.

`disclosed_historical_const_regressions_cross_serialized_admission` checks that
the older const functions survive serialized admission and that the current
forms remain ineligible for this extraction profile. Tests own these inputs;
they do not load evaluation files or automatically track corpus changes.
