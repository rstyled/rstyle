# Checked native reader dependency audit

Source inspection after Data-v3 3.44.0; this is a planning boundary, not a
supported capability or a new frozen selection.

## Source-selected family and context

The next connected candidate is regex-automata 0.4.18 `src/util/wire.rs`:
`util::wire::try_read_u16`, `try_read_u32`, and `try_read_u128` (lines 705,
720, and 735). Retain the complete original default-feature Cargo library
context used by the native-prefix milestone, including its no_std root,
neighboring types, derives, and imports. Do not replace its error type with a
scalar fixture to claim natural admission. Existing frozen identities and
all denominators remain unchanged.

## Full known dependency chain

1. Each reader takes `&[u8]` and `&'static str`, calls
   `check_slice_len::<u8>`, propagates its Result with `?`, then constructs
   `Ok((read_uN(slice), size_of::<uN>()))`. Native prefix leaf extraction is
   supported; the composed checked reader is not.
2. `check_slice_len<T>` (line 775) is generic over the slice element. Its
   instantiated body must be checked, rather than assigning a summary based
   on its name. It compares slice length to the requested bound, explicitly
   returns an error on the short branch, and otherwise returns `Ok(())`.
3. The error constructor `DeserializeError::buffer_too_small` (line 159)
   wraps `DeserializeErrorKind::BufferTooSmall { what }` in the tuple struct
   `DeserializeError`. The enum also contains other variants, including
   PatternIDError and StateIDError payloads. Both error declarations derive
   Debug. Neighboring declarations and potential drop behavior require a
   complete declaration dependency audit; recognizing this one constructor
   does not establish that audit.
4. The current general data interface (`data_types::TypeToken`) has Option
   and the specialized slice-array conversion Result, but no general Result
   or nominal enum token. Reusing the specialized byte-conversion Result
   would lose the actual error payload and is invalid. Tuple support alone
   does not cover `Result<(uN, usize), DeserializeError>`.
5. `?` introduces resolved Try/FromResidual behavior and early exit. The
   success continuation must consume the actual Ok payload; the error branch
   must retain the actual propagated error and skip the native read entirely.
   This cannot be replaced by an unconditional call or a synthetic success.
6. `helper_calls::check_boundary` explicitly rejects subslice places,
   copy-result and unwrap constructs. Existing summaries derive
   `may_panic=false` and `always_returns=true` only after that boundary passes.
   All three native readers violate it. Their extraction does not authorize
   transporting their omitted panic behavior into a caller summary.
7. A preceding successful length check does establish the intended runtime
   safety of the following fixed-width read, but no current contract exports
   that relational fact across these helpers. Supporting partial helper
   effects and proving caller-side preconditions are distinct possible designs;
   neither may silently set the existing nonpanicking summary flags.

Remaining unknowns: exact captured generic instantiation and residual HIR
shapes in this complete unit; the transitive error declaration/drop closure;
which independently verified minimal representation can transport conditional
helper reads without duplicating callee graphs or introducing a cache. Source
inspection is not a complete compiler-fact audit and does not settle these.

## Independently specified expectations

For widths 2, 4, and 16, inputs shorter than width return BufferTooSmall carrying
exactly the original description; they must not panic or read content. Inputs
of at least width return the native-endian unsigned value of exactly the first
width bytes and the consumed width. Extra bytes do not affect the value.
Empty descriptions and long static descriptions preserve their identity/value.
All shorter lengths, exact length, tails, zero/high-bit/max patterns, and
independent numeric byte accumulation belong in the runtime oracle before
measuring any admission gain.

Composition cases must include saved slice selection, argument evaluation
before helper execution, early errors skipping subsequent reads, and a caller
with an insufficient check. A bare native-reader caller remains potentially
panicking; an unrelated or inverted length check must not certify it as total.
A same-named custom checker, custom residual conversion, missing declaration,
unsupported drop body, or incomplete dependency must remain excluded/unknown
as appropriate. Signed readers and closure-based `*_as_usize` wrappers are
outside this candidate family and must not inherit support from its name.

## Implementation decision gate

Do not start with general `?` lowering: at least four independent boundaries
would still prevent a practical gain. First inspect captured facts for the
three selected readers and their complete dependency closure using the existing
unit store. Choose between bounded partial scalar-helper transport and concrete
Result/error representation only after that evidence is available. Reuse
existing resolved types, definitions, operand edges and saved origins. Add no
parallel provenance table, source copies, cache, or generic specialization
framework merely for this family.

Any foundational increment must name the exact chain above that it advances.
Before a second increment without natural gains, reassess the family against
other source-selected read-only callers. Activation still requires original
Cargo capture, independent document and producer verification, strict admission,
matching, unchanged-selection deltas, adversarial composition cases, rstyle zero
violations/pattern candidates, and bounded performance comparisons. No new
admission or performance result is claimed by this audit.

## Existing complete-capture baseline

The saved Data-v3 3.44.0 complete-unit roster confirms **0/3** selected
readers admitted, with both dependency functions also excluded. Each source
digest below was rechecked against the original file bytes during this audit.
These are candidate identities, not additions to the frozen selection.

| Function | Byte span | SHA-256 |
| --- | --- | --- |
| `util::wire::try_read_u16` | 27879..28093 | `3707b6f20da7d8dd8764c106dabe5d335917c5ff6ddd5863c5358a510fd8794a` |
| `util::wire::try_read_u32` | 28481..28695 | `7fbbabf9a4fdb9c6f687e3fafdbe23a67db991bb5f25fecb24ac7664b1c9b1b6` |
| `util::wire::try_read_u128` | 29085..29304 | `54df143a143a2b6ba12cd335a957ad2200e154dda14a3706cef58232c86f26e4` |
| `util::wire::check_slice_len` | 30713..30965 | `d2fb054c313fd376ce0c006238e49f50aa37e49ec84807fba1b16dcb8cd53f19` |
| `util::wire::DeserializeError::buffer_too_small` | 6900..7050 | `294b8c2c73d68e637c40e58c668e2d71294f91d476fba37ea969a3d3fae5ad66` |

All three readers report the additional-types source gate, missing match
source/HIR correspondence for residual lowering, unsupported result types,
and unavailable/unsupported helper dependencies. The generic checker reports
its lifetime-only concrete contract restriction as well as unsupported types
and call summaries. The error constructor reports unsupported concrete types.
These diagnostics confirm multiple simultaneous blockers; they do not replace
the source dependency analysis or establish that all transitive blockers have
been enumerated.

Evidence: the complete current report recorded in the native-prefix contract,
functions 2314–2316, 2320 and 2280 respectively. No new capture or duplicate
full report was necessary to establish this baseline.

## Captured-fact follow-up and runtime oracle

The existing shared unit confirms that both slice and static-string reader
parameters are already captured. Reader results and the error constructor result
are Unsupported concrete observations; the generic checker retains `T` and an
unsupported slice parameter, rather than a separately instantiated u8 body.
The native leaf calls already carry resolved direct-callee definition identities,
and size calls retain their supported scalar-size evidence.

Crucially, each `?` is an `expr_match` with no captured children. The driver
`Context::children` captures only `MatchSource::Normal`; it does not traverse
Try desugaring. Therefore the checker call and residual operations inside that
match are absent from the retained subtree. Adding a Result type token cannot
recover them. Any future Try capture must explicitly distinguish synthetic
compiler nodes from original source operations and retain the operand and both
continuations; it must not reinterpret this empty match as a supported prefix.
Enum construction is likewise unnormalized, with no resolved constructor record
in the current nested error expressions. These are actual capture gaps, beyond
the admission contract gaps above. No production grammar was widened here.

`checked-native-readers-oracle.py` copies the complete original crate, verifies
all five source identities, appends only its runtime test module, and executes
the original default-feature library tests with a name filter and locked Cargo
dependencies. Three tests pass on ARM Linux: all 65536 u16 words, every bit of
the wide input, every short length, tails and exact sizes, and BufferTooSmall
payload value/pointer preservation for three static descriptions. This uses
the actual original error enum and helpers, not substituted fixture bodies.
It specifies runtime results only; it does not establish extraction admission,
absence of speculative machine loads, or native execution on other targets.

The next production contract must account for this missing Try operand capture
as well as the helper/error chain. Keep general enums and arbitrary generic
specialization out of a scalar-helper increment; such an increment must preserve
partial behavior and cannot claim the checked family complete. No admission
change is expected from the runtime oracle or this audit.

## Observational Try capture increment

Data-v3 now tags compiler `MatchSource::TryDesugar` as `expr_try_desugar`
and retains its scrutinee and both arms through the existing HirNode tree.
The scrutinee preserves the original operand under the generated branch call;
the arms retain patterns, payloads and residual return expressions. The distinct
node kind prevents compiler-generated continuations from being interpreted as
an ordinary source match. No synthetic arm is certified as original syntax.
Historical profiles retain their previous empty-match observation shape.

This is observational only: no new capability, type grammar, summary identity,
admission rule, source exception or purity assumption is introduced. The new
node remains unsupported. Result and nested Option examples remain excluded,
and producer construction rejects the nested Option example. Tests additionally
check that each nested Try has both arms and its original operand exactly once.
Existing saved reports above remain historical evidence and are not overwritten.

This advances the missing-operand link in the declared reader chain. It does
not solve generic helper instantiation, nominal enums/drop, Result interfaces,
residual verification or partial-helper summaries. There is no natural admission
gain claimed for this foundational increment.

## Post-3.46 error dependency closure

Reassessment uses the fresh complete 3.46 original-library roster: the three
selected readers, generic checker and error constructor retain the exact five
source identities above and remain excluded. Native prefix leaves are now
supported, but their partial-helper boundary is still a separate blocker.

Source inspection resolves the error field chain more precisely:
`DeserializeError` owns `DeserializeErrorKind`; its ten variants contain only
static strings, u32, usize, PatternIDError and StateIDError. The latter two
declarations are generated by `index_type_impls!` in `util/primitives.rs`, each
wrapping `SmallIndexError { attempted: u64 }`. Their derives are Clone, Debug,
Eq and PartialEq, without Copy. This is not the checked builtin-Copy authority
used for the byte-membership family. Borrowed-only nominal support likewise
cannot authorize these owned error results.

The independent original-crate oracle now checks `needs_drop` for all five
declarations and the wide reader Result; all are false in the tested ARM Linux
default-feature context. All four oracle tests pass. This settles a runtime
drop expectation for that context, not a pipeline declaration certificate or
source correspondence for macro-generated declarations. No declaration may be
silently omitted merely because the selected constructor uses BufferTooSmall.

The next implementation decision must therefore account for bounded owned
non-Copy declaration verification and the exact local macro-generated payload
declarations, as well as concrete Result, generic checker instantiation, residual
semantics and partial-helper transport. Observational Try capture has already
advanced one prerequisite without admission gains: do not follow it with an
isolated Result token and call this family supported. Reassess a smaller natural
Option/read composition before committing to this entire closure. No production
code, capability, provenance table, cache, admission or frozen selection changes
in this follow-up.

Follow-up validation: the four original-crate tests pass, standalone Rust
formatting and whitespace checks pass, rstyle reports 2733 functions with zero
violations, and patterns reports zero candidates. Production Rust is unchanged;
the earlier 867-test milestone remains its last full workspace validation.
