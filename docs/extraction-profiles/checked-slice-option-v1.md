# Bounded checked scalar slice reads

Data-v3 3.36.0 adds `rust.checked-slice-option` and the
`rstyle.data.slice.get-scalar/1` summary. This connects the
[checked source contract](checked-option-source-contract-v1.md) to lowering,
independent document checking, strict admission and matching. Matching thresholds
and the matching profile are unchanged.

The supported family reads immutable bool/integer slices through `get(usize)`
and matches exactly two unguarded resolved core variants: `Some(&scalar)` and
`None`, in either order. The scalar binding is immutable. Variant import renaming,
saving the Option in a local, receiver rebinding after the save, scalar short
circuits and scalar-result helper calls compose within existing source and
reference-flow contracts. Compiler commit, core crate and definition identities,
receiver adjustments, concrete types and original operand correspondence remain
mandatory for each supported target. Both std and no_std preludes are supported:
only their exact Some/None display aliases share the same pinned definition
checks. No other path alias is inferred.

Receiver evaluation precedes index evaluation. The get operation retains both
ordered value operands, forms an Option reference and performs no content read.
Its normal result is Some when the index is less than the receiver length and
None otherwise, including `usize::MAX`. Operand effects and possible failure
remain in the graph. No unwrap, invented zero value or panic precondition is
substituted for None.

Each match retains explicit Some and None selectors on the same saved value.
The Some true branch contains a scalar memory read followed by its lexical
binding declaration. The None branch does not load an element. Serialized checks
verify the exact operation inventory, interpretations, operands, binding and
memory effects. Removing the Some true transfer must render its payload read
unreachable. Existing source-to-CFG verification additionally checks whole graph
structure and evaluation order.

The conditional address projects the saved receiver's formal origins through
`CheckedElement`. This denotes a potential scalar location, not a load or proof
that different indices are independent. The exact index remains a get operand;
the selector and payload retain dependence on that saved get result. Existing
reference snapshots preserve origins across rebindings; alias uncertainty remains
conservative. Scalar helper read summaries retain this formal projection.
There is no ABI representation inference, new capture inventory, provenance
cache or per-invocation identity.

General Option interfaces, construction, reference payload bindings (`Some(b)`),
guards, wildcard arms, array unsizing, range access, mutable access, nested or
owning payloads, general enums, Result and `?` remain outside this contract.
Historical profiles cannot acquire Option semantic types through this capability.
Support for this bounded family does not establish rewrite safety, abstraction
activation or held-out detector quality.

Validation combines the source-selected natural UTF-8 boundary predicate,
independent string-boundary and saved-pointer runtime oracles, real Cargo capture,
strict bundle import, equivalent-function matching, and document corruptions for
semantics, reads, operands, variant identity, scalar binding and Some guards.
The roadmap records measured admissions and validation results for each run.
