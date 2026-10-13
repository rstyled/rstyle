# Pinned read-only length registry, version 1

Implementation baseline: `affb3a1`. This prospective supplement completes the
provenance requirements of `read-only-lengths-v1.md` and
`receiver-verification-v1.md`; it authorizes no call before implementation/tests.
Existing profiles, primitive summaries, matching and thresholds remain unchanged.

Initially register the official `nightly-2026-09-30` core build for
`aarch64-unknown-linux-gnu`, rustc commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`, pointer width 64. Other compiler/target/
core builds remain unavailable until separately specified and tested. Actual
selected-unit build facts must agree; an observed display path is not authority.

The separately specified and tested x86_64 target authority is registered in
[read-only-lengths-x86_64-v1.md](read-only-lengths-x86_64-v1.md). Its distinct
core/artifact/definition identities preserve this original aarch64 registration.

Core crate hash: `bdb4eda0487b663ec13960d84fc76cc8`.
Compiler-resolved definition hashes (each has common first Fingerprint component
`5614226571623188065`):

| Registered summary | Exact core path | Second Fingerprint component |
| --- | --- | --- |
| `rstyle.data.slice.len/1` | `core::slice::<impl [T]>::len` | `14534832972469071209` |
| `rstyle.data.slice.is-empty/1` | `core::slice::<impl [T]>::is_empty` | `10769634512631306716` |
| `rstyle.data.str.len/1` | `core::str::<impl str>::len` | `16819204418595550297` |
| `rstyle.data.str.is-empty/1` | `core::str::<impl str>::is_empty` | `7884780508868330188` |

For required core callees, capture actual library source paths from the compiler's
used-crate-source facts, their formats, sizes and SHA-256 digests before resolving
handoff. Hash those exact files, without scanning old directories or searching by
basename. Capture once per selected compiler invocation and retain immutable
facts; this is invocation evidence, not an evidence cache. Limit each file to
128 MiB and four supplied library formats; read/I/O/budget failures are incomplete
capture. Missing/corrupt required provenance is incomplete handoff, never ordinary
rejection. A genuinely different, consistently captured build remains unsupported.

Registered SHA-256 digests, excluding the `sha256:` prefix:

| Artifact format | Digest |
| --- | --- |
| rmeta | `13d50a081c1aed124692349ee3832a0b2b26ab91e2637bf695a348cf9bc7280e` |
| rlib | `084465a1c561cd981b32d322e6cdd701880d99564cdf26c745b06781da48a287` |

Use a code-owned versioned registry resource, not runtime documentation paths.
The registry's semantic basis is the reviewed pinned rust-src definitions, not
fingerprint discovery alone. Source module SHA-256 digests are
`062320a86dc6a3d20718bc8fca824bd1f3df0bb8bb0c3d72e101be989cf576bf`
for `library/core/src/slice/mod.rs` and
`bb2e4e174e8db5c79f6c602565c1f97f049f10ca600540bfb1d3c7f382de9bc5`
for `library/core/src/str/mod.rs`. Slice len uses pointer metadata; slice is_empty
compares that length with zero. Str len uses the byte-slice representation and
slice length; str is_empty compares that byte length with zero. The reviewed
transitive boundary is core pointer metadata/its compiler intrinsic and the valid
str-to-byte-slice representation; these do not read contents or mutate, allocate,
panic or diverge on valid supported shared references. Library-internal unsafe
representation code does not authorize unsafe caller source. Test zero-sized
slice elements, empty/nonempty slices and multibyte UTF-8 independently with the
pinned compiler; preserve source/build provenance and licenses.

The instantiated interface is exactly one shared slice/str receiver and a usize
or bool result. Slice elements must have the existing verified drop-free concrete,
non-reference-bearing boundary. Require original unexpanded method syntax with no
arguments/turbofish, exact resolved core definition and signature, and verified
formal origins for the receiver. Allow only one builtin shared dereference
followed by a shared reborrow preserving the same referent (or no adjustment when
an exact shared receiver already has that formal type). Reject array coercion,
owned/overloaded/mutable receivers and extra shared dereferences. Source spelling,
caller effect flags and successful compilation cannot substitute for these checks.

Keep CALL operations and receiver evaluation/value edges. Carry exact summary,
instantiated types, library/build evidence and receiver/origin facts on both paired
documents. Independently check actual serialized CALL membership/identity, callee,
operand positions/types/value edges, effects and exact summary inventory without
producer re-emission. No buffer read, generic memory node, write, exceptional exit
or inlining is introduced. Metadata dependence flows from the receiver; shared
roots conservatively may alias. Existing reference counts, projection, nine
features, gates, WL and scoring are preserved; native usize build compatibility
remains mandatory. Trace collection must not change outcomes.

Before admission, test compositions with stored slice/str fields, lifetime-only
receivers, branching, casts and supported scalar operations, plus mismatched
summary/core/signature/adjustment/origin facts and serialized CALL tampering.
Exercise fresh/warm Cargo runs, edits/build changes, contexts, full exhaustive
pairs and historical profile exclusions. Natural applicability targets remain a
separate preregistered inventory; these fixtures are correctness evidence only.
