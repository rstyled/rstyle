# Independent method receiver verification, version 1

Implementation baseline: `7ccfc10`. This Data-only supplement checks observational
receiver evidence before assessment; it does not authorize length or other new
calls. Existing profiles and matching behavior remain unchanged.

Reconstruct the complete HIR method inventory without invoking capture or graph
lowering. Require one record per exact method identity, its source span and first
receiver child identity; reject missing, duplicate or extra records. Reparse the
original function using the independent source correspondence inventory. Every
unexpanded method must have exactly one original method invocation correspondence;
expanded methods remain observational and unavailable for registered Data calls.
Source operations removed by cfg remain under the existing unknown-source policy;
no receiver record fabricates their presence.

Compare captured concrete receiver/result types with the independently retained
HIR value types. Verify captured trees with the closed concrete schema/budgets.
Known builtin shared dereference transitions remove exactly one Shared wrapper;
known shared-borrow transitions add exactly one. The last target equals the
adjusted receiver type; zero adjustments preserve the original type. Unsupported
states are retained without transition certification. Mutable references and
unsupported conversions remain unavailable to the library registry. Known target
types cannot be passed off as unsupported mutable-reference targets. Stable
unsupported reasons must belong to the declared closed conversion set.

Function signature observations remain observational until the pinned library
registry independently establishes the exact instantiated interface. Neither
signature agreement nor successful transition checks authorize effects or calls.
No graph operation, threshold or projection changes. Source identities, origins,
core/build fingerprints and serialized CALL semantics remain additional admission
obligations.

Current real Data handoffs require the receiver inventory even when empty, then
validate it before per-function assessment. Corrupt/missing evidence is incomplete
execution, not ordinary exclusion or a complete zero-pair result. Standalone
semantic checking may accept absent inventory only when there are no HIR method
calls, to keep historical empty observational records readable. Non-Data records
must not acquire this extension. Check inventories independently on the paired
frontend evidence during serialized Data verification, without producer re-emission.

Test complete captured families and changed identities/types/adjustment order,
missing/duplicate/extra records, forged unsupported reasons and legacy absence.
Library calls remain excluded until the separate pinned summary contract is wired.

Existing registered scalar primitive summaries already establish an instantiated
interface. For those methods, require the observed formal inputs to agree exactly
with the registered argument types, in addition to the checked result. Unregistered
method signatures remain observational. This does not add a summary, authorize a
length call or broaden any historical primitive instantiation.
