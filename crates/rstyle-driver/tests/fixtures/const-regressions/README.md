# Historical const regressions

`lz4.rs` and `time.rs` are independent copies of
`eval/ccgraph-v2/corpus/{lz4,time}/sample/src/lib.rs` at commit `1d1cb4cf`.
The original study and its provenance are available at that Git revision;
they are no longer part of the active checkout.

`disclosed_historical_const_regressions_cross_serialized_admission` checks that
the older const functions survive serialized admission and that the current
forms remain ineligible for this extraction profile. Tests own these inputs;
they do not load evaluation files or automatically track corpus changes.

Upstream fixture sources and licenses:

- lz4: https://github.com/PSeitz/lz4_flex, revisions `cc09800f0741fdecb01561138eae16691973640c`, `f6251928e30af958243a3ef8f22e3071d241543d`. See `LICENSE-lz4`.
- time: https://github.com/time-rs/time, revisions `d97594b9f5cfeab0335c16687e26a8b51c56a526`, `3c3c546a661ac59e1a586a4edc65adff04fd1335`, `9b189628ecd0c5805b117a75729ad908626fd5f2`. See `LICENSE-time-1`, `LICENSE-time-2` and `LICENSE-time-3`.
