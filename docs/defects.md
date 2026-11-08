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

## `deviant_argument`

A call that passes one variable where the same call in its sibling methods all pass another. Methods of one type
written as near copies of each other, such as a store's `Create`, `Get`, `Update` and `Delete`, agree on what they
pass, so the copy that doesn't stands out. It's "Bugs as Deviant Behavior" one level down from `ignored_result`:
the calls decide what an argument should be.

```text
pkg/apiserver/rest/dualwriter_mode3.go:106: deviant_argument: `recordStorageDuration` is passed `name` here, where 6 of its 7 calls in sibling methods pass `options.Kind`
```

Siblings are methods of the same type that share at least half their callees. A call is compared with the calls of
the same callee at the same argument in at least 4 of them, counting its own, and every other call must agree.

A difference is reported only when nothing but a slip explains it:

| Case | Why it stays quiet |
| --- | --- |
| A literal or a constant, such as `http.StatusNoContent` where siblings pass `http.StatusOK` | A different value is a choice; a slip reads the wrong variable |
| A call that differs in another argument too, such as `log.Infof("denied: %v", err)` | It's another call, not the same call with one thing wrong |
| A variable of another type than the siblings pass, such as decoding into `respDTO` where siblings decode into their `result` | It's another value, not the same one misnamed |
| A variable that holds nothing yet at the call, such as `membs := convert(members)` | The siblings' variable couldn't have been passed there |
| A variable computed from the siblings' one, in any number of steps, such as `body := read(resp.Body)` then `decode(wrap(body))` | Deriving one from the other is deliberate |
| A builtin, such as `len` or `append` | What it's asked varies by design |
| A function that isn't a method, or a callee nothing resolves | There are no siblings to weigh it against |

A call inside a closure, such as a goroutine's, is judged where the closure is made, with what the closure binds
before the call. Calls a test file holds, or an excluded path, neither count nor get reported.

A difference can still be deliberate, such as a method that reads its old value where its siblings read the new one.
It doesn't check that the siblings' field exists on the variable passed here: `r.User` where siblings pass `r.Name` is
reported even when this `r` has no `Name`, since frontends don't describe the fields of every type. Suppress those with
a reason.

## `taint_flow`

Input reaching where it's dangerous: a value from a source, through what your functions do with it, to an argument a sink
treats as more than data.

```text
common/path.go:42: taint_flow: input from `os.Getenv` on node/defaults.go:124 reaches `os.Open` (path traversal, CWE-22): check how it's validated
```

It's a path to look at, not proof that it can be exploited, and no result doesn't mean there isn't one. It's reported at the
sink, in whichever function holds it, and `rstyle-allow` goes above that function.

Which calls are sources and sinks is data, per language, in `cpg/contracts/taint/`:

| Language | Sources | Sinks |
| --- | --- | --- |
| Go | Query values, form values, headers, `os.Getenv`, `os.LookupEnv`, `io.ReadAll` | `database/sql` query text, `net/http` URLs, `os/exec` programs, template source, `os` file paths |
| C and C++ | `getenv`, `fgets`, `read` and others | `system`, `popen`, the `exec` family, `fopen`, `open` and others |
| Rust | `std::env::var`, `std::env::args`, reading stdin and others | `std::process::Command` programs and what a shell runs, `std::fs` paths |
| Python | `os.environ`, `os.getenv`, `sys.argv`, `sys.stdin`, `input` | `os.system`, `subprocess` with `shell=True`, `eval`, `exec`, `sqlite3` query text, `pickle.loads`, `open`, `os` and `pathlib` paths and others |

Only an argument the sink treats as more than data counts, so a query parameter doesn't, and neither does an ordinary
argument to a program, unless a literal shell and `-c` say it's code. Python's `subprocess` runs a shell only when it's
told to, so it's a sink only where a call passes `shell=True`.

A source can be a library's variable as well as a call, so reading `os.environ` or `sys.argv` is where input starts, and
`os.environ.get("HOME")` and `sys.argv[1]` hold it. A model can say what cleans input too: Python's says `shlex.quote`
does.

A Rust `Command` is built a call at a time, so its arguments are what `arg` adds and each element of a list written out for
`args`, in the order the calls ran. So `Command::new("sh").arg("-c").arg(input)` is found, and so is the same through
`let mut cmd`, while `.arg("script.sh").arg(input)` and `.args(["-c", SCRIPT, "sh", &input])` aren't. A call between them
with no body here, such as `current_dir`, keeps the command as it was.

`rstyle.toml` has the last word. The standard library is all the models know, so name your own crates and packages here:

```toml
[taint]
sources = ["myapp::read_request"]   # calls whose result is input
sanitizers = ["myapp::escape"]      # calls whose result no longer is, for every kind of sink
ignored = ["std::env::var"]         # calls that are no end of a flow here, or a model's sanitizer that isn't one

[[taint.sinks]]
callees = ["sqlx::query"]
arguments = [0]                     # a receiver is 0, and otherwise the first argument is
risk = "SQL injection"
cwe = "CWE-89"
```

A callee is named by its operation, its name or its identity. A command-line tool is meant to read its own environment, as that
example is, and every result on go-ethereum and on rstyle itself is an environment variable choosing a path or a program, so
`ignored` is often the right answer for those.

Ignoring a sink ignores each way it's dangerous. So ignoring `std::process::Command::new` ignores both the program it's
given and what a shell runs in a command it made, chained or through `let mut cmd`, as ignoring `os/exec.Command` does in
Go. Ignoring `std::process::Command::arg` or `std::process::Command::args` ignores only what that call adds.

### How it follows a value

Each function says where its values go, by what its tree's operands and its reaching definitions show, so overwriting a value
clears it, `q += input` holds it, and `b.WriteString(input)` leaves it in `b`. A summary of each function says which parameters
reach a sink or what it returns, and whether it returns input of its own, so a value is followed across a run.

| What | What happens |
| --- | --- |
| A call with a body here | Gives back what its body does |
| A call with no body here | Gives back everything it was handed |
| A call that gives back a reference to what it was called on | Writes what that writes, so `cmd.arg("-c").arg(script)` adds both to `cmd` |
| A number or a truth value | Holds no input, so `len(input)` and `%d` of one are clean |
| A value that only decides a branch | Isn't followed: this is about what a value holds |

### What it misses

- **Fields.** A value of your own types isn't followed through, since one field from a request says nothing of the others. So
  input stored in a field and read back isn't found, and a builder holding input in a field isn't either. Following them gave
  go-ethereum 8 more results from one HTTP header, by way of `cfg.client = makeClient(ctx)` reading as `cfg.filterQueryFile`
  being input.
- **A call through an interface or a function value** gives back what it's handed, and a sink behind one isn't seen. Neither is
  what a closure returns.
- **A Rust command built in more than one function**, or handed a list that isn't written out in the call, as
  `args(&argv)` and `args(vec![..])` are: what's added can't be counted, so nothing after it is read as code. Neither is
  anything after a call with a body here, which could add arguments, or after a flag or program that isn't a literal. A call
  on a branch is read as having run, so `cmd.arg("-c"); if verbose { cmd.arg("-x"); } cmd.arg(input)` isn't found. The
  shells are Go's, all told to run code by `-c`, so `cmd /C` and PowerShell aren't modeled.
- **Python's frameworks.** The models only know the standard library. Flask's `request` is a variable, so with Flask
  installed a project can name `flask.globals.request` under `sources`, and `request.args` holds what it does. Django's
  `request.GET.get(...)` is `django.utils.datastructures.MultiValueDict.get` once django-stubs gives Django types, but
  `request.GET["q"]` is a subscript, not a call, so nothing can name it.
- **A program Python's `subprocess` runs.** A list is one argument here, so input choosing the program, as in
  `subprocess.run([input, ...])`, isn't told from an ordinary argument, and neither is a shell in a list, as in
  `subprocess.run(["sh", "-c", input])`. Nor is `shell` passed anything but a literal `True`.
- **What `shlex.quote` cleans.** It quotes for a shell, but like any sanitizer here it clears a value for every kind
  of sink, so input it quotes and then opens as a path isn't found.
- **A Python keyword argument is numbered by where it's written,** so `f(b=x)` reads `x` as `f`'s first parameter.
- **A Python `with`'s target** isn't given what its expression returns, so `with open(path) as f` leaves `f` clean.
- **What Python can't resolve** ends a flow: `sys.stdin.buffer`, since typeshed's `sys.stdin` can be `None`, and
  `shutil.rmtree` and `io.open`, which typeshed declares as variables, so a call of one is through a value.
- **Functions it can't read** are `unknown`, as for every rule here.
