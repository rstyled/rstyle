# Runtime bounds

`--complexity` adds a search for runtime growth to [pattern mining](patterns.md):

```sh
cargo rstyle patterns --complexity --top 20
cargo rstyle patterns --complexity --replay-run RUN --format json
```

```text
src/lib.rs: scan: loop body entries <= len(xs)#0 + len(xs)#0 * len(ys)#1; whole runtime unknown: loop and call counts do not bound operation costs
  src/lib.rs:19:5: loop body entries <= len(xs)#0
  src/lib.rs:20:9: loop body entries <= len(xs)#0 * len(ys)#1
```

It reports upper bounds on loop-body entries and modeled call-site invocations, with input measures and
loop locations. For example, restarting an inner scan can give `n + n * m` body entries, and a
persistent cursor can give `n + m`.

These counts don't establish a tight runtime bound or the cost of a called function. Whole-function
runtime stays explicitly unknown.

Rows rank by polynomial degree, then logarithmic factors. That's a search heuristic, not a comparison
of runtimes across independent input sizes. `--top` selects rows independently of abstraction scores,
and JSON keeps the full roster and the selected row indices.

## Coverage in document reports

`rstyle.cpg-runtime-bounds-report/1` rows include `identity`, `compilation`, `function`, `root`,
`line`, `column` and `test`. Match across captures using compilation and stable identity; function and
root IDs distinguish occurrences within a capture. Names and paths alone are not unique. Duplicate
compilation/identity keys remain distinct rows and cannot supply an unambiguous composed summary.

The additive `coverage` object always counts the full input before document-report `--top` selection.
Its maps omit zero counts:

- `roster` separates authored functions from non-function bodies, generated code and unavailable records.
  Bounds rows and the other coverage maps cover authored functions only.
- `capture`, `local` and `composed` keep unavailable capture and unknown, partial, budget-exceeded or
  absent results separate from proven results.
- `proven_local_loops` separates owners without modeled loops, constant loop work and input-dependent
  loop work. `proven_composed_loops` separates zero, constant and input-dependent loop work through callees.
  A zero composed bound does not say whether the owner contains a modeled loop.
- `library_steps` counts complete versus opaque library bounds independently of the composed verdict.
  A complete library bound can be zero when there are no library calls; it does not prove whole runtime.

`input_complete` requires captured bounds for every authored function and no unavailable roster records,
including those outside `--top`. It describes capture availability, not proof success: captured programs
may still yield unknown or partial results. These counts add no proofs, do not count every latent blocker,
and describe only the supplied input roster, not functions absent from the capture.

The compiler report (`rstyle.compiler-runtime-bounds-report/2`) includes the same stable context and
coverage maps. Its `coverage_scope` explicitly limits the denominator to `assessed_functions_in_unit`
and the requested `authored` or `captured` source population. Owners absent from that handoff are not
counted. The compiler keeps every scoped row and uses `selected` indices for `--top`; selection does not
change coverage. Generated empty functions can add zero-loop proofs, without adding a scan proof.

## In the check run

When bounds run alongside `check`, each function with a proven bound of degree two or more is a
`algorithmic_complexity` result in `check.sarif` and `report.sarif`, at its outermost loop. Linear bounds and
partial bounds report nothing.

A bound that multiplies one input by itself, as nested scans of one collection do, is an error. One that
multiplies different inputs is a warning, since that's often the work itself. `rstyle.toml` can lower the
error to a warning:

```toml
[algorithmic_complexity]
severity_level = "warning"   # "error" by default
```

To silence one function, put an `rstyle-allow` above it. Its bound stays in the JSON report:

```rust
// rstyle-allow(algorithmic_complexity): at most a dozen items
```

## What's modeled

**Counters and traversals.** The model supports:

- primitive increasing counters
- unsigned exclusive ranges
- traversal of shared slices and standard `Vec` values, including fields reached through shared
  references
- a range a library call makes, by a contract's `[[range]]` role, as Python's `range(n)` counts from
  zero up to `n`, and a call that traverses one of its arguments, by a `[[traversal]]` role, as
  `enumerate(xs)` counts what `xs` counts
- a tuple, a sequence whose type has one fixed length, and an integer, which a `for` counts from zero up
  to, as Go's `for i := range n` does

A source a loop counts this way is evaluated once before its first pass. A signed count, such as a Python
`range`'s, bounds only from a constant start of at least zero, so `range(a, b)` and a range that counts
down stay opaque. A loop's source is the call it's written with, as `for x in enumerate(xs)` is. Only a
compiler-generated `for` wrapper is looked through, so a call no role names is never counted by the
length of one of its arguments. Python's `sorted` has no role, since it compares the items, which can
run their own methods. Go writes no length for an array type, so a Go array isn't counted.

**Loops without a guard.** A loop with no guard of its own, as Rust's `loop`, Go's `for {}` and C++'s
`for (;;)` are, may run any number of passes and leaves through a `break`, a `return` or what it raises.
Its own entries are an opaque `?loop` factor, and the rest of the function keeps its bound. A language's
contract file can say a loop's guard is the last statement of its block, as rstyle-cxx writes a `do`
statement. That guard runs after each pass, and `continue` goes to it. A guard that is false wherever it
runs, as a macro's `do { .. } while (0)` is, runs the body once, so the statement isn't a loop. "False" here
means a false or zero literal, `&&` with such an operand, and an operator the contracts give
the `Sequence` role, such as C++'s comma, whose last operand is one.

**Signed values.** A signed counter needs a `<` guard, a unit increment and a constant start of at
least zero, and any other start makes that loop opaque. A signed input counts as its positive part, so
`while i < n` over an `i64` gives `max(n, 0)`, and that floor survives composition with a caller's
argument. A conversion between signed and unsigned stays unknown.

**Short-circuit guards.** A pure leading numeric guard in `while i < limit && predicate` can bound
the later predicate and body. The predicate retains its calls and short-circuit evaluation, including
a final false result. Counter progress, invariant limits and no-overflow checks still apply. Arbitrary
Boolean guards and loops whose branches advance different counters remain conservative.

**Unbounded integers.** A language whose integers have no width, as Python's don't, says so with
`integer_overflow: unbounded` and leaves `bits` out of its integer types. They count as signed values, read
at 64 bits. What's proved of a counter under its guard holds at any width, and a constant too large for 64
bits stays unknown, so that can only lose precision.

A function declared inside another, as Python's nested `def` is, has its own bound, and what it can write counts
against the function declaring it: an exclusive capture of a counter or an extent leaves that function unknown,
as a closure's does.

**`for` patterns.** A pattern may declare any new bindings, since they can't change how often the loop
runs. A pattern holding anything else is unsupported, as is a declaration that writes what a loop's
length is measured from.

**Refutable declarations.** A declaration with a failure branch, such as Rust's `let-else`, evaluates
its initializer once, then chooses between binding the pattern and running the failure branch.
Both paths remain possible: `continue`, `return`, counter writes and calls keep their control and
cost effects. A failure path that can bypass a counter increment does not establish loop progress.
Pattern guards, writable reference patterns and hidden cleanup keep their existing restrictions.

**Primitive scratch writes.** A direct write to a local or by-value parameter of primitive Boolean,
character, floating-point or unit type can preserve bounds when the store affects no modeled scalar
or selected extent. The right-hand side still runs, including its calls, counter writes and cleanup.
Fields, indexes, references, aggregates, adjusted places, unknown types and overloaded assignments
retain conservative handling. A Boolean assignment does not prove progress of a Boolean-controlled loop.

**Fresh output storage.** A resolved builtin allocation with an explicit storage contract can certify
a fresh native sequence with builtin element writes. An ordinary indexed assignment can then preserve
a scan bound when the sequence is held in an immutable local, its value shape is complete, and its
base and index are effect-free. The allocation and right-hand side still run. Unknown setters,
overloaded assignments, indirect bases, effectful indexes and possible cleanup retain conservative
handling. This certifies unrelated storage and does not prove library operation costs.

**Exact allocation lengths.** An allocation contract may identify the argument that supplies the
result's exact length. For a certified fresh native sequence in an immutable direct local, bounds
can use a nonnegative literal length or the selected length of an immutable direct formal sequence.
Both headers must pass the existing whole-body privacy checks: rebinding, borrows, captures,
escaping places, and incomplete facts prevent the relation. This first increment does not follow
local chains or changing scalar length variables. The relation affects lengths only; it does not
make allocated elements aliases of the formal's elements. Allocation, length, copy, sort, callback,
and right-hand-side evaluation remain in the captured program.

**Arguments.** A call evaluates its receiver or callee, then what each argument passes. An argument that
holds its value under a name, as Python's `print(total, end="")` does, passes that value. One that unpacks
a sequence or a mapping, as `*items` and `**options` do, runs an unresolved call of its own, since
iterating or unpacking its operand can reach a method the operand's type defines.

**Language extensions.** A node a language registers as its own, such as Python's `py:dict` or Go's
`go:slice`, is lowered by the role its contract file gives it. It evaluates its children once, in the order
the profile writes them, and a pattern among them is a binding it writes. When the role says the node also
runs an operation the document names no callee for, as formatting a value reaches its type's
`__format__`, that operation is an unresolved call at the node: the function's own bound holds, and its
composed result carries an opaque factor for the call. An extension without a role stays unsupported.

| Language | Evaluates only its children | Then runs an unresolved call |
| --- | --- | --- |
| Python | `py:fstring`, `py:slice`, a keyword argument with a name | `py:format`, `py:dict`, `py:star`, `py:await`, `py:import`, `py:comprehension`, a `**` keyword argument |
| Go | `go:slice` | |

A comprehension runs the `for` loops it desugars to where it's written, so they're bounded as any `for` is,
and then builds its result. A node whose role names a `lazy` modifier, as a generator expression has
`py:lazy`, runs where what it makes is consumed. It's lowered in place only as an argument of the call
that consumes it, directly or through a keyword or a starred argument, and one kept for later stays
unsupported, since it could run against values that changed.

**Exceptions.** A role can also say how a node raises an exception or handles one. Python's `raise` and
`assert` and C++'s `throw` raise. Python's `try` and C++'s `try` handle what their body raises, and so does
Python's `with`, whose context's `__enter__` and `__exit__` are unresolved calls at the node.

Anything in a handled region can raise, not only a node that says so. A region's handlers can therefore
start where its body ends and before every jump out of it, with every counter the region names unknown.
A path that raises partway and is handled is a prefix of one of those, so its loops and calls are
counted. A `finally` block, or a `with`'s exit methods, runs once on every way out and then goes on to any
of them, which only adds paths. An `else` block is read as a `finally` block for the same reason.

What's raised outside every handled region leaves the function. In a document whose destructors are
calls, as C++'s are, it goes on as if it weren't raised instead: the normal path runs every destructor
that unwinding would, and more. A `with` that binds an attribute stays unsupported.

**Fixed shared sequences.** A shared sequence with compiler-proven fixed length uses that constant for
its scans, resolved standard traversal chains and length queries. A shared array-to-slice coercion is
accepted only when its type facts preserve the element type. Callback loops still contribute their work.
Writable arrays, unknown constant-generic lengths, custom iteration, computed receivers and lengths
outside the target pointer width retain conservative handling. A `for` over an owned array is counted
by its fixed length, as any fixed-length sequence is.

Resolved inherent standard `str.chars`, `str.bytes`, `str.char_indices` and `str.lines`
sources use the same shared traversal model. Their receiver must be a stable shared
string place; its extent is UTF-8 byte length. `bytes` yields exactly that many items,
while Unicode scalar and line iteration yield at most that many, including zero for an
empty string. The resulting loop bound retains the receiver's actual formal or local
provenance. Adapter callbacks, their invocation counts, and owning yielded-value cleanup
still pass through the existing chain checks. Generic pattern sources such as `split`,
custom methods, computed receivers and owned `String` dereference are not covered by
these four contracts.

**Collections.** Resolved collection length queries and traversal adapters are abstracted and excluded
from call counts. Ordinary body calls can have unknown cost while preserving a bound on a private scalar
counter.

**Iterator chains.** Compiler-resolved standard `Iterator` chains over those sources become synthetic
scans. Closure callbacks, and immutable local closures that are only called directly, are inlined at
each invocation, so the callback calls in
`xs.iter().map(|x| ys.iter().filter(|y| p(x, y)).count()).sum::<usize>()` are bounded by
`len(xs) * len(ys)`. Filters, skips, takes and searches may exit early, and `flat_map` rescans per outer
item.

**Immutable iterator locals.** One supported lazy pipeline can be stored in a simple
immutable local and consumed exactly once by the immediately following terminal call
in the owner's root block. Resolved references, reads, writes, places and captures must
agree that the local has no alias, borrow, escape, reassignment or other use. Calls
must have complete matching signatures and select the standard implementations.
A named callback plan must agree exactly with the modeled pipeline's callbacks,
including those stored in the local; empty, duplicate, missing or additional entries
are rejected. Explicit driven-code rosters remain unsupported.

The initializer's source is a stable shared formal place. A stage must explicitly
declare deferred construction, and its stored state, intermediate items and captures
must have inert cleanup. Initializer closures may capture only immutable direct scalar
formals by shared reference. The source and other initializer operands evaluate at the
declaration, once; callback bodies run at the consumer. Downstream operands still
evaluate there. Callback environments with unmodeled cleanup and flattened
continuations remain unsupported. Downstream values retain the existing consumer's supported cleanup.
This proves a scan bound from the original extent, without inventing a size for a
partially consumed, returned, captured or otherwise unknown iterator.

**Callables run at most once.** A standard function or member that runs what it's handed at most once for the
call is a branch: its receiver or callee and its other arguments are evaluated, and then the closure or
function it's handed runs or doesn't. A loop in such a closure counts once for each time the call is reached,
so `opt.map(|v| ys.iter().filter(|y| **y > v).count())` inside a scan of `xs` is bounded by
`len(xs) * len(ys)`. A member handed two callables that runs one of them, such as `map_or_else`, is a branch
over both. The member adds no call of its own. Each language's contracts say which these are:

| Language | Functions and members |
| --- | --- |
| Rust | `Option::map`, `unwrap_or_else`, `bool::then` and the like, and `Result`'s |
| Go | `sync.Once.Do` |
| C++ | `std::call_once`, and `transform`, `and_then` and `or_else` of `std::optional` |

**Closures called where they're written.** A closure that's called as it's written, as in Go's
`func() { .. }()` and C++'s `[&] { .. }()`, runs there, so its body is lowered in place.

**Ordinary calls without callbacks.** A conditional library contract can refine a coarse
may-call flag after the core verifies the exact resolved standard operation, call form,
arity and receiver type. Present named callback or driven-effect rosters prevent this
refinement. Required discarded operands must have complete cleanup-free storage facts;
references do not destroy their referents, while owned values with destructors or unknown
storage fail the check. Cleanup checking is bounded by depth and total work, including
sum variants, and budget exhaustion leaves the call unresolved.

The initial contracts cover `bool::then_some` with cleanup-free payloads,
`Option::copied`, whose inherent Copy operation invokes no user method, and
`Result::ok` with a cleanup-free receiver. Receiver and argument evaluation still run:
`false.then_some(compute())` still calls `compute`. The ordinary library call and its
cost uncertainty remain. Callback freedom supplies context to existing effect classes;
it does not declare purity or remove unknown effects.

`Option::unwrap_or` is also callback-free when its eager fallback has cleanup-free
storage. The contained value is moved out of the option, so its type need not be
cleanup-free for this call; cleanup of the returned value remains the caller's
responsibility. A borrowed fallback does not destroy its referent, even when that
referent owns values. An owned fallback with unknown storage or a workspace destructor
keeps the call unresolved. Both the option and fallback are evaluated once before
selection, including calls that construct an unused fallback. The library call and
its existing step contract remain visible; this certificate introduces no fallback
invocation, cost contract or new size relation.
**Callbacks held in standard state.** A separate contract can certify a parameterless
iteration-interface method from its selected standard implementation and exact nominal
receiver. The declared member, implementation link, exclusive receiver, instantiated
signature and complete actual generic roster must agree. Each permitted generic argument
is an exact primitive type key, either directly or behind one shared reference. Missing
facts, callable patterns, other adapter types and named execution remain unresolved.

The first state contract covers `RSplit<&str>` and `RSplit<char>` at `Iterator::next`;
the matching `str::rsplit` constructor accepts only those primitive pattern shapes.
The searcher state is private and fixed by the actual pattern type, so aliases and prior
pulls cannot introduce callbacks. Both calls, their evaluation, existing mutation rules
and surrounding cleanup remain visible. This proves no source-length relation or pull
count: splitting `"abc"` with an empty pattern yields five items, and splitting an empty
string yields two. Generic predicate patterns can run user code and stay unknown.

These stay unknown: unresolved or escaping callbacks, user iterators, mutable captures of a measured
value, and chains that dispatch to workspace `Clone`, `Ord`, `Sum` or `FromIterator` impls. A closure
handed to any other call leaves the whole function unknown. So does one that Go defers or starts with `go`,
which runs later than where it's written.

## What a partial result means

A loop that can't be proven, such as one with a mutable limit, an unproved increment or an unrecognized
guard, doesn't discard the rest of the function. The result is `Partial`, and that loop contributes an
explicit opaque `?loop(site)` factor: the number of times its header runs per entry of its enclosing
loop, which bounds its body and everything nested in it. `n * ?loop(src/lib.rs:12:5#4)` still shows the
proven nesting. The `#4` names the header, so inlined copies stay distinct.

Hazards that affect the whole body, such as unsupported control flow, escaped or aliased counters and
unsafe code, produce an explicit unknown result. Exhausting an analysis budget is reported separately.

## Composed results

Each row also has a `composed` result that adds resolved workspace callee summaries. A call contributes
its count times the callee's bound, with callee inputs replaced by per-invocation upper bounds of the
caller's arguments. Lengths follow shared places and field paths, so passing `&g.edges` into a slice scan
contributes `len(g.edges)`.

Call arguments run in written evaluation order, while their summary inputs follow the formal positions
recorded by the frontend. Thus `scan(unused=0, limit=n)` passes `n` to the formal `limit` even when it
is written last. Missing arguments retain unknown sizes; the analysis does not guess their defaults.
Summary inputs require a known, fixed callable interface. Unresolved positions, duplicate bindings,
spreads, variadic interfaces and mismatched receiver layouts leave argument sizes opaque, while the
call and its argument evaluation remain counted. Argument rosters are bounded before allocation.

An immutable local alias initialized once outside loops can retain the formal input's length
provenance when its initializer is a fully typed shared place rooted in an immutable shared formal.
Field paths must have complete structural type facts. Both the local alias and its original formal
place are checked for writes, borrows and captures. Unknown projections, computed initializers,
destructuring and aliases of other locals retain local or unresolved size information.

A call of a type, such as Python's `Made(xs, ys)`, runs the function its own reference names when the
frontend resolves that statically, here `Made.__init__`. The value the call makes is that function's
receiver, so `xs` and `ys` are the parameters after `self`. A frontend leaves the reference unresolved when
making the value runs anything else, and the call stays opaque.

These contribute opaque factors, which make the composed result `Partial` and are each listed with the
reason:

| Factor | Comes from |
| --- | --- |
| `?call(name)` | The loop work of a recursive, dynamically or generically dispatched, or missing callee |
| `?invocations(name)` | The same callee's own calls |
| `?input(...)` | A callee size with no caller-side bound |

Standard-library bodies are outside this cost unit only when they can't call back into workspace code,
through either a callable argument or a trait bound on a workspace type. Other standard calls are opaque.
Workspace `Drop` glue and overloaded auto-deref into workspace impls make the owner unknown. A reference or a pointer
that doesn't own its referent drops nothing behind it, so it doesn't count. A frontend that declares `implicit-calls` as
`supported` emits each destructor as a call instead, which counts like any other, so the C and C++ route isn't held back
by this rule. Its destructors' bodies count wherever they run.

Identities include the compilation, so lib, bin and test builds never share a summary. The report lists
its assumptions, and results keep each compilation context.

## Library steps

Composed results also report `library_steps`: element operations in standard calls, from the data
contracts in [`runtime_bounds/contracts/`](../crates/rstyle-core/src/runtime_bounds/contracts) rather than
engine code.

The frontend keys each standard call by its resolved self type and member, such as `rust:BTreeMap.get`,
and a contract bounds its steps over argument sizes up to a constant factor. That's `log(len(map))` per
B-tree lookup, so a lookup per key gives `len(keys) * log(len(index.names))`. `log(x)` is the bit length of
`x`.

A key without a contract, or a size without a stable caller-side bound such as a locally built map, gives
an opaque `?library` or `?input` factor listed in `library_factors`. These never change the composed
verdict.

### Contract files

Another language's frontend supplies its own contract file and keys. The same files declare iteration
roles, so frontends carry no method tables:

- A `[[stage]]` or `[[consumer]]` gives an operation such as `rust:Iterator.map` one of a closed set of
  per-item effects (`Pass`, `Map`, `Filter`, `Skip`, `Take`, `TakeWhile`, `FlatMap`; `Drain`, `Each`,
  `Search`), its arity, the callback argument run once per item, and the type arguments whose trait impls
  it may run, which must be standard.
  A stage's optional `deferred = true` additionally certifies that constructing it only
  stores its evaluated operands and runs no callback or trait operation; wrapper cleanup
  only cleans up those stored operands. Core still verifies their cleanup before
  retaining the pipeline in a local. The default is false; consumers cannot declare it.
- A `[[callable]]` names functions and members that run a callable for the call itself, the arguments
  that are callables, counted after the receiver or the callee, and how often it runs one. One that runs
  one at most once lowers to a branch around each.
- `[[allocation]]` certifies the storage created by a builtin allocator, independently of its cost.
  `FreshNativeSequence` promises native element writes without user setters; a declared static type
  operand is skipped only after matching a childless type annotation at the certified position.
  An optional `length_argument` certifies exact result cardinality at a distinct original post-callee
  argument position. Resolved builtin `len` reads native sequence lengths; maps and channels are
  excluded. These reads are optional extent sources, so an unrelated mutable length read does not
  reject otherwise known loop limits.
- A `[[callback_free]]` gives a resolved operation's `Member` or `Function` form, its
  post-receiver/callee arity, exact type-key guards and operands requiring cleanup-free
  storage. Member operand 0 is the receiver; Function operand 0 is the first argument.
  This role declares neither a cost, purity nor a callable invocation count.
- A `[[state_callback_free]]` names a declared iteration operation, the exact standard
  nominal receiver identity and its complete generic type roster. Each position lists
  permitted primitive keys and whether one shared reference is required. The core checks
  the selected parameterless implementation and actual exclusive receiver before refining
  a coarse may-call fact. This role declares no traversal extent, purity or cost.
- `[[extent]]` marks members that read a stable length (`Length`) or traverse each entry once
  (`Traversal`).
- An `[[extension]]` gives a registered extension, named without its namespace, the operation it runs
  after its children: `Never`, `Always`, or `WithoutMember`, for a node that runs one only when it names
  no member, as a `**` keyword argument does. Its `lazy` names the modifier of a node that runs where
  it's consumed. Its `control` says instead how it raises (`Throw`,
  `Assert`) or handles (`Try`, `With`), and a `Try`'s `handlers` name the extensions among its children
  that are handlers, as a `match` among them is.
- A `[[range]]` gives a call that makes a range of integers the argument that holds its `start`,
  `stop` and `step` at each `arity` it's called with. A `[[traversal]]` gives a call that yields at most
  one item per item of its argument `over` the `arities` it has that role at.
- A `[[loop]]` gives a loop node's original kind, such as `DoStmt`, its `guard`: `Last`, the last
  statement of its block. An `[[operator]]` gives a language's own operator, such as `comma`, a role core
  evaluates: `Sequence`.

A frontend lowers effects, never method names, and an operation without a role keeps ordinary call
lowering.
