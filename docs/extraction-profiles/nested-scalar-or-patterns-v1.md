# Nested and parenthesized scalar OR patterns

Data-v3 extraction 3.10.0 adds `rust.nested-scalar-or-patterns`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Integer/bool OR patterns may recursively contain
supported literal, constant, integer-range and wildcard leaves, with transparent
parentheses. Leading `|` syntax is supported, including rustc's one-child OR node.
Named constants/range bounds retain their existing same-unit/nongeneric contract.
The existing arm limit, scalar type checks, guard restrictions and whole-function
admission requirements still apply.

Capture keeps the compiler's nested `pat_or` tree, original spans, leaf types and
literal/constant/range records. The independent source inventory traverses each
OR/group node once, matches transparent groups to the resolved HIR and retains
all corresponding supported leaves. It descends into OR alternatives only when
the parent has unique compiler-HIR correspondence; unsupported opaque patterns
do not trigger inventories of absent children. Comments inside parentheses are checked as
syntax when the existing whitespace/parenthesis fast path cannot decide the
correspondence. Non-group wrappers cannot substitute for parentheses. Unsupported
parents/leaves still fail full source/HIR eligibility; this does not authorize a
supported remainder. Parentheses around standalone supported scalar patterns use
the same correspondence and range-bound checks.

Lowering flattens only OR nodes into ordered leaf references for one selector.
Literal-only leaves use the existing `match.or` literal list; mixed leaves use
`match.or-patterns` with existing kind/payload pairs, including `pat_wild` with a
null payload. Repeated alternatives and original literal spellings remain present.
Flat OR patterns use the existing encoding without allocating a flattened leaf
vector. Nested selectors allocate one leaf vector, without copying HIR records,
expanding numeric ranges or generating a Cartesian product. No arm or guard is
duplicated, and the scrutinee is evaluated once.

A wildcard anywhere in an OR makes that pattern irrefutable, so its selector has
no false continuation. A failed guard still proceeds to the next arm. Independent
exhaustiveness also composes with nested literal/constant/range alternatives and
transparent scalar groups. Interval collection appends into one shared vector
instead of allocating and copying a vector at each nesting level. Existing final
wildcard matches retain their direct coverage fast path.

Strict reconstruction and projection retain the ordered selector payload and
control/data edges under the existing matching vocabulary. No frontend fields,
serialized witnesses, provenance tables, source snapshots or caches are added.
Earlier Cargo selections retain their flat/grouping boundaries. Enum/payload,
reference/slice, `@` binding and non-scalar patterns, character ranges and nonlocal/
generic/trait constants remain outside this slice; F06/F07 remain Partial/Missing
for those broader forms.

Validation composes nested/flat equivalence of literal payloads, mixed constants
and ranges, grouped negative literals and named bounds, comments, exhaustive bool
matches, leading pipes, repeated alternatives and guarded wildcard alternatives.
It checks one selector/guard, retained nested capture, ordered duplicate payloads,
absence of an impossible wildcard failure edge, unsupported-pattern exclusion,
earlier selections and rejection of a corrupted selector. Source-correspondence
checks reject array, call, tuple, block and unbalanced wrappers. The frozen natural
inventory measures actual admissions separately.
