# Owned Copy receivers for read-only helpers

Data-v3 3.38.0 adds `rust.owned-copy-receiver`. Contract specified before enabling admission: the complete selected
`Position::new` → `Span::new` → owned local → `Span::is_empty` chain must preserve
both constructor inputs and return exactly `x == y`. The existing frozen
selection and build contexts remain unchanged. Use the separate position/span
corpus in its complete mixed module context, on the pinned nightly.

Support is bounded to a concrete owned struct whose scalar/nested-struct Copy
proof has independently passed the selected nominal source boundary. The exact
receiver adjustment must be one shared borrow to the callee's instantiated first
parameter. Remaining parameters retain exact source types. Only scalar-returning,
verified read-only, nonpanicking helper summaries qualify. References cannot
escape. Mutable, overloaded and additional dereference adjustments, missing Copy,
reference-bearing structs and unknown neighboring expansions remain excluded.

Every callee read occurrence, including intermediate aggregate reads, must retain
its operation/access identity, ordered field path and concrete value type. For
an owned receiver these are projections of the existing CALL input value snapshot,
not reads of an invented formal memory location. Shared actual arguments retain
the existing formal-origin substitution. Verification must recompute both forms
and reject missing or changed snapshot read interpretation. No capture inventory,
source buffer, cache, alias independence or purity claim is added.

Positive expectations include owned parameters, owned locals, constructor/helper
results and repeated getter calls. Negative expectations include non-Copy owned
receivers, mutable/shared-result helpers and corrupted adjustment/read evidence.
Independent runtime checks compare the composed result for distinct constructor
components. Matching completeness is separate from semantic equivalence: the
existing offset-versus-column detector limitation remains open.
