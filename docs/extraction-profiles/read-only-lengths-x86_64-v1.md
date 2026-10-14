# Pinned x86_64 read-only length authority, version 1

Registered at implementation baseline `6439720`, after Ubuntu CI run
`38461968140` exposed the aarch64-only authority boundary. This supplements
`read-only-length-registry-v1.md`; it changes no supported signatures, effects,
source correspondence, projection or matching semantics. R0 and scalar profiles
remain unchanged. The original aarch64 registry is byte-for-byte unchanged.

The official rustup `nightly-2026-09-30` target distribution for
`x86_64-unknown-linux-gnu`, rustc commit
`5c543b0b8c73c7b72bc8284ced4fb22ead15734d`, pointer width 64, receives separate
identity `rstyle.rust.read-only-length-registry.x86_64/1`.
Its core crate hash is `f961ecb4c84cffc7c73b0d153b2677a7`.
The source definitions and reviewed transitive metadata boundary are the same
pinned rust-src modules as the aarch64 authority; their digests are independently
checked for both registrations. This adds target authority, not arbitrary core
acceptance. Other targets/compiler/core builds remain unsupported.

| Summary | Second definition Fingerprint component |
| --- | --- |
| `rstyle.data.slice.len/1` | `4301500677719972613` |
| `rstyle.data.slice.is-empty/1` | `15784485536792729161` |
| `rstyle.data.str.len/1` | `1291990020261160599` |
| `rstyle.data.str.is-empty/1` | `939469410544972960` |

Each first Fingerprint component is `2625310331494506890`.

| Artifact | SHA-256 |
| --- | --- |
| rmeta | `f60295cda5a38bd81f3153cbe361654e5b65b13a67cdc775b3f8ac762e2712ba` |
| rlib | `92a2e5fe2a37ab925f8d4b3781bc227c14363442afa02fa2c669d65464fcf131` |

Evidence was captured with the real pinned driver and a fresh four-function
slice/str len/is_empty Cargo workspace, using explicit
`--target x86_64-unknown-linux-gnu`. The official target was installed with rustup;
actual resolved core fingerprints and artifact hashes agree with the user-provided
Ubuntu failure log. The rlib was hashed independently from the same official
distribution. Before registration all four functions were correctly excluded.
The captured target artifacts are identical across the aarch64 cross compiler
and Ubuntu x86_64 native compiler; compiler host paths are navigation, not identity.

Selection requires exact captured compiler commit, target and width, followed by
exact core hash, each supplied artifact digest, resolved definition and signature.
Serialized evidence identifies the selected target registry. Neither host cfg nor
method spelling authorizes effects. Wrong artifacts for a registered build remain
incomplete execution; a different consistently captured core remains unsupported.
No tests are disabled, architecture-gated or weakened. Existing real-command
families and corruption tests exercise both native CI architectures. Cross-target
capture additionally checks the x86_64 source-to-pair boundary from the ARM host.

The x86_64 four-function command admits all four and evaluates all six pairs;
warm/trace-off execution preserves every semantic outcome and pair decision.
The BOM/CRLF fifteen-function length family from the disclosed CI regression
admits all fifteen and evaluates all 105 pairs. Capture, assessment, enumeration
and evaluation each report complete. All thirty family documents validate against
unchanged vendored schemas (the four-function run's eight documents also validate).
These are correctness fixtures, not natural applicability or quality measurements.
Logs: `target/x86-core-{before,after,warm}.json`,
`target/x86-length-family.json` and the corresponding schema logs.

`cargo test --workspace --locked` passes 591 tests, zero failed or ignored,
including all 104 real-command tests on the aarch64 host. Formatting and workspace
all-target Clippy with warnings denied pass. Style reports 1,656 functions and
zero violations; patterns reports zero candidates. Inventory and harness scripts
pass eleven and forty-three tests. Native x86_64 CI must still confirm the pushed
repair; the local x86_64 evidence above uses real cross-target capture.
