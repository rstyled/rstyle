# Caller-visible scalar mutation effects

Request the optional consumer separately from direct mutation budgets:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --locked --mutation-effects --format json --report-detail compact
```

`mutation_effects` (`rstyle.mutation-effects/1`) describes original direct writes
and calls to original same-selected-unit scalar leaves. It does not require strict
clone admission. The initial vocabulary covers ordinary nongeneric free functions
with owned `u32` or direct `&mut u32` inputs, `u32` or unit results, and primitive
scalar bodies. A callee containing any call cannot supply a leaf summary. Calls to
external functions, function pointers, recursive or nonleaf helpers, opaque
arguments, aliases, reference redirection, closures, unsafe operations and
nontrivial owned values remain incomplete.

The separate compiler `consumer_formal_types` observation records direct mutable
builtin `u32` formal types. Source spelling and rendered compiler type names do
not establish primitive referents. This observation supplies no graph admission,
alias, access or rewrite permission. Original source ownership, native binding
identity, signatures, call arguments and selected-unit context still have to agree.

Each function's evidence reports `caller_visible_mutation`:

| Value | Meaning |
| --- | --- |
| `none_proven` | Complete bounded analysis found no caller-visible scalar writes. |
| `writes_formal` | Complete bounded analysis found writes through mutable formals. |
| `witnessed_writes_formal_with_unknown_remainder` | Checked write witnesses survive incomplete evidence elsewhere. |
| `unknown` | Incomplete evidence with no checked caller-visible write witness. |

These values make no claim about purity, panic freedom, termination, general
effects, dynamic write frequency, density or safe extraction. `none_proven` is
limited to this mutation vocabulary. An unused unsupported formal still prevents
a complete result: dropping a value may execute behavior absent from explicit
assignments.

Direct assignment-site counts stay with their original owner. Callee-owned local
rebinding does not become a caller write. Every summarized original call has its
own event, while repeated calls reuse a single completed leaf summary in the
owning analysis. A mapping retains the callee assignment-site index and the exact
caller formal root. Root sets deduplicate only identical owner-scoped bindings.

Function `unit` and call `summary_index` select the report's summary row with the
same `unit` and `summary_index`. Summary `function_index` is the original owning
unit's function ordinal. `callee_site_index` indexes that summary's original
`sites`; site spans are relative to the callee's function source, while event spans
are relative to the caller. These are report-local handles, never stable review
identities or executable serialized proof authority.

`--max-analysis-functions` bounds inspected owners including required callees.
Additional bounded call, summary and witness work can also exhaust the analysis.
Omitted prerequisites remain incomplete. Requested incomplete effect scope exits
two; the consumer adds no mutation-budget findings. Direct `--mutation-rules`
configuration and metrics remain separate. Default runs do not request effects.

An explicit `--replay-run PATH --mutation-effects` reevaluates retained original
facts without source files or compiler tools. It recomputes summaries inside each
immutable owning unit; it never imports a serialized clean-effect verdict or
shares summaries across analyses or selected units.
