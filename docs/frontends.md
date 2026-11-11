# Runner and frontends

`rstyle` is the runner for every language frontend. It finds the frontend, reads the `compat.cpg`
documents it produces, runs the selected analyses once in core, and saves the run.

```text
rstyle go                  rstyle cxx                  rstyle py                  rstyle rust
rstyle go --check          rstyle cxx --check          rstyle py --check          rstyle rust --check
rstyle go --patterns       rstyle cxx --patterns       rstyle py --patterns       rstyle rust --patterns
```

`cargo rstyle go`, `cargo rstyle cxx`, `cargo rstyle py` and `cargo rstyle rust` are the same commands.

## Finding the frontend

- **Go:** rstyle uses `$RSTYLE_GO` if set, then `rstyle-go` where Go installs binaries or on `PATH`, and
  otherwise runs `go install`. While `rstyle-go` is private, set `GOPRIVATE=github.com/rstyled` or point
  `$RSTYLE_GO` at a local build. `--source DIR` picks the directory, and by default it's the module
  holding the current directory.
- **C and C++:** rstyle uses `$RSTYLE_CXX` if set, then `rstyle-cxx` on `PATH`. It's built against a specific
  LLVM, so rstyle can't install it, and a missing one says where to build it from. `--source DIR` picks the
  directory, and by default it's the nearest directory holding `compile_commands.json` or
  `build/compile_commands.json`, then the repository. `--compilation-database PATH` names the database when it's
  elsewhere, and `--include DIR` and `--define NAME[=VALUE]` add to the flags.

  Without a named database, rstyle-cxx works one out: one the project has, its own `.rstyle/compile-database.sh`,
  or one its CMake, Meson or Make build writes. Its README has the order. Its own options go after `--`:
  `rstyle cxx -- --no-generate-database` runs none of the project's build scripts, and `rstyle cxx --
  --require-database` stops when there's no database instead of guessing flags.
- **Python:** rstyle uses `$RSTYLE_PY` if set, then `rstyle-py` on `PATH`. It pins its analyzer and needs an environment
  of its own, so rstyle doesn't install it, and a missing one says how: `uv tool install
  git+https://github.com/rstyled/rstyle-py`, which needs access to it while it's private. `--source DIR` picks the
  directory, and by default it's the nearest directory holding `pyproject.toml`, `setup.py` or `setup.cfg`, then the
  repository.

  Types from third-party packages come from where they're installed. The frontend reads an environment it finds in the
  project, or the active one, without running anything in it. `--python PATH` names an interpreter instead, and
  that one it runs. With neither, those calls stay unresolved, and the run says so.

  A file that doesn't parse ends the run, naming every such file and how to [leave it out](#leaving-files-out).
- **Rust:** the frontend is the bundled driver. `--manifest-path`, `-p` and `--all-targets` apply.
  It reads the same capture as `cargo rstyle` and `patterns`, so only what changed compiles.

## Selecting analyses

With no filter you get everything. `--filter` takes names, comma-separated or repeated, and `--check` and
`--patterns` are shorthand for two of them:

| Name | Runs |
| --- | --- |
| `check`, `patterns`, `bounds`, `flow`, `defects` | Core's analyses |
| `go:<name>` | An analysis local to the Go frontend, which offers `forcetypeassert` |
| `cxx:<name>`, `py:<name>`, `rust:<name>` | An analysis local to that frontend; none exist yet |

An unknown name fails before anything runs, as does `--fail-on-candidates` without patterns.

`flow` is most of a report's size: on gstyle, 126 MB of the 176 MB printed and 6.2 MB of the 7.5 MB saved,
for about 2 s more. `--filter check,patterns,bounds` leaves it out.

## Leaving files out

`exclude` in `rstyle.toml` names paths and globs a run leaves out, and `--exclude GLOB`, which can be repeated, adds to it for
one run. Both are relative to the source directory, as reports show paths.

- **One syntax everywhere.** `*` and `?` match across `/`. `**/` at the start and `/**/` in the middle match any number of
  directories, and `/**` at the end everything under. `[ab]`, `[!ab]`, `{a,b}` and `\` work as in a shell. A path, or a glob
  that matches a directory, takes everything under it, so `tests/fixtures`, `tests/fixtures/` and `tests/fixtures/**` leave out
  the same files. [`cases.tsv`](../crates/rstyle-core/src/exclusions/cases.tsv) shows how core reads each form.
- **What's excluded isn't reported.** Core leaves its functions out of every analysis, from any frontend and on a replay. A
  callee it removes keeps its callers' effects incomplete.
- **Only Python skips reading it.** A Python file stands alone, so one that's meant not to parse can be skipped. Go and C++ read
  an excluded file, since its package or whatever includes it needs it to type-check, and leave its functions out of the
  documents. One that doesn't parse doesn't stop their run either: Go skips it, and C++ counts its errors like any other.
  Rust compiles the whole crate, and core leaves them out. A frontend says on stderr how many files that left out.
- **A run records them.** `run.json` lists what the run excluded, and `--from` leaves that out too, with whatever `exclude` and
  `--exclude` add now. `--format json` lists everything a report left out as `exclude`, and the text names it when it's more
  than the default, `target/**`.

## Saved runs

A finished run is published whole, so a rejected one leaves nothing behind.

Every language saves to `.rstyle/runs/<UTC timestamp>-<language>/` in the project root, meaning the Go
module, the C, C++ or Python project, or the Cargo workspace. That folder ignores itself in Git. Each language keeps its newest 5 runs, and
`RSTYLE_KEEP_RUNS` sets another count, or `all`.

`cargo clean` doesn't touch these runs, though it clears the capture a Rust run reuses, in
`target/rstyle/runs/`. Deleting `.rstyle/` is always safe.

`--out DIR` saves to a new directory instead, which is never pruned. Each run holds:

```text
documents/                 what the frontend produced
report.json.zst            the complete report
report.sarif               SARIF 2.1.0: every run below in one log
check.sarif                SARIF 2.1.0, category rstyle/check/
patterns.sarif             SARIF 2.1.0, category rstyle/patterns/
defects.sarif              SARIF 2.1.0, category rstyle/defects/
frontend/<analysis>.sarif  a frontend analysis's results, as it wrote them
run.json                   the language, source root, document count and what was excluded
```

`--from DIR` analyzes a saved run without its frontend, and saves only with `--out`. Naming a frontend analysis
that reads documents, as `--filter <frontend>:<name>`, replays it too, which needs the frontend but not the source
or a compiler. One that reads the source is refused. `--format json` prints plain JSON to stdout, with `run` naming where
the run was saved. Two runs over the same source give byte-identical reports and SARIF. The Rust frontend
derives a function's node and symbol IDs from its path rather than its position in the crate, so adding or
removing an item leaves other functions' IDs as they were. For the same reason a definition's
`rust:compiler-identity` names it, and its parent, by def path rather than by `DefId`.

## SARIF

SARIF carries findings and candidates, one run per analysis, so nothing interleaves. A frontend analysis's
runs also join `report.sarif` and the JSON report's `frontend` list, with a category such as `go/calls/`
and a source root if they have none. Its results don't change the exit code.

Runtime bounds, signature groups and control flow stay in the report, which remains the complete one.

## Exit codes

The worst result of what ran:

| Code | Meaning |
| --- | --- |
| `0` | Clean |
| `1` | Findings, including candidates left with `--fail-on-candidates` |
| `2` | A tool error, missing analysis or failed frontend analysis |

## Writing a frontend

A frontend is a program with up to three commands:

| Command | Does |
| --- | --- |
| `describe` | Prints `{"schema":"rstyle.frontend-description/1","language":"go","options":["exclude"],"analyses":[{"name":"calls"},{"name":"nilerr","input":"documents"}]}` |
| `cpg --source DIR --output-fd N` | Writes an RFC 7464 JSON text sequence to the inherited descriptor, then `RS null LF` only if generation succeeded |
| `analyze NAME --source DIR --out FILE` | Writes SARIF 2.1.0 to `FILE`, for each name it describes |

- `N` is a descriptor on Unix and an inherited handle's value on Windows.
- Arguments rstyle has for that language come after these, such as `rstyle-cxx`'s `--compilation-database`.
- `options` lists what `cpg` takes that not every frontend does. With `exclude`, the runner passes `--exclude GLOB` for each glob
  the run [leaves out](#leaving-files-out), after the language's arguments, `target/**` included. The frontend must leave
  their functions out of its documents, and may skip reading the files. One that doesn't list it isn't passed it, and core
  still leaves those functions out of the report.
- Whatever followed `--` on rstyle's command line comes last, after `--exclude`, unchanged: `rstyle cxx --
  --require-database` hands the frontend `--require-database` last. rstyle knows nothing of what they mean.
- A file that doesn't parse should fail the run, and the failure should name `--exclude` as the way to skip it.
- A non-zero exit means the run is incomplete.
- A frontend's stdout goes to rstyle's stderr.
- The descriptor must not reach the frontend's own child processes, or rstyle never sees end of file.

The runner keeps what it receives, so a frontend needs no compression or storage code of its own. Documents
are derived on a worker pool as they arrive, and records are put in a canonical order, so arrival order
can't change a result.

### Analyses that read documents

An analysis works from the source unless its description says `"input":"documents"`. Then the runner also
passes `--input-fd N`, after the other arguments, and writes the run's documents to that inherited descriptor
or handle while the analysis runs:

- It's the sequence `cpg` writes: each document as the frontend sent it, in the order it sent them, then
  `RS null LF`. A sequence that stops without that record is incomplete, and the analysis must fail.
- The analysis must read to the end. One that exits successfully while the runner is still writing has failed.
- Selecting only such an analysis still runs `cpg`, and the run keeps the documents.

This gives an analysis exactly what core's analyses read, with no second load of the source and no decompression
or file handling of its own.

### Suppressing a frontend's rule

`// rstyle-allow(go:nilerr): reason` above a function names a frontend's rule the way `--filter` does. Core accepts
that spelling and doesn't check the rule, and the frontend decides what the comment does for its own analysis.

### Generated code

A frontend records every function the compiler holds and marks what a macro writes by its provenance. What
each analysis does with it is core's to decide, on every route.

A macro writes a function when it writes both its signature and its body's root. A macro that only wraps a
function, as `#[instrument]` does, or rewrites its signature, as `#[async_trait]` does, leaves it the
author's.

A function a macro writes gets no rule findings or pattern candidates, since nothing original was written to
change, but its effects reach its callers and runtime bounds compose through it. Report a macro's own tokens
as generated even when they carry the span of one of its inputs, as `quote_spanned!` writes them.

A file the build writes has `origin: "generated"`, and its functions are treated the same way. The Rust
frontend marks what a build script writes to `OUT_DIR`.

### Bodies

What a module runs when it's loaded, or a type when it's defined, is its author's code but wasn't written as a
function, as a Python module or class body is. Record it as a function whose signature and root have `desugaring`
provenance with the construct `module` or `type`, and the body as the call site. Core calls it a body. Like a function
a macro writes, it gets no rule findings or pattern candidates and its effects reach its callers. Unlike one, merge
risk compares it as a function, by its graph and by its own text: its span, less the functions written inside it.
A change to a method is that method's, and a decorator's is the module's, since that's where it runs. A desugaring
from any other construct, such as an `async fn`'s, leaves the function the author's.

### A language's words

Core says its reasons, labels and proposals in neutral terms, such as a writable reference or a closure
capture. `cpg/contracts/wording/<language>.toml` in `rstyle-core` says any of them in one language's terms,
for units whose `language.name` it names: `rust.toml` says borrow and `static mut`, `cxx.toml` non-const
reference and lambda, `go.toml` pointer and package variable, and `py.toml` `nonlocal` capture and module
variable. A language without a file reads the neutral text, so a new frontend needs no code for its words.

A file lists messages by key. `{name}` is a placeholder and `{{` and `}}` are braces. An unknown key, or a
placeholder a message leaves out or doesn't have, is an error when the file loads. `wording.rs` lists every
key with its neutral text and placeholders.

A pattern candidate describes the interface it proposes, as "one interface for `equals`, which Dog, Cat
define with matching signatures". Only `rust.toml` writes it as a declaration, `trait Shared { ... }`.

## The `cpg` commands

These run core's analyses directly over `compat.cpg` documents, with no compiler:

```text
cargo rstyle cpg check DOC.json... [--config rstyle.toml] [--format text|json]
cargo rstyle cpg patterns DOC.json... [--complexity] [--config rstyle.toml] [--format text|json]
cargo rstyle cpg flow DOC.json... [--function NAME] [--format text|json]
cargo rstyle cpg analyze DOC.json... [--filter check,patterns,bounds,flow,defects] [--sarif-dir DIR]
cargo rstyle cpg emit --out cpg/     # a fresh check of a Rust workspace with the driver
```

`check`, `patterns` and `analyze` also read other inputs in place of the document list, and `receive` verifies
them and returns a receipt:

```text
cargo rstyle cpg receive --capture CAPTURE_DIR --format json
cargo rstyle cpg check --capture CAPTURE_DIR [--format text|json]
cargo rstyle cpg patterns --capture CAPTURE_DIR [--complexity] [--format text|json]
cargo rstyle cpg receive --stdin --format json
cargo rstyle cpg check --stdin [--format text|json]
cargo rstyle cpg patterns --stdin [--complexity] [--format text|json]
cargo rstyle cpg analyze --stdin [--filter check,patterns,bounds,flow,defects] [--sarif-dir DIR]
```

A run is any number of units: bundled unit documents, and split unit documents with their function documents.
Read together, effects propagate across units as they do across a Rust workspace's crates. Read one at a time,
a unit sees only its own calls.

Core derives every rule fact from the documents: statement counts, complexity constructs, calls, call edges,
effects and mutations. The `compat.cpg` profile defines each derivation. No compiler runs, and only an explicit
`--config` applies.

These are rejected with exit code 2:

- documents from another profile version
- documents with undeclared coverage or without file content
- split function documents that don't match their unit's digest
- a function read twice

`cpg patterns` exits 1 with `--fail-on-candidates` when a candidate is left.

`cpg analyze` runs the selected analyses in one pass, so its results are what `cpg check`,
`cpg patterns --complexity` and `cpg flow` report separately. With no `--filter` it runs them all, and
`--sarif-dir` writes `check.sarif`, `patterns.sarif`, `defects.sarif` and `report.sarif`. It derives documents on a worker pool.
The `rstyle` commands above use it, and add the frontend and the saved run.

### Sending documents to rstyle

Saved Rust CPG documents use streaming JSON and Zstandard by default. See
[CPG storage and transport](cpg-transport.md) for plain JSON compatibility, seals and frontend interoperability.

**Completed captures** from native Go or the streaming helper carry a manifest. `cpg receive` verifies all listed
files and CPG inputs and returns an acceptance or rejection receipt. Directory-based check and pattern reports
include that receipt, and release each bundled unit's raw JSON after deriving its records. Analysis records stay in
memory, and split-layout inputs are retained until EOF.

**A local pipe** needs no capture directory, manifest or compression. A frontend sends the helper's uncompressed JSON
text sequence to `--stdin`: RS (`0x1e`), one JSON object, LF, repeated per document, then RS `null` LF and EOF after
successful generation. The receiver requires the completion marker, EOF and all CPG validation before accepting.
Interrupted input returns a rejected stream receipt and exit 2. Analysis reports include this as `capture_receipt`.
See the [pipe example and receipt contract](cpg-transport.md#receiving-a-local-pipe) for a runnable producer.

On Unix, `--input-fd N` selects a caller-supplied inherited descriptor instead of stdin. All three receiver commands
support it and identify the descriptor in their stream receipt. The receiver duplicates it for reading and keeps the
original descriptor open.

### The Rust route

`cargo rstyle` and `cargo rstyle patterns` read Rust the same way. The driver writes each crate's documents and a
manifest sealing them into its unit directory, and everything the report needs is derived from those documents
afterwards, in core. A saved run therefore replays without a compiler or the sources, and its documents answer any of
these routes. The driver writes nothing else, and no route reads rustc directly.

### Control flow

`cpg flow` reports what each function's control flow establishes, from any frontend that emits one:

- immediate dominators and post-dominators
- each loop's back edges
- a dependence graph of data, control and execution edges

A data edge is a write that can reach a read. A control edge marked `termination` holds only because a loop before its
target might never finish.

Each function says how far this can be relied on:

| Standing | Meaning |
| --- | --- |
| `known` | The results can be relied on |
| `approximated` | Holds with the listed reasons, such as a binding that can also change through a borrow |
| `unknown` | The frontend emits no flow |

A closure's body is opaque and has no flow of its own. A coroutine's, which is all of an `async fn`, runs in place.
Unwinding is not a transfer.

### Merge risk

`rstyle risk --base RUN --head RUN` compares two saved runs from these same facts, so a frontend gets it with no code of
its own. See [merge risk](merge-risk.md).

### Defects

`taint_flow` reads which library calls and variables are sources and sinks from `cpg/contracts/taint/<namespace>.toml`, by a
definition's `operation` or `name`, so a new language needs that file and no code. A sink can depend on the literal a
call passes to a parameter it names. A frontend whose language passes arguments by name says which parameter each binds to
with a call reference's `arguments`, as rstyle-py does for Python's keyword arguments, and each is then numbered by that
parameter. Without them, an argument is numbered by where it's written and names none. A sink whose command is built a
call at a time, as Rust's `Command` is, names the calls that add its arguments under `appends`.

`--filter defects` runs the rules in [Defects](defects.md) over each function's facts. Its results don't change the
exit code yet.
