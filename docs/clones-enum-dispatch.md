# Local enum-dispatch advice

`cargo rstyle clones --extraction-profile hir-data-v3 --enum-dispatch` assesses
original captured functions independently of strict pair matching. It proposes an
advisory abstraction when a free function consists of a single tail-expression
exhaustive two-arm match on a shared local enum
calls distinct local methods on the corresponding tuple payloads. Both selected
methods must have the bounded `fn(&self) -> u32` signature. A method body may be
excluded from clone matching, for example because it contains a closure.

The consumer checks original owned source, enum variants, payload binders,
receivers and method declarations. Exact shared local trait-member origins
suppress proposals; an unrelated trait name does not. This evidence establishes
a repeated capability at the dispatch site. It does not establish body purity,
equivalence, safe trait extraction, lifetime or drop behavior.

The initial contract covers concrete nongeneric local types in the same original
compiler-selected root file and module, including its inline modules with checked
ownership. External module files are currently unavailable. Imported,
external or ambiguous owners and unsupported candidate syntax remain explicitly
unavailable. Guards, wildcards, extra arm work and macros cannot be discarded to
produce a suggestion. Expansion or hidden/nonlocal impl declarations anywhere in
the root source can make method origins unavailable. Competing inherent/trait
receivers outside the bounded selection rule are unsupported. Ordinary bodies without a candidate top-level match are
known nonfindings, even when their bodies are opaque to strict matching.

JSON reports retain `enum_dispatch` execution, coverage, expected/assessed and
evaluated/unavailable function counts, findings and individual reasons. Proposals
retain original byte spans for the caller, enum and selected method headers and
arms. Function keys join the report's unit contexts and source navigation. Full
and compact modes use the same consumer report. `--max-analysis-functions` leaves
remaining owners explicitly unevaluated. Incomplete scope exits 2; findings exit
1 only with `--fail-on-findings`. A finding can coexist with incomplete scope.
The original root file is parsed/indexed once per immutable source owner, with a
256 KiB source bound, conservative raw delimiter-depth bound of 256, 4,096
declarations/members and inline-module depth 32. Limit failures retain budget
execution status; input is never truncated to manufacture a proposal. Exact
function ranges are indexed once rather than rescanning every declaration per
query.

Explicit `--replay-run` reevaluates the original owned sources without compiling
or reading current source files. The original extraction profile must match.
Legacy suppression and review entries are not applied. This slice does not retire
legacy patterns or implement general trait/capability/generic proposal families.
