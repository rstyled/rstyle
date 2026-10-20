# Tagged byte-unit constructor and projections

This is the next source-selected development target after Data-v3 3.46, not an
activated capability. The frozen inventory and its denominators stay unchanged.

## Original selection and semantics

Use regex-automata 0.4.18, complete default-feature Cargo library, original
`src/util/alphabet.rs`, with all neighbors, imports, derives and impls retained.
The selected family is `Unit::u8`, `Unit::as_u8` and `Unit::as_eoi`:

| Function | Bytes | SHA-256 |
| --- | --- | --- |
| `Unit::u8` | 4985..5053 | `2433cddaafe16f0cc09ef07d8d3c6f9240f63a5bee12b515b979b420046b14d9` |
| `Unit::as_u8` | 5977..6128 | `a27ac16335fbe204e853b2665b5db25c033ade8be5978f7143c8d090995142f8` |
| `Unit::as_eoi` | 6302..6469 | `b019b70f74e1c40b13b7e6788b7f2b3584de2d870fc92e84c0342e3096f3e825` |

All bytes roundtrip through construction and `as_u8`, with `as_eoi` returning
None. Every EOI u16 payload projects unchanged through `as_eoi`, with `as_u8`
returning None. Zero byte and zero sentinel are distinct tags. The getter
interface must preserve all u16 payloads, including those outside the public
sentinel constructor's 0..=256 domain. Never infer a u8 or 0..=256 payload bound
from the commonly used constructor.

The original `Unit::eoi` is explicitly outside this initial family: its assertion,
checked usize-to-u16 conversion and unwrap require additional resolved contracts.
The original `as_usize` also requires exact u16-to-usize From authority, while
`is_byte`/`is_word_byte` require Option combinators and callback/closure support.
Supporting the selected three methods does not activate those neighbors or the
broader DFA caller chain. `is_eoi` additionally requires Option::is_some.

`tagged-units-oracle.py` verifies the three identities and the original eoi
constructor identity, copies the complete crate and appends only tests. Three
tests pass in ARM Linux default features: all 256 bytes, all 65536 sentinel
payloads, all valid public sentinel arguments and representative invalid
arguments including usize::MAX. These exercise original methods and their Copy
reuse, without substituting declarations or method bodies. This is runtime
semantic evidence, not extraction or native execution on other targets.

## Full blocker chain and bounded contract

`Unit` is a tuple struct containing `UnitKind`, a two-variant enum with one u8 or
u16 payload. Both declarations derive builtin Copy and Clone, alongside equality
and ordering derives. Unit's custom Debug impl and all generated neighboring
methods must remain present; their operations are not executed by these methods.
This family avoids the checked-reader error chain's non-Copy ownership, generic
checker, local macro-generated payload declarations and residual semantics.

Known simultaneous blockers are:

1. Concrete type capture currently handles structs, not local enums. Unit's
   complete field tree therefore fails too. Source/nominal verification must
   independently inventory both variants and payload types, and reuse existing
   declaration ownership and positive builtin-Copy observations. No parallel
   declaration table, duplicated source inventory or cache is justified.
2. The current nominal mixed-unit boundary has no enum field contract. Admit
   neither only the selected variant nor an unsupported field prefix. Require
   the full finite declaration, exact owning module/import context and positive
   Copy evidence; exclude custom derives, Drop, generic/recursive/reference
   payloads and unresolved expansions under this bounded contract.
3. General local variant construction has no registered semantics or exact
   resolved constructor check. Constructors must retain the variant tag and
   payload evaluation, rather than representing an enum as its scalar payload.
4. Normal match arms are captured, but both tuple-variant patterns currently
   become childless `pat_unsupported`. The payload binding path remains in the
   arm body while its defining pattern is absent. Merely adding an enum token
   cannot recover its declaration or certify that read.
5. Match lowering only accepts existing scalar/checked-Option patterns. Enum
   selectors and immutable scalar payload declarations must be conditional on
   the exact selected variant, with independently exhaustive disjoint arms.
   A missing arm, wrong variant identity or payload type cannot certify a match.
6. The resulting scalar Options, by-value Copy receiver, constructor result,
   helper transport and saved results must compose through real Cargo capture,
   verification, strict admission and matching. Existing support for these
   pieces separately is not evidence of complete enum composition.

Unknowns to resolve before activation: original constructor HIR identity shape,
exact variant-pattern resolution/capture and enum declaration source span
ownership under current builtin derives; complete helper snapshots for tagged
values; resulting performance on unchanged scalar controls. No ABI tag layout
or representation-based equality is required or authorized.

## Independent development classification and baseline

`eval/coverage-development/tagged-units.rs` is a separate fixed corpus. Before
assessment, six functions are intended in scope: `Unit::new_byte`, `new_end`,
`byte`, `end`, `roundtrip` and `saved`. Both constructor tags and both getters
must compose, including a saved result after the receiver local is rebound.
Three named negatives remain outside the bounded contract: `nested` returns
Option<Unit>, `reference_payload` returns an Option slice reference, and
`residual` uses unsupported Try. The two generated Clone methods also remain
excluded. This is not a new frozen selection.

Real Cargo Data-v3 3.46 capture assesses all eleven functions: **0 admitted /
11 excluded /0 unknown**, complete capture/assessment and empty-pair enumeration
and evaluation. The fresh original 3.46 roster likewise excludes all three
selected methods with the exact identities above. No new natural capture is
claimed by this audit and no natural gain is claimed.

Activation requires the six complete development positives and three original
methods, independently verified complete compositions and matching, unchanged
selection deltas, adverse declaration/tag/payload/arm/snapshot cases, existing
regressions and bounded performance comparisons. If the first foundational
increment does not produce natural gains, name its concrete remaining chain;
reassess before a second such increment. Keep extraction, rewrite safety,
abstraction activation and detector quality separate.

Audit validation: all three original-crate oracle tests pass; Python compilation,
standalone Rust formatting and whitespace checks pass. Rstyle reports 2733
functions and zero violations; patterns reports zero candidates. Production
Rust is unchanged, so this audit does not replace the preceding milestone's
867-test workspace validation or its performance measurements.

## Observational tuple-variant pattern increment

Data-v3 now retains local enum tuple-pattern constructor identity and ordered
payload patterns in the existing HIR node tree as `pat_local_variant`. This
repairs blocker 4's missing binding declarations: the scalar binding HirId and
symbol now remain alongside the body read of that binding. The type observation
remains unsupported; no enum token, capability, source exception, match grammar
or helper-summary permission is activated. All eleven development functions
remain excluded. Historical profiles retain childless unsupported patterns.

The observation requires an actual local enum type and a resolved tuple
constructor belonging to that enum. It reuses the existing callee identity
representation, including the established local-unencoded crate marker, without
requesting local metadata before encoding. Tuple structs and patterns with a
rest marker remain unsupported. Reference payload bindings retain their distinct
unsupported binding-pattern shape rather than acquiring scalar binding support.
The special checked core Option path remains first and unchanged.

Two real-Cargo regressions check exact constructor paths, payload arity, binding
identity and declaration inventory, wildcard preservation, old-profile shape,
reference/rest/tuple-struct boundaries, unchanged exclusion and producer rejection.
This is the first production prerequisite after the source-selected audit; the
remaining connected chain is complete enum capture/declaration proof, variant
construction, conditional match/payload semantics and helper composition. Do not
follow it with a second isolated observational increment without reassessing
practical family progress. No natural admission gain is claimed here.

Validation: all 869 workspace nextest tests pass with five existing skips;
all-target Clippy with warnings denied, formatting and whitespace checks pass.
Rstyle reports 2735 functions with zero violations and patterns zero candidates.
No thresholds, suppressions or ignore entries change.

Against isolated 347ddce, one warmup and seven alternating quiescent pairs per
workload retain all source identities and outcomes. The 16-function byte-window
control stays at eleven admissions; baseline/current median .89/.88s (ranges
.86–.90/.87–.92), median RSS 122240/122228 KiB. The six-function cyclic control
stays at four admissions; .64/.65s (ranges .63–.67/.62–.66), RSS 111076/111076.
The eleven-function enum corpus stays at zero admissions; .30/.29s (ranges
.27–.34/.28–.35), RSS 110664/110496. All fourteen retained unchanged-control
matching reports and seven enum empty matching reports are exactly identical.
The cyclic +.01s median shift has overlapping ranges; these bounded observations
do not establish a broad performance guarantee or a natural admission gain.
