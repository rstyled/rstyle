# ASCII word classifier family

Source-first selection at 06946bc, pinned nightly-2026-09-30, original default
feature debug Cargo libraries. Preserve the frozen sixty and earlier corpora.

| Original function | File bytes | SHA256 | 3.54 |
| --- | --- | --- | --- |
| regex-automata 0.4.18 Unit::u8 | util/alphabet.rs 4985..5053 | 2433cddaafe16f0cc09ef07d8d3c6f9240f63a5bee12b515b979b420046b14d9 | admitted |
| Unit::as_u8 | util/alphabet.rs 5977..6128 | a27ac16335fbe204e853b2665b5db25c033ade8be5978f7143c8d090995142f8 | admitted |
| Unit::is_word_byte | util/alphabet.rs 7464..7573 | a05491861eabe0175122984fad387ad80c6cdf2f81772c5afccd728a58b735e3 | excluded |
| utf8::is_word_byte | util/utf8.rs 549..1240 | 7ecb732d59ed12c899a2751079b468495fcbeb77b88f5fb6ff3740c434029ed0 | excluded |
| nested mkwordset | util/utf8.rs 597..1174 | 5925bf5644910d68ae3444334f20a0e11006e91fa13e5593282224123d115cbf | excluded |
| regex-syntax 0.8.11 is_word_byte | lib.rs 15436..15578 | ff3e3442df02adf3fff09830d8440166026f641d5b705f13aa1febb560ca163d | excluded |

## Complete blockers and implementation order

1. **Byte scalars:** source scanning rejects byte expressions and byte range/OR
   patterns, despite rustc's concrete u8 observations. Exhaustiveness also lacks
   decoded byte bounds. This blocks the complete original regex-syntax classifier
   and the scalar callback leaf. Connect expressions, literal/range/OR patterns,
   escape decoding, exhaustive matches, checked access and helper composition.
2. **Compile-time table:** rustc scalar-constant capture omits aggregate values;
   original utf8::is_word_byte contains a nested const fn and local array const.
   Prove declaration/source ownership, complete evaluated immutable bool-array
   contents and indexed value/bounds. Existing unsigned-range inference does not
   propagate `u8 as usize`; preserve that lossless bound for complete helper
   certification (the current helper boundary rejects unproved value indexing).
   Retain the nested runtime function as its
   own outcome: its memory writes remain unsupported. Do not silently erase its
   declaration, replace the table or authorize runtime mutation.
3. **Named callback:** current map_or contract requires a closure/environment.
   Prove the exact local function item, complete helper signature/body/summary,
   zero callable storage/drop, eager receiver/default and Some-only invocation
   and result selection. Aliases require compiler resolution; pointers, indirect
   callbacks and unsupported effects remain excluded.

Unit's owner/import/as_u8 chain already passes S09. Five natural LookMatcher
ASCII boundary callers also depend on the table, but separately reject their
non-Copy owner and some primitive arithmetic. They are follow-up candidates,
not promised gains. Unknowns: evaluated-array declaration transport,
named-callback summary transport and those callers' complete owner/effect chain.
Reuse existing compiler/type/callee/source evidence; no new census/cache is justified.

## Independent expectations

`byte-scalar-classifiers.rs` fixes nine intended complete positives and two
negatives (characters and byte_string). `word-byte-classifiers.rs` fixes thirteen
intended positives: Unit's four methods; table::is_word_byte; word_byte,
word_unit, saved, empty_default, scalar, named, named_none and aliased. Its seven
negative outcomes retain both generated Clone methods, nested runtime mkwordset,
indirect, mutable, effectful_callback and through_effectful. These selections
precede admission measurements; leave their identities/counts unchanged.

ASCII membership is the independently enumerated 63-byte alphabet encoded in
two fixed u64 bitsets. Check all 256 bytes, all 65536 sentinel payloads, eager
default/callback ordering and absence of callback invocation on None. The oracle
runner verifies original identities across both automata files before testing
the complete disposable crate (`--family word-byte-classifiers`); use
`--family syntax-word-byte` for the original syntax classifier.

Delivery requires std/no_std Cargo capture on all three installed CI targets,
independent source/document verification and strict admission, complete selected
matching, forged value/type/source/policy/range cases, existing regressions and
unchanged natural/frozen admission deltas. Measure performance against 06946bc.
Keep reports to contract, measured delta, validation and remaining blockers;
large captures use text output and stream the persisted complete report.

## Source-first baseline

Data-v3 3.54 captures all **20 table/callback functions: 3 admitted /17 excluded**,
and **11 byte-scalar functions: 0 admitted /11 excluded**, consistently in all six
std/no_std target contexts. Capture, assessment and selected matching complete;
admitted constructor/projection documents independently verify. Both original
crate oracles pass (four tests). Rstyle: **0 violations /2937 functions**;
patterns: **0 candidates**. Production policy and frozen outcomes are unchanged.


## Data-v3 3.55 delivery

Byte expressions and literal/range/OR selectors retain their exact source
spelling and concrete u8 type; escape decoding proves values and exhaustive
coverage. Independent document checks reject changed values, bounds, types,
operand dependencies, effects and policy. Existing checked-access, saved-value
and helper contracts compose without source replacement or new provenance/cache.

| Unchanged selection | 3.54 admitted | 3.55 admitted |
| --- | ---: | ---: |
| Byte development /11 | 0 | 9 |
| Table/callback development /20 | 3 | 4 |
| regex-syntax /956 | 22 | 23 |
| regex-automata /2582 | 83 | 83 |
| lz4_flex /150 | 8 | 8 |
| time-core /226 | 11 | 11 |
| Frozen /60 | 23 | 23 |

Zero losses; all identities/build contexts and frozen denominators agree.
Frozen outcomes remain 23 admitted /30 excluded /7 inactive. The sole natural
gain is the complete original regex-syntax classifier. Unit::is_word_byte still
needs evaluated-array/declaration proof, lossless cast bounds and named-callback
transport; its broader family is not complete. Characters, byte strings,
unsupported callbacks and runtime memory writes remain excluded.

Validation: **918 tests pass /5 skipped**, six target/std contexts, original-crate
and exhaustive composition oracles, Clippy/doctests/fmt, **rstyle 0 /2952 functions**
and **patterns 0**. All **138 admissions** independently strict-verify. Matching
checks both copies of the original classifier, then compares nine unique
function keys: all **36 comparisons complete**. Full natural matching remains
explicitly budget-limited.


Quiescent controls against clean 06946bc, seven alternating samples after warmup
(seconds, median [range]): copy **0.64 [0.60–0.67] → 0.63 [0.61–0.66]**;
cyclic **0.63 [0.60–0.63] → 0.63 [0.61–0.65]**, identical matching/admissions.
Byte corpus **0.28 [0.26–0.31] → 0.36 [0.33–0.38]** does 0 → 9 admissions
and 0 → 36 comparisons; median peak RSS 106.3 → 107.8 MiB. These are bounded
controls. Text output keeps the complete natural report on disk without a second
JSON rendering; that run peaks at 6.1 GiB. No general performance claim.

## Const-table support contract (3.56)

Connect the original utf8 table helper and direct development caller before named
callbacks. Support original function-local `[bool; 1..=4096]` constants initialized
by a safe, nongeneric, zero-argument sibling const fn with the same result type.
Retain complete declaration source/identity; reject attributes, macros, unsafe,
nested declarations and indirect initializers. Rustc owns compile-time execution:
read every evaluated byte with bounds, layout, initialization and pointer-provenance
checks; accept only bool representations. Declarations have no runtime operation;
the separately captured runtime builder still excludes for memory writes.

A value path must agree with exactly one owned declaration and all evaluated
entries. Documents retain the full value, exact type and zero runtime effects or
operands. Unsigned casts carry saved upper bounds only when the concrete source
and result are unsigned and the saved bound fits the result on the captured
target. Signed/widened/rebound indices and the 255-versus-256 edge retain possible
panic and cannot certify helpers. Expected unchanged development gain: table
helper plus word_byte; named callback and Unit callers remain excluded. Validate
all 256 entries against the independent ASCII bitset and reject forged contents,
length, effects, operand edges, ownership, initializer and historical policy.

### 3.56 measured result

Original regex-automata utf8::is_word_byte now admits through real Cargo capture,
independent verification, strict admission and matching. Unchanged selections:
**regex-automata 83 → 84 /2582**, table/callback development **4 → 6 /20**;
byte development **9/11**, regex-syntax **23/956**, lz4_flex **8/150**, time-core
**11/226** unchanged. Zero losses; identities/build contexts agree. Frozen sixty
retain **23 admitted /30 excluded /7 inactive**, identical denominators.

All **141 admissions across these selections** independently strict-verify;
**15 comparisons among six unique function keys** complete without diagnostics.
The verbatim development table verifies separately from the original matching
selection. Full natural matching remains explicitly budget-limited. **921 tests
pass /5 skipped**, including the reported assertion/subslice regressions and six
target/std contexts; original-crate oracles, Clippy/doctests/fmt pass.
**Rstyle 0 violations /2983 functions; patterns 0 candidates**. Native macOS
execution was not performed here.

Clean ba2a7ae controls, seven alternating measured samples: copy median
**0.65 → 0.64s**; table **0.55 → 0.60s**, doing **6 → 15 comparisons**.
Cyclic initially **0.63 → 0.66s**; a fifteen-sample repeat gives **0.63 → 0.62s**.
RSS remains stable. The single full natural capture takes **884s /6.1 GiB**
(previous 874s /6.1 GiB); these bounded observations make no general performance
claim. Text capture retains complete reports without a second JSON rendering.

Remaining chain: exact named map_or function-item identity, checked helper
summary, eager arguments, Some-only invocation and result merge. Unit::is_word_byte
and seven intended development positives await that composition. Five LookMatcher
callers retain separate owner/arithmetic blockers. Runtime mutation, function
pointers and effectful callbacks remain excluded; no scoring/admission relaxation,
new census/cache, abstraction activation or rewrite-quality claim.

## Named callback support contract (3.57)

Finish the selected Unit family with immediate, original
`Option<u8>::map_or(bool, fn(u8) -> bool)` function items. Compiler FnDef identity
must resolve one local, safe, nongeneric Rust free function with a fully verified
returning, memory-free, panic-free scalar summary. Qualified paths and imported
aliases use that same definition identity. FnDef has no captured storage or drop;
this does not authorize function pointers, stored callable values or closures.

Preserve receiver, eager default and function-item materialization in source
order. Only Some may extract the saved receiver payload and invoke the helper;
None selects the saved default. Independent document checks require exact event
inventories, guarded payload/call, helper identity/operands, evaluation order and
result alternatives. Reuse owning-unit helper links and summaries without a new
census/cache. The unchanged source-first expectations are **13/20** development
positives and original Unit::is_word_byte; all seven negatives remain excluded.


### 3.57 measured result

Original Unit::is_word_byte completes its constructor/projection, Option, named
callback and const-table chain through Cargo capture, independent strict admission
and matching. **regex-automata 84 → 85 /2582; development 6 → 13 /20**. All
source/build identities agree, zero losses. Byte development **9/11**, syntax
**23/956**, lz4 **8/150**, time **11/226** and frozen **23 admitted /30 excluded
/7 inactive /60** are unchanged. All seven development negatives remain excluded.

**149 admissions independently strict-verify; 91 comparisons among fourteen unique
function keys complete.** Validation: **924 tests pass /5 skipped**, six target/std
contexts, original exhaustive/runtime oracles, forged source/identity/signature,
guard/order/payload/result/effect/policy cases, Clippy/doctests/fmt, **rstyle 0
/3007 functions; patterns 0**. Full natural matching remains explicitly budgeted;
native macOS execution was not performed here.

Clean 9aa81d0 controls, seven alternating samples (median seconds): copy
**0.65 → 0.65**, cyclic **0.66 → 0.64**, callbacks **0.58 → 0.71**, doing
**15 → 78 comparisons**. Callback RSS grows about 2 MiB; other controls remain
stable. Full natural capture **881s /6.1 GiB** versus **884s /6.1 GiB** previously.
These are bounded observations, not a general performance claim.

Next audit the five direct LookMatcher ASCII predicates and their negating
wrapper. Existing borrowed-owner support rejects the nested checked-Copy DebugByte
field; length-summary resolution also invokes that same frontend check. Confirm
that dependency before adding a separate length capability. Guarded slice reads,
possible panic and helper transport remain to be assessed as a complete chain.
