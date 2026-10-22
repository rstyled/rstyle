# Calendar readiness checkpoint

This is an audit, not an activated extraction profile. The unchanged selection
contains three `time` callers and five required `time-core` helpers in six
contexts. No admission count is available until all selected units are captured.

| Dependency stage | Confirmed restrictions | Required evidence |
| --- | --- | --- |
| Registry dependency capture | `clones/capture.rs` sets `RUSTC_WORKSPACE_WRAPPER`; registry dependencies are outside that wrapper | Actual Cargo units, original source census, resolved `--extern` edges and dependency features |
| Helper linking | `helper_references::same_unit` requires identical selected-unit context and invocation | Separate bounded cross-unit proof; preserve historical same-unit checking |
| Public leap leaf | Original body calls `i64::unsigned_abs` and `u64::wrapping_mul`; neither has an explicit current PDG operation implementation | Captured core identities, full i32-domain arithmetic semantics, strict replay and mutated-callee rejection |
| Selected attributes | `function_attributes` rejects `track_caller`, `cold`, `deprecated`; census module policy rejects `expect` | Exact attribute grammar and semantics, including panic caller location; no blanket attribute permission |
| Complete `Month` owner | Fieldless source grammar rejects `repr(u8)` and variant `expect(missing_docs)` | Complete twelve-variant source, exact u8 discriminants, captured compiler agreement, Copy/derive/drop ownership and builtin cast semantics |
| Owning namespaces | `month.rs` imports core fmt/num/str, powerfmt, sibling errors and `self::Month::*`; other impls remain present | Exact selected-owner/import proof retaining neighboring items, generated names, traits, macros and active cfgs |

The public leap leaf uses the full-domain algorithm. The separate
`range_validated` module is retained in the census but is not a selected callee.
`hint::cold_path` is an actual transitive helper even though its body is empty;
neither its presence nor its attributes can be silently discarded.

Next checkpoint: capture-only dependency closure, then classify every selected
function against all these restrictions. Confirm actual resolved identities and
additional operation/source failures before designing the cross-unit linker.
Do not infer that captured leaves admit, or that arithmetic names prove purity.
An absent dependency remains incomplete/unknown, not excluded or zero.

Activation requires the original complete chain through capture, verification,
strict admission and matching. Independent runtime expectations already pass
against unchanged libraries in both native contexts; this provides no extraction
or cross-target execution claim. Adversarial expectations remain in
`selection.json`, including stale edges, replaced transitive callees, wrong build
contexts, altered discriminants and missing source. No admission gain is claimed
for this checkpoint; frozen inventories and historical policies are unchanged.
