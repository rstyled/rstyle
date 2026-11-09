# Merge risk

`rstyle risk` compares two saved runs of a codebase and says what changed, what the changed code newly does, and what
reaches it. It reads only the runs' documents, so it needs no compiler or frontend, and every language gets it.

```text
rstyle go --out base
git checkout my-branch
rstyle go --out head
rstyle risk --base base --head head [--format json | --markdown [--source-url URL]]
```

```text
Merge risk: high ⚠️
- core/rawdb/freezer_meta.go:133: newMetadata: added call: go::(*os.File).Truncate

Code impact: partial 👀
Affected functions: 110.

core/rawdb/freezer_meta.go:133: newMetadata: changed, 1 callers, reached by 176
  signature changed
  makes a call whose target can't be known
  gains io: (*os.File).Sync
  gains io: (*os.File).Truncate
  calls more: go::(*os.File).Sync
  calls more: go::(*os.File).Truncate
  calls more: go::(io/fs.FileInfo).Size
beacon/params/networks.go: changed outside any function
Changes: 60 changed, 44 added, 4 removed, 2 text only, 0 unknown; 10604 unchanged. Files changed outside any function: 3. Test functions changed: 34. Generated functions not compared: 601, 0 of them changed.
```

That's go-ethereum over 10 commits, trimmed, with `pdg_change = "medium"` and a `high` rule for `go::(*os.File).Truncate`. It
exits 0 whatever the rating is. It's a report, and doesn't authorize or perform a merge.

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

## How it rates a change

What changed is measured, and how much that matters is yours to say. With no rules, a change is **unrated**, and the report
says so. Severities go in `rstyle.toml`, and only `rstyle risk` reads them:

```toml
[risk]
pdg_change = "medium"             # low, medium or high: what a function depends on or does changed, or it's new or gone
pdg_low_risk_max_functions = 3    # lowers a medium pdg_change to low when at most this many functions changed

[[risk.calls]]                    # calling exactly this callee more often than before
callee = "go::os.WriteFile"       # its identity, as the report prints it under `calls more`
level = "high"
```

Nothing here has a default, so nothing is low because of a guess. A mistyped level, a rule with no callee, two rules for one
callee, or a limit with no `pdg_change` are errors. The limit is a policy on size, not a proof that behavior is preserved,
and it only lowers `pdg_change`.

The rating is the strongest of its reasons, in this order: `high`, `unknown`, `unrated`, `medium`, `low`. So a measured high
stands whatever else is unclear, and what can't be seen can't be rated away. These are always **unknown**, under any policy
that has rules:

| Reason | Meaning |
| --- | --- |
| `pdg_change`, on a `text only` or `unknown` function | Its text changed and the compared facts don't say how |
| `unresolved_calls` | A changed function, in either run, makes a call whose target can't be known, so call rules can't speak for it |
| `unclassified_call` | It newly calls a library function that nothing says the effect of |
| `analysis_unavailable` | A file changed outside any function, or a function a macro or a build writes changed |

`unclassified_call` is why a new library call can't pass as a clean one. Go's and C++'s effect tables give a call with
no entry the `unknown` effect. Rust's only lists standard methods and functions that have no effect whatever type they're
called with, such as `slice::iter` and `std::mem::swap`. A trait method's call is read by the implementation it selects, so
`?` on a `Result` is classified, and `core::clone::Clone::clone` is when it runs `String`'s, but not when it runs `Option`'s,
which runs the element's `clone`. That call, a function like `std::cmp::max`, which runs the caller's `Ord`, and any other
crate's function count until [`side_effects.paths`](style-checks.md) or a rule here names them. The report names the
method the call names, as the dependence graph does. A `[[risk.calls]]` rule for that
callee, at any level, is you saying what it is. A comment edited inside a function also reads as `text only`, since core
has no tokens to read around one.

`--format json` keeps every reason, with `risk`, `policy_configured` and `schema_version`. The text gives what can't be seen
by kind, and each function's lines say which.

`--markdown` writes the report for a pull request's description. Under the rating, each check above low says what it
found, and the callees it names. Each check is one of the rules above, so the rating is the worst check. Two folds follow:
**At a glance**, a table of impact, signatures, calls, effects and coverage, and every changed function in a table per
file, with what changed in it and how many functions use it, directly and through others. Library calls nothing
classifies are counted by callee, the most first, and files changed outside any function are listed last.

`--source-url` is what each file's path is joined to for a link, such as `https://github.com/OWNER/REPO/blob/HEAD_SHA`;
a removed function isn't linked, since the head doesn't hold it. Callees drop the directories of their import path, so
`go::github.com/o/r/native.load` reads as `go::native.load`. The text and JSON keep the full identity a `[[risk.calls]]`
rule names.

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

A run keeps the `rstyle.toml` it read. `--config` or the one the base run kept decides what's excluded and how a change is
rated, not whatever the checkout holds by the time they're compared, so a change can't relax what it's judged by.
