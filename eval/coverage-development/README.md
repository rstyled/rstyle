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
