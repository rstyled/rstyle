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
