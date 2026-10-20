# Function-family development targets

This development work is separate from `eval/hir-coverage/selection.json` and its
historical denominator. It is not a holdout or detector-quality study. Expectations
below are specified from original source before a new admission experiment.

## Initial target: regex-syntax 0.8.11 position/span data APIs

Use the unmodified published package, existing pinned archive/lock from
`../hir-coverage/provenance.json`, library target, default features, development
profile, test=false, pinned workspace compiler, Data-v3. Preserve the complete
Cargo unit. Do not extract the structs into a clean miniature crate as evidence
of natural-crate support. The first target is four functions in `src/ast/mod.rs`:

| Function | Source semantics / independent positive oracle |
| --- | --- |
| `Position::new(offset, line, column)` | Return the three arguments in their exact corresponding fields. |
| `Span::new(start, end)` | Return both complete Position values in their respective fields. |
| `Span::is_empty(&self)` | Compare only start.offset and end.offset for equality. Different line/column values must not affect the result. |
| `Span::is_one_line(&self)` | Compare only start.line and end.line for equality. Different offsets/columns must not affect the result. |

These functions are selected for ordinary data behavior and a shared declaration
chain, not for admission success. The source byte identities are already frozen
in the original inventory; that inventory is not being edited. At initial selection they were all
excluded; current measured outcomes are recorded below. The intended bounded contract is nongeneric, immutable scalar
struct construction/projection/comparison with exact local nominal ownership and
transitive drop-free fields, inside a mixed-content crate. It does not admit the
neighboring enum, formatting, trait dispatch or destructor bodies.

Follow-on composition targets are `Span::splat` (calls Span::new and duplicates a
Copy value), and `Span::with_start` / `with_end` (by-value self plus struct update).
Do not count the initial target complete until all four functions pass actual
Cargo capture, independent verification, strict admission and matching. Do not
count follow-on targets from isolated constructor or update tests.

## Blocker audit (2026-10-20)

This is a source and implementation audit, not a claim that all blockers have
already been discovered experimentally.

| Shared or function-specific blocker | Evidence and required investigation |
| --- | --- |
| Root attributes | lib.rs has no_std and cfg_attr(docsrs_regex, feature(doc_cfg)); census boundary_attributes accepts only doc/lint attributes. Must distinguish semantic crate configuration from unsupported expansion without dropping attributes. |
| Module closure | Root includes cfg-selected std/test items and many external modules; Closure::child rejects every module attribute. Need selected presence/path and original ownership correspondence for every captured file. |
| External/named imports | ast imports core::cmp::Ordering and alloc items; resolution indexes all use items, rejects globs/leading-colon paths, and has no general external nominal proof. Prove that imported bindings cannot be mistaken for the local Position/Span declarations; do not silently discard imports. |
| Target derives | Both structs derive Clone, Copy, Eq, PartialEq and cfg_attr(arbitrary). Current declaration/census checks reject expansion attributes. Need exact built-in derive identity and active cfg evidence; spelling alone cannot justify trusted expansion. arbitrary-enabled context remains out of initial scope. |
| Neighboring items | ast contains enums, trait implementations, formatting macros and Drop for Ast and ClassSet. The recursive unit whitelist rejects these. Need exact ownership/dependency analysis and complete treatment of items capable of defining Drop or altering selected names. Neighboring destructors cannot be treated as absent. |
| Transitive type evidence | Position has three usize fields; Span contains two Position fields. Driver checks local struct shape, no destructor and no UnsafeCell and recursively captures field types. Independent source proof must establish corresponding declarations and drop behavior; rustc's observation alone is insufficient. |
| Constructor/result composition | Existing aggregate representation is relevant but must be tested with both nested nominal results and real crate ownership. No presumed admission from syntax support. |
| splat follow-on | Callee certification and complete nominal result transfer depend on Span::new; existing unsupported-helper rejection is downstream, not a separate purity proof. |
| Update follow-on | supported_receiver currently accepts shared reference receivers only. By-value self and update evaluation order are independently required. |

Unknowns to resolve before implementation: exact selected module/cfg closure in the
captured unit; full expansion ownership of target derives; whether all needed
checks can reuse existing capture evidence; any further rejection after boundary
proof; cost of dependency analysis. Inspect the existing unit/source evidence
before adding transport records, snapshots or caches. Reuse parsed unit state;
avoid a complete source/trait rescan per function.

## Independent negative and composition expectations

- Swapping constructor fields changes the graph/value oracle. Equal scalar values
  in one test must not hide swapped dependencies.
- Getter graphs must preserve the complete start/end and offset/line paths; a
  column read, missing field read or invented alias independence must fail.
- A selected struct with Drop, a field with Drop/interior mutability, unresolved
  expansion affecting that struct, ambiguous import/owner or missing source must
  remain excluded or incomplete as appropriate, including nested and cfg forms.
- A neighboring unrelated destructor may become compatible only after proving
  its exact owner and all selected types' drop closure. A macro that can introduce
  a relevant impl is not automatically unrelated.
- Custom derives, active arbitrary derive, unresolved cfg/path modules and
  missing expansion ownership remain outside the initial contract.
- Mutated source buffers, field declarations, nominal identities, ownership and
  graph accesses must fail independent verification and strict replay.
- Complete Cargo tests must exercise constructor results consumed by getters and
  the follow-on constructor call chain. Matching must distinguish altered field
  paths while accepting independently justified equivalent family members.

Before each implementation milestone, refine this audit from actual captures and
record every discovered blocker, preserving unknown/inactive/incomplete outcomes.
Report new/lost admissions for this unchanged four-function target and the frozen
60-definition inventory separately. Reassess if successive foundational milestones
produce no natural admission gains. Keep abstraction activation, rewrite safety
and detector-quality claims separate.


## Captured-evidence findings and first implementation step

Inspection of the completed 3.30 capture (`run-1sbtdgtr`) for Span::is_empty
confirms aarch64-unknown-linux-gnu, default/std/unicode features, debug assertions,
overflow checks and panic=unwind. The existing build cfg list and source
correspondence cfg list contain the actual selected compiler configuration; the
nominal census contains 31 original files. No new cfg record or source buffer is
needed. Function-local ExpansionSite only stores an outer callsite and snippet;
it is not a unit-wide resolved derive/impl ownership inventory. A derive spelling
allowlist cannot supply that missing proof.

`Configuration::effective_attributes` now expands nested active cfg_attr metadata
using the existing cfg set, preserves ordinary attributes (including derives,
path, and cfg), and rejects unsupported predicate operators. Its output is not
item-presence, expansion-support or drop certification. Tests independently check
inactive versus active custom derives, nested conditions, path effects, retained
cfg presence and unknown predicate rejection. Admission is deliberately unchanged:
this API has no production caller yet. It advances the conditional-attribute
part of the declared boundary chain; it does not solve the missing generated-impl
ownership evidence. Two focused tests, workspace clippy, formatting, and repository
rstyle/pattern gates pass with zero violations/candidates. No natural admission
experiment was rerun for this helper-only increment; no gain is claimed.

Next connect a selected-unit boundary proof to the existing cfg/ownership evidence
and determine the smallest justified generated-impl proof. Preserve the full unit
and relevant neighboring Drop/macro effects. Do not stack further isolated syntax
increments without progressing this four-function target.

## Reassessment: active read-only target

After the planning and conditional-attribute checkpoints produced no natural
admission gain, inspection established that Position/Span needs a unit-wide
resolved expansion/impl proof absent from the current capture. It remains the S09
investigation target, but is no longer the next delivery target. Implementing a
large general expansion inventory before demonstrating a read-only family gain
would not satisfy the current priorities.

The active target is the safe `lz4_flex` 0.14.0 `get_batch` / `get_batch_arch`
family in src/block/compress.rs. Use the whole published library with default
features (including safe-encode), development profile, test=false, the pinned
workspace compiler and native target. Retain existing source and archive identities
from the frozen inventory. This changes no frozen selection or denominator.
The unsafe cfg alternatives must remain inactive in this context and unsupported
when selected without safe-encode. Implement get_batch end to end first, then its
native-width companion; the family is not complete until both work together.

Independent source expectations established before new admission evaluation:
get_batch returns the native-endian interpretation of exactly input[n..n+4];
get_batch_arch uses pointer-width bytes from input starting at n. Bytes before n
and after the window must not contribute. Every selected byte must contribute in
native-endian order. Wrong lengths, failed conversion/unwrap, n+width overflow and
out-of-bounds access must preserve the declared normal-completion requirements,
not become certified total/nonpanicking calls. Target endianness and pointer width
must be known. Wrong endian conversions must be distinguishable. Evaluate input,
range endpoints and calls in Rust order, preserving earlier snapshots.

Full known blocker chain, from source and the existing 3.30 capture:

- Subslice index dispatch is **overloaded**, not the builtin scalar index contract;
  its exact core range indexing authority and shared origin/offset/length are needed.
- Rust range syntax lowers to expr_struct with expansion callsite n..n+4; the
  current generic macro rejection is not evidence that a user macro exists.
  Prove original range syntax and both endpoints, rather than dropping expansion.
- TryInto resolves to a trait method with a concrete Result<[u8;4],TryFromSliceError>
  or Result<&[u8;N],TryFromSliceError> result. Exact selected implementation and
  conversion semantics are needed; the method name alone is insufficient.
- Result::unwrap needs connected discriminant/payload and failure behavior,
  not an invented pure/nonpanicking summary.
- u32/usize::from_ne_bytes currently lacks a direct callee record because the
  helper-callee capture excludes external associated functions. Reuse existing
  library Callee path/crate/hash/definition identity and typed Dispatch; this is
  observation, not admission authority.
- get_batch_arch also has a local const initialized by size_of::<usize>(), a
  named array length, a reference-valued conversion result and explicit array load.
  Each requires exact source, target and value/origin correspondence.
- Initial arithmetic n+width must compose with existing partial-flow treatment;
  cannot assume it never overflows simply because the intended window is small.

No nominal interface is involved in these two functions; S09 is not on their
critical path. Unknowns remaining: selected TryInto impl evidence availability,
precise pinned core source authority for bounds/conversion failure, and any
further source-correspondence failures after the range/call chain is connected.
Development tests must exercise the entire expressions through Cargo capture,
independent verification, strict admission and matching, plus corrupt evidence,
wrong windows/endian/length, side-effecting endpoint snapshots and helper results.
Do not infer family support from separately admitted components.


External associated-callee capture validation: 767 workspace tests pass (5 existing
ignores), clippy/fmt pass, rstyle checks 2415 functions with 0 violations and
patterns reports 0 candidates. Frozen rerun `target/hir-coverage/run-h5cz2c75/`
preserves all 60 identities/outcomes, denominators and context counts relative to
`run-1sbtdgtr/`: 11 admitted, 42 excluded, 7 inactive; 0 gained and 0 lost.
The 14 inventory tests pass. Both safe readers remain excluded, with no claim of
new end-to-end support. No new capture record type, snapshot or retained cache is
introduced. This resolves the missing external associated-callee identity on the
active reader chain; admission still requires the remaining connected semantics.

## Subslice capture and evaluation-order evidence

The driver now retains the overloaded Index callee in the existing PathRes/Callee
representation and marks resolved range-type identity only for rustc RangeExpr
desugaring. It preserves the expansion record. A user-written Range struct does
not receive this tag. Bounded, open and inclusive source ranges are captured but
remain excluded; scalar builtin index capture remains unchanged. The new Cargo
regression checks these distinctions without authorizing any range operation.

The independent executable [subslice-order.rs](subslice-order.rs), compiled and
run with the pinned compiler, establishes a critical difference from builtin
scalar indexing: a range access snapshots the shared slice base **before** its
start/end expressions. Rebinding a local or a field-bearing holder in the start
expression retains the old slice; computed bases also retain their snapshot.
Explicit effects show base, start, end order (123). Pinned MIR shows the base copy
before rebinding and before the Range construction/Index call. Do not apply
`data_index_reads::delayed_base` to overloaded subslicing.

Reproduce the independent oracle without changing crate sources:

```sh
rustc --edition=2024 eval/coverage-development/subslice-order.rs -o /tmp/rstyle-subslice-order
/tmp/rstyle-subslice-order
rustc --edition=2024 --emit=mir eval/coverage-development/subslice-order.rs -o /tmp/rstyle-subslice-order.mir
```

This does not establish admitted range graphs or checked-access semantics.
Next connect original range endpoint correspondence and pinned core authority,
then lower a view with the proven base/start/end ordering and normal-completion
bounds; reference snapshots and later element reads must compose through that
view. Preserve the distinction between view construction and reading bytes.


Subslice capture checkpoint validation: 768 workspace tests pass, with 5 existing
ignores; clippy/fmt pass. Repository rstyle checks 2416 functions with 0 violations;
patterns reports 0 candidates. The 14 inventory tests pass. Frozen rerun
`target/hir-coverage/run-tw6vnfnv/` preserves all 60 identities/outcomes,
denominators and context counts from `run-h5cz2c75/`: 11 admitted, 42 excluded,
7 inactive (0 gained, 0 lost). Capture validates 26 graph documents. The independent
runtime oracle and MIR emission pass. Range capture is evidence preparation,
not admitted subslice functionality; both natural reader functions remain excluded.


## Bounded subslice views implemented (Data-v3 3.31.0)

The [contract](../../docs/extraction-profiles/shared-subslice-views-v1.md)
connects exclusive bounded shared-slice views, exact endpoint source/callee
identity, base-before-endpoints snapshots, bounds requirements, saved/nested
views and subsequent scalar reads. Existing shared helper results can be bases;
view-containing helper bodies remain outside current summaries. The composed
Cargo family has 12 functions, 7 admitted and 5 deliberately excluded, with
independent verification, strict replay and matching. A separate two-function
Cargo regression compares overloaded-range and builtin-scalar rebinding origins
against the independent runtime/MIR oracle. Graph and raw-unit corruption cases
reject lost bounds/operand roles, invented reads and wrong Index/Range identities.

Frozen rerun `target/hir-coverage/run-yy464a72/` preserves all 60 identities and
outcomes, denominators and context counts relative to `run-tw6vnfnv/`: 11 admitted,
42 excluded, 7 inactive (0 gained, 0 lost), with 26 validated graph documents.
The actual safe get_batch rejection set now contains only call-summary/helper
resolution restrictions: its previous index/range expansion/additional-type
rejections are gone. get_batch_arch additionally retains the local size constant
and reference-result composition blockers. Neither reader is yet admitted.

The next milestone must connect concrete slice-to-array conversion,
Result/unwrap failure and native-byte semantics to obtain a natural get_batch
admission. Do not split these into disconnected fixture/capability milestones.
The architecture companion then also needs its local size_of constant/named
length and borrowed-array conversion/result composition. Preserve the original
source/build selections and independent byte/window/failure expectations above.

No capture record type, source snapshot or retained cache is added. View bounds
reuse typed graph annotations. Index and view verification share one existing
HIR operation inventory, avoiding a second whole-body scan; functions without
views skip view installation. No broader performance or detector-quality claim
follows from these structural checks.

Final view-slice gates: 770 workspace tests passed (5 existing ignores), workspace
clippy and formatting passed, repository-built rstyle checked 2441 functions with
0 violations, patterns had 0 candidates, and all 14 inventory tests passed.
The shared-inventory scan refactor was followed by a second complete workspace
run. No broad runtime performance claim is made.

## Independent byte-window oracle (preparation, no admission milestone)

`byte-window-oracle.rs` reproduces the two selected expressions and checks their
results against arithmetic byte-position expectations, without using a byte
conversion API in the expected-value calculation. Run from the repository root:

```sh
rustc --edition=2021 --test -C overflow-checks=yes \
  eval/coverage-development/byte-window-oracle.rs -o /tmp/rstyle-byte-window-oracle
/tmp/rstyle-byte-window-oracle
```

The four tests cover both widths, every selected byte at multiple offsets,
outside-window independence, distinguishable endian/window mistakes, copied and
borrowed conversion failures, borrowed pointer identity, invalid windows and
endpoint overflow. All four pass on the pinned native compiler. These are runtime
semantic expectations, not Cargo capture/verification/admission tests. The file
does not replace the whole published library or change the frozen inventory.

Pinned core source inspection confirms the blanket `TryInto` implementation in
`convert/mod.rs` delegates to `U::try_from(self)`. The array implementations in
`array/mod.rs` distinguish copied `[T; N]` from borrowed `&[T; N]` results;
`Result::unwrap` in `result.rs` returns the Ok payload or calls `unwrap_failed`
on Err. Consequently the existing nonpanicking metadata-call summary cannot be
reused unchanged. The connected increment still needs concrete selected-conversion
authority, a real typed Result intermediate, copy/read versus borrowed-origin
semantics, failure requirements and endian-sensitive native-byte interpretation.
Current method capture records the trait definition, not a selected implementation;
rustc `Instance::try_resolve` is available, but has not been added to capture or
treated as sufficient proof. No extraction behavior or admission count changed in
this preparation. The next delivery milestone remains the complete get_batch chain.

## First natural byte reader admitted (Data-v3 3.32.0)

The safe, unmodified `block::compress::get_batch` now admits in the declared whole
lz4_flex default-feature build context. The subslice, concrete copied Result,
unwrap failure requirement and native u32 byte interpretation compose as separate
typed operations. The [support contract](../../docs/extraction-profiles/copied-byte-windows-v1.md)
specifies bounds, read locations, evaluation order, source/library authority and
remaining exclusions. Native-only by-value array decoder helpers also compose;
the panicking reader itself is not certified as a nonpanicking helper.

The connected development test independently specifies 16 function classifications:
11 admitted and 5 excluded. Positives include the complete expression, saved
views, helper-selected slice bases, local Result/array values, endpoint rebinding
and native-only decoder helper composition. A wrong three-byte window is admitted
only as a partial computation with explicit conversion failure and unwrap
normal-completion requirements. Wrong-endian/other-width calls, borrowed-array
conversion, generic conversions and a caller requiring a reader helper summary
remain excluded. Endpoint rebinding retains formal root zero; helper-selected
inputs retain both possible roots. Semantic/effect/operand/build-summary and
selected-implementation corruptions must fail independent verification.

Frozen `run-57ulgkuc` versus `run-yy464a72` retains every identity and denominator:
12 admitted / 41 excluded / 7 inactive, with one gain (safe get_batch at bytes
1298..1416), zero losses and 28 validated graph documents. This is extraction
coverage evidence, separate from held-out detector quality or rewrite safety.

Next complete the architecture companion as one connected delivery: its local
size_of constant and named array length, borrowed-array conversion/Result origins,
explicit array load and usize native conversion remain blockers. The inspected
core conversion distinguishes borrowing from copying; no byte read may be
invented at borrowed conversion, and the later dereference must retain its exact
array/view dependency. The source/build selections and frozen inventory stay fixed.

Final gates: 776 workspace tests passed (5 existing ignores), clippy with warnings
denied and formatting/diff checks passed, repository rstyle checked 2468 functions
with 0 violations and patterns had 0 candidates. All 14 inventory tests passed.
Final rerun `run-n1ejrbwy` exactly reproduces `run-57ulgkuc` on selected identities,
outcomes, denominators and context counts. The retained natural get_batch bundle
was also independently loaded and document-verified, strictly admitted and matched
against the declared development expression: evaluated, Accepted, score 1, no
diagnostics. This checks end-to-end composition; it is not detector-quality or
rewrite-safety evidence. Byte and metadata verification reuse one source-call
inventory, and functions without byte operations skip byte installation.

## Architecture-reader target established before support

`arch-readers.rs` is a separate eight-function development corpus. It does not
edit the frozen selection or replace the natural library. The CLI regression
compiles the complete file as a Cargo library, pinned native aarch64, development
profile, test=false, no features, Data-v3. The natural target remains the original
lz4_flex 0.14.0 default-feature library context described above. Both contexts
must pass complete capture, independent verification, strict admission and
matching before the two-reader natural family is complete.

The following intended classifications were specified from source before the
first capture of this corpus. Current 3.32 capture excludes all eight and retains
every function; that baseline is tested separately from these intended results.

| Function | Intended bounded classification and semantic expectation |
| --- | --- |
| batch_arch | Admit the complete original size_of/borrowed Result/unwrap/load/native-usize chain; arithmetic byte significance equals the natural reader. |
| saved_arch | Admit; saving a view before conversion preserves its pointer, offset and width. |
| result_local_arch | Admit; saving the borrowed Result before unwrap preserves conditional Ok origins, not a copied payload or an unconditional valid reference. |
| snapshot_arch | Admit; the array read uses a (formal 0), while the separately returned current slice is b (formal 1). Base evaluation precedes endpoint rebinding. |
| wrong_length_arch | Admit only as a partial computation: width-minus-one conversion is Err, so unwrap has no normal continuation. Never invent a byte read on that conversion. |
| endian_arch | Exclude under the proposed native-only contract; asymmetric bytes distinguish big-endian from native decoding. |
| other_size_authority_arch | Exclude under the proposed usize-size-only initializer contract even though size_of::<u64>() has the same value on this target. Equal values do not prove allowed initializer authority. |
| caller_arch | Exclude until helper summaries can transport the reader's possible panic and normal-completion requirements. Do not reuse the existing nonpanicking helper summary. |

The runtime oracle now directly exercises this corpus, independently computing
expected scalar values with arithmetic. It also checks an offset borrowed-array
pointer, Err for zero/short/long views, the deliberately failing reader and the
rebound slice's separate pointer. Runtime success is not extraction evidence.

### Complete known blocker chain and bounded implementation plan

Retained natural `run-n1ejrbwy` reports reference-valued source operation,
original-source deferred blocks/calls, nested-item rejection, missing/mismatched
primitive summaries and a missing helper link. These diagnostics overlap; the
following dependency audit distinguishes the underlying causes rather than
treating the first rejection as the whole task.

1. The local constant is a separately owned item. Both original-source and HIR
   gates reject it. Support only a complete top-level function-local const usize
   initialized by resolved core size_of::<usize>(), with exact declaration/value,
   target pointer width and no attributes/expansions. Resolve its initializer in
   its own type-checking context. Do not traverse it as a runtime function child
   or merely accept the source spelling. Existing ScalarConstant evidence can
   carry referenced scalar values but does not prove initializer authority.
2. The explicit array type names that exact declaration. Verify the named length
   against original source and the selected constant, not only rustc's normalized
   eight-element array observation. Other nested items remain rejected.
3. Capture already observes Result<&[u8;8],TryFromSliceError> and the selected
   blanket TryInto implementation. Admission still needs exact borrowed signature
   and pinned conversion semantics: success iff length is eight, no content read
   at conversion, conditional Ok payload origins. Unwrap retains its panic and
   Ok normal-completion requirement.
4. Shared-reference origin propagation must compose through the borrowed Result,
   unwrap, locals/rebinding and saved views. Formal slice origins currently cannot
   be projected to the returned array referent: field_locations::source::origin_value
   returns borrowed TypeTree values and only handles shared/tuple/struct steps.
   Its two callers are source alias verification and derived referent locations.
   Both must independently validate the new access-relative array view. Reuse
   ByteWindow rather than inventing absolute aliases or independent roots.
5. Builtin array dereference is already captured and supported in isolation.
   Its indirect physical read needs the converted view's parameterized location
   and occurrence identity. Different windows sharing a formal root cannot be
   collapsed into one absolute read or treated as alias-independent.
6. Native usize decoding needs a separate exact eight-byte summary and source
   check. Existing native-u32 authority does not authorize it. The retained core
   definition fingerprint low word is 10425104633548385397; this observation alone
   is insufficient without complete signature, build and source verification.
7. Helper inventory/summary selection must recognize the supported primitive
   calls without accidentally certifying a panicking reader helper. Preserve the
   copied reader's existing admission and its negative tests.

Unknowns: additional rejection after the combined implementation; precise
Result-origin validation across local reassignment; location occurrence handling
for repeated indirect views; source-length verification interactions with
shadowing; cost of any changed origin representation. Test forged initializer
identity/value/length, missing reads or bounds, wrong operand/order, wrong origin,
missing source and forged native authority. No new unit-wide source-boundary
relaxation is justified for this family: the copied natural reader already admits
in the same complete library. S09's unresolved derive/impl ownership dependencies
remain as audited above, rather than being silently ignored.

Deliver the chain as one connected support increment, then rerun the unchanged
natural selection and measure new/lost admissions. The development baseline is
0 admitted / 8 excluded; this preparation changes no extraction behavior and
claims no new natural admissions. It does not add a capability or policy revision.

Validation for this target-preparation checkpoint: 780 workspace tests passed,
5 existing ignores, including the assertion-receiver regression; the seven-test
standalone runtime oracle also passed with overflow checks enabled. Workspace
clippy with warnings denied, formatting and diff checks passed. Repository-built
rstyle checked 2468 functions with 0 violations; patterns reported 0 candidates.
No frozen rerun was performed for these test/documentation-only changes; the last
measured natural result remains run-n1ejrbwy (12 admitted / 41 excluded / 7 inactive).

## Complete natural byte-reader family (Data-v3 3.33.0)

The selected safe get_batch/get_batch_arch family now composes through whole
unmodified lz4_flex library Cargo capture, document verification, strict admission
and matching. The [architecture contract](../../docs/extraction-profiles/native-borrowed-byte-readers-v1.md)
connects original local-size authority and named lengths, borrowed conversion,
conditional Result payload origins, unwrap failure, the explicit array content
read and native usize byte significance. It preserves the copied reader and its
existing negative tests. General Option/Result branching and panicking helper
transport are not implied.

All eight preselected development definitions are retained: the five intended
positives admit and all three intended negatives remain excluded. The original
0/8 baseline above is historical evidence, not a changed expectation or sample.
Additional Cargo composition tests exercise shared-helper-selected roots,
native decoder helpers and two different windows with separate read occurrences;
nested constants, literal initializers and arithmetic annotation lengths remain
excluded. Independent mutations reject initializer definition/argument/value,
source authority, conflicting scalar use/named length, invented/missing content
reads, lost conditional payload origins and changed failure/endian rules.

Frozen run-gldoldo0 compared with run-n1ejrbwy preserves all 60 identities and
all denominators. The sole transition is safe get_batch_arch, compress.rs bytes
1784..2020, SHA 15fb3015ac81edc5139d11679038a7e7f5d8292da7096b611681051a98889e2b,
excluded to admitted: 13 admitted / 40 excluded / 7 inactive, one gained and
zero lost. The 60 function contexts (53 active and 7 inactive) are preserved;
their outcome counts change by that same single transition. There are 30
validated graph documents: 14 lz4_flex, 0 regex-syntax and 16 time-core.

The retained natural architecture bundle was independently loaded and verified,
strictly admitted and matched against the predeclared batch_arch expression:
evaluated, Accepted, score 1, no diagnostics, under unchanged matching settings.
Saved-view spellings need not themselves score 1 against different spellings;
their semantic runtime expectations and strict composition tests are checked
separately. No scoring/threshold changes were made to force equivalence.

No new source snapshots, persisted caches or per-call provenance were introduced.
The local compile-time declaration reuses ScalarConstant and Callee evidence;
byte operations reuse existing type, origin, location and summary records.
Ordinary formal referents use borrowed type views; only the slice-to-array view
needs an owned type. Functions without a local size declaration skip the added
size-use/annotation pass. This is structural evidence, not a runtime benchmark.

The selected two-reader target has no remaining blockers in its declared context.
The broader roadmap remains unfinished. An adjacent count_same_bytes audit shows
mutable cursor writes, iterator/chunk/zip/closure semantics, open-ended subslices,
nested helper ownership and further scalar summaries together; admitting the
readers does not make that function supported. S09 still requires mixed-content
derive/impl/drop ownership proof; no unit-wide whitelist was relaxed here. The
next target selection should prioritize F10 and concrete checked-access/Option
compositions with tractable full blocker chains, rather than enter iterator or
mutable-memory tiers just because they neighbor these readers.

## Scalar Euclidean reduction target (specified before support)

The next bounded F10 target is the production `frac::gcd` in
`crates/rstyle-ccgraph/src/frac.rs`, bytes 5143..5272 at revision 1f867f1,
source SHA-256 cab1f7e45cca54d452ddb6c50ab82781229cf81bd320a413868fbb4f647c0648.
It normalizes gcd(0,0) to one for exact-score rational reduction. This is an
additional development selection, not a replacement or expansion of the frozen
60-function inventory. It was selected from its source before capture, because
its final conditional is outside the loop and its remaining scalar operation
chain is bounded. It is a repository function, not an external holdout.

Natural build context: the complete unmodified rstyle-ccgraph Cargo library,
default features, test=false, pinned compiler, aarch64-unknown-linux-gnu,
development overflow checks enabled. Capture command:

```sh
cargo rstyle clones -p rstyle-ccgraph --extraction-profile hir-data-v3 --max-pairs 0 --format json
```

The complete capture retains 413 functions, all excluded at this baseline.
For the selected gcd, every observed rejection reduces to two connected causes:

1. Tuple reassignment `(a,b)=(b,a%b)` becomes a rustc block containing a generated
   tuple-pattern let, temporary reads and two writes. Capture currently marks the
   tuple pattern unsupported and omits its generated bindings. Source/HIR
   correspondence then rejects unmatched source operations; downstream reads lack
   bindings. Accepting the generated prefix or sequentially evaluating the RHS
   after writing a would be wrong. Both RHS values must be saved before either
   write; division failure must precede all writes.
2. The final value-producing `if` follows the loop but is rejected by the
   function-wide F10 eligibility rule. The independent serialized cyclic-value
   verifier also rejects every merge, and existing acyclic value-site validation
   enumerates paths. Eligibility alone cannot solve this. The next implementation
   needs finite CFG proof for merges outside cycles, preserving strict operand
   availability and alternative-completion checks without enumerating loop paths.

No nominal owner, import or S09 unit whitelist blocks this primitive function;
neighboring derives and impls were present in its complete natural capture.
The routine has no calls, macros, unsafe, memory-backed targets or destructors.
Its u128 parameters/result, builtin remainder/comparison and scalar loop already
have contracts. Unknowns remain: exact independent correspondence for rustc's
assignment temporaries, outside-cycle value availability through divergent paths,
and final end-to-end graph/matching behavior. No supported classification follows
until those are resolved and the original whole-library capture succeeds.
`Frac::new` is explicitly not part of this bounded target: its closure, checked
conversions, Result/Option and `?` chain needs a separate connected contract.

The separate [development corpus](loop-merges.rs) was specified before its first
capture. Real Cargo baseline: **1 admitted / 6 excluded**, including the exact
natural gcd source (read from its production file, not a maintained copy).

| Function | Independent intended bounded classification |
| --- | --- |
| natural gcd | Admit complete loop, simultaneous scalar tuple assignment and final merge; return max(greatest common divisor,1). |
| sequential | Preserve admission; return the unnormalized greatest common divisor, including zero for (0,0). |
| after_loop | Admit final merge after a sequential loop; same normalized result as natural gcd. |
| before_loop | Admit a saved pre-loop conditional value used after the loop; same normalized result, with no stale branch outcome. |
| tuple_step | Admit distinct scalar-local tuple targets with RHS snapshots; (17,5) becomes (5,2), not (5,0). A zero divisor remains a partial computation. |
| cyclic_merge | Remain excluded in the initial outside-cycle contract; a controller revisited per iteration requires epoch-aware proof. This does not close all of F10. |
| memory_target | Remain excluded; destructuring assignment cannot introduce indirect writes or alias-independence assumptions. |

The [runtime oracle](loop-merge-oracle.rs) enumerates divisors independently for
all 1024 input pairs below 32 and supplies wide-u128 boundary expectations.
The actual production gcd source is compiled with the corpus and this oracle,
with overflow checks enabled. Checks include the zero pair, zero/nonzero inputs,
large common powers of two, consecutive maxima, simultaneous old-value reads and
zero-divisor failure. Runtime success specifies semantics, not extraction support.
The baseline CLI test retains every function and independently verifies the one
admitted sequential-loop pair. Next delivery must exercise the complete natural
chain, negative cases, serialized-evidence mutations, strict admission and matching.
No extractor behavior, authority records or persistent provenance are added by
this target-preparation checkpoint.

### Tuple-assignment foundation (Data-v3 3.34.0)

The [scalar tuple-assignment contract](../../docs/extraction-profiles/scalar-tuple-assignment-v1.md)
resolves the natural target's tuple-pattern temporary/binding/source chain. A
complete original-library recapture retains `frac::gcd` as excluded, now solely
for F10. Its source identity and context are unchanged. The original seven-function
development selection advances **1/6 to 2/5 admitted/excluded**, with tuple_step
gained and none lost. Natural admission gains are still zero for this target.
This foundation is useful only as part of the declared chain: next is finite
outside-cycle merge proof in both eligibility and independent serialized-value
verification. Merely widening the syntax gate remains insufficient.

A separate composition fixture admits exact duplicate steps, an update with RHS
assignment effects, a scalar tuple loop and a single bool target. It excludes
repeated targets, nested tuples, indirect writes and nonliteral tuple RHS values.
Independent bundle import exposed a missing RHS constructor during development;
final lowering retains that constructor once, then uses saved scalar operands
for ordered writes. The LHS place list performs no reads. Tests check source
operands and original parameter reaching definitions, reject swapped serialized
write operands and corrupted target/RHS evidence, and exercise strict matching.
The original runtime oracle and natural source expectations remain unchanged.

### Complete selected scalar reduction chain (Data-v3 3.35.0)

The [outside-loop merge contract](../../docs/extraction-profiles/outside-loop-value-merges-v1.md)
now connects the preceding tuple-assignment foundation to the natural gcd's final
conditional. The exact original-library `frac::gcd` source remains SHA
cab1f7e45cca54d452ddb6c50ab82781229cf81bd320a413868fbb4f647c0648 and advances
excluded to admitted. Its declared natural selection has one function/context:
**1 admitted / 0 excluded; 1 gained, 0 lost**. The broader 413-function library
capture supplies the complete context; it is not a new frozen denominator.

The unchanged seven-function development selection advances **2/5 to 5/2
admitted/excluded**: gcd, after_loop and before_loop gained; none lost. All intended
classifications are now met. The natural bundle was independently imported,
verified and strictly admitted, then matched against development gcd: Accepted,
score 1, no diagnostics, under unchanged matching rules. This is a repository
function/composition target, not an external holdout or detector-quality claim.

The verifier uses finite relevant-guard states and independent must-evaluation
intersections, preserving the existing no-merge path. Runtime divisor expectations
and a separate small-graph fixed-point oracle pass. Additional real Cargo cases
cover nested/short-circuit/match values, break exits, repeated loop-guard outcomes,
loop-containing alternatives and complete unreachable source after divergence.
Consistently corrupted paired guards/snapshots, frontiers and controllers fail
independent validation and strict admission.

There are no remaining blockers for this declared scalar leaf family. General F10
remains Partial: conditional values inside iterations require iteration-specific
proof. Concrete checked-access/Option families and S09 mixed-content nominal
ownership remain next audits; `Frac::new` is still excluded and was never included
in this target. No helper totality, alias independence or broad enum/drop support
follows from admitting gcd.

## Checked UTF-8 boundary target (specified before capture)

The next selected natural leaf is `regex-automata` 0.4.18
`src/util/utf8.rs::is_boundary`, bytes 3947..4747, source SHA-256
`e676b73906d493177219bcc72dbbb29fce630d0c07570e1ec1b7c3378b10c826`.
The build context is the complete unmodified Cargo library with default features,
test=false, pinned nightly, aarch64-unknown-linux-gnu, development overflow checks.
This separate one-function development selection does not change the frozen 60.
Selection is based on source, before inspecting admission: checked access,
concrete `Option<&u8>`, exhaustive None/Some matching, reference-pattern payload
binding, and scalar short-circuit evaluation form one complete bounded chain.

Independent intended classification: admit the entire function, including both
arms and the out-of-bounds path. On valid UTF-8, its result must equal
`str::is_char_boundary(i)` for every offset, including the end, past the end and
usize::MAX. Empty input at zero is true; empty input at one is false. Continuation
bytes inside valid multibyte sequences are false. No out-of-bounds input may
panic. Invalid UTF-8 has unspecified API behavior; observations on such bytes
must not be promoted to a stronger Unicode correctness claim.

Known missing contracts before capture: resolved slice `get(usize)` authority
and its conditional load/bounds semantics; concrete Option representation and
variant discrimination; branch-dominated extraction of a shared byte payload;
reference-pattern binding; composition with existing scalar match/short-circuit
value merges. Unlike unchecked indexing, failed checked access returns None.
Payload reads on None must remain impossible. No purity or alias independence
may be inferred from the method spelling. Negative expectations include custom
`get` methods, wrong core/compiler authority, mutable payloads, unmatched or
forged variant evidence, and payload use without the Some guard.

The leaf has no nominal interface, derives, macros, helper calls, drop-bearing
locals or mutation in its body. Whether the full library introduces a source
boundary blocker, whether current HIR retains the Option constructor/pattern
identities, and whether serialized verification can express conditional payload
availability are unknown until capture and verifier inspection. S09 remains an
open shared dependency for nominal families; scalar interface alone does not
justify ignoring neighboring items for nominal proof.

`decode` and `decode_last` are follow-on callers, not part of this bounded leaf
claim: nested Option/Result, char conversion, UTF-8 validation and iterator
semantics add separate blockers. The commented-out `next` routine in the same
file is inactive source, not a second natural selection.

### Checked-boundary baseline evidence

The real default-feature library capture on 2026-10-20 is **incomplete**: the
driver aborts during `inherent::capture_unit` with
`Ambiguous inherent source-definition association`. Cargo dependency resolution
first required fetching a missing dev dependency; retrying online reached the
actual compiler capture failure. No natural admission denominator or gain can
be reported from this failed run. Do not silently switch to a reduced feature
context or a source-only copy to claim success.

The unit association validator currently requires globally unique
`(source_file, function_span)` across all concrete inherent definitions. The
producer collects those records before helper linking and validates the entire
inventory. The next investigation must identify the colliding definitions and
whether they originate in macro expansion, source rebasing or duplicated owners;
merely removing the uniqueness check or dropping a record is not justified.
This is a newly confirmed source-boundary dependency ahead of Option semantics.

The separate [composition corpus](checked-boundaries.rs), combined with the
exact natural leaf extracted from its original source, captures all five
functions: **0 admitted / 5 excluded** under Data-v3 3.35.0. Intended positives
are the natural leaf, `checked_byte` (zero on failed access), `helper_boundary`
(same boundary predicate through a helper), and `saved_byte` (a successful saved
payload still reads the first input after rebinding; None returns the second
input length as u8). `mutable_byte` is the negative admission expectation: its
successful access writes 42 and requires a mutable-memory contract outside this
read-only target. Runtime validity does not imply extraction support.

The [independent runtime oracle](checked-boundary-oracle.rs), appended to that
combined source and compiled with `rustc --edition 2021 --test`, passes both
tests. Valid string cases include ASCII, NUL/newlines, two-, three- and four-byte
Unicode and mixed strings; every offset through len+2 and usize::MAX is compared
to `str::is_char_boundary`. Saved payload and failure cases cover all 256 byte
values, distinct input lengths, and maximum indices. The mutation negative is
also executed to establish that its effect must never be modeled as a read.

Captured get calls already carry a resolved core path/hash/definition identity
and library-resolution evidence, but their Option result has no normalized type.
The Some payload binding is absent from captured bindings. Thus additional
provenance records are not the first step: investigate existing resolved pattern
and type capture, then connect bounded type/operation/guard semantics. Existing
core authority and source correspondence must remain mandatory.

### Expanded inherent ownership boundary

The full natural capture identified both colliding sources precisely:
`util/primitives.rs` bytes 26841..26919 invokes
`index_type_impls!(PatternID, PatternIDError, PatternIDIter, WithPatternIDIter)`;
bytes 26921..26991 invokes the corresponding StateID macro. Each call produces
12 inherent methods at one source-callsite range. These are expanded signatures,
already excluded with `macro_generated_function`; their method bodies must not
claim unique original-source inherent ownership at that shared callsite.

The producer now withholds original inherent-definition associations for expanded
signatures. It retains all function facts and macro diagnostics, and keeps the
selected-unit association uniqueness validator unchanged. This is not macro
support, source omission or nominal certification. The regression retains two
expanded methods and an ordinary method beside an unrelated admitted scalar
leaf; only the ordinary method has a definition association in every function's
unit inventory. No new serialized field, evidence record, cache or source copy is
introduced; the added check is one signature-expansion query per otherwise eligible concrete
inherent definition. Profiles without inherent-call support and free functions
return through the existing eligibility check first.

The unmodified natural default-feature library now finishes Cargo compilation
and reports `capture_complete: true` with no capture failures. Assessment remains
**incomplete**: handoff rejects `Original owned source inventory unavailable:
expected fn`. The retained unit is therefore evidence for resolving the capture
abort, not an admission gain. The next investigation is independent receiver/
source inventory validation for generated functions. It must retain unknown
source correspondence and per-function macro exclusions, without accepting
forged receiver evidence or bypassing whole-unit transport validation.

Streaming inspection of the retained natural bundle confirms all 2,582 function
records remain, including 550 macro-generated functions. None of those expanded
functions claims an original inherent-definition association. The selected
`util::utf8::is_boundary` remains present with its two parameter bindings and
captured source correspondence. The stream inspection does not certify its
Option payload or override the handoff failure.

Frozen validation: `target/hir-coverage/run-p_h_oqof` against `run-h7tfyznr`
preserves all 60 exact source identities and outcomes, and every denominator:
53 active/captured/assessed, seven inactive. Counts remain 13 admitted / 40
excluded / 7 inactive (zero gains/losses), with 30 verified documents. Workspace
validation passes 788 tests with five existing ignores; all-targets clippy,
formatting, diff checks, rstyle and patterns are clean.

### Receiver observations do not require function-source certification

The next handoff failure came from the generated
`util::pool::inner::THREAD_ID::__rust_std_internal_init_fn`. Its retained source
is the `thread_local!` invocation, while initializer methods contributed from the
macro argument have unexpanded expression spans. Independent receiver validation
attempted to parse the whole invocation as an ItemFn and aborted the unit before
per-function exclusions could be reported.

Receiver verification now exposes observational `verify_capture` separately from
full `verify`. Both check exact method inventory, receiver and span identities,
concrete receiver/result consistency, registered scalar signature observations,
closed schemas, bounds and adjustment transitions. Handoff and library authority
inventory validation use the observational check. Semantic receiver resolution,
registered call source checking and paired document verification keep the full
original-function correspondence check. This does not authorize generated calls
or invent a source function around a macro invocation.

A real Cargo thread-local initializer regression reaches complete assessment,
retains the generated function as excluded, and admits an unrelated scalar leaf.
Independent strict bundle import retains no graph for the generated initializer.
Its raw receiver observations validate; the full receiver check still fails on
unavailable source correspondence. Changing a receiver identity is rejected by
the observational verifier itself. Existing graph/source mutation regressions
remain mandatory. The contract is documented in
[receiver verification](../../docs/extraction-profiles/receiver-verification-v1.md).

No serialized fields, provenance records, caches, thresholds or matching changes
were introduced. The same method/type/adjustment traversal remains; the raw
handoff no longer reparses function source solely to retain unsupported method
observations. This removes source-parsing work from inventory validation rather
than adding a new whole-unit pass.

Frozen validation `run-wxw_9e2e` against `run-p_h_oqof` preserves every one of the
60 source identities/outcomes and all denominators: 53 active/captured/assessed,
seven inactive; 13 admitted / 40 excluded / 7 inactive, zero gains/losses, 30
verified documents. The full workspace passes 789 tests with five existing
ignores; clippy, formatting/diff checks, rstyle (2,521 functions, zero violations)
and patterns (zero candidates) pass.

The full unmodified natural library now has **complete capture and assessment**:
all 2,582 functions are retained, with six admitted / 2,576 excluded; all 550
macro-generated functions remain excluded. `is_boundary` is function u0:f2580
and remains excluded for checked get, non-scalar Option matching and missing
payload binding. The previous whole-library baseline was incomplete, so its
six currently admitted functions are not reported as six measured gains. The
selected natural one-function denominator remains zero admitted / one excluded;
the unchanged five-function development selection still needs its connected
contract. Pair enumeration intentionally remains incomplete under max-pairs=0:
six imports require 15 pairs. Assessment completeness does not imply matching
completion.

The persisted natural report is 7.9 GB because the existing readable report
expands shared unit/source observations per function. JSON stdout then formats a
second full copy; source compilation and assessment had already finished during
that rendering. Future full-library exploration should use text stdout and
stream the retained report for selected fields. This finding is not a benchmark
or a performance guarantee, and does not justify new provenance tables/caches or
changing the selected build context.

Reassessment after the two source-boundary foundations: both confirmed context
blockers are now resolved without weakening source-to-pair requirements. The
next delivery must advance the selected get/Option/payload composition rather
than add more broad capture infrastructure. Source inspection confirms why the
payload is missing: the HIR pattern frontend retains unsupported tuple-struct /
reference patterns without their nested bindings, and concrete type observation
has no Option grammar. Both must connect to registered get bounds/conditional
load behavior, variant guards, saved payload origins and strict serialized proof.
No nominal S09 redesign or general macro support is required for this scalar
leaf; the separate Position/Span ownership investigation stays open.

Execution note for the natural run: after complete persisted-report publication
and a successful full streaming parse of its function inventory, redundant JSON
stdout rendering approached the 16 GiB cgroup memory limit. The owned renderer
was deliberately terminated (exit 143); its handle is terminal. This is not a
successful CLI-exit claim. Capture/assessment completeness and the reported
function outcomes above come from the complete validated `run-HCmpVF/report.json`
and retained bundle, not the empty stdout redirect. No capture or assessment was
restarted. The frozen runs and test/style gates all reached their normal terminal
results. This existing readable-report cost remains a reporting limitation;
future full natural runs should use text stdout.


### Bounded core Option observations retain payload bindings

Data-v3 now observes core Option with scalar or shared-scalar payloads and retains
resolved Some/None patterns, immutable reference patterns and their nested
bindings. Renamed imports retain compiler identities; lookalike enums, mutable
and nested Option payloads remain unsupported. Historical Data-v1/v2 records
remain unchanged. Owning nominal structs containing Option remain unsupported
as a whole; no fields are omitted. See the
[observation contract](../../docs/extraction-profiles/checked-option-observations-v1.md).

This is a foundation for the selected checked UTF-8 boundary, not semantic
Option support: semantic type parsing and admission remain closed. The unchanged
five-function development selection remains zero admitted / five excluded, now
retaining the natural leaf's `b` binding and Some/None relationship. Remaining
blockers are registered checked-get bounds and conditional reference origins,
variant guards and guarded payload loads, then saved Option/helper composition
through independent verification, strict admission and matching. D17 stays open.

Frozen run `run-uhc11x7c` matches `run-wxw_9e2e` on all 60 identities, outcomes
and denominators: 13 admitted / 40 excluded / 7 inactive, 30 verified documents,
zero gains/losses. Validation: 792 workspace tests pass with five existing
ignores, including the three reported assertion/subslice regressions; focused
Option tests and all-targets clippy pass. Formatting/diff checks, rstyle (2,529
functions, zero violations) and patterns (zero candidates) pass.


The unmodified default-feature natural library run `run-8UPUKk` completed with
text stdout: capture=true and assessment=true, 2,582 assessed, six admitted /
2,576 excluded, zero unknown/failed. The retained exact `is_boundary` source
identity and its payload binding were inspected in the bundle. Admissions are
unchanged from `run-HCmpVF`; the selected leaf remains excluded. Exit 2 reflects
intentional max-pairs=0 incompleteness (enumeration/evaluation=false), not a
capture failure; no end-to-end matching gain is claimed.


### Checked Option source and type contract

The selected boundary chain now has independently checked get/Some/None
compiler authority, exact shared-slice/usize/Option instantiation, original
operand spans, exhaustive unguarded variant shapes and scalar-copy binding
identity. Real Cargo captures and contract checks pass on aarch64 Linux,
x86_64 Linux and Apple Silicon macOS, including renamed imports, reversed arms
and saved operands. See the
[bounded contract](../../docs/extraction-profiles/checked-option-source-contract-v1.md).

Review of pinned core slice indexing establishes `i < len` as Some of the
reference at i, otherwise normal None; forming the Option does not load the
byte. The independent host oracle now additionally verifies receiver-before-index
evaluation and saved-reference identity during index-side rebinding. All three
oracle tests pass, alongside guard/reference-pattern/array/range negatives and
corrupted authority, operand, type and binding checks.

No complete function family becomes supported in this source-contract increment.
Semantic Option parsing, summaries and admission remain closed; the extraction
policy/capabilities and matching profile are unchanged. The concrete remaining
chain is explicit discriminators and conditional payload origins, guarded scalar
loads, saved Option flow, and helper composition through independent serialized
verification and strict matching. D17 stays Missing. Further observation or
source-context infrastructure is deferred: the next integration must exercise
that complete graph chain rather than count these local checks as admissions.

The contract reuses the existing target definition map and borrowed source nodes;
there are no new serialized records, provenance tables, caches or per-function
capture passes. Patterns identified duplicate original-expression parsing with
tuple assignments; both now use one parser, preserving expansion rejection.
The workspace passes 795 tests with five existing ignores; all-targets clippy,
formatting/diff checks, rstyle (2,545 functions, zero violations) and patterns
(zero candidates, unchanged existing ignores) pass.


Frozen run `run-efb6__3d` was compared with `run-uhc11x7c`: all 60 source
identities, outcomes, counts and denominators are identical. Results remain
13 admitted / 40 excluded / 7 inactive, 30 verified documents, zero gains/losses.
The unchanged five-function development selection retains identical source
hashes and outcomes, with complete capture/assessment and zero admitted / five
excluded. No new full-library natural run or matching gain is claimed here.

Direct inspection of the retained development facts through the new APIs also
verifies both local contracts for the exact natural `is_boundary` excerpt,
`checked_byte` and `saved_byte`, and rejects `mutable_byte`. The helper caller
has no local get/match contract to check; its eventual support still depends on
the connected callee summary. These local results do not alter their excluded
admission outcomes.

## Connected checked scalar Option integration (Data-v3 3.36.0)

The unchanged checked-boundary selection now yields four admitted functions:
`is_boundary`, `checked_byte`, `helper_boundary` and `saved_byte`.
`mutable_byte` remains excluded. Relative to the source-contract baseline this
is four gains and zero losses; all five names and source digests are unchanged.
Capture and assessment are complete, and matching evaluates all six admitted
pairs without diagnostics or incompleteness. This is a separate development
selection; it does not alter the frozen 60-function denominator.

The [connected contract](../../docs/extraction-profiles/checked-slice-option-v1.md)
adds exact get operands and a no-content-read summary, Some/None selectors,
guarded scalar payload reads, immutable binding declarations, saved Option
origins and scalar helper read composition. The independent Unicode/byte/pointer
oracle passes all three tests. Cargo composition tests strictly import bundles,
verify serialized documents and equivalent-function matching, and reject altered
interpretations, invented/missing reads, wrong operands, variants, bindings and
guards. Saved receiver and helper locations independently remain on parameter 0.

The frozen replay `run-goy11xai` matches `run-efb6__3d` on every selected identity,
outcome and denominator: 13 admitted / 40 excluded / 7 inactive, with 30 verified
documents. This bounded Option integration adds no frozen gains or losses.
General Option interfaces/construction, reference payload bindings, guards,
mutable/array/range access, Result and `?` still require connected support.

Full natural capture `run-O3vn08` retained 2,582 functions and exposed the
no_std Some/None display alias as the last blocker. After the exact alias fix,
all raw records were reassessed, their complete proof inventory serialized and
strictly imported. Current replay gives 7 admitted / 2,575 excluded / 0 unknown:
only the exact selected `util::utf8::is_boundary` gains admission relative to
`run-8UPUKk`. Every retained name/digest/span and all other outcomes are unchanged.
The old capture report remains historical (6/2,576); it is not relabeled.
The independently imported natural leaf and exact development copy match with
score 1, Accepted, complete evaluation and no diagnostics. The natural library's
all-pairs run remains intentionally unperformed. Final frozen replay
`run-579vcwmp` also preserves all 60 selected identities/outcomes/denominators and
30 verified documents. The nominal `Input::haystack`/`is_char_boundary` caller
chain remains excluded for owner/type and method/reference-summary dependencies.

Final gates: 796 workspace tests pass with five existing ignores; all-targets
clippy, formatting/diff checks, rstyle (2,579 functions, zero violations) and
patterns (zero candidates, 15 unchanged ignores) pass.


## Position/Span Copy observation milestone (2026-10-20)

Full unmodified-package replay `run-o52m9whk` confirms the optional declaration
observation reaches all four initial target functions. Position's builtin Copy
derive token is at src/ast/mod.rs bytes 13835..13839; Span's is at 12999..13003.
Both resolve the core trait and builtin derive with distinct definition hashes.
The original function selections and build context above are unchanged.

All four remain excluded (zero new/lost admissions); frozen outcomes remain
13/40/7 across the same 60 definitions. Capture now supplies the previously
missing positive observation needed to investigate Copy's no-Drop law. It does
not independently authorize that law or unrelated derives. The next connected
work must pin/check trait and macro identities, prove active attribute ownership
from the existing file snapshot, and resolve selected module/name/declaration
closure in this mixed crate before exercising constructor/getter composition.
A lifetime-only impl relaxation cannot solve this chain by itself. Manual Copy,
custom derives, unresolved expansions and missing observations remain outside
the observation contract; none is treated as a negative or purity proof.


## Checked Copy evidence and next natural-family boundary

Selected-unit binding now verifies the pinned core trait and builtin derive
identities separately, and checks active derive-token ownership using the
existing original-file parse. Strict import of all 956 functions in the retained
natural regex-syntax bundle succeeds, including the exact four selected source
digests. They remain excluded: zero gains/losses. Missing observations cannot
support drop claims; inactive/custom derives and hidden neighboring effects have
not become supported.

After these two zero-gain S09 prerequisites, the next milestone is the connected
four-function family. Its environment currently fails the global recursive item
whitelist before checking relevant declarations. Selected module cfg/path and
name dependencies must replace that coarse restriction together, including
imports, neighboring generic structs, derives and potential hidden Drop behavior.
The existing source snapshot and parsed module environment should suffice; do
not add another source inventory/cache. Preserve exact attribute-excluding
declaration ranges across the origin and resolution indexes. A syntactic local
Copy check alone is not a whole-module or whole-family certificate.


## Natural Copy boundary results and remaining bridge (2026-10-20)

Data-v3 3.37.0 admits the exact four initial Position/Span functions in fresh
unmodified-package Cargo replay `run-9t3zkk5i`, and the natural Span::splat caller.
All four initial source identities pass independent document verification, strict
admission and selected matching against the development counterparts (score 1).
All 28 pairs across those eight functions evaluate completely. Frozen identities
and denominators remain unchanged: 18/35/7, five gains and zero losses relative
to 13/40/7. The complete 956-function natural unit retains every identity and
source span, with nine gains and zero losses; extra gains are not a newly selected
family or detector-quality evaluation.

`position-span.rs` is a separate ten-function development corpus, evaluated inside
ordinary external modules beside generic/enum/Drop/macro contents. Data-v3 3.38.0
admits all ten cases, including the complete constructor → owned local → shared
getter chain. This advances the unchanged selection from nine admissions to ten,
with no losses and all 45 comparisons complete. See the
[owned Copy receiver contract](../../docs/extraction-profiles/owned-copy-receiver-v1.md).
Explicit local-storage borrowing, non-Copy owned receivers and by-value self
updates remain outside this contract.

`position-span-oracle.rs` specifies independent runtime expectations against the
published regex-syntax crate: distinct scalar components and full nested values,
splat preservation, all 64 binary combinations of getter inputs, and the complete
constructor-to-owned-getter result against independent offset equality. The three
tests also exercise the selected development composition, including `usize::MAX`,
in a temporary Cargo harness depending on `regex-syntax = "=0.8.11"` with default
features. Runtime behavior does not replace capture/verification/admission checks.

The originally requested matching distinction for altered field paths is not
achieved: the offset and column getters retain distinct extraction paths but the
current detector gives score 1 to both shapes. Record this as a separate detector
limitation; it is not equivalence evidence. Thresholds/scoring remain unchanged.
Negative Cargo cases still reject missing Copy evidence, relevant unknown macros,
primitive aliases, relevant globs, and a custom extra derive even alongside a
valid builtin Copy observation. Unknown sibling effects are never certified.


## Copy value-self updates

`span-updates.rs` adds a separate source-selected ten-function selection to the
same mixed module as `position-span.rs`. Seven positive cases cover both endpoint
updates, chaining, helper-result receivers, shared getters, retained Copy values
and UFCS; three negative cases retain mutable/typed self and shared-to-owned
exclusions. Data-v3 3.39 admits seven versus zero under 3.38 on identical source.
The original ten remain admitted: all 136 pairs among seventeen positives complete.
The fourth independent runtime oracle checks exact replacement and preservation
against published regex-syntax 0.8.11. See the
[value-self contract](../../docs/extraction-profiles/owned-copy-self-v1.md).
Direct owned-formal field getter calls still encounter the shared-only helper-read
boundary; this result does not certify arbitrary by-value methods.


## Borrowed scalar owner family

`borrowed-scalar-owners.rs` is a separate ten-function selection for the natural
regex-syntax `utf8::ScalarRange` getter family. Baseline f2d0152 admits zero;
Data-v3 3.40 admits four (the two getters and two borrowed wrappers), with six
unchanged exclusions and no losses. All six pairs complete. Non-Copy owned and
destructor-bearing cases do not acquire Copy or drop-free permission.

`borrowed-scalar-oracle.rs` contains independent boundary expectations. For runtime
checking, append it to an otherwise unchanged published regex-syntax 0.8.11
`src/utf8.rs` in a temporary directory; use a rustc test harness with
`extern crate alloc`, that `utf8` module, and this development source as module
`development`. It exercises the actual private natural methods without changing
the separate Cargo capture sources. The five tests (one oracle and four natural
module tests) pass. See the [bounded contract](../../docs/extraction-profiles/borrowed-scalar-owner-v1.md).


## Cyclic prefix scan family

`cyclic-prefix-scans.rs` fixes a separate six-function development selection:
the two natural prefix-scan shapes, alternating loop-local values and a saved
short-circuit result are the four intended positives; mutation and value-carrying
break are retained negatives. The declared Data-v3 3.40 baseline captured all six with zero admissions.
Data-v3 3.41 admits all four intended positives, retains both exclusions, and
completes all six comparisons. The two source-selected natural scanners also
pass capture, independent verification, strict admission and matching against
their development counterparts (score 1).
The ordinary arithmetic composition retains existing overflow/partial-flow rules;
it does not require an unrelated wrapping-method summary.

`cyclic-prefix-oracle.rs` appends to that source in a temporary rustc test harness.
Its three tests pass: capped zero-prefix lengths, all 256 translation bytes and
explicit table changes, empty/out-of-range starts, and 1025 iteration-value/final
condition cases. The oracle specifies behavior independently of the extraction
checker and does not establish pipeline support. See the
[selected natural family and blocker audit](../../docs/extraction-profiles/cyclic-prefix-scans-v1.md).


## Scalar Option byte-width family

`option-byte-widths.rs` fixes a separate six-function selection before semantic
activation. Data-v3 3.41 admits none. Data-v3 3.42 admits match_width, branch_width,
width_or and saved_width, retaining nested/custom exclusions and all source
identities; all six positive comparisons complete. The original top comment in
the corpus is retained as part of that frozen development source.

Append `option-byte-width-oracle.rs` to the unchanged corpus in a temporary rustc
`--test -C overflow-checks=on` harness. Its two tests independently specify all
256 byte widths, fallback values 0/5/usize::MAX, saved rebinding and the deliberate
non-validating C0/C1/F5/F6/F7 cases. Pipeline coverage is separate from runtime
behavior. The source-selected natural classifiers, build contexts and remaining
decoder blockers are recorded in the
[scalar Option contract](../../docs/extraction-profiles/scalar-option-byte-widths-v1.md).


## Checksum-state roundtrip family

`checksum-state.rs` declares five intended positives before admission assessment:
the state constructor, borrowed checksum getter, constructor-result roundtrip,
saved input composition and scalar widening. Custom trait implementation/caller
and floating conversion are retained negatives. See the
[source-selected adler2 contract](../../docs/extraction-profiles/checksum-state-roundtrips-v1.md).

Append `checksum-state-oracle.rs` in a temporary Cargo test harness depending on
`adler2 = "=2.0.1"` with default features. It specifies whole-input identity for
262144 u32 words against the original methods and development compositions,
including distinct high/low halves, and checks all 65536 u16 widening inputs.
This is a runtime expectation; it does not establish pipeline support.

Data-v3 3.43 admits all five declared positives (baseline 3.42 admitted only the
constructor), preserving all 11 captured identities and six exclusions including
derived methods. All ten comparisons complete. The original corpus comment is
retained as part of the source identity; measured support is recorded here and
in the contract. The oracle is also run with adler2 default features disabled.


## Native unsigned prefix readers (Data-v3 3.44.0)

`native-prefix-readers.rs` fixes 11 functions before admission assessment. The
eight intended positives connect RangeTo prefixes, concrete size_of calls,
2/4/16-byte copied Results/unwrap and u16/u32/u128 native decodes, with selected,
saved and endpoint-rebound bases. Three negatives retain suffix, signed and
panicking-helper boundaries. Baseline Data-v3 3.43 admits only choose in both
std-root and ordinary external-module no_std development contexts (1/11 each).
Do not edit the corpus to improve these classifications.

`native-prefix-readers-oracle.py /path/to/regex-automata-0.4.18/src/util/wire.rs`
checks the three original function spans/hashes, builds a temporary pinned-rustc
harness with `native-prefix-readers-oracle.rs`, and runs four independent numeric,
short-input and saved-base tests. No original Cargo neighbors are omitted from
pipeline coverage by this runtime-only harness. See the
[full source-selected contract and blocker audit](../../docs/extraction-profiles/native-prefix-readers-v1.md).


The unchanged std/no_std contexts each now admit **8/11**, gaining seven and
losing none, with all 28 comparisons complete. The original complete
regex-automata unit preserves all 2582 identities and moves **9 →14 admissions**:
the three declared readers plus two scalar-size wrappers. All 14 original-unit
admissions independently verify. The unchanged frozen selection remains
**20 admitted /33 excluded /7 inactive**; the full regex-syntax roster additionally
gains the strictly verified `LookSet::read_repr`. No original unit was reduced to
make these functions pass. The contract records the complete blocker chain,
independent expectations, negative boundaries, matching limitations, measured
performance costs and remaining caller/Result work. Neither the oracle nor these
similarity results authorize rewrites or detector-quality claims.

## Checked native reader expectations (not activated)

Run `python3 eval/coverage-development/checked-native-readers-oracle.py
/path/to/regex-automata-0.4.18` with the repository toolchain on PATH. The runner
checks the five selected reader/helper source identities, copies the complete
original crate into a temporary directory, and appends only a test module.
`cargo test --locked --lib` retains original default features, dependencies,
error enums and helper bodies. Dependencies may need downloading on the first
run. No original source or manifest is changed in place.

Three independent tests check all 65536 two-byte words, every wide bit position,
all shorter lengths, exact and longer buffers, and preservation of the original
BufferTooSmall payload including its static description pointer. Expected values
use explicit byte accumulation rather than another native conversion call.
The tests pass on the current ARM Linux host; they establish runtime expectations,
not extraction support, native cross-platform execution or detector quality.
The [dependency audit](../../docs/extraction-profiles/checked-native-readers-audit.md)
records the still-excluded complete original function family and remaining gates.

## Scoped imports and scalar flag owners (development contract)

`scoped-import-flag-owners.rs` fixes five intended positives: the Copy scalar
constructor/getter, constructor helper, and two by-value helper compositions.
The ordinary sidecar `macros.rs` contains only
`macro_rules! harmless { () => { 42 }; }`; `exports.rs` contains only
`pub struct Neighbor { pub value: u8 }`. These sidecars are installed verbatim
by the CLI integration tests. The generated Clone method is a retained negative,
so capture contains six functions. Baseline 745bb18 admits none.

`scoped-import-flag-owners-oracle.py /path/to/regex-automata-0.4.18` verifies the
two original method identities and appends only an independent test module to
a disposable copy of the complete original crate. It runs locked default-feature
Cargo tests on the original Epsilons declaration and methods. Zero is the only
empty value; all 64 isolated bits, all u16 low words and all shifted high words
are covered, and original constructor/getter composition must return true.
This runtime check does not establish extraction, rewrite safety or detector
quality. The [scope contract](../../docs/extraction-profiles/scoped-import-flag-owners-v1.md)
records the exact natural selection, import restrictions and remaining boundaries.

Data-v3 3.45 activates all five intended positives (0/6 to 5/6, no losses),
with complete strict verification and all ten development comparisons. The
unchanged complete natural library gains both selected Epsilons methods and
43 incidental admissions (14/2582 to 59/2582, no losses). Final independent
verification checks all 59 natural and five development admissions; the
selected seven graphs complete 21 comparisons, two Accepted and 19 Rejected
under unchanged scoring. See the contract for bounded performance evidence
and remaining restrictions. Whole-library matching is not claimed.

## Nested-array byte membership

`byte-membership-oracle.py /path/to/regex-automata-0.4.18` verifies both selected
ByteSet source identities and tests their original complete-crate implementation
with default features and locked dependencies. Expectations include all empty-set
queries, every singleton/query pair, dense patterns and all-bits-set queries.
The [contract](../../docs/extraction-profiles/byte-membership-v1.md) records the
source-selected chain and bounded proof. The immutable `byte-membership.rs` corpus
fixes five complete positives, five partial leaves and five exclusions. Data-v3
3.46 connects all five intended complete functions through Cargo capture, strict
verification/admission and matching; overall admissions move 4/15 to 10/15, with
no losses. The sixth gain is explicitly partial, and partial helper callers
remain excluded. The unchanged complete original library gains seven admissions (59/2582 to
66/2582), zero losses, including both selected methods. All 66 natural and five
development admissions pass independent strict import/verification. The selected
seven graphs complete 21 comparisons, two Accepted and 19 Rejected under unchanged
scoring. Whole-library matching remains budget-incomplete.

## Tagged byte units (not activated)

`tagged-units.rs` fixes six intended constructor/projection/composition positives,
three named negatives and two excluded generated Clone methods. Its Data-v3
3.46 Cargo baseline is 0 admitted /11 excluded, with complete capture and
assessment. The [contract](../../docs/extraction-profiles/tagged-units-v1.md)
selects original Unit::u8, as_u8 and as_eoi, enumerates the complete known blocker
chain and separates unsupported neighbors. `tagged-units-oracle.py
/path/to/regex-automata-0.4.18` checks original source identities and runs three
tests in a disposable complete original crate: all bytes, all u16 sentinel
payloads and the public sentinel constructor's checked domain. These runtime
expectations do not activate enum extraction or modify the frozen inventory.
