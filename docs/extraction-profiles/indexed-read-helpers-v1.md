# Indexed reads through verified helpers

Target: the unchanged ASCII selection in
[ascii-boundaries-v1](ascii-boundaries-v1.md). Before this increment the seven
original methods admit 6/7; development admits 8/19, with ten independently
specified positives. The remaining positives are the original negating wrapper
and the development negating/repeated callers. Existing line-boundary methods
and the frozen sixty retain their identities and denominators.

Selection: `hir-data-v3`, `rstyle.rust.hir-data/3.59.0`, capability
`rust.indexed-read-helper-calls`. Matching/scoring are unchanged.

## Contract and complete blocker chain

1. Independently verify each callee's complete source/CFG/PDG, including every
   dynamic index's saved operands, bounds condition, guard and formal origin.
2. Derive each read occurrence from that graph. Index reads cannot cross as an
   ordinary caller-relative `IndexedElement { operand_position: 1 }`: the
   caller's operand 1 need not compute the callee's index.
3. A `HelperIndex` projection instead scopes the exact callee operation/access
   and projection tail to the source CALL span. Existing unit links, verified
   callee graphs and ordered CALL operands provide its interpretation. Nested
   helpers retain nested scopes; repeated calls and reads retain distinct keys.
   CALL effects conservatively include possible reads; the referenced callee
   graph retains their execution guards, rather than claiming unconditional reads.
   No new serialized census, callee body copy, registry or verification cache.
4. Substitute the shared actual's saved origins, preserving alternative and
   possibly aliasing roots. Later argument evaluation/rebinding cannot change
   an already evaluated reference argument. Recheck concrete projected types.
5. Shared scalar indexing now explicitly retains possible panic in the CFG
   assessment and helper summary. A bounds *requirement* is not proof of totality.
   Propagate that uncertainty through all caller levels; do not invent purity,
   guaranteed return, cleanup behavior, or a valid-position precondition.

Only verified read-only bodies with reference-free checked-Copy results and
Copy/shared-reference parameters/locals cross this boundary. General ownership,
mutable/indirect writes, looping/recursive helpers, reference-bearing results,
missing origins and unsupported source remain excluded. Existing depth/read
budgets still apply. Historical selections/documents gain no capability.

## Independent expectations and validation

The original runtime oracle specifies ASCII membership independently for every
byte and byte pair, including negation and invalid-position panics. Its separate
CRLF tokenization also retains line-boundary expectations. Original capture uses
the complete unchanged crate, not a substituted helper or declaration.

A four-function composition test independently requires two distinct reads in
repeated and nested callers, the *first* argument's original formal root when a
later argument rebinds it, and partial-flow status for all dynamic-index callers.
Changed CALL spans, callee operations, access ordinals or selector paths must fail
serialized verification and strict admission. The earlier index regression's
`index_helper` becomes supported; owned unbounded `array_helper` stays excluded.

## Measured outcome

Against `5cf1365`, the unchanged full regex-automata 0.4.18 selection gains exactly
`LookMatcher::is_word_ascii_negate`: **97 → 98 /2582**, zero losses, identical
source/build identities. Original ASCII methods reach **7/7**; fixed development
**8 → 10 /19**, all ten source-specified positives. Word/byte remain **13/20,
9/11**. Other packages remain **8/150, 23/956, 11/226**; the frozen sixty remain
**23 admitted /30 excluded /7 inactive**, exact identities/denominators.

Independent bundle/document/producer/strict verification passes for **172
admissions**; **276** comparisons over 24 selected functions complete without
diagnostics. Full natural pair enumeration remains explicitly limited to zero.
**929 workspace tests pass /5 skipped**; six target/std contexts and original
runtime oracles pass, as do Clippy, doctests and formatting. **rstyle: 0 violations
/3014 functions; patterns: 0 candidates**. Native CI for this increment is pending;
the preceding commit passed macOS, Linux x86 and Linux ARM.

Performance: isolated clean baseline/current builds, one warm-up and seven
alternating samples. Median seconds: Copy **0.65 → 0.63**, cyclic **0.64 → 0.63**,
ASCII **1.08 → 0.85** while admissions grow 8→10 and comparisons 28→45. Matching
outputs for unchanged controls are identical. Single full-library runs:
**896.65 → 881.67 s**, peak **6,249,648 → 6,393,560 KiB**. These bounded
observations do not establish general speed or detector-quality claims.
