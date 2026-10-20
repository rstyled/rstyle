# Scalar Option byte widths: declared development target

This is a source-selected target, not active support. The current extraction
policy remains Data-v3 3.41.0. Keep the frozen 60-function identities,
denominators and historical outcomes unchanged.

The selected natural family is two UTF-8 leading-byte width classifiers in
unmodified default-feature library Cargo contexts on the pinned compiler:

- regex-automata 0.4.18, src/util/utf8.rs, util::utf8::len. The original `fn len`
  fragment occupies bytes 3370..3676, SHA256
  `fe0699f136d80684c991fa07d7c114eee694c8e31343f177df2590e3c56e0012`.
- regex-syntax 0.8.11, src/debug.rs, debug::utf8_decode::len. The original nested
  `fn len` fragment occupies bytes 2772..3170, SHA256
  `a4566fa67dbb9e00d3a5c10d09b7ed5449ad83543baabf8678ded2d6b4c3c27b`.

These fragment identities are specified from published source before admission
assessment. Compiler-captured function identities must be checked separately,
including the outer module/function context and applicable attributes. The
nested helper cannot be certified by omitting its surrounding decoder or its
neighbors from source inventory. The classifiers' decoder callers remain
outside this bounded target: they also require char and nested Result values,
guarded payload patterns, UTF-8 library/iterator summaries and error propagation.
No supported prefix of either decoder may be certified as the complete decoder.

## Independently specified behavior and scope

Both classifiers return exactly this partition for all 256 byte values:

| Inclusive byte range | Expected result |
| --- | --- |
| 0..127 | Some(1) |
| 128..191 | None |
| 192..223 | Some(2) |
| 224..239 | Some(3) |
| 240..247 | Some(4) |
| 248..255 | None |

This is width classification, not validation of a complete Unicode scalar
encoding. In particular C0/C1 return Some(2) and F5/F6/F7 return Some(4); later
decoder validation remains necessary. None is a concrete variant without a
payload, not an unknown result or a zero payload. Some retains the exact scalar
and does not imply any memory load, allocation, alias independence or purity of
its operand. Operand effects and early returns must retain their existing flow.

The bounded intended extension is core Option with one bool/integer scalar:
resolved Some/None construction, parameters/results/locals, exhaustive unguarded
Some(scalar)/None payload binding, scalar-result consumers and supported helper
transport. Preserve saved values across subsequent scalar rebinding. Existing
reference-Option checked access must retain its guarded memory-load semantics;
scalar payload binding must not fabricate a reference location or a read.
Reuse existing concrete observations, resolved definition records, source spans,
CFG/PDG value operands, guards and merge evidence. No new serialized inventory,
epoch/provenance table, source copy or persistent cache is justified.

Nested Option/Result, owning nominal or reference payload construction, arbitrary
enums, wildcard/guarded variant patterns, if-let, question-mark desugaring,
mutable access and general methods remain outside this first extension. Renamed
core variant imports must retain their actual identities; a custom enum or
function named Some/None/Option must not gain core authority through spelling.
Unsupported, unknown and inactive capture outcomes remain distinct.

## Full blocker audit before implementation

Source inspection establishes these connected gates; first-rejection evidence
alone will not establish the family:

- The driver already observes scalar core Option through its diagnostic-item
  identity. concrete_types validates that observation, but data_types semantic
  parsing only admits the shared-scalar payload required by checked slice get.
  Scalar Option interfaces and expression types therefore lack semantic support.
- Existing constructor lowering covers plain aggregates and registered calls,
  not core Some/None values. Some calls need exact constructor/type/source and
  pinned definition checking; None paths currently use the generic nonlocal
  path resolution rather than a resolved Option-variant record. Reuse that
  existing record vocabulary instead of inventing constructor provenance.
- checked_options types, pattern verification and lower/options currently
  require Some(&scalar) and perform a guarded memory copy. Owned scalar binding
  needs a distinct verified value projection with no invented content read.
- Scalar Option values must survive conditional/range-match merges, explicit
  early returns, local saves, scalar rebinding and helper result transport.
  Every type/interface, value-operand, snapshot, return and helper-summary gate
  must be checked; a type-parser change alone cannot activate support.
- Current scalar range patterns already exist and must retain their exact
  endpoint/type/source evidence. This target is not a reason to create another
  syntax capability for the existing match ranges.
- Historical policy gates must keep older shared-only Option contracts closed
  to scalar construction and binding. General semantic parsing cannot silently
  extend old document policies or Data-v1/v2.

Unknowns to resolve from actual capture and code are nested-helper source-owner
correspondence, exact Some constructor versus pattern definition identity on all
pinned targets, early-return and merge transport for a non-scalar token, and
whether existing helper summaries handle reference-free Option results without
additional summaries. Confirm dependencies before redesigning these paths.
Neither selected function needs nominal ownership or a relaxed mixed-unit rule;
neighboring macros, derives and unsupported decoder bodies remain accounted for.

## Fixed development selection and validation requirements

The separate corpus eval/coverage-development/option-byte-widths.rs fixes four
intended positives before evaluation: match_width, branch_width, width_or and
saved_width. The first two implement the source-selected natural shapes;
width_or consumes a saved helper result with an arbitrary fallback, and
saved_width must preserve that result after rebinding its byte input to zero.
The independent oracle specifies all 256 results, fallback values 0/5/usize::MAX,
saved-result behavior and the explicit non-validation cases above.

Two declared negatives remain excluded: nested returns Option<Option<usize>>;
custom uses a user enum whose variants have familiar core names. Neither is an
incomplete approximation of an intended positive. Capture the entire six-function
selection, retain every identity, and compare all outcomes and complete positive
pair evaluations. Exercise original natural Cargo contexts and matching with the
development counterparts before claiming useful admission gains.

Independent document checks must reject wrong constructor/variant identity,
missing/swapped payload operands, manufactured None payloads, stale local/helper
snapshots, missing Some guards and incorrect payload bindings or memory effects.
Retain existing checked-reference Option, early-return, loop-merge and historical
profile regressions. Run workspace tests, clippy, formatting, rstyle and patterns;
require zero style violations/candidates without threshold or suppression changes.
Compare unchanged selections for new/lost admissions and measure relevant
performance before activation. A green constructed suite alone is insufficient.

## Baseline evidence and refined dependency audit

The original Cargo-captured natural names, spans and source digests exactly match
both declared fragments. util::utf8::len is excluded in the complete Data-v3
3.41.0 regex-automata capture. debug::utf8_decode::len is excluded in the retained
complete Data-v3 3.40.0 regex-syntax capture; its identity is explicitly nested
and retained, not omitted with the unsupported outer decoder. A fresh/replayed
current-policy natural comparison remains required before claiming gains.

The six-function development corpus completes capture/assessment under 3.41.0:
0 admitted /6 excluded, no unknown outcomes. Both positive classifiers report
source-type, expression-type, unresolved constructor and nonlocal path gates;
the two scalar consumers additionally report uncertified helpers and unsupported
owned payload patterns. Both declared negative forms retain their distinct
unsupported concrete type outcomes. No admission rule has changed.

The first source-type error is `Nominal path requires only bounded lifetime
arguments`, from concrete_types/lifetimes and data_frontend's SourceProof route.
A scalar Option interface therefore needs a precise source/type correspondence
route for core Option and its scalar generic argument. Do not relax nominal
lifetime rules for arbitrary structs or ignore neighboring declarations. Confirm
builtin/prelude/import resolution and retain shadowing/alias restrictions before
changing this route. This adds a source-boundary dependency to the semantic type,
construction, payload projection, flow and helper gates already listed above.

Two independent runtime tests pass: the explicit six-part byte partition over
all 256 values (including three arbitrary fallback extremes and saved rebinding),
and the non-Unicode-validation cases C0/C1/F5/F6/F7. This establishes expectations
for a bounded family; it does not establish source-to-pair support or decoder
coverage. No serialized provenance, production code, scoring, thresholds,
suppressions or historical artifacts changed in this audit.

## Constructor authority capture checkpoint (no admission activation)

The existing expression walk now retains `resolved_option_variant` for actual
core Some calls and None paths with an already supported observational payload.
It reuses the same Callee identity record and diagnostic-item check as Option
patterns. Some retains its original argument child; None has no fabricated
payload. Local functions and user enums with similar names do not acquire this
marker. No dispatch/purity summary, semantic type, constructor operation,
extraction capability, policy version or admission permission is added.

Real Cargo tests verify exact core crate/definition hashes from the existing
three target registries, including renamed imports, std/no_std contexts, custom
enum variants and a local function named Some returning the real Option type.
All six installed CI-target/std combinations pass. These are cross-target
captures, not native execution on macOS. Historical Data-v2 expression capture
remains unchanged, and all eight identity-test functions remain excluded.

This removes the missing constructor-authority evidence in the declared chain.
Precise source-type correspondence, scalar Option semantic types, source-checked
construction, payload value projection, merges/early returns/local snapshots and
helper-result composition still need connected implementation and independent
verification before any natural admission gain can be claimed. The declaration's
four intended positives and two negatives remain unchanged.
