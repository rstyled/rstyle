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
in the original inventory; that inventory is not being edited. They currently
remain excluded. The intended bounded contract is nongeneric, immutable scalar
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
