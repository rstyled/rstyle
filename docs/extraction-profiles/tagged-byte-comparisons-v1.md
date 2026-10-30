# Tagged byte comparisons: source-first target

This source-first target was specified before implementation in Data-v3 3.48.
Its historical baseline below activates no new capability. Data-v3 3.49 connects
the bounded family; measured results appear below. The frozen 60 identities and
denominators and both existing tagged-unit development selections are preserved.

## Original selection and independent expectations

Select original regex-automata 0.4.18 `util::alphabet::Unit::is_byte`,
`src/util/alphabet.rs` bytes 7039..7135, SHA-256
`380abc4e459093f751245bd243296c420abbc176fda0777abea3de15fd0fc58b`.
Use the complete default-feature debug Cargo library, including its original
modules, imports, derives, custom Debug impl and neighboring methods, under
nightly-2026-09-30 / rustc 5c543b0b8c73c7b72bc8284ced4fb22ead15734d,
aarch64-unknown-linux-gnu. Cross-target authority must also retain the existing
x86_64 Linux and aarch64 macOS pinned contexts before activation; runtime evidence
on ARM Linux is distinct from native execution elsewhere.

For every byte payload and every query byte, the result is the mathematical
comparison payload == query. Every u16 EOI payload returns false for every query;
never constrain sentinel payloads to 0..=256. The original-source oracle now
binds this exact identity and tests all 65536 byte/query pairs, plus all 65536
sentinel payloads against queries 0, 1, 127 and 255. This extends the complete
original crate with tests only; no original body or declaration is substituted.

The selected source is `self.as_u8().map_or(false, |b| b == byte)`. Unit::as_u8,
its complete checked-Copy enum owner and scalar Option transport are already
supported. The new target must preserve that entire helper chain rather than
rewrite the original into a supported match expression.

## Simultaneous blockers and bounded contract

The complete 3.48 natural roster excludes this original function. Its reasons
include deferred call summaries/additional types, unmatched owned-source/HIR
operations and an excluded separately owned closure body. Source inspection adds
these connected requirements:

1. Option::map_or needs exact pinned core definition and instantiated signature
   authority. This is an owned Option<u8>, a bool default and a concrete closure
   returning bool. Neither the method spelling nor a generic FnOnce bound proves
   the callback's identity, effects or result. A custom map_or stays opaque.
2. The current HIR child collector has no closure-body branch. An expr_closure
   remains childless, so its argument pattern, payload binding and comparison
   are unavailable. Original-source inventory also intentionally stops at closure
   ownership boundaries. Retain the complete bounded callback body and its owner,
   parameter identity/types and source correspondence; never certify a prefix.
3. The closure has no supported ordinary data type. Do not enable arbitrary
   closure storage, escaping values or general callable interfaces to solve this
   one immediate call. Confirm what existing HIR/local/type observations prove
   before adding any missing compiler-capture facts. Reuse existing evidence
   representations; no parallel global provenance table or source census is
   justified by this audit.
4. The original predicate captures its scalar query by shared reference. Prove
   the actual capture kind, binding, primitive type and complete environment.
   Do not invent value reads at capture creation, independent aliases, capture
   purity or drop-free ownership. The initial contract is an immediate,
   nonescaping, acyclic scalar equality predicate with one immutable by-value
   payload parameter and only checked primitive scalar captures. No mutable or
   reference payloads, arbitrary callback bodies, nested callbacks or escaping
   closures are included.
5. Core map_or consumes the Option, evaluates its default argument eagerly, then
   creates the callback argument. It invokes that callback exactly once only
   for Some; None returns the already evaluated default and drops the callback
   environment. Preserve this evaluation order and positively justify the
   environment's bounded drop behavior. Default expressions must retain all
   existing scalar evaluation/write/panic evidence; do not delay them until None.
6. Payload binding and each callback-body read must be conditional on Some.
   None must never manufacture a payload, a zero value, a binding definition or
   a callback read. Reuse independently checked conditional payload/value-merge
   machinery where it applies. Full callback/body/capture inventories, saved
   results and caller/helper snapshots must survive strict serialized checking.
7. Require complete Cargo capture, independent verification, strict admission
   and matching for the natural method and all declared composed positives.
   Individual Option/closure fixtures do not establish this connected family.

Fresh development capture confirms that the existing method-receiver record
already has owned Option<u8> receiver/adjusted types with no adjustments, a bool
default input, an explicitly Unsupported closure input, and bool result. The
resolved core map_or definition is already retained. The closure node has zero
children, and its payload parameter is absent from the binding inventory. Thus
registration, scoped body/binding/capture proof and dispatch normalization are the
gaps; adding another receiver provenance table would duplicate existing facts.
The ordinary dispatch still displays Option<u8>/closure types rather than a fully
supported concrete callable signature. Preserve that Unsupported observation
until a bounded callback contract supplies the missing evidence.

Unknowns before implementation: compiler closure-owner/typeck and capture APIs,
the minimal missing capture observations, precise conditional argument/body
representation, callback-environment discard proof and source binding coverage.
Measure the cost on unchanged controls against exact c875a72. Do not add a cache
or duplicate source to mask an expensive proof design.

The original Unit::eoi and ByteClasses::eoi remain separate formatted-assertion,
checked-conversion/Result/unwrap and checked_sub/Option chains. is_word_byte uses
a function callback whose own constant-table initializer/body still needs proof.
This target does not activate that function callback, general Fn traits, arbitrary
closures, macros, async, unsafe or FFI.

## Fixed development extension and baseline

Append `tests/fixtures/function-families/tagged-byte-comparisons.rs` to the unchanged
eleven-function `tagged-units.rs`. Before admission, specify seven new positives:
Unit::is_byte, byte_matches, end_never_matches, saved_comparison, local_option,
default_before_capture and none_uses_default. Byte constructors, sentinel
constructors, scalar Option locals, helper results and saved values must compose.
The default-before-capture result is actual == 1 regardless of the initial query;
the None/default-true result is true without a payload read. These independently
specified cases distinguish eager default evaluation from conditional invocation.

Six named negatives remain outside the initial boundary:
callback_reference_payload, mutating_capture, escaping, nested_callback,
custom_map and callback_panics. The counterfeit method invokes black_box, so a
familiar method name must not acquire library purity. Its generated Clone and
method remain excluded. Preserve all eleven old classifications.

Real Cargo 3.48 baseline passes for all 26 functions: **six admitted, twenty
excluded**, with independent checks of the six existing admissions and complete
capture, assessment, enumeration and evaluation. The selected original remains
excluded in the unchanged 72/2582 natural baseline. The declared connected
expectation is thirteen admitted /thirteen excluded in this development corpus;
do not change its selection to obtain that result.

Activation requires independent original-source semantics, adversarial binding/
capture/evaluation-order/guard cases, all composed positives, existing regressions,
unchanged natural and frozen selections, completed selected matching and measured
new/lost admissions. Preserve historical reports and distinct unsupported,
unknown, inactive and incomplete outcomes. Keep rewrite safety, abstraction
activation and detector-quality claims separate from extraction coverage.

Audit validation: the 26-function Cargo baseline regression passes, as do all six
complete-original-crate oracle tests. CLI integration-test Clippy with warnings
denied, workspace/standalone formatting, Python compilation and whitespace pass.
Rstyle reports 2786 functions and zero violations; patterns reports zero candidates.
Production Rust and the 3.48 policy are unchanged. This audit does not replace the
preceding 880-test workspace run or claim a new natural/performance result.

## Compiler dependency investigation

A disposable rustc-private probe against the fixed development source confirms
that closure expression, parameter and body HirIds share the surrounding function
HIR owner. The existing HIR-tree binding collector can therefore retain payload
bindings once the complete callback subtree is captured; a separate binding
registry is unnecessary. `TyCtxt::closure_captures` supplies root HirIds, capture
modes and projection/type information, while the closure's tupled upvar type
supplies the actual stored environment fields.

The intended immediate predicates have one ByRef(Immutable) u8 root, no
projections, and an environment tuple (&u8,). The mutating negative instead has a
mutable capture/dereference and (&mut u8,) storage. Importantly, the compiler
reports FnOnce for the pure immediate predicates too. FnOnce is a call contract,
not evidence of mutation or purity. Admission must inspect complete captures,
the body, ownership/drop and the registered immediate invocation, rather than
infer those properties from the closure-kind label. No production observation or
capability is activated by this probe.

## Connected implementation (3.49)

The working implementation now collects complete callback parameter/body
subtrees and actual capture environments behind the
`rust.immediate-option-callbacks` gate. Callback parameter bindings do not become
outer-function parameters. Capture roots, modes and projection counts stay on
the closure node; actual stored environment types reuse the existing bounded
concrete-type observer. Unsupported storage stays explicitly unsupported. No
global provenance registry, source copy, cache or ordinary closure data type is
introduced.
Collection is scoped to the immediate callback argument of a compiler-resolved
core Option::map_or. Other escaping or unrelated closure bodies keep their
existing unsupported boundary, avoiding a general traversal of neighboring
closures merely to implement this family.

The predicate verifier checks original closure/equality syntax, exact
payload and capture binding identities, a distinct callback lexical scope, and
one complete (&u8,) environment. Other stored environments remain explicitly
unsupported without traversing and duplicating arbitrary captured nominal
declarations. The invocation verifier checks the existing receiver/signature
records and exact pinned map_or definitions, captured separately on all three
installed CI targets. Observed closure type strings remain consistency checks;
they do not supply callable semantics or an ordinary supported data type.

A disposable capture experiment exercises all four selected callback bodies on
ARM Linux, x86-64 Linux and ARM macOS, including missing body/environment,
by-value/projected capture, outer-parameter misclassification, missing dispatch,
incorrect default types and counterfeit callee mutations. All selected source
contracts pass; mutations fail. This is cross-target compiler evidence, not
native runtime execution on the other targets or strict graph admission.

The gated lowering evaluates receiver and default once, records shared
environment creation without a scalar content read, selects Some, binds/reads
the payload and evaluates the real equality body only on that branch, and merges
with the saved default on None. Independent serialized checks validate the full
operation inventory, argument order, branch guards and saved result. Strict
bundle re-import exercises the same connected family through matching.

The fixed 26-function development selection now has thirteen admissions and
thirteen exclusions: seven gains and no losses against its source-first baseline.
The complete family passes Cargo capture, independent verification, strict
admission and matching on all three configured compiler targets. Eight serialized
graph mutations and six captured-fact mutations are rejected. Independent runtime
oracles cover exhaustive byte comparisons, all u16 sentinel payloads, eager
default effects and the None/default path. All six original-crate oracle tests pass.
Cross-target capture does not claim native runtime execution on those targets.

## Measured results and remaining boundaries

The complete original default-feature library retains all 2582 exact function
identities and build contexts against 3.48. Admissions increase **72 to 73**:
Unit::is_byte is the sole gain, with **zero losses**, unknown or failed outcomes.
Complete capture and assessment are distinct from the intentionally budgeted-out
full pair enumeration (2628 possible pairs). Independent serialized verification
checks all 73 admitted natural documents and all thirteen development documents.
All 190 comparisons among seven selected original and thirteen development
functions complete with unchanged defaults, no incomplete evaluations and no
diagnostics. The full original-source oracle passes all six tests.

The unchanged frozen selection remains **20 admitted /33 excluded /7 inactive**
with all sixty identities and denominators preserved. Complete package rosters
also retain their identities, contexts and outcomes: lz4_flex 150 functions /seven
admitted, regex-syntax 956 /twenty, and time-core 226 /eight. Frozen replay verifies
seventy documents. Historical reports are not rewritten.

Performance uses the exact c875a72 source baseline, the pinned compiler, one warmup
and seven alternating quiescent pairs per workload. Median elapsed seconds
(baseline/current) are Copy control **0.89 /0.87**, cyclic control **0.62 /0.62**,
and callback development **0.78 /0.94**. Observed ranges are respectively
0.86–0.91 /0.86–0.90, 0.60–0.67 /0.60–0.65 and 0.75–0.82 /0.91–0.96 seconds.
Median peak RSS (KiB) is **123064 /123204**, **111760 /111572**, and
**121700 /124208**. Control matching results are identical. The callback corpus
performs additional capture, proof and matching as its admissions rise from six
to thirteen; this is not an equal-work performance comparison. These bounded
measurements show no control regression and make no broad performance claim.
No cache or additional global provenance registry is introduced.

Unit::eoi remains blocked by formatted assertions, concrete checked conversion/
Result/unwrap, helper links and owned-source/HIR correspondence. Unit::is_word_byte
passes a function item to map_or; it needs complete callable/helper identity and
its constant-table body proof, beyond this immediate closure contract.
ByteClasses::eoi still combines checked_sub/Option unwrap with the unsupported
Unit::eoi helper. Broader payloads, captures, move/escaping/nested closures,
iterators, async, unsafe and FFI remain unsupported. This increment claims no
abstraction activation, rewrite safety or held-out detector quality.

Validation: workspace Clippy with warnings denied, doctests, formatting and
whitespace checks pass. Fresh rstyle reports **2827 functions /zero violations**;
patterns reports **zero candidates**. The final full workspace CI run passes
**884 tests /five skipped**, including the reported assertion/subslice regressions
and the corrected definition-count assertion.
