# Merge risk

`rstyle risk` compares two saved runs of a codebase and says what changed, what the changed code newly does, and what
reaches it. It reads only the runs' documents, so it needs no compiler or frontend, and every language gets it.

```text
rstyle go --out base
git checkout my-branch
rstyle go --out head
rstyle risk --base base --head head [--format json] [--check]
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

That's go-ethereum over 10 commits, trimmed, with `pdg_change = "medium"` and a `high` rule for `go::(*os.File).Truncate`.
Without `--check` it exits 0 whatever the rating is. It's a report, and doesn't authorize or perform a merge.

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
no entry the `unknown` effect. Rust's only lists standard methods that have no effect whatever type they're called with,
such as `slice::iter`, so a trait method like `core::clone::Clone::clone`, whose implementation could do anything, a
free function like `std::cmp::max`, and any other crate's function count until [`side_effects.paths`](style-checks.md)
or a rule here names them. A `[[risk.calls]]` rule for that callee, at any level, is you saying what it is. A comment
edited inside a function also reads as `text only`, since core has no tokens to read around one.

`--format json` keeps every reason, with `risk`, `policy_configured` and `schema_version`. The text gives what can't be seen
by kind, and each function's lines say which.

## Gating a merge

`--check` exits 1 unless the change rates `low`, so CI can hold a merge for a person. `medium`, `high`, `unknown` and
`unrated` all exit 1, and the report is still written. That's gstyle's gate. There's no flag for another threshold: a
project that would let a medium change through says so in its policy, such as with `pdg_change = "low"`, which the base
run decides and a change can't relax.

| Code | Meaning |
| --- | --- |
| `0` | The change rates `low`, or `--check` wasn't given |
| `1` | With `--check`, the change rates anything but `low` |
| `2` | No report: a run can't be read, the runs are of different languages, or the policy is invalid |

```text
git checkout "$BASE" && rstyle go --out "$RUNNER_TEMP/base"
git checkout "$HEAD" && rstyle go --out "$RUNNER_TEMP/head"
rstyle risk --base "$RUNNER_TEMP/base" --head "$RUNNER_TEMP/head" --check
```

### Changed files nothing here can see into

A run's documents only hold the files its frontend read, so a changed script, workflow, build file, lockfile or config
would otherwise read as no change at all. So each run records the commit it read in its `run.json`, and `rstyle risk` asks
git, from the current directory, for every path that differs between the two commits. Each one is:

- **compared**, when either run's documents hold it, since a function change or `changed outside any function` already
  shows it
- **excluded**, when the base's `exclude` matches it. It's listed in `changed_files_excluded`, and isn't rated, as with a
  function in it
- **documentation**, when it's the repository's `README.md` or Markdown under its `docs/`, but not in a dotted folder or a
  file an agent reads, such as `AGENTS.md` or anything under `skills/`
- otherwise **unread**, and an `analysis_unavailable` reason makes the change `unknown`

A symlink, a submodule or an executable file is unread even where a document holds its path, since its text isn't all of
what it is. A changed test file is unread too: a run that doesn't read tests, as Rust's doesn't by default, holds none, and
a test file a frontend marks, such as Go's `_test.go`, isn't rated. gstyle lets a new file through once it has parsed it
as nothing but plain Go tests. A document can't say a file is that plain, so here a new test file needs a person too.

```text
Merge risk: unknown 👀
- changes outside what's compared: 2
- changed files nothing here can see into: 5
- new library calls nothing classifies: 55
...
crates/rstyle-core/src/suppress.rs: changed outside any function
crates/cargo-rstyle/tests/cpg/runner.rs: changed, and no document holds it
crates/rstyle-core/src/cpg/defects/ignored_result/tests.rs: changed, and no document holds it
crates/rstyle-core/src/cpg/defects/tests.rs: changed, and no document holds it
crates/rstyle-core/src/defects_report/tests.rs: changed, and no document holds it
rstyle.toml: changed, and no document holds it
```

That's rstyle's own `main`..#77, whose two changed pages under `docs/` aren't listed. JSON lists the files in
`changed_files_unread`, by path from the repository's root, with `special` for a test file, symlink, submodule or
executable.

### When the commits can't say

The change is `unknown`, `changed_files_unread` is `null`, and the report says why, when:

- a run doesn't record its commit, because an older rstyle saved it or its source isn't in a git checkout
- a run's checkout had changes to tracked files when it began or when it was saved
- a run read a file its commit doesn't hold, such as one not yet added
- the runs read different folders of their repository
- git can't compare the commits from the current directory, as in a shallow clone without the base

Run it from a clone holding both commits. Untracked files a run didn't read don't matter, so runs can be saved inside the
checkout.

## What it can't see

A change outside any function can change what a program does, and no fact here says how. A written file whose text changed
with no reported function in it is listed as **changed outside any function**, for a person to read. On go-ethereum, one
of these added a hard fork to a package-level `var`. A changed file no document holds is listed from the runs' commits, as
[above](#changed-files-nothing-here-can-see-into).

- A frontend may only list the files that hold functions, as C++'s does, so a header of only types and constants isn't
  held. The commits still list it, and a constant's new value shows at each function that uses it.
- A file with both a function change and another change is only reported for the function.
- What a macro expands to is compared by its graph, not its values, since those can hold anything a build knows, such as
  its own revision.
- Test functions are counted but not reported, where a frontend [marks its test files](defects.md). Generated functions
  are counted and not compared. `exclude` removes paths from both runs.

A run keeps the `rstyle.toml` it read. `--config` or the one the base run kept decides what's excluded and how a change is
rated, not whatever the checkout holds by the time they're compared, so a change can't relax what it's judged by.
