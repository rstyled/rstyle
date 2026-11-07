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
- the frontend doesn't say what its calls resolve to

A function a macro or a build writes has no author to tell, so it's left out, as are the paths `exclude` removes. So is a
function in a file the frontend marks as a test, since tests drop and overwrite values to set things up. Go's `_test.go`
files are marked. Rust's default run doesn't read tests, and with `--all-targets`, or in C and C++, they aren't told apart,
so `exclude` them if they're noisy.

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

## `ignored_result`

A call that drops its result where nearly all of the run's other calls of that callee use it. Nothing in a signature says
a result matters, so the calls decide, as in "Bugs as Deviant Behavior" (Engler et al., SOSP 2001):

```go
json.Unmarshal(raw, &out) // 90 of this callee's 93 calls use the error
```

A callee needs at least 5 calls that count, and at least 90% of them using the result. Those defaults were read against Go,
Rust and C++ modules, and what's normal differs between languages and codebases, so `rstyle.toml` can change both:

```toml
[ignored_result]
min_calls = 5
min_used_percent = 90
```

`min_used_percent` decides most of it. At 90%, one dropped call already takes 10 calls, so `min_calls` only matters once the
share is lower. On go-ethereum's code outside tests, 95% reports 10 results, 90% reports 14 and 80% reports 53. A share of
100 reports nothing, since then no call can have dropped the result.

Dropping means the call is all of a statement. These don't count either way:

| Case | Why |
| --- | --- |
| A call with no result, or one of a type the frontend can't describe or a type parameter | There's nothing to judge |
| A method that returns what it was called on, such as `x.SetUint64(1)` | The result is only there so calls can chain |
| A `defer`, or anything else an extension runs | It says how the call runs, not what becomes of its result |
| A call of a function value, or one nothing resolves | No callee to weigh it against |
| A statement a lowering made up, or a call a macro wrote | Nobody wrote it |

Giving a result a name to discard it, as in `_ = f()` or `let _ = f();`, uses it, since that says it's deliberate.

Calls are weighed by the callee they name, so everything that implements an interface method counts together. Calls in a
test file or an excluded path neither count nor get reported, so only the code you ship is weighed.

A result may be dropped on purpose, such as calling a function for what it caches. Suppress those with a reason.
