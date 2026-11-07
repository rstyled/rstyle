# Defects

`rstyle go --filter defects`, `rstyle rust --filter defects` and `cargo rstyle cpg analyze --filter defects` run rules
that say what a function's own facts show is probably wrong. A style rule says a limit has passed. A defect says what
this code does, so it comes with a reason to look, not a number.

Each rule reads the same `compat.cpg` documents for every language, so a new frontend gets it without any code here.

```text
core/txpool/blobpool/conversion.go:192: dead_store: the value stored in `txTasks` here is never read: every path replaces it or leaves first
Defects: 2 defects in 15784 functions, 0 suppressed, 229 unknown
```

## No result doesn't mean nothing's wrong

A function the rules can't read is `unknown`, with why, and never counted as clean. That happens when:

- the frontend emits no control flow for it, or doesn't say what can alias in it
- the frontend records no reads and writes for it
- it doesn't hold all of its syntax, such as when the compiler couldn't parse part of the file
- it holds a construct the frontend doesn't model

A function a macro or a build writes has no author to tell, so it's left out, as are the paths `exclude` removes.

## Exit code

Defects don't change the exit code yet. They're in `report.json.zst`, `defects.sarif` and `report.sarif`, and in a run's
text.

## Suppressing a finding

As for style rules, put a comment with a reason directly above the function:

```rust
// rstyle-allow(dead_store): the first call's effect is what matters
fn start() { /* ... */ }
```

## `dead_store`

A value stored in a binding that nothing can read, because every path from the store replaces it or leaves the function
first. A value was computed for a reason, and the commonest one is an error nobody checked:

```go
x, err := f()
y, err := g(x) // the first err is gone
```

A store means an assignment to a binding or a declaration's computed initializer. These stay quiet:

| Case | Why |
| --- | --- |
| `count := 0`, then assigned | A starting value is a choice of style, and drops nothing computed |
| `var x int`, or a binding a `for` or match makes | Nothing was computed to store |
| Anything a closure captures | It reads when it's called, whenever that is |
| A binding borrowed mutably, reached by a raw pointer or written in part | What's read isn't only what the name says |
| A named result | The return reads it |
| A read flow can't place | It can't be put before or after a store |
| A type with a destructor | Going out of scope is the point, as with a lock guard |
| An opaque type or a type parameter | Writing one could do anything, as with a volatile int |
| A name starting with `_` | That says it's deliberate |
| A store a macro or lowering wrote | The author didn't write it |

It reports a real store, and the store may still be harmless, such as clearing a local before returning. It doesn't look
at fields or what a pointer holds: `t.a = 1` isn't a store here. A compiler already warns about many of these in Rust.
