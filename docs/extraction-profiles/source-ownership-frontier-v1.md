# Source ownership frontier: audit v1

Status: **blocked; no production activation**. Audit 2026-10-21. P05.D read
original registry sources and current proof code; no builds or captures were run.

## Selection and scope decision

No residual `#[path]` positive occurs in the four existing natural selections
(regex-automata 0.4.18, regex-syntax 0.8.11, lz4_flex, time-core), nor in the
located libm 0.2.16 source. Existing cfg-selected ordinary modules already have
independent closure proof. Registry path examples inspected include ryu-js 1.0.3
integration tests, rustls 0.23.43 crypto modules, syn 3.0.6 parser modules, tokio
1.53.1 process modules and rand 0.10.3 distribution modules. These do not establish
a tractable complete nominal family blocked solely by path ownership: the ryu
examples import scalar helpers, while the others add substantial unrelated
interfaces/dispatch/effects. Do not implement path permission merely to fill D.
This is a bounded source audit, not proof that no useful path target exists.

Reprioritize D to C's demonstrated selected owner prerequisite. Freeze the
unchanged regex-automata `Input::haystack → Input::is_char_boundary →
util::utf8::is_boundary` chain and exact source/build identities already retained
in [the borrowed-result contract](borrowed-result-chain-v1.md). The getter byte
span is 24270..24334, SHA-256
`9c29bc68d97eff53c40572a6f600a1f421c83f314403784b48c8e724e3889bb8`;
wrapper 29204..29316, SHA-256
`0f83c1f1788b7fe1b5a04b8c67da890aa5239dbbf1cf3a5a9326887348f74e9d`.
Owning unit is regex_automata lib/test=false, default features, edition 2021,
nightly-2026-09-30, aarch64-unknown-linux-gnu, debug/overflow checks/panic unwind.
No frozen membership changes and no new admission measurement in this audit.

## Verified dependency graph and correction

1. Callable selection rejects `impl<'h> Input<'h>` explicitly in
   `nominal_boundary/resolution.rs::supported_inherent_member`: any enclosing
   impl parameter is rejected. Existing `concrete_types::lifetimes` already
   verifies bounded lifetime-only declarations/arguments, and driver
   `concrete_declaration` verifies resolved lifetime arity. Thus **nominal lifetime
   capture itself is not wholly missing**; extend exact callable owner proof,
   without type/const generics, bounds or a where clause.
2. `Input` has all four fields: shared haystack, Span, Anchored and earliest.
   `resolution/selected/borrowed.rs::field_type` accepts scalar, scalar-array and
   proved Copy-struct children, but neither Shared nor Enum. This is a complete
   owner-shape blocker; ignoring unused fields is forbidden.
3. Anchored is the original mixed enum No/Yes/Pattern(PatternID), and PatternID
   contains SmallIndex(u32). Existing scalar-payload and new fieldless enums do
   not prove this child. A complete bounded mixed Copy enum declaration proof
   (or another independently justified complete borrowed-owner contract) is a
   named P08/P05 prerequisite. General macro permission is not established:
   index_type_impls generates nearby methods, not these three declarations.
4. C then owns stored-reference-slot read versus referent origin, saved receiver
   substitution and conditional consumer read composition. Ordinary source
   census/path proof is not the demonstrated limiting seam for this chain.
5. Unknowns needing targeted capture before activation: exact current owner
   observation failure for Anchored; resolved lifetime receiver adjustments;
   precise stored-reference path/origin transport. Existing historical failures
   alone do not prove the post-3.60 complete blocker set.

## Bounded expectations and write assignment

Permit only source-proved lifetime-only callable owners with all fields checked,
shared receiver and no owned Input construction/consumption/drop. Preserve exact
module/import/derive/build ownership and alias uncertainty. Positive callable
controls should use already-supported borrowed scalar/Copy-child owner shapes;
reject changed owner lifetime arity, type/const generics, lifetime bounds/where
clauses, wrong impl Self identity, omitted/changed fields, stale source buffers,
shadowed imports and historical policy downgrade. These controls establish a
foundation, not natural delivery until C's complete chain admits strictly.

C's independent runtime expectations remain authoritative: exact haystack
pointer/length; unrelated field changes do not change it; codepoint boundaries
and end offsets agree with valid UTF-8 strings; past-end offsets return false
without panic; separate arbitrary-byte expectations use the classifier formula.
Save receiver identities before rebinding; retain getter slot reads separately
from conditional byte reads. All selected matching and strict retained-bundle
verification remain pending.

Proposed D exclusive leaf: a new lifetime-only callable proof helper under
`nominal_boundary/resolution/selected/` plus dedicated ownership tests after
approval. Lead owns edits to `resolution.rs`, type vocabulary, driver capture,
policy and registration. Do not edit `census.rs` absent a demonstrated path
blocker. C retains helper_references/helper_views. D cannot activate the natural
family independently of the mixed enum/borrowed-field prerequisite; recommend
lead decide that shared contract before authorizing production. Readiness:
**audit complete, implementation blocked on the declared shared owner contract**.
