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
   contents and indexed value/bounds. Retain the nested runtime function as its
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
not promised gains. Unknowns: completed constant-index helper certification,
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
