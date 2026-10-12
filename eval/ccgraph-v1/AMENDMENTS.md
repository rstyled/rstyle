# Harness amendments (frozen protocol and inputs unchanged)

Before development evaluation or any heldout evaluation: the first development
smoke command succeeded but harness mapping failed on two compiler-generated
libtest entrypoints. Retained raw directory:
`artifacts/smoke-controlled-development-default-20261012T050520Z`.
The report contained all 30 selected source contexts plus two excluded generated
entrypoints. Empty byte span/source, incomplete source scan and macro-generated
exclusion identify these additional unselected functions; they cannot be admitted
or scored. Harness now retains them explicitly, requires unambiguous target/test
context and exclusion evidence, and separately counts selected source observations.
The frozen selected universe stays 435 pairs, not the 496-pair universe over all
observed functions. No predictions/labels or production behavior changed. Original
smoke remains invalid under the original harness; fresh smoke retry is documented.
This is an inventory bookkeeping repair, not a protocol/input amendment.

Development infrastructure restart: a required concurrent CLI validation build
replaced both `target/debug` binaries during natural-development-default. Hash
drift invalidated that run; raw output/invalid marker retained in
`artifacts/development-natural-development-default-20261012T050713Z`.
The successful original controlled overlap/cosine evaluations remain committed in
`results/prior-development/`; their unfavorable FN remain visible. To ensure
one identical binary pair across all final development and heldout comparisons,
copy the now-validated executables into ignored `artifacts/frozen-bin`, record
hashes, and restart the development execution with unchanged frozen inputs and
configurations. No algorithm/extraction/threshold/label/split changes. The copied
binaries contain current study-commit producer metadata; their production tree
is identical to 06d7e8e and the reviewed baseline. Report producer metadata and
actual build hashes rather than claiming that the study commit itself is the
original baseline. This repeat is infrastructure comparability work, not
selection among successful results. Heldout not yet opened; will run once.
