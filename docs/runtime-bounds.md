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

## What's modeled

**Counters and traversals.** The model supports:

- primitive increasing counters
- unsigned exclusive ranges
- traversal of shared slices and standard `Vec` values, including fields reached through shared
  references

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

**Collections.** Resolved collection length queries and traversal adapters are abstracted and excluded
from call counts. Ordinary body calls can have unknown cost while preserving a bound on a private scalar
counter.

**Iterator chains.** Compiler-resolved standard `Iterator` chains over those sources become synthetic
scans. Closure callbacks, and immutable local closures that are only called directly, are inlined at
each invocation, so the callback calls in
`xs.iter().map(|x| ys.iter().filter(|y| p(x, y)).count()).sum::<usize>()` are bounded by
`len(xs) * len(ys)`. Filters, skips, takes and searches may exit early, and `flat_map` rescans per outer
item.

These stay unknown: unresolved or escaping callbacks, user iterators, mutable captures of a measured
value, and chains that dispatch to workspace `Clone`, `Ord`, `Sum` or `FromIterator` impls.

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
- `[[extent]]` marks members that read a stable length (`Length`) or traverse each entry once
  (`Traversal`).

A frontend lowers effects, never method names, and an operation without a role keeps ordinary call
lowering.
