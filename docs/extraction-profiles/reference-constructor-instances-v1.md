# Checked reference constructor instances

Data-v3 3.85 activates the unchanged regex-automata 0.4.18
`util::search::Input::new::<[u8]>`; 3.86 adds `Input::new::<str>` with explicit
string-byte-view evidence. Both use the native pinned compiler/target,
default and no-default-features contexts selected in
[the development corpus](../../eval/coverage-development/input-instances/selection.json).
Original generic definitions and frozen inventory membership remain unchanged.
This is a concrete constructor increment; the complete new/From/foreign-caller
family is still incomplete.

The private owning token requires the complete original source, selected-unit
ownership, original generic binders and exact concrete AsRef implementation,
one saved conversion, full constructor/length flow, lexical nominal paths and
independently proved no-drop result closure. Only the formal interface, binding,
pattern and exact formal reads receive concrete type normalization. Original
source, deferred finding, generic callee/dispatch and other types remain intact.

The exact byte AsRef CALL evaluates its sole receiver once, has no content-read,
write, panic or divergence effects, and returns that shared formal's referent.
Its summary and source/flow/serialized-graph origins agree independently. The
existing pinned slice-length contract operates on the saved result. Input/Span
slots preserve original field order and evaluation dependencies. `Anchored::No`
is a symbolic unit variant of the complete mixed enum, including Yes and the
Pattern(PatternID(SmallIndex(u32))) payload; it has no invented numeric literal.
Only the token's exact product types and constructor sites gain permission.

The incomplete source scan remains visible. Activation covers exactly its single
proved deferred AsRef diagnostic; missing, additional or edited findings reject.
Receiver inventory proof uses the complete checked constructor roster rather
than treating the original generic receiver as an ordinary concrete method.
Private immutable source-owner proof is established at minting, not repeated
for each analysis query. Export/replay recheck the boundary independently.

The separate reference-constructor sidecar carries proof inputs, original owner
namespace, definition ordinal and concrete argument. Strict replay remints the
instance and independently validates its complete graph. Ordinary bundles,
scalar instances and dependency closures cannot acquire reference authority.
No new checksum format, SHA operation or serialized lookup key is introduced.

The string operation preserves the same shared lifetime and storage through
`SharedReferent → Utf8Bytes`, with exact `str → [u8]` typing. It returns all UTF-8
bytes without allocation, mutation or a boundary check. Its distinct summary and
new capability leave 3.85 byte-only. Source, reaching-value and graph origins
check the input root before projecting; component summaries independently require
the exact projected output type/path. Missing/duplicate projections, slice inputs,
changed element types and wrong formals reject. Unicode lengths count bytes.

Expected positive behavior is unchanged original field values and saved input
origins. Existing independent original-source oracles cover byte/text values,
stateful/static-origin and panicking AsRef controls. Natural activation probes
require strict producer admission, complete identity matching under unchanged
thresholds and rejection of tag, literal, origin, summary and policy corruption.
Raw concrete facts alone must remain unavailable.

From activation, foreign reference-instance links and foreign component summaries
remain separate connected blockers.
General generic/trait selection, arbitrary mixed-enum operations, mutable memory,
owned allocation and destructor/unwind support are outside this contract.
