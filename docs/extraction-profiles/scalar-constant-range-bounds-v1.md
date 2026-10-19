# Resolved constant range bounds

Data-v3 extraction 3.5.0 adds `rust.scalar-constant-range-bounds`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Integer ranges may use same-unit scalar constants
and nongeneric inherent associated constants as either endpoint. Imports, mixed
literal/constant endpoints, inclusive/exclusive/open bounds, OR alternatives and
scalar guards compose under the existing match restrictions.

Capture retains one `pat_range` and its compact integer-range descriptor. Literal
bounds keep their original spellings; named bounds contribute canonical scalar
values. Each named endpoint is a `pat_constant` child with its original endpoint
span, resolved integer type and the existing constant resolution record. Rustc
owns name resolution and initializer evaluation, under the
[constant contract](scalar-constants-v1.md). Only referenced constants are evaluated;
the driver reuses their records when constructing the descriptor.

Independent admission parses the original range source and checks bound syntax,
inclusivity, type, value and declaration consistency. The number of captured
constant bounds must match the original named endpoints. Each record must match
the exact original endpoint byte span; the resulting canonical descriptor must
equal the captured selector descriptor. An absent upper bound remains exclusive
in the descriptor, as in the [literal-range contract](scalar-range-patterns-v1.md).

Lowering retains the existing single `match.range` or mixed `match.or-patterns`
selector over a saved scrutinee. Bound records do not emit runtime reads or
additional executable operations. No per-integer expansion, duplicated guard/body,
new frontend field, source snapshot, provenance table or cache is introduced.
Constant edits change dependent captured evidence. Strict graph verification and
matching retain the canonical bounds and existing control/data dependencies.

Nonlocal, generic and trait constants, character ranges and payload patterns remain
outside this slice. The final wildcard and existing type/control/helper limitations
still apply. Earlier Cargo selections do not admit constant range bounds.

Validation covers imported/inherent endpoints, signed values, u128 extremes,
exclusive and open ranges, guarded mixed alternatives, dependency edits,
earlier-profile/nonlocal exclusion, and malformed selector/source correspondence.
Independent record tests reject missing records, wrong positions and wrong values.
Natural coverage is measured separately with the unchanged frozen inventory.
