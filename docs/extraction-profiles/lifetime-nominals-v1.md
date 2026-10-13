# Lifetime-only concrete nominals, version 1

Implementation baseline: `6d30c00`. This supplement belongs only to the separately
registered local `hir-data-v1` contract. It supersedes that profile's previous
exclusion of lifetime-only nominal declarations; type/const generics and all
historical extraction/matching profiles remain unchanged.

Support local structs and inherent impls with only unbounded, unattributed lifetime
parameters and no where clause. Every instantiated compiler argument must be a
region; any type/const argument remains unavailable. Field types must be resolved
using that exact instantiation before bounded recursive capture. The independent
original-source census must recover the exact declaration, owning impl and named
imports/aliases. Declared lifetime-parameter counts and explicit terminal path
lifetime-argument counts must agree. Elided lifetime arguments and original shared
reference lifetime spelling remain subject to successful compilation of the exact
source/build context. Lifetime arguments on nonterminal path segments, type/const
arguments, qualified-self paths, impl lifetime bounds and where clauses are outside
this initial slice.

Nominal identity still includes package source/version/manifest and exact
declaration path/digest. Lifetime spelling is not a matching type discriminator:
regions do not change fields or layout in this finite model. This is the existing
data interface interpretation, not structural erasure or graph canonicalization.
Integer widths, shared mutability, reference/referent distinctions and ordered
nominal field identities remain exact. No unrelated nominal interfaces become
compatible. Compiler lifetime validity is evidence for the compiled function,
not an assertion of lifetime-parametric semantic equivalence.

Existing source-backed field reads, field-place borrows, stored-reference origins,
branch alternatives, normal CFG and dataflow rules apply without new operations.
Origin paths remain formal projections, including SharedReferent for loaded
stored references. All shared-bearing input roots may alias. The independent
drop-free/interior-mutation census and recursive type boundary remain required;
compiler destructor flags alone remain insufficient. No mutable references, raw
pointers, ownership/drop summaries, trait dispatch or memory writes are added.

Independent verification must check lifetime-only declaration/impl grammar, exact
path resolution and argument arity, resolved field/type correspondence, source
ownership and actual serialized locations/value origins. Altering declaration
parameters, explicit argument counts, field types, owning impl or source digests
must fail closed. Existing budgets remain exact and never truncate. Real Cargo
fixtures must exercise shared receivers, stored scalar/str/slice references,
nested nominal fields, by-value values and conditional outputs, including aliases
across modules and distinct lifetime instantiations. Historical helper/expanded/R0
selection decisions and trace on/off outcomes require regression checks. Generic
type/const cases, bounded lifetime impls and reference-bearing array element values
remain explicit negatives. Exhaustive source-to-pair evaluation is required before
claiming this slice admitted; no natural coverage or quality claim follows.
