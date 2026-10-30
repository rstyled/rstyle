# Calendar helper dependency family

Source-first selection of unchanged registry `time 0.3.47` and its pinned
`time-core 0.1.8` dependency. `selection.json` freezes original function/owner
bytes, spans, file hashes, complete source-tree identities and six build contexts
before admission measurement. This separate development corpus changes no frozen
inventory or historical denominator.

The natural chain is `Month::length` and deprecated `days_in_year_month` →
`time::util::days_in_month` → `time_core::util::days_in_month` →
`{is_leap_year, days_in_month_leap → hint::unlikely → hint::cold_path}`.
The last empty helper is part of the closure, including its cold/inline attributes.
Complete package source identities retain neighboring imports, derives and items.

Use an exact-version registry Cargo consumer (`time =0.3.47`,
`time-core =0.1.8`), with default/no-default time features on ARM Linux, x86 Linux
and ARM macOS. Capture the original dependency library units under those consumer
flags. Direct original-manifest offline resolution needs unavailable dev
packages; changing to dependency compilation is explicit, not an interchangeable
baseline. Missing selected dependency capture is **unknown/incomplete**, never
excluded or zero. The capture-only checkpoint observed all eight selected originals before linking.
The connected implementation now admits all eight in every context; see readiness.md
for the measured comparison and exact remaining boundaries. `large-dates` is deferred as a separate feature context.

The bounded linker covers nongeneric direct free scalar functions with empty
type/const substitutions, exact Cargo `--extern`/artifact/package/build/definition
relationships and the complete transitive closure. `Month` independently needs
complete local `repr(u8)`, discriminants, Copy/drop, cast and source-boundary proof.
Generic trait dispatch, cross-unit nominal transport, references and ownership
are outside this contract. All scalar/assertion/evaluation effects still require
existing strict evidence; linking never invents purity.

Independent expectations use Gregorian divisibility and a twelve-entry month
length table, covering every month in years −9999…9999, negative/zero/century years
and i32 endpoints. Debug-domain invalid numeric months must panic. Required
adversarial cases include absent/stale dependency or transitive units; wrong
package/version/source/definition hash; wrong target/features/build; incorrect
`--extern`; and resealed same-name transitive replacements. These define acceptance obligations; the linked contract identifies implemented
proof checks and adversarial controls.

`oracle.py --verify-only` checks identities without building. When scheduled,
`oracle.py --output /tmp/calendar-native.json` builds a temporary exact-registry
consumer and runs `oracle.rs` against unchanged public APIs in both host feature
contexts; Cargo metadata must resolve the frozen source roots. Runtime checks do
not claim cross-target execution, extraction, admission or matching. Selection preceded evaluation. Native validation now passes in both feature
contexts: 240,216 month/year cases and nine expected debug panics each, with zero
mismatches. All eight originals now admit in all six contexts: 18 new contextual admissions
and zero losses against the capture-only baseline. Strict closure replay and all
153 pair evaluations per context complete. No equivalence or held-out quality
claim follows from these matching results.
See `readiness.md` for measured blockers and the next checkpoint.
