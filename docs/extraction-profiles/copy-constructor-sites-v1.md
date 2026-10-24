# Exact Copy constructor sites

Data3.102 adds `rust.checked-copy-constructor-sites` for original nongeneric
inherent constructors returning `Self` with exactly one supplied `&'static str`
and one fixed bool field. Acceptance targets are unchanged time0.3.47
`error::component_range::ComponentRange::{unconditional,conditional}` in the
default parsing library context recorded by the error-conversion development
selection. The source-derived expectation is preservation of the supplied string
backing/length and a false/true flag respectively; neither constructor allocates,
executes a helper, or drops an owned field.

The verifier checks original declaration identity, ordered fields and static
lifetime, exact builtin derived Copy, complete interface and transport origins,
unique inherent owner, active attributes/ancestors, and complete source/HIR body,
spans, types, literals and formal-local identities. The body must be one complete
struct literal. Calls, macros, locals, updates, extra work and omitted fields do
not satisfy this contract. Out-of-module impls remain outside this bounded route.

Positive Copy in a successfully captured complete compilation excludes Drop for
the target; both fields have independently known cleanup. This target cleanup
receipt is distinct from the exact constructor namespace receipt. Neither grants
whole-unit expansion/import closure or certifies neighboring functions. The
declaration and function namespaces must match the checked site before bypassing
their coarse namespace restriction. All other functions retain their existing
source and effect requirements. Historical documents cannot consume this proof.

The original error-conversion family remains incomplete: nominal enum/Result
payloads, guarded origins and selected conversion-member summaries still need
connected proof. General generated namespace exports, arbitrary Copy products,
ownership/drop protocols and rewrite safety are not delivered by this contract.
