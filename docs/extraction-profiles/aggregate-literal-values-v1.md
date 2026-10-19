# Tuple and array literal construction

Data-v3 extraction 3.12.0 adds `rust.aggregate-literal-values`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Nonempty tuple literals and explicit array
literals produce aggregate values from ordered, independently typed components.
Empty array literals are supported; the existing empty unit tuple remains a unit
literal. Nested tuple/array values, formal component values, scalar calculations
and supported conditional component values compose.

The driver captures array children in source order; tuple children already existed.
Existing concrete type tokens describe each constructor and component. Independent
frontend checks compare source syntax, arity, each component type and exact original
component spans in order. They reject swapped same-typed children, not merely a
mismatched multiset. Parentheses around components remain transparent.

The initial value boundary excludes shared/reference-bearing components, unsized
values and directly diverging components. Struct construction/update, repeat
arrays, general aggregate local reads and local/temporary borrows remain separate
unfinished D11–D13 work. Declaration of a supported interface type does not bypass
these operation and origin requirements. Earlier selections retain their original
construction exclusions. Source component lists currently require complete
one-to-one correspondence; cfg removal within those lists remains unfinished
conditional-source composition.

Lowering evaluates each component once in source order and emits one constructor
computation with ordered value operands. It does not pretend that a nonempty tuple
is the unit literal. Component values survive later component effects. Unit-valued
components with no emitted value receive a `unit.value` computation after their
normal completion, so later operand positions cannot collapse. Independent acyclic
and cyclic value validators allow unit edges only into correctly typed constructor
slots under this capability; unit scalar operands remain forbidden. Constructor values
retain their complete tuple/array type under the existing graph vocabulary.

Operand collection appends to one vector; it no longer reconstructs the accumulated
vector for every operand. Constructors retain their component values directly,
avoiding the scalar operand helper's node scan and temporary vector per component. Verification compares the array length without allocating
an expanded list of component types. Serialized constructor checks build temporary
node/operand lookup maps once per function, then check exact component positions,
source values, types and absence of invented memory/scalar effects. These maps are
local verification work, not retained caches or serialized witnesses. No frontend
record, provenance table or source snapshot is added.

Strict reconstruction additionally verifies the complete CFG/PDG pair. A real
Cargo fixture composes nested values, nonunit conditional components, empty arrays,
unit blocks/arrays, scalar reassignment after an earlier component read, scalar
operations and an evaluated equivalent tuple pair with score 1; legacy, aggregate-local, reference-bearing
and repeat-array exclusions remain explicit. Tampering tests swap same-typed HIR
children and serialized operand positions and require frontend, independent data
document and strict producer checks to reject them. Natural frozen admissions are
measured separately in the roadmap continuation log.
