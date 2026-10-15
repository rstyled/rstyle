# Read-only library resolution, version 2

The normal driver trusts the supported local rustc/toolchain installation. It
captures compiler-resolved core crate identity without opening `.rmeta`, `.rlib`,
`dylib` or `sdylib_interface` files. Exact artifact bytes are no longer part of
summary authorization. Modified/custom sysroots are outside this supported
assumption; this is not protection against malicious compiler replacement.

`data_library_resolution` replaces `data_library_artifacts`. Its closed record
has identity `rstyle.rust.library-resolution/2` and a bounded `libraries` list;
each library contains only `crate_name` and compiler-provided `crate_hash`.
Paths, byte counts and `sha256` fields are removed. Old resolution records do
not satisfy the new format. No compatibility runtime or artifact cache is added.

Code-owned `read-only-lengths-v2.json` and the x86_64 counterpart retain the
supported rustc commit, target, width, core identity, exact resolved definitions,
reviewed semantic source constants and operation semantics. Registry identities
advance to `/2`; the v1 files remain historical constants and are not loaded by
runtime code. Serialized CALL evidence uses `core_resolution` in place of
`core_artifacts`, with the new registry identity. Summary operation versions and
mathematical meaning are unchanged.

Authorization still requires the selected compiler/target/width, resolved core
crate and definition, direct resolved method dispatch, supported concrete shared
receiver, exact instantiated signature, argument count/order, receiver origins,
original source invocation and registered operation semantics. User lookalikes,
unresolved calls, unsupported toolchains/receivers and contradictory evidence
remain unavailable or fail import. A consistent serialized record does not grant
current-run trust: strict import still validates the source/graph/paired CALL
relationship under strict policy.

Retired guarantees: normal capture no longer detects accidental or intentional
replacement of library bytes that leaves compiler identity claims unchanged, and
library file I/O/size budgets no longer apply because those files are not read.
Inventory completeness, selected-unit/run association, source correspondence,
effect checks, resource budgets for actual analysis, and checked publication are
unchanged. Separate experiments may fingerprint artifacts independently if they
need exact provenance; no such consumer is introduced here.
