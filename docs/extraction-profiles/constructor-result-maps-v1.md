# Generated checked constructors

Data-v3 3.99 (`rust.constructor-result-maps`) connects unchanged
`regex-automata 0.4.18::util::primitives::PatternID::new` to `SmallIndex::new`.
The [development selection and independent oracle](../../tests/fixtures/function-families/checked-index-construction/README.md)
retain original source/build identities and specify the interval
0..=2,147,483,646, with the exact attempted u64 preserved on error.

The bounded expression is a fresh local helper Result followed by `map` and
`map_err` with actual unary tuple constructors. Its three concrete Result types
remain distinct. Success products require independent Copy proof; ordinary and
generated non-Copy error products each require original-source no-drop proof.
Both branches inspect the discriminant, move exactly their selected payload once,
and rewrap that payload in the next Result type. Only the selected branch runs its
constructor. Helper return, panic and divergence evidence remains mandatory.

Admission joins the original lexical macro definition/invocation, complete selected
impl/member roster, exact source coordinates, HIR identities and hygiene. The leaf
argument must name the source parameter and resolve to its HIR binding. Constructors,
fields, substitutions and selected core Result implementations have independent
identity checks. Neighboring items and derives remain in source/cleanup closure.
An incomplete unit observation table cannot silently supply a selected missing
member; unrelated unsupported observations remain explicitly unsupported.

The independent document verifier checks stage-specific source roles even when
both stages share one invocation span, exact types, branch guards, payload and
constructor dependencies, unchanged branches, merge and whole return. Tests
reject swapped branches, copied/missing roles, surplus consumption, stale source,
wrong constructor selection and historical-policy downgrade. Raw receiver
inspection validates observations; semantic source proof is required before
admission. Source initialization never invokes completed helper/transport proof.

This contract excludes arbitrary callbacks, stored/reused owned Results, payload
binding, borrowing, arbitrary nested error shapes, general macro support and
unknown cleanup. `StateID::new` remains unavailable behind unknown-expansion
source lookup; `PatternID::must` additionally requires its own panic/formatting
proof. No detector-quality, abstraction-activation or rewrite-safety claim follows.

Routine integrity uses existing cached XXH3 and immutable snapshots. SHA-256 is
reserved for explicitly encountered historical identities and reviewed compiler
source pins. Generated capture is indexed once per owning unit; checked module
source is reused only under the identical configuration/proof mode.
