# Selected cross-unit scalar helpers

Data-v3 3.82 connects the unchanged calendar family in
[the separate development selection](../../eval/coverage-development/calendar-helpers/selection.json).
`Month::length` and deprecated `days_in_year_month` call the local
`time::util::days_in_month`, which calls the original `time-core` dependency.
All five selected core helpers remain part of the complete required chain.

## Authority and support boundary

Explicit `--capture-dependency NAME[@VERSION]` selection captures ordinary,
non-test check-library units. Linking joins the actual Cargo graph edge and
extern alias, successful emitted artifact, caller `--extern`, rustc loaded rmeta,
stable crate ID and exact definition identity. The pinned metadata header's native
crate hash must agree with rustc's loaded hash. Reading that field is an identity
consistency check, not authentication or independent recomputation of rustc's hash.

Bodies stay in separate owning unit stores; foreign references include the owning
namespace. Raw `/5` bundles remain independently importable and cannot acquire
foreign-call authority alone. A separate run closure records owning ledgers,
namespaces and checked associations without copying nested callee bodies.
`--replay-run DIRECTORY --extraction-profile hir-data-v3` strictly imports the
original units, reconstructs artifact associations and recertifies the closure.
The original absolute paths and emitted artifacts must remain available; this
initial replay format is not relocatable.

Only direct nongeneric free calls with scalar/unit interfaces and empty type/const
substitutions cross units. Compiler, producer, target/layout, pointer width,
optimization, panic, overflow and debug evidence must be known and compatible.
Each unit retains its own edition, features, cfg and invocation. Existing summary
checks preserve return, panic, divergence, argument order and the bounded helper
depth. Recursive calls, cross-unit nominal/reference/memory results and mutable
summaries remain unsupported. Captured binary/test closure contexts are outside
this first linker boundary and fail explicitly; their inventory is not omitted.

## Month source proof

The complete selected lexical owner independently proves exact `repr(u8)`, all
unit variants/discriminants in 0..255, and pinned builtin Copy/derive evidence.
Compiler repr and discriminant observations must agree. HIR snippets that omit
outer attributes cannot supply this proof. The existing default-repr contract
remains available for declarations that independently satisfy it; another layout
attribute cannot fall through that contract.

Source resolution adds exact `self::Enum::*` variant exports and bounded local
nominal reexport chains, with collision/cycle/missing-target checks. This closes
`util::Month` → root reexport → `month::Month`. Only the required literal
`deprecated(since=..., note=...)`, `expect(deprecated)` and variant
`expect(missing_docs)` metadata are added. Unknown attributes, active path changes,
unproved expansions, imports and neighboring source obligations retain their
existing restrictions. These additions are capability gated; historical profiles
retain their earlier behavior. No general derive, module-glob or alias permission
is implied.

## Performance and evidence

Transport consistency uses existing XXH3-64 seals once per actual artifact path.
There is no new SHA-256 or body serialization in lookup paths. Existing source
identities remain unchanged and memoized. Cargo routes and definition links are
indexed; dependency scheduling is linear in units and edges; shared dependency
analyses reuse their immutable owners within one analysis construction.

All eight selected originals admit in six unchanged contexts: default/no-default
features on ARM Linux, x86 Linux and ARM macOS. Against the capture-only baseline,
the three callers produce 18 contextual gains and zero losses, including complete
package roster comparisons. Strict saved-run replay completes all 153 pairs per
context. Matching is structural evidence, not a claim that differently structured
wrappers must match or that matched functions are equivalent.

The independent unchanged-library native oracle passes 240,216 month/year cases
and nine expected debug panics per feature context. Cross-target capture is not
native execution on those targets. Resealed artifact replacement, stale/missing
owners, alias/namespace/context conflicts, unknown compiler settings, historical
policy and malformed enum/source controls exercise the corresponding proof
boundaries. Frozen inventory identities, denominators and historical results are
preserved. Full quality-gate results are recorded with the roadmap milestone.
