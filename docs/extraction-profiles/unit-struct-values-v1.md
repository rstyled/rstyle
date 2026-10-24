# Unit struct values

Data-v3 extraction 3.17.0 adds `rust.unit-struct-values`, extending
[named](named-struct-values-v1.md) and [tuple](tuple-struct-values-v1.md)
construction. Matching remains `rstyle.ccgraph.data-calls/1.0.0`.

Compiler-resolved unit-struct constructor paths, including aliases and inherent
`Self`, normalize to the existing `expr_struct` aggregate kind. Ordinary local and
constant paths keep their previous resolution and verification. Constructor
classification applies only to path expressions and checks the resolved constructor
before selecting the capability; scalar literals and other expressions acquire no
new classification work. The original-source inventory maps the path syntax to
its normalized aggregate in the existing traversal.

Independent verification resolves the exact original nominal path, checks its
concrete type and parses the already-retained original declaration. The declaration
must have unit fields, distinguishing `struct Unit;` from both `struct Empty {}`
and `struct Empty();`. There must be no runtime children. This adds no shape flag,
frontend record, provenance, source witness or cache. Existing whole-unit nominal
and drop-free checks still apply, including the requirement that nominal resolution
have the relevant type tree.

Lowering uses the existing `data.struct` computation with an empty slot vector
`[]` and no value operands. Independent document verification derives this empty
mapping again, while strict producer verification reconstructs the complete graph.
Unit structs are nominal aggregate values, not the scalar unit value `()`.
Nested aggregates, supported conditional results and repeat arrays with supported
counts compose without per-field expansion or additional value evaluation.

The real Cargo fixture checks aliases, grouping, `Self`, nested values, a
zero-length repeat, conditional results and supported scalar effects before a unit
value. Every admitted pair passes independent document and strict producer checks;
equivalent functions reach evaluated accepted matching with score 1. An invented
slot is rejected. Struct-typed constants remain excluded, and relabeling a constant
whose type is an empty named struct as a unit constructor fails independent
source/declaration verification. Legacy construction exclusions remain intact.

General aggregate local propagation, reference-bearing construction, broader repeat
counts and qualified qself constructors remain unfinished. D11 remains Partial;
D12–D13 and the full roadmap remain active. Frozen inventory results are recorded
separately in the roadmap continuation log.
