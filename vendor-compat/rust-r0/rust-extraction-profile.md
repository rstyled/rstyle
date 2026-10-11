# Rust HIR R0 extraction profile

**Profile ID:** `compat.rust.hir-r0` · **Version:** `0.1.0`  
**Contracts:** [CFG 0.2.0](cfg.schema.json), [PDG 0.1.0](pdg.schema.json).  
**Status:** normative extraction target, not an implemented producer or a semantic
support claim. Semantic support remains **unknown until independent fixtures pass**.
The new-profile capabilities remain unimplemented as recorded in
[rust-capabilities.md](rust-capabilities.md). Nothing here implements `rstyle`,
changes the schemas, or asserts legacy conformance.

MUST, MUST NOT, SHOULD and MAY are normative. This profile specializes the
[decision record](ccgraph-decisions.md). Matching is separately specified in
[ccgraph-profile.md](ccgraph-profile.md) and its
[machine configuration](ccgraph-profile.json); those companion documents do not
supply extraction evidence. Extraction eligibility is independent of size,
thresholds, scores or matching success.

## 1. Analysis boundary and eligibility

The producer MUST resolve source-facing HIR, types and lexical bindings, lower
semantic operations to an operation-granularity CFG, and derive the PDG from that
CFG **plus** binding, definition and operand-value facts. A syntax tree alone or
CFG successor edges alone are insufficient. MIR MAY corroborate facts but MUST
NOT replace source-facing operations with optimizer artifacts or erase source
operations. Optimization, constant folding, algebraic normalization and dead-code
elimination MUST NOT determine the extracted operation inventory.

R0 accepts compiler-valid ordinary synchronous safe function bodies with:

- Primitive `i8/i16/i32/i64/i128/isize`, `u8/u16/u32/u64/u128/usize`, `bool` and
  unit `()` parameters, local values and results. Types MUST resolve before
  monomorphization to these primitives; aliases MAY resolve to them, retaining
  their original spelling as evidence. Generic/operator-overloaded cases are not
  accepted by assuming a later primitive instantiation.
- Named, explicitly typed single-binding parameters and initialized `let x = E` /
  `let mut x = E` declarations (optional supported type annotations), local/parameter reads,
  lexical shadowing, direct local/parameter assignments and built-in compound
  assignments. Mutable parameters are bindings too.
- Integer/boolean/unit literals, grouping, ordinary blocks and tails, built-in
  scalar operations below, boolean `if`/`else` (including `else if`), explicit
  short-circuit `&&`/`||`, and explicit/early or implicit returns.

Only a single identifier, optionally `mut`, is an accepted binding pattern.
Receiver syntax (`self`, `&self`, typed receiver forms) is outside R0 even if a
receiver resolves to a primitive; diagnose `rust.scalar-interface`. Receiver roles
remain reserved in shared matching metadata for later registered extraction
policies. Function-level generic parameter declarations and non-Rust ABI/variadic
signatures are also outside this first profile, even when some body operations
would resolve to scalars. Names such as `_x` are ordinary named bindings. `_`, destructuring, `ref` binding,
unnamed/unit-destructured parameters, uninitialized declarations, `let else`,
`if let` and let chains are rejected in 0.1.0. Unit is accepted as a **type**,
not as a parameter destructuring pattern. A discarded expression statement
`E;` is accepted; `let _ = E` is not an unrestricted hole escape hatch.

Accepted built-in operators (subject to Rust's actual type rules) are:

| Operands | Operators |
| --- | --- |
| integers | unary signed `-`, unary `!`; binary `+ - * / % & | ^ << >>`; comparisons `== != < <= > >=` |
| boolean | unary `!`; eager `& | ^`; comparisons `== != < <= > >=`; lazy `&& ||` |

| integer/boolean direct mutable binding | applicable compound forms `+= -= *= /= %= &= |= ^= <<= >>=` |

Cast expressions are deferred in 0.1.0, including otherwise built-in `as` casts.
No operator may be admitted merely because its spelling appears in the table:
resolved operand types and built-in dispatch MUST be evidenced. Built-in integer
and boolean comparisons' implicit borrow semantics do not introduce reference
graph nodes, trait-call nodes or alias effects; explicit source borrows remain
unsupported. **Unit comparisons are deferred**: `() == ()`, `!=`, `<`, `<=`, `>`
and `>=` use standard-library `PartialEq`/`PartialOrd` implementations, not the
admitted built-in scalar comparison operations. Reject them with capability
`rust.scalar-computation`; no unit-comparison exception or call summary is
implemented in R0. Do not smuggle the underlying trait call into a `computation`
node or infer support from unit being an accepted interface/binding type.

### Strict source scan (including unreachable source)

Before eligibility or graph omission decisions, the producer MUST scan the entire
function signature and owned source body, including source after `return`,
constant-condition arms, and syntactically short-circuited RHSs. It MUST reject
unsupported constructs even if they cannot execute. Scan the pre-expansion source
and resolved HIR together: an unsupported macro invocation cannot disappear
because expansion resembles supported scalar code. Under a pinned `cfg`, disabled
source within the owned body is also scanned; `cfg` removal is not an eligibility
escape hatch. Conditional presence of the whole function determines which function
is being assessed and MUST be recorded.

R0 rejects all nested item declarations (including nested functions, constants,
types and `use` items). Report the nested item at its declaration span; do not
inline or traverse its separately owned body as outer-function operations. Likewise
reject a closure/async construct at its source site, excluding its separately owned
body from outer CFG extraction. This ownership rule avoids attributing nested
operations to the outer function, but does **not** make the containing function
eligible. Independently assessing a nested function is a separate assessment.

Arbitrary calls (including methods, intrinsics and apparently pure calls), explicit
references/raw pointers/dereferences, fields/indexes, statics/global accesses,
indirect or memory-backed writes, loops/`while`/`for`/`break`/`continue`, `match`,
macros (including generated functions), closures, async/await, unsafe functions or
blocks, const blocks, `?`, divergence and all unlisted types/constructs are
unsupported. An encountered unspecified evaluation order is rejected, not
arbitrarily ordered. Compilation or binding/type resolution failure yields
unknown evidence, never an invented supported graph.

Rejection MUST identify the source span, stable capability ID, construct and
remediation/deferred scope. Diagnostic partial graphs MAY be retained, but MUST
NOT be labeled conformant or offered to conformant matching. Unsupported source
MUST NOT be silently deleted and the remainder certified. Capabilities with absent
constructs may be `not_applicable`, not globally proven `supported`.

## 2. Evidence, identity and interchange

Every assessment MUST pin producer name/version/revision, exact rustc toolchain
(including commit), Rust edition, language-version evidence, target triple and
target data layout/pointer width, active `cfg` flags and Cargo features, and
settings affecting semantics (`overflow-checks`, `debug-assertions`, panic strategy,
optimization and relevant frontend options). `isize`/`usize` MUST NOT be treated as
fixed-width without target evidence. Missing required settings block conformance.
Fixture configuration describes a test input, not a toolchain run.

Spans MUST be half-open zero-based UTF-8 **byte** offsets into exact source bytes,
not characters, lines, normalized text or JSON escapes. Source `content_identity`
MUST be a known `sha256:<lowercase hex digest>` of those bytes for conformance.
CFG may embed `content`; PDG 0.1.0 does not permit that field in source records.
Source paths, digests, function/HIR IDs and spans are navigation/trace evidence,
not clone labels. Unknown facts use `{status, reason}`, never empty placeholders.

A deterministic scope key SHOULD be
`scope:<source-id>:<kind>:<start-byte>:<end-byte>` for the function or lexical block.
A binding key MUST include the declaring identifier token span and owning scope;
spelling alone is never identity. A declaration's initializer resolves in the
previous environment; the new binding begins after initialization. Thus
`let x = x;` can read an outer `x` and define a different inner `x`. Successive
same-spelling bindings in one scope still have distinct token keys. Declared source
names MUST be preserved in `symbols[].name` but excluded from base node matching.
Each value definition has a fresh graph-local ID independent of its symbol ID.

Use `profile.name = "compat.rust.hir-r0"` in CFG 0.2.0 and
`profile = {"id":"compat.rust.hir-r0","version":"0.1.0"}` in PDG 0.1.0.
All R0 node/edge capability policies and provenance policies MUST reference that
same ID/version; subkind names do not imply differently versioned policies.
CFG profile fields MUST be:

```json
{
  "name": "compat.rust.hir-r0", "version": "0.1.0",
  "granularity": "operation",
  "evaluation_order": {
    "sequencing": "language_defined", "short_circuit": "explicit_branches",
    "unspecified_order": "reject"
  },
  "exceptional_flow": "omitted",
  "exit_policy": "one normal exit; source returns only; no fabricated divergent fallthrough",
  "unreachable_code": "preserve"
}
```

PDG extension keys MUST satisfy its namespace regex
`^[a-zA-Z][a-zA-Z0-9_.-]*:[a-zA-Z0-9_.-]+$`. Use keys such as
`compat.rust.hir-r0:operation-map`, **not** bare `compat.rust.hir-r0`.
CFG top-level `extensions` accepts the same keys by this profile's convention.
No new fields may be inserted into closed schema objects.

Required correspondence in both graph documents' top-level
`extensions["compat.rust.hir-r0:operation-map"]` is an array of
`{cfg_node_id, cfg_operation_id, pdg_node_id}`. Each reachable or unreachable
source semantic operation has exactly one triple, one non-boundary CFG vertex,
one CFG operation, and one corresponding PDG operation node. No two distinct
operations share a CFG vertex in R0. Both documents identify the pair using
`extensions["compat.rust.hir-r0:paired-graph"]` with exactly
`{cfg_document_key, pdg_document_key}`. The external run/fixture manifest maps those
unique keys to known SHA-256 identities of the exact serialized documents. Do not
embed mutually dependent whole-document hashes in the two documents: that would
create a circular content-identity calculation. Validate cross-document references
against the externally identified pair.
Bindings/definitions/operand facts MUST also be available to the PDG derivation;
the CFG schema alone does not encode them.

CFG `kind` is `condition` for predicates, `transfer` for returns, and `block` for
all other semantic operations; synthetic `entry`/`exit` have no operations.
PDG boundaries, if retained, are synthetic `entry`/`exit` with analysis provenance,
no definitions, and no value/data dependencies. One synthetic normal exit is used.
Every actual return has a CFG `return` edge to it. There is no exceptional exit,
virtual executable exit, or implicit edge past an explicit return.

## 3. Precise operation and value lowering

Lower subexpressions recursively according to Rust evaluation rules, preserving
all source operations, including unused reads/computations and literal occurrences.
Statements sequence in block order; grouping and block wrappers are not runtime
computations. Declaration syntax wrappers are not additional runtime computations:
the binding initialization itself is the single `declaration` operation.
An expression result is a reference to its evaluated value producer, not its
variable spelling or a request to read a variable later. It survives later writes.

### Operations and schema attributes

| Source semantics | PDG category and required facts | Values / definitions |
| --- | --- | --- |
| named parameter | `formal_input`; `position`, `symbol_ids`, `defines` | One initial definition for that binding; not a CFG operation |
| literal (`1`, `true`, explicit `()`) | `computation`; known `literal`, resolved type extension | One value, no binding definition |
| binding read occurrence | `read`; `reads:[{symbol_id,operand_position:0}]` | One snapshot value; data from each reaching definition |
| initialized `let x = E` | `declaration`; `symbol_ids`, `defines`, `writes`, `write_form:"initialization"` | New symbol and fresh definition; nonunit initializer value input |
| `x = E` | `assignment`; `symbol_ids`, `defines`, `writes`, `write_form:"reassignment"`, known operator `=` | Fresh definition of existing symbol; target is **not a read** |
| scalar operator | `computation`; known `operator`, resolved operand/result types | Value inputs at semantic operand positions |
| branching decision | `control`; `construct:"if"/"&&"/"||"`, `outcomes:["true","false"]` | Consumes already evaluated condition value at position 0 |
| compound update | decomposed `read`, `computation`, `assignment`; final `write_form:"compound_assignment"`, known original compound operator | Read current target after RHS, combine snapshots, fresh definition |
| explicit or implicit return | `return`; `exit_kind:"normal"`, return-form extension | Nonunit value input at position 0; unit return has **zero data inputs** |
| alternative expression results | analysis-synthetic `merge` | Alternative value inputs; no binding definition, no CFG operation |

`attributes.defines` MUST identify exact entries in `definitions` with this node's
`node_id` and its binding `symbol_id`. Only formal inputs, initialized declarations
and assignments define bindings. Reads do not define bindings; computations,
predicates, returns and expression merges do not invent binding definitions.
`memory_locations` is empty for conformant R0. Do not encode scalar binding writes
as unknown abstract memory.

Original frontend kinds MUST be retained as known `original_kind` facts on source
PDG nodes and as strings on CFG operations. Category normalization MUST NOT overwrite
HIR kinds. Every decomposed source operation MUST carry the actual original source
span and provenance (`source` or `desugaring` with originating spans and explanation).
Compound target read uses the target token span; its computation/assignment may
both use the full compound expression span. A predicate may overlap the condition
read/computation span. Overlapping spans are legal and do not imply duplicate
operations. Trace the common source construct and each role using
`attributes.extensions["compat.rust.hir-r0:source-operation"]` with
`{construct_span, role}`. Every semantic/value node carries
`annotations["compat.rust.hir-r0:type"]` as a known string fact containing its
normalized resolved scalar value type (the matching vocabulary, with `unit` for
`()`); administrative boundaries do not require a value type. Reads/computations/
formal inputs/merges use their value type, predicates use `bool`, declarations and
assignments use expression-result `unit`, and return nodes use the function-result
payload type, not the Rust diverging expression type `!`. Written binding types
are separately recorded on symbols. Neither display text nor provenance descriptions
are matching labels.
CFG node `origin` MUST explain source extraction/decomposition. Because CFG
operations have no provenance field, their structured provenance MUST be stored
in top-level `extensions["compat.rust.hir-r0:operation-provenance"]`, an array of
`{cfg_operation_id, provenance, construct_span, role}`. The `provenance` object
uses the PDG provenance contract and agrees with the mapped PDG operation; do not
add forbidden fields to the CFG operation object.

Analysis-introduced nodes have `synthetic:true`, analysis provenance, and an
explicit known profile-defined `original_kind` such as
`compat.rust.hir-r0.expression_merge`, rather than a fabricated HIR kind. Formal
inputs are synthetic nonexecuting value-interface entities anchored to their
parameter token spans, retaining the actual frontend parameter kind. Implicit
returns are source-backed lowered operations (`synthetic:false`, desugaring
provenance), anchored to the tail expression, or the actual closing-brace token
for a body with no tail. They do not require an invented synthetic CFG operation.
An `other` node requires `attributes.description` under the schema and a named,
justified meaning; it MUST NOT be used to smuggle unsupported source into R0.

### Evaluation order and snapshots

The [Rust Reference operand-order rule](https://doc.rust-lang.org/reference/expressions.html#evaluation-order-of-operands)
requires unary operands before their operator and ordinary binary/comparison
operands left-to-right before applying the operator. Distinguish evaluation order
from operand positions: binary left is position 0 and right is position 1.
Emit a read node per source occurrence, even for `x + x`; no read sharing.
For `x + { x = 2; x }`, the left read is a snapshot of the old definition and
feeds the final computation even though the right block reassigns `x`.

[Basic assignment](https://doc.rust-lang.org/reference/expressions/operator-expr.html#basic-assignments)
evaluates RHS first, then the target place and write. For a direct binding target
there is no runtime place operation or target read: `x = x + 1` has the RHS read,
literal and computation, then the assignment only.

[Primitive compound assignment](https://doc.rust-lang.org/reference/expressions/operator-expr.html#compound-assignment-expressions)
evaluates RHS first when both operand types are known primitive before
monomorphization. R0 MUST lower `x += RHS` as:

1. Evaluate **all of RHS** to its snapshot result, including its writes/branches.
2. Read `x`'s then-current value (a separate source-backed target read).
3. Compute `+` with target read at **operand 0**, saved RHS at **operand 1**.
4. Assign the result to `x`, creating a fresh definition; preserve `+=` on the
   assignment and compound-source provenance on every decomposed operation.

This is **not** ordinary binary evaluation order. For
`x += { x = 2; x }`, both the target read and RHS read reach the inner assignment,
not the pre-RHS declaration. Overloaded/generic compound operations, whose order
can differ, are rejected rather than importing this rule.

### Blocks, unit, returns and unreachable code

A block result is its evaluated tail producer; no block-value copying operation
is added. A semicolon discards a result without deleting its producing operations.
An empty block `{}` or a no-tail block has known unit result but is not a literal
computation or runtime wrapper. Explicit source `()` has one `computation` node
with known literal `()`. **Omit ALL unit-typed `value_operand` and
`value_alternative` edges**, whether the unit value is explicit, implicit, read
from a binding or produced by an effectful expression. This rule also applies to
unreachable diagnostic value edges. The edge's type is the transferred expression
value, not the consumer's result type: a nonunit RHS still feeds an assignment
whose expression result is unit. Retain unit literal, read, declaration and
assignment operations, their CFG/control/execution relationships and fresh binding
definitions. Unit reads MUST retain their `reaching_definition` data edges from
unit formal-input/declaration/assignment definitions; those are binding dependencies,
not omitted value operands. **No unit merge nodes are emitted.** A unit return
always has zero data inputs, even with `return ();`, a unit read tail or effectful
unit expression. Those source operands still execute first and retain their own
operations. No unit materializer is added merely for implicit function fallthrough.

Nonunit `return E` evaluates `E`, then emits a distinct return consuming its
producer. A nonunit tail similarly feeds a distinct implicit return. An early
return prevents subsequent source from executing. If all branches return, do not
append a fabricated implicit return. Each reachable normal path ends at an actual
return and the one normal exit.

Preserve entry-unreachable source operations and their original spans. Lower a
maximal unreachable source region with its internal sequencing/branches and
return-to-exit edges, but no fabricated incoming edge from a terminating path.
Mark reachability in `extensions["compat.rust.hir-r0:reachability"]` as CFG node-ID
lists `reachable` and `unreachable`, derived from entry, not constant folding.
Dependent PDG operations inherit that state. Reaching-definition, dominance and
postdominance edges are certified only for entry-reachable operations; do not
invent reachable definitions for an isolated suffix. Internal operand-value edges
may be preserved diagnostically on unreachable operations, explicitly outside the
certified reachable slice. No such edges affect matching or reference counts.

## 4. Expression-valued branches without synthetic CFG operations

Evaluate the condition/left boolean **once** to a value producer. A separate
predicate operation consumes that snapshot and branches. `if` evaluates only its
selected arm. `&&` evaluates RHS only on `true`; `||` only on `false`. The shortcut
result is the **already evaluated left boolean**, not a new constant or reread.
The other result is the evaluated RHS boolean, not the predicate node's output.

For a nonunit expression with multiple normally completing alternative producers,
emit one analysis-synthetic PDG `merge`, even if its result is subsequently
discarded. Returning arms supply no continuation alternative. With just one
normally completing producer, forward it directly; with none, no continuation
value exists. Unit alternatives MUST NOT create a value merge or value edges;
their source/effect operations and branches are still explicit. A nested nonunit
merge can be an outer merge's alternative producer.
No merge creates a binding/definition or SSA phi for variable reaching-definition
sets. Assignment into `x` in either arm is handled by sets of real definitions.

**CFG 0.2.0 has no synthetic-operation representation.** Never add a fake merge,
formal input, no-op join or copy to its operation array. CFG branches reconnect
directly at the first real successor semantic operation (possibly a later read,
computation, declaration, write, predicate or return). That operation executes
only when the continuation is taken: association with a merge does not hoist it
into both arms or turn an effectful successor into unconditional execution.

Record every merge in both documents' top-level
`extensions["compat.rust.hir-r0:value-sites"]`:

```text
{ pdg_node_id, construct_span, controller_pdg_node_id,
  cfg_successor_node_id,
  annotations: { "compat.rust.hir-r0:guards": DNF },
  alternatives: [
    { value_pdg_node_id, outcome, completion_cfg_node_ids: [cfg_node_id, ...],
      annotations: { "compat.rust.hir-r0:guards": DNF } }
  ] }
```

Here `cfg_successor_node_id` is the first real continuation operation, not a merge
vertex. Each `completion_cfg_node_ids` is the **exact nonempty frontier** of real
CFG operations completing that alternative immediately before continuation. It
is a set serialized as a duplicate-free array, not a single representative or a
list of merge nodes. A nested merge value can complete at several real operations.
Every frontier member MUST have an immediate CFG edge to the successor on a path
satisfying that alternative's guard. Every normally completing path satisfying
the guard MUST finish at one frontier member; no returning arm or unrelated path
may contribute a member. On a short-circuit shortcut the frontier is the predicate.
Outcomes are `true`/`false` at the recorded controller.

### Guard shape and availability validation

`compat.rust.hir-r0:guards` has exactly this DNF shape, wherever used:

```text
[
  [ { controller_pdg_node_id: <control-node-ID>, outcome: "true" | "false" }, ... ],
  ...
]
```

The outer array is disjunction (OR), each inner array is conjunction (AND).
`[]` means impossible availability; `[[]]` means unconditionally available.
Each atom has exactly the two shown fields and refers to a real predicate
operation and one of its declared boolean outcomes. An atom is satisfied only
if that predicate was evaluated and took that outcome on the path; an unevaluated
inner condition is not assigned a truth value. Do not duplicate atoms or terms,
or encode contradictory outcomes for the same controller in one conjunction.
Equivalent simplified DNFs are permitted, judged on structural normal CFG paths,
not on constant-folded input values. Missing guard metadata is not `[[]]`.

Every value-site record MUST include the merge's availability DNF in `annotations`;
it MUST agree with the PDG merge node's `annotations["compat.rust.hir-r0:guards"]`.
Every alternative MUST include its selection/availability DNF in `annotations`;
it MUST agree with the corresponding `value_alternative` edge's annotation.
Validate both against normal CFG paths reaching the recorded successor: the merge
is available exactly on paths that normally completed its source expression;
each alternative is available exactly on the subset selecting its value. Each
alternative guard MUST imply the merge guard and its recorded controller outcome.
The alternatives MUST cover the merge's available paths and be mutually exclusive
there. On each such path the selected producer's snapshot MUST have been evaluated
(or an input value be available); for a merge producer, its own availability guard
must hold and its recursively selected producer must be available. Validate this
value dependency DAG without creating executable merge vertices.

Nested merges MAY share a successor, but sharing a site does not imply sharing
availability. For `if a { if b { 1 } else { 2 } } else { 3 }`, both merges attach
to the actual return. The outer merge is available under `[[]]`; the inner merge
is available only under `[[{controller_pdg_node_id:<a-predicate>,outcome:"true"}]]`.
The outer true alternative's frontier contains both literal-1 and literal-2 CFG
operations; literal-3 belongs only to the outer false alternative. Inner literal
alternatives additionally require their own `b` outcome. Thus an inner merge
cannot supply a value on the outer false path even though the return executes.

An association is metadata, **not** execution, control or data flow by itself.
Values are available at the site under the recorded guards, and may be consumed
later after other operations. Their snapshots survive intervening writes.
Data edges run alternative producer **to merge**, with subkind
`value_alternative`, `outcome`, deterministic position (true = 0, false = 1), and
the full DNF guard annotation; merge **to consumer** uses `value_operand` and the
consumer's semantic operand position. Emit exactly one alternative edge for each
semantic `(producer, merge, position, outcome)` alternative: multiple completion
frontier members, CFG paths or DNF terms MUST NOT duplicate that edge. Aggregate
those paths into its one frontier and DNF. Distinct semantic alternatives retain
their positions/outcomes. No unit value edges or unit merges are permitted.
Predicates do not directly supply expression values. Merges and formal inputs
have **no incoming or outgoing execution edges**, are not CFG vertices, and are
not targets of the operation-only control analysis. Retain meaningful nonunit
merges in matching when their guarded availability intersects reachable normal
paths. Retain all structurally reachable alternatives; do not prune by constants.

## 5. Binding data, control and execution dependence

All dependence edges MUST carry ID, family `kind`, specified `subkind`, R0 policy,
capability evidence and analysis provenance, with optional annotations only in
schema extension locations. PDG is a directed **multigraph**: two families on the
same endpoints remain separate; repeated operands and alternatives preserve their
edge identities. Directions are producer → consumer, controller → controlled,
and prerequisite → constrained operation, respectively. Operand positions and
outcomes are evidence, not permission to reverse these directions.

### Data

Initialize parameter reaching sets with formal-input definitions at CFG entry,
without executing the formal inputs. On the reachable DAG, for each symbol:
`IN(n) = union(OUT(p) for reachable predecessors p)`; an initialized declaration
or assignment kills prior definitions of its own symbol and generates its fresh
definition. Other operations preserve the set. Scope exit does not conflate inner
and outer symbols. Each read gets one `data/reaching_definition` edge from each
real defining node in its exact `IN` set, with `symbol_id`, `definition_id`, read
operand position 0 and `loop_carried:false`. No phi node is added at these joins.
Missing/unresolved definitions prevent conformance. No anti/output dependencies
are emitted in R0; local updates are represented by real reads/writes and execution
prerequisites, not invented data dependencies from a killed definition.

Expression producer-to-consumer edges use `data/value_operand`, with zero-based
semantic `operand_position`; they do not carry a fake binding `definition_id`.
Reads themselves consume reaching definitions and then supply snapshot values.
For unit snapshots, retain the binding `reaching_definition` edges but omit all
`value_operand`/`value_alternative` edges, including from explicit unit producers;
no unit merges are permitted. Literal, computation and merge values need no
symbol-table entries. R0 data
analyses are exact for this normal-flow path abstraction, not a promise to exclude
all infeasible paths by value analysis.

### Control: one-exit postdominance

Analyze only the entry-reachable normal-flow CFG, including its one normal exit.
Every such vertex MUST reach that exit and the subgraph MUST be acyclic. Define
`a postdominates b` iff every path from `b` to the exit contains `a`, with reflexive
membership. Compute immediate postdominators there, not from syntax nesting.
For each predicate edge `p -> s` whose successor does not postdominate `p`, walk
from `s` up its immediate-postdominator chain, stopping **before** `ipdom(p)`.
For each visited semantic operation `n`, emit `control/postdominance` from `p` to
`n`, retaining the edge's `true`/`false` outcome. Boundary nodes and nonexecuting
entities are excluded; no entry-control scaffolding edges are emitted. Preserve
distinct outcomes; do not emit the transitive closure of control dependencies.

Consequently a common join is not controlled merely because it follows an `if`.
An early-return predicate can control all later operations on the continuing arm,
even outside its syntactic block. Control and execution on the same endpoints
remain different edges.

### Execution: nearest dominating semantic operation

In the same reachable CFG, `a dominates b` iff every entry-to-`b` path contains
`a`. For each **semantic operation** `n`, choose its closest strict dominator
that is also a semantic operation, equivalently walk the immediate-dominator
chain past nonoperation boundaries. If it exists, emit exactly one
`execution/nearest_dominating_operation` edge `d -> n`. If only entry dominates
it, emit none. Do not introduce entry, formal-input or merge execution edges;
do not fake parameter order. Do not copy possible CFG successors, emit the full
dominance closure, or separately sequence alternative arms into a join.

In operation granularity, a straight-line chain gets adjacent execution edges;
a two-arm join usually gets its prerequisite from the predicate, not either arm's
last operation. A later operation with an early-return bypass can still have the
predicate as nearest dominator. These edges express prerequisites conditional on
the target being executed, not a claim that the target executes on all paths.
Cycles/divergence require a later policy; never fabricate fallthrough to satisfy
these analyses.

## 6. Normal-flow limitations and capability certification

Integer operator spelling, widths, signedness, literal suffixes and dispatch MUST
remain source-faithful. Do not substitute mathematical unbounded arithmetic,
canonicalize commutativity, or label accepted operations/functions pure. Integer
arithmetic may panic: division/remainder by zero and signed minimum divided by
`-1`, enabled overflow checks, signed negation and invalid shifts require explicit
assessment. The Reference's literal-negation exception MUST NOT be forgotten.
No rewrite-safety, totality, panic-freedom or complete Rust execution claim follows
from R0. Normal-flow paths omit panic transfers and cleanup.

If omitted panic/exception flow affects a function (possible under its evidenced
types/settings, unless independently proved impossible), CFG
`extraction_evidence.status` MUST be `partial` with specific nonempty limitations.
It MUST NOT be `complete` simply because normal-flow construction succeeded.
An assessment may nevertheless be `conformant` to this **bounded normal-flow**
profile once its fixture-backed requirements pass, with the limitation also in
PDG diagnostics and exceptional-flow capability description. Unsupported source
or missing required normal-flow facts cannot use this exception to become eligible.
For a function with no relevant omitted exceptional flow, complete CFG evidence
may be claimed only after semantic validation. Unknown implementation evidence
remains `unknown` / `not_evaluated`, regardless of schema validity.

PDG capability statuses are exactly `supported`, `approximated`, `unsupported`,
`unknown`, `not_applicable`; planning states such as `unimplemented` and fixture
`not_run` are **not** schema capability values. Supply every required schema key:

| Schema capability | R0 interpretation after validation, not current support |
| --- | --- |
| `node_extraction`, `scalar_reaching_definitions`, `control_dependencies`, `execution_dependencies`, `returns`, `parameters` | `supported` only for the evidenced applicable R0 scope |
| `branch_merges` | `supported` for required expression values; not SSA joins; `not_applicable` when absent |
| `loop_carried_dependencies`, `calls`, `closures`, `macros`, `memory_aliases`, `concurrency` | `not_applicable` if absent; `unsupported` if encountered, blocking conformance |
| `exceptional_flow` | `unsupported` with specific omissions when relevant; `not_applicable` only with evidence of no applicable exceptional transfers |
| `reference_counting` | `supported` only after verified distinct-binding counting; otherwise `unknown` |

Additional capability keys SHOULD use the stable IDs from the backlog:
`rust.scalar-interface`, `rust.local-bindings`, `rust.local-writes`,
`rust.scalar-computation`, `rust.blocks`, `rust.branching`, `rust.short-circuit`,
`rust.returns`, `rust.control-dependence`, `rust.execution-dependence`.
Deferred diagnostics use its `rust.call-summaries`, `rust.abstract-memory`,
`rust.loops`, `rust.match`, `rust.for-iteration`, `rust.closures`, `rust.macros`,
`rust.async`, `rust.unsafe`, `rust.unwinding-cleanup`, `rust.unspecified-order`,
`rust.additional-types` IDs as applicable. Required schema keys and these planning
IDs are different namespaces of evidence; neither substitutes for the other.

Conformance records use `{profile:{id,version},status,reasons}`; schema statuses
are `conformant`, `non_conformant`, `not_evaluated`. Initially use `not_evaluated`
with a reason identifying unavailable fixture-backed producer evidence. An
encountered rejection gets `non_conformant` and nonempty reasons. Required positive,
negative, evaluation-order, scope/span and boundary fixtures must pass before
`conformant`/`supported` claims advance. The initial fixture pack is not exhaustive
and does not independently certify all capabilities.

## 7. Interface and matching handoff

`function.inputs` records every named parameter in source order, its parameter
role, binding ID, name/type facts and known `variadic:false`. Preserve original type
spelling in schema type facts. Every input/output interface element and symbol has
`extensions["compat.rust.hir-r0:resolved-type"]`, a known string fact containing the
normalized resolved primitive token (`i32`, `bool`, `unit`, etc.). Resolve it from
compiler evidence, not string substitution; retain alias spelling separately.
The matching adapter validates these extension facts and supplies the normalized
interface types. Unknown resolved type blocks conformance. Empty inputs mean
known no parameters only with complete interface evidence. Retain one meaningful
formal-input value entity and its initial definition for **each** parameter,
including unused and unit parameters; these entities have no CFG execution slots.
`function.outputs` has exactly one position-0 `return` metadata element with known
original type evidence plus resolved primitive type, including unit,
`variadic:false`, and name `not_applicable` with a
reason. **R0 emits no `formal_output` graph nodes.** Actual return nodes carry
nonunit output values; output metadata does not add a second runtime operation.

A verified reference-count fact counts distinct parameter/local **symbol IDs**
read by entry-reachable retained read operations, including decomposed compound
reads; repeated reads count once and shadowed bindings separately. Write-only
bindings, parameter metadata and implicit borrow syntax add no read occurrence.
Attach policy evidence identifying this R0 rule. Unknown is not zero.

The separate matching projection MUST preserve meaningful value/operation nodes,
including formal inputs and required merges, and preserve edge direction, family
and multiplicity. It may remove `entry`/`exit` only when positively identified as
administrative boundaries with no value definitions/semantics, dropping incident
edges without reconnecting neighbors. Synthetic origin alone is not a removal
rule. Exclude entry-unreachable operations and their dependent nonexecuting entities;
retain formal inputs as interface values, not by testing nonexistent CFG reachability.

Category normalization and matching labels are separate:

| PDG category | CCGraph base label |
| --- | --- |
| `declaration` | `DECL` |
| `assignment` | `ASSIGN` |
| `control` | `CTRL` |
| `call` | `STATE` (imported matching graphs; source calls unsupported in R0) |
| `read`, `computation`, `formal_input`, `return`, `formal_output`, `merge`, `other` | `OTHER` |

R0 does not emit `memory_summary`; it requires a later extraction/projection
policy. A hypothetical `formal_output` mapping does not authorize R0 to create
one. Original HIR kind, names, source navigation, operator/literal, operand positions
and branch outcomes remain extraction evidence, not base node/edge labels. The
base edge label is its distinct `data`/`control`/`execution` family. No dependence
family may be erased when labels coincide.

## 8. Independent fixture expectations

[fixtures/rust-r0-extraction.json](fixtures/rust-r0-extraction.json) uses its own
`compat.rust.hir-r0.fixture-expectations` format version `0.1.0`; it is **not** a
CFG/PDG schema-conformant document and does not claim compiler extraction. Aliases,
compact tuples and token selectors describe expected facts for a future harness.
All observations are `not_run`; acceptance is an expectation, not a result.

The pack supplies exact CFG transfers, local definition/read sets, operand data,
nearest-dominator execution and postdominance control for straight-line,
reassignment/two-arm branch, short-circuit AND/OR, expression-valued if and early
return examples. It also distinguishes effectful join sites, old-value snapshots,
RHS-first compound updates, shadowing, nested-merge completion frontiers and DNF
availability guards, unit bindings without unit value operands, empty unit
fallthrough, rejected unit comparisons and unreachable calls/operations. These are independently hand-authored oracles, not snapshots
derived from the analyzer they are intended to check. A future validator MUST
check exact sets/multisets and reject extra edges, not just find expected subsets.
Negative fixtures expect visible ineligibility and diagnostics, not successful
remainder extraction. Further fixtures from the capability backlog are required
before any complete semantic-support claim.
