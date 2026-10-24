# Pinned primitive constants

Data3.77 (`rust.pinned-primitive-constants`) connects original ryu1.0.23
`f2s_intrinsics::mul_shift_32`; the existing `mul_shift_64` remains an admitted
anchor. [Selection](../../eval/coverage-development/primitive-constant-selection.json)
fixes source, build contexts, independent expectations and unsafe negative controls.

Only original unexpanded `u32::MAX` expressions qualify. A separate wire record
binds the primitive owner, member, evaluated value and exact core definition/hash
to reviewed compiler/target registries. The source-hashed unsigned integer macro
implements `pub const MAX: Self = !0`; the verifier independently derives the
canonical u32 maximum. Observed/normalized types, known target width and sysroot
roots must agree. Checked lexical ownership rejects type/import/module shadows;
complete existing source, macro and neighboring-item checks remain mandatory.
No runtime read, panic or helper summary is invented for the constant itself.

Local constants retain their historical declaration proof. Other primitives,
MIN/BITS, aliases and qualified spellings remain outside this bounded authority.
Historical document policies cannot acquire primitive-constant permission.
Missing or malformed observations, wrong target/compiler/core definitions,
contradictory values/types and resealed source-shadow attacks reject.

Original arithmetic and assertions retain their order and hazards: shifts<=32
panic at the first assertion; shifts>=96 panic at the checked u64 shift. For
33<=shift<=95 the mathematical quotient floor(m*factor/2^shift) returns if it fits
u32, otherwise the last assertion panics. No valid-input precondition is assumed.
The independent arbitrary-precision oracle uses direct multiplication/shift,
not the implementation's split products. Native execution is an explicitly
labeled source-hashed private-source witness; natural acceptance instead uses
the unchanged complete Cargo package. Unsafe table wrappers and complete float
formatting remain separate blockers. Final measurements/gates live in the root
roadmap; cross capture/replay does not claim native x86/macOS execution.
