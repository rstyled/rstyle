> Historical version 1 format. Current reports use
> [unit-owned build contexts and nominal binding version 2](../performance/unit-build-contexts.md).
> The former `build_digest` is replaced with a unit-scoped `build_id`.

# Nominal compilation-unit binding, version 1

This is an observational prerequisite for the unavailable read-only data tier,
not an extraction or matching profile and not a drop-free/effect certificate.
It extends neither pinned R0 nor the registered expanded/helper admission grammar.
No vendored schema changes, detector changes or matching compatibility changes
are authorized by this contract.

## Package authority

For opt-in `hir-helpers-v1` capture only, the fresh invocation plan freezes
`rstyle.rust.cargo-package-capture/1` for each selected package: Cargo metadata's
opaque resolved package ID, package name/version (including workspace-inherited
values), absolute manifest path, exact UTF-8 manifest bytes and SHA-256 digest.
The bytes are read while planning, before the selected build. The existing
planned/observed target and unit checks must agree on this package identity.
A captured nominal's name/version/manifest digest must agree with this independent
plan snapshot. No parsing of opaque package IDs or inference from names is used.
The package snapshot is bounded to 1 MiB; exhaustion is explicit failure.

Original declaration origins remain `rstyle.rust.declaration-origin/1` with their
unchanged semantics. Compiler-reported source-file paths are made absolute against
the actual compiler working directory. Binding requires absolute source names,
lexically normalized within the selected manifest directory; relative names and
paths outside that root cannot obtain a binding. Symlink target identities are
not inferred by rereading source files. This initial boundary does not certify
source aliasing through symlinks or remapped paths.

## Independent serialized handoff

After exact unit artifact inventory checks and frontend invocation verification,
verify the original declaration inventory against every successfully captured
interface tree. Reject unknown versions, missing/duplicate/conflicting origins,
changed nominal observations, altered bytes/digests, wrong item intervals and
package mismatches. The existing origin source parser remains an independent
check, not a re-emission of producer output.

Across one captured unit, each absolute declaration-file name must denote the
same exact original bytes. Any function captured from such a file must have its
exact original function byte interval and source digest agree with that file.
Nested functions retain their own intervals; interval containment is not used to
select an owner. No compiler artifact ordering or cross-invocation directory
searching is allowed.

Publish `rstyle.rust.nominal-unit-binding/1` per observed concrete interface in
the function report: selected package ID, manifest digest, declaration nominal
identities and absolute source-file identities/digests, and an exact digest of
captured build evidence. The build digest identifies the actual capture context;
it is **not** a semantic compatibility fingerprint or a new matching gate.
Functions with no concrete observations receive no binding. Orphan origin facts
or unavailable required evidence cause incomplete handoff, never empty success.
The additional report field is omitted for existing scalar/default paths.

## Scope and verification obligations

This contract binds package/source observations to a selected unit. It does not
prove effects, Drop/Freeze properties, lifetime/reference origins, alias freedom,
named type aliases, layouts, memory operations or document lowering. Shared
references remain potentially aliasing. The data tier remains unavailable for
admission until its complete registered producer and matching contracts are wired.

Independent fixtures must cover identical nominal spelling/manifest bytes under
different package roots, nominal package/version/digest mismatches, contradictory
original file observations, exact function range mismatches, malformed or missing
observations, outside-root names, BOM/CRLF, and inherited Cargo package versions.
Real command fixtures must exercise module/package capture, fresh/warm runs,
multiple target contexts and unchanged default/expanded decisions. Existing
style and patterns checks must report zero violations/candidates without tuning.
