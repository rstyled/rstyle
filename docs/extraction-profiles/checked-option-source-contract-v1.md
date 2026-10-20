# Checked Option source contract

This contract is the source/type half of the selected checked UTF-8 boundary
chain. It does not register Option semantic type parsing, a call summary, graph
lowering, extraction capabilities or admission. `checked_options::verify_get`
and `verify_match` return borrowed source nodes for the connected graph work;
their success cannot certify a function. D17 remains missing.

The bounded target is an original immutable scalar slice receiver, one `usize`
index and the instantiated `Option<&Scalar>` result. Supported scalar payloads
are the observed bool/integer types. Array unsizing, ranges, mutable receivers,
overloaded dereferences, explicit generic arguments and user `get` methods are
outside this first contract. The match shape has exactly two unguarded arms,
one resolved core None and one resolved core Some with explicit `&scalar`
copy binding. Either arm order and renamed variant imports are valid. Wildcards,
guards, binding a reference instead of copying the scalar and other payload
patterns require separate semantics. Whole-function operand effects remain the
responsibility of eligibility; this local contract does not claim purity.

Definitions were captured from actual Cargo builds on the pinned nightly for
aarch64 Linux, x86_64 Linux and Apple Silicon macOS. The existing target registry
contains the three additional get/Some/None identities. No new per-function
records, source copies, provenance tables or caches are introduced. The contract
checks the compiler build/core hash, definition, exact instantiated signature,
builtin receiver adjustments, original child spans and nested binding inventory.
Historical Data-v1/v2 contracts remain unavailable. Observational Option tokens
are inspected here without opening general semantic type parsing.

The independent semantic specification follows reviewed pinned core source:
`slice/mod.rs:574` delegates get to the concrete index; `slice/index.rs:179`
implements `SliceIndex<[T]> for usize`; `option.rs:598` defines the two variants.
After evaluating the receiver once and then the index once, `i < len` produces
Some of the reference at i, otherwise None. The call forms a reference and does
not load scalar contents. None is normal completion, including for usize::MAX;
no arithmetic wrap, panic precondition or fabricated zero value is substituted.
Some(&b) then loads the scalar under the Some guard. Operand evaluation can
itself panic or have effects; this rule does not erase them or prove totality.
No ABI niche, null representation or alias independence is assumed.

The whole slice module digest is already checked by the length registry tests.
Additional reviewed source digests, checked by a toolchain test:

- slice/index.rs: `sha256:080ad30d5226cc0089ba585248b88118921d19440f9c57c48371bca0d67e219d`
- option.rs: `sha256:693df7245cf3e669b2c442b62c96c76c43ad1f9cfe79f6bf4306e71c32fa9f5d`

The runtime oracle compares the exact selected natural function against
`str::is_char_boundary`, checks all 256 byte values and out-of-range indices,
and independently checks receiver/index evaluation order and saved-reference
identity when index evaluation rebinds the receiver local. Cargo contract tests
cover all three installed CI targets, renamed imports, reversed variants and
saved operands; negatives cover guard/reference-pattern/array/range boundaries,
forged core identities, swapped operands, wrong index types, missing or forged
payload bindings and mismatched build targets. These functions remain excluded.

Next connect explicit discriminators, conditional origins and guarded loads to
lowering and independent serialized graph checks, including the saved Option
and helper-result cases. Until that complete chain yields strict documents and
matching, source-contract test success is not an admission gain.
