# Merge risk

`rstyle risk` compares two saved runs of a codebase and says what changed, what the changed code newly does, and what
reaches it. It reads only the runs' documents, so it needs no compiler or frontend, and every language gets it.

```text
rstyle go --out base
git checkout my-branch
rstyle go --out head
rstyle risk --base base --head head [--format json]
```

```text
core/rawdb/freezer_meta.go:133: newMetadata: changed, 1 callers, reached by 176
  signature changed
  gains io: (*os.File).Sync
  gains io: (*os.File).Truncate
  calls more: go::(*os.File).Sync
  calls more: go::(*os.File).Truncate
beacon/params/networks.go: changed outside any function
Changes: 60 changed, 44 added, 4 removed, 2 text only, 0 unknown; 10604 unchanged. Files changed outside any function: 3. Test functions changed: 34. Generated functions not compared: 601.
```

It doesn't rate a change yet. It reports the evidence, and exits 0 whatever that is.

## What it compares

Functions correspond by a name that survives renumbering, so adding one Rust `impl` block doesn't read as every later method and its
callers changing. A function whose name or, in C++, whose parameters change has a new identity, and reads as one removed
and one added.

| Kind | Meaning |
| --- | --- |
| `changed` | Its dependence graph differs, lines aside, or a value known at a node it wrote does, as a constant's does at a use |
| `added`, `removed` | Only one run has it |
| `text only` | Its text differs and nothing compared does. That says those facts didn't change, not that behavior didn't |
| `unknown` | It differs, and its tree isn't all of it, such as when the compiler couldn't parse part of the file |

Each one also says:

- whether its **signature** changed, by what its types are and how they're written, so a parameter whose type moved to
  another library counts even when it reads the same
- which **calls** it makes more or less often, by the callee's identity
- which **direct effects** it gained or lost, using the same classes as `side_effect_density`
- how many functions **call it**, and how many **reach it** through any chain of calls, tests aside

## What it can't see

A change outside any function can change what a program does, and no fact here says how. A written file whose text changed
with no reported function in it is listed as **changed outside any function**, for a person to read. On go-ethereum, one
of these added a hard fork to a package-level `var`.

- A frontend may only list the files that hold functions, as C++'s does, so a header of only types and constants isn't
  listed. A constant's new value still shows at each function that uses it.
- A file with both a function change and another change is only reported for the function.
- What a macro expands to is compared by its graph, not its values, since those can hold anything a build knows, such as
  its own revision.
- Test functions are counted but not reported, where a frontend [marks its test files](defects.md). Generated functions
  are counted and not compared. `exclude` removes paths from both runs.

A run keeps the `rstyle.toml` it read. `--config` or the one the base run kept decides what's excluded, not whatever the
checkout holds by the time they're compared, so a change can't relax what it's judged by.
