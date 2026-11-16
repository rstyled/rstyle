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
- a set, and a dict's items, keys or values by a `[[extent]]` traversal role whose `receiver` is a `map`,
  each counted by its entries; a frontend gives a container those types only where nothing from the
  function's start through the loop can change it, as rstyle-py does
- a tuple, a sequence whose type has one fixed length, and an integer, which a `for` counts from zero up
  to, as Go's `for i := range n` does

A source a loop counts this way is evaluated once before its first pass. A signed count, such as a Python
`range`'s, bounds only from a constant start of at least zero, so `range(a, b)` and a range that counts
down stay opaque. A loop's source is the call it's written with, as `for x in enumerate(xs)` is. Only a
compiler-generated `for` wrapper is looked through, so a call no role names is never counted by the
length of one of its arguments. Python's `sorted` has no role, since it compares the items, which can
run their own methods. Go writes no length for an array type, so a Go array isn't counted.

**Conditions that run something.** A `while` condition runs before every pass and once more. When
evaluating it runs anything, as a call or an overloaded operator does, it's lowered before the first test
and again at the end of each pass, where a `continue` goes too, so every pass returns to the loop's one
guard. The comparison a condition tests is read after what it runs first: a block's tail, as in Rust's
`while { f(); i < n }`, and the last operand of an operator the contracts give the `Sequence` role, as C++'s
`while (f(), i < n)` has. A guard that isn't a counter's `<` leaves the loop's entries opaque.

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

**Calls run later, removals and chains.** A call Go defers, or starts on a goroutine, is evaluated where
it's written and counts once each time its statement runs; one that holds a closure stays unsupported,
since the closure reads values that can change first. Python's `del` writes a name and removes an item or
an attribute by an unresolved call, and a chain of comparisons evaluates each next operand only while
every comparison so far held, with an unresolved call for each comparison of values other than numbers,
booleans and strings.

What's raised outside every handled region leaves the function. In a document whose destructors are
calls, as C++'s are, it goes on as if it weren't raised instead: the normal path runs every destructor
that unwinding would, and more. A `with` that binds an attribute stays unsupported.

**Collections.** Resolved collection length queries and traversal adapters are abstracted and excluded
from call counts. Ordinary body calls can have unknown cost while preserving a bound on a private scalar
counter.

**Iterator chains.** Compiler-resolved standard `Iterator` chains over those sources become synthetic
scans. Closure callbacks, and immutable local closures that are only called directly, are inlined at
each invocation, so the callback calls in
`xs.iter().map(|x| ys.iter().filter(|y| p(x, y)).count()).sum::<usize>()` are bounded by
`len(xs) * len(ys)`. Filters, skips, takes and searches may exit early, and `flat_map` rescans per outer
item.

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
- A `[[callable]]` names functions and members that run a callable for the call itself, the arguments
  that are callables, counted after the receiver or the callee, and how often it runs one. One that runs
  one at most once lowers to a branch around each.
- `[[extent]]` marks members that read a stable length (`Length`) or traverse each entry once
  (`Traversal`). Its `receiver` names the type kinds the receiver must have for the role to hold, as
  Python's `dict.items` needs a `map`.
- An `[[extension]]` gives a registered extension, named without its namespace, the operation it runs
  after its children: `Never`, `Always`, or `WithoutMember`, for a node that runs one only when it names
  no member, as a `**` keyword argument does. Its `lazy` names the modifier of a node that runs where
  it's consumed. Its `control` says instead how it raises (`Throw`,
  `Assert`), handles (`Try`, `With`), runs a call later (`Deferred`), removes (`Delete`) or compares in a
  chain (`Chain`), and a `Try`'s `handlers` name the extensions among its children
  that are handlers, as a `match` among them is.
- A `[[range]]` gives a call that makes a range of integers the argument that holds its `start`,
  `stop` and `step` at each `arity` it's called with. A `[[traversal]]` gives a call that yields at most
  one item per item of its argument `over` the `arities` it has that role at.
- A `[[loop]]` gives a loop node's original kind, such as `DoStmt`, its `guard`: `Last`, the last
  statement of its block. An `[[operator]]` gives a language's own operator, such as `comma`, a role core
  evaluates: `Sequence`.

A frontend lowers effects, never method names, and an operation without a role keeps ordinary call
lowering.
