# Guard regression sources

These are independent regression copies of `eval/clones-capability/corpus/src/`
at commit `1d1cb4cf`. `guards/mod.rs` writes them into temporary compiler
workspaces, preserving the original module names and negative controls.

Tests own these copies. Evaluation corpus changes do not implicitly change
regression expectations, and test edits do not rewrite historical studies.
